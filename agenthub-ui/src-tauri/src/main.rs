use agenthub_core::{
    Agent, AgentKind, AuditManager, AuditQuery, BackupManager, Catalog, CommunityManager,
    ConfigManager, ConfigValue, DiagnosticManager, Installer, MarketplaceManager, MemoryManager,
    MemoryScope, MemoryType, Monitor, Notifier, OverviewReport, Platform, PluginManager,
    PricingTable, PromptManager, RealCommandRunner, Result, SessionManager, SkillManager,
    StatusOverview, UserManager, WorkflowManager, WorkflowStep,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentInfo {
    id: String,
    name: String,
    description: String,
    kind: String,
    provider: String,
    homepage: String,
    status: String,
    installers: Vec<InstallerInfo>,
    catalog_verified_at: Option<String>,
    installer_verified_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallerInfo {
    platform: String,
    manager: String,
    package: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallResult {
    success: bool,
    message: String,
    agent_name: String,
    command: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u64,
    timed_out: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchResult {
    total: usize,
    success: usize,
    failed: usize,
    results: Vec<InstallResult>,
}

pub struct AppState {
    catalog: Arc<RwLock<Catalog>>,
    platform: Platform,
    config_manager: Arc<ConfigManager>,
    skill_manager: Arc<SkillManager>,
    prompt_manager: Arc<PromptManager>,
    session_manager: Arc<SessionManager>,
    memory_manager: Arc<MemoryManager>,
    audit_manager: Arc<AuditManager>,
    backup_manager: Arc<BackupManager>,
    overview_report: Arc<OverviewReport>,
    monitor: Arc<Monitor>,
    pricing_table: Arc<PricingTable>,
    /// Per-agent cancellation flags for in-flight install/uninstall operations.
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

fn agent_to_info(agent: &Agent) -> AgentInfo {
    let installers: Vec<InstallerInfo> = agent
        .installers
        .iter()
        .map(|(platform, config)| InstallerInfo {
            platform: format!("{:?}", platform),
            manager: format!("{:?}", config.manager),
            package: config.package.clone(),
        })
        .collect();

    AgentInfo {
        id: agent.id.clone(),
        name: agent.name.clone(),
        description: agent.description.clone(),
        kind: format!("{:?}", agent.kind),
        provider: agent.provider.clone(),
        homepage: agent.homepage.clone(),
        status: format!("{:?}", agent.status),
        installers,
        catalog_verified_at: agent.catalog_verified_at.map(|d| d.to_string()),
        installer_verified_at: agent.installer_verified_at.map(|d| d.to_string()),
    }
}

fn get_current_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Linux
    }
}

fn load_catalog() -> Result<Catalog> {
    // Try embedded catalog first
    const EMBEDDED_CATALOG: &str = include_str!("../../../agents.json");
    if let Ok(catalog) = Catalog::from_json(EMBEDDED_CATALOG) {
        return Ok(catalog);
    }

    // Fallback: try to find agents.json in the filesystem
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let catalog_path = current.join("agents.json");
        if catalog_path.exists() {
            return Catalog::from_file(&catalog_path);
        }
        if !current.pop() {
            break;
        }
    }

    Err(agenthub_core::AgentHubError::CatalogLoadError(
        "Could not find agents.json".to_string(),
    ))
}

#[tauri::command]
async fn list_agents(
    agent_type: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<AgentInfo>, String> {
    let catalog = state.catalog.read().await;
    let agents: Vec<AgentInfo> = match agent_type.as_deref() {
        Some("cli") => catalog
            .filter_by_kind(AgentKind::CLI)
            .iter()
            .map(|a| agent_to_info(a))
            .collect(),
        Some("desktop") => catalog
            .filter_by_kind(AgentKind::Desktop)
            .iter()
            .map(|a| agent_to_info(a))
            .collect(),
        _ => catalog.agents().iter().map(agent_to_info).collect(),
    };
    Ok(agents)
}

#[tauri::command]
async fn search_agents(
    query: String,
    agent_type: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<AgentInfo>, String> {
    let catalog = state.catalog.read().await;
    let results = catalog.search(&query);

    let filtered: Vec<AgentInfo> = match agent_type.as_deref() {
        Some("cli") => results
            .into_iter()
            .filter(|a| a.kind == AgentKind::CLI)
            .map(agent_to_info)
            .collect(),
        Some("desktop") => results
            .into_iter()
            .filter(|a| a.kind == AgentKind::Desktop)
            .map(agent_to_info)
            .collect(),
        _ => results.iter().map(|a| agent_to_info(a)).collect(),
    };

    Ok(filtered)
}

#[tauri::command]
async fn install_agent(
    name: String,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<InstallResult, String> {
    let catalog = state.catalog.read().await;
    let agent = catalog.find_by_name(&name).cloned();

    match agent {
        Some(agent) => {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            state
                .cancellations
                .lock()
                .await
                .insert(agent.name.clone(), cancel_flag.clone());

            let _ = app.emit(
                "install-progress",
                serde_json::json!({
                    "name": agent.name,
                    "step": 1,
                    "total_steps": 3,
                    "message": "Preparing installation..."
                }),
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let _ = app.emit(
                "install-progress",
                serde_json::json!({
                    "name": agent.name,
                    "step": 2,
                    "total_steps": 3,
                    "message": "Downloading and installing..."
                }),
            );

            let platform = state.platform;
            let agent_clone = agent.clone();
            let agent_name = agent.name.clone();
            let cancel_inner = cancel_flag.clone();

            let result = tokio::task::spawn_blocking(move || {
                let installer =
                    Installer::new(platform, Box::new(RealCommandRunner::new(platform)));
                installer.execute_install_cancellable(&agent_clone, false, None, Some(cancel_inner))
            })
            .await;

            state.cancellations.lock().await.remove(&agent_name);

            if cancel_flag.load(Ordering::Relaxed) {
                let _ = app.emit(
                    "operation-cancelled",
                    serde_json::json!({ "name": agent_name.clone() }),
                );
                return Ok(InstallResult {
                    success: false,
                    message: "Operation cancelled".to_string(),
                    agent_name,
                    command: String::new(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                    timed_out: false,
                });
            }

            let result = result.map_err(|e| format!("Task failed: {}", e))?;

            match result {
                Ok(result) => {
                    let _ = app.emit(
                        "install-progress",
                        serde_json::json!({
                            "name": agent_name,
                            "step": 3,
                            "total_steps": 3,
                            "message": "Completed"
                        }),
                    );
                    let _ = state.audit_manager.record(
                        "gui",
                        "install",
                        &agent.name,
                        Some(&format!(
                            "success={} cmd={}",
                            result.success, result.command
                        )),
                        result.success,
                    );
                    Ok(InstallResult {
                        success: result.success,
                        message: result.message,
                        agent_name,
                        command: result.command,
                        exit_code: result.exit_code,
                        stdout: result.stdout,
                        stderr: result.stderr,
                        duration_ms: result.duration_ms,
                        timed_out: result.timed_out,
                    })
                }
                Err(e) => {
                    let _ = app.emit(
                        "install-progress",
                        serde_json::json!({
                            "name": agent_name,
                            "step": 3,
                            "total_steps": 3,
                            "message": format!("Failed: {}", e)
                        }),
                    );
                    let _ = state.audit_manager.record(
                        "gui",
                        "install",
                        &agent.name,
                        Some(&format!("error={}", e)),
                        false,
                    );
                    Ok(InstallResult {
                        success: false,
                        message: e.to_string(),
                        agent_name,
                        command: String::new(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        timed_out: false,
                    })
                }
            }
        }
        None => Err(format!("Agent '{}' not found", name)),
    }
}

#[tauri::command]
async fn uninstall_agent(
    name: String,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<InstallResult, String> {
    let catalog = state.catalog.read().await;
    let agent = catalog.find_by_name(&name).cloned();

    match agent {
        Some(agent) => {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            state
                .cancellations
                .lock()
                .await
                .insert(agent.name.clone(), cancel_flag.clone());

            let _ = app.emit(
                "uninstall-progress",
                serde_json::json!({
                    "name": agent.name,
                    "step": 1,
                    "total_steps": 3,
                    "message": "Preparing uninstallation..."
                }),
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let _ = app.emit(
                "uninstall-progress",
                serde_json::json!({
                    "name": agent.name,
                    "step": 2,
                    "total_steps": 3,
                    "message": "Removing package..."
                }),
            );

            let platform = state.platform;
            let agent_clone = agent.clone();
            let agent_name = agent.name.clone();
            let cancel_inner = cancel_flag.clone();

            let result = tokio::task::spawn_blocking(move || {
                let installer =
                    Installer::new(platform, Box::new(RealCommandRunner::new(platform)));
                installer.execute_uninstall_cancellable(
                    &agent_clone,
                    false,
                    None,
                    Some(cancel_inner),
                )
            })
            .await;

            state.cancellations.lock().await.remove(&agent_name);

            if cancel_flag.load(Ordering::Relaxed) {
                let _ = app.emit(
                    "operation-cancelled",
                    serde_json::json!({ "name": agent_name.clone() }),
                );
                return Ok(InstallResult {
                    success: false,
                    message: "Operation cancelled".to_string(),
                    agent_name,
                    command: String::new(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                    timed_out: false,
                });
            }

            let result = result.map_err(|e| format!("Task failed: {}", e))?;

            match result {
                Ok(result) => {
                    let _ = app.emit(
                        "uninstall-progress",
                        serde_json::json!({
                            "name": agent_name,
                            "step": 3,
                            "total_steps": 3,
                            "message": "Completed"
                        }),
                    );
                    let _ = state.audit_manager.record(
                        "gui",
                        "uninstall",
                        &agent.name,
                        Some(&format!(
                            "success={} cmd={}",
                            result.success, result.command
                        )),
                        result.success,
                    );
                    Ok(InstallResult {
                        success: result.success,
                        message: result.message,
                        agent_name,
                        command: result.command,
                        exit_code: result.exit_code,
                        stdout: result.stdout,
                        stderr: result.stderr,
                        duration_ms: result.duration_ms,
                        timed_out: result.timed_out,
                    })
                }
                Err(e) => {
                    let _ = app.emit(
                        "uninstall-progress",
                        serde_json::json!({
                            "name": agent_name,
                            "step": 3,
                            "total_steps": 3,
                            "message": format!("Failed: {}", e)
                        }),
                    );
                    let _ = state.audit_manager.record(
                        "gui",
                        "uninstall",
                        &agent.name,
                        Some(&format!("error={}", e)),
                        false,
                    );
                    Ok(InstallResult {
                        success: false,
                        message: e.to_string(),
                        agent_name,
                        command: String::new(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        timed_out: false,
                    })
                }
            }
        }
        None => Err(format!("Agent '{}' not found", name)),
    }
}

#[tauri::command]
async fn batch_install_agents(
    names: Vec<String>,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<BatchResult, String> {
    let catalog = state.catalog.read().await;
    let platform = state.platform;
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    for (index, name) in names.iter().enumerate() {
        let agent = catalog.find_by_name(name).cloned();

        match agent {
            Some(agent) => {
                let _ = app.emit(
                    "batch-progress",
                    serde_json::json!({
                        "current": index + 1,
                        "total": names.len(),
                        "agent": agent.name,
                        "action": "install"
                    }),
                );

                let cancel_flag = Arc::new(AtomicBool::new(false));
                state
                    .cancellations
                    .lock()
                    .await
                    .insert(agent.name.clone(), cancel_flag.clone());

                let agent_clone = agent.clone();
                let agent_name = agent.name.clone();
                let cancel_inner = cancel_flag.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let installer =
                        Installer::new(platform, Box::new(RealCommandRunner::new(platform)));
                    installer.execute_install_cancellable(
                        &agent_clone,
                        false,
                        None,
                        Some(cancel_inner),
                    )
                })
                .await;

                state.cancellations.lock().await.remove(&agent_name);

                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = app.emit(
                        "operation-cancelled",
                        serde_json::json!({ "name": agent_name.clone() }),
                    );
                    results.push(InstallResult {
                        success: false,
                        message: "Operation cancelled".to_string(),
                        agent_name: agent_name.clone(),
                        command: String::new(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        timed_out: false,
                    });
                    fail_count += 1;
                    break;
                }

                let result = result.map_err(|e| format!("Task failed: {}", e))?;

                match result {
                    Ok(result) => {
                        results.push(InstallResult {
                            success: result.success,
                            message: result.message,
                            agent_name: agent_name.clone(),
                            command: result.command,
                            exit_code: result.exit_code,
                            stdout: result.stdout,
                            stderr: result.stderr,
                            duration_ms: result.duration_ms,
                            timed_out: result.timed_out,
                        });
                        if result.success {
                            success_count += 1;
                        } else {
                            fail_count += 1;
                        }
                    }
                    Err(e) => {
                        results.push(InstallResult {
                            success: false,
                            message: e.to_string(),
                            agent_name: agent_name.clone(),
                            command: String::new(),
                            exit_code: None,
                            stdout: String::new(),
                            stderr: String::new(),
                            duration_ms: 0,
                            timed_out: false,
                        });
                        fail_count += 1;
                    }
                }
            }
            None => {
                results.push(InstallResult {
                    success: false,
                    message: "Agent not found".to_string(),
                    agent_name: name.clone(),
                    command: String::new(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                    timed_out: false,
                });
                fail_count += 1;
            }
        }
    }

    Ok(BatchResult {
        total: names.len(),
        success: success_count,
        failed: fail_count,
        results,
    })
}

#[tauri::command]
async fn batch_uninstall_agents(
    names: Vec<String>,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<BatchResult, String> {
    let catalog = state.catalog.read().await;
    let platform = state.platform;
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    for (index, name) in names.iter().enumerate() {
        let agent = catalog.find_by_name(name).cloned();

        match agent {
            Some(agent) => {
                let _ = app.emit(
                    "batch-progress",
                    serde_json::json!({
                        "current": index + 1,
                        "total": names.len(),
                        "agent": agent.name,
                        "action": "uninstall"
                    }),
                );

                let cancel_flag = Arc::new(AtomicBool::new(false));
                state
                    .cancellations
                    .lock()
                    .await
                    .insert(agent.name.clone(), cancel_flag.clone());

                let agent_clone = agent.clone();
                let agent_name = agent.name.clone();
                let cancel_inner = cancel_flag.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let installer =
                        Installer::new(platform, Box::new(RealCommandRunner::new(platform)));
                    installer.execute_uninstall_cancellable(
                        &agent_clone,
                        false,
                        None,
                        Some(cancel_inner),
                    )
                })
                .await;

                state.cancellations.lock().await.remove(&agent_name);

                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = app.emit(
                        "operation-cancelled",
                        serde_json::json!({ "name": agent_name.clone() }),
                    );
                    results.push(InstallResult {
                        success: false,
                        message: "Operation cancelled".to_string(),
                        agent_name: agent_name.clone(),
                        command: String::new(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        timed_out: false,
                    });
                    fail_count += 1;
                    break;
                }

                let result = result.map_err(|e| format!("Task failed: {}", e))?;

                match result {
                    Ok(result) => {
                        results.push(InstallResult {
                            success: result.success,
                            message: result.message,
                            agent_name: agent_name.clone(),
                            command: result.command,
                            exit_code: result.exit_code,
                            stdout: result.stdout,
                            stderr: result.stderr,
                            duration_ms: result.duration_ms,
                            timed_out: result.timed_out,
                        });
                        if result.success {
                            success_count += 1;
                        } else {
                            fail_count += 1;
                        }
                    }
                    Err(e) => {
                        results.push(InstallResult {
                            success: false,
                            message: e.to_string(),
                            agent_name: agent_name.clone(),
                            command: String::new(),
                            exit_code: None,
                            stdout: String::new(),
                            stderr: String::new(),
                            duration_ms: 0,
                            timed_out: false,
                        });
                        fail_count += 1;
                    }
                }
            }
            None => {
                results.push(InstallResult {
                    success: false,
                    message: "Agent not found".to_string(),
                    agent_name: name.clone(),
                    command: String::new(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                    timed_out: false,
                });
                fail_count += 1;
            }
        }
    }

    Ok(BatchResult {
        total: names.len(),
        success: success_count,
        failed: fail_count,
        results,
    })
}

// ============ Config Commands ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ConfigInfo {
    agent_id: String,
    environment: String,
    settings: HashMap<String, String>,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct NativeConfig {
    agent_id: String,
    config_path: String,
    config_content: String,
    config_format: String,
    parsed: Option<serde_json::Value>,
}

/// Cancel an in-flight install/uninstall operation for the given agent.
/// Returns true if a cancellable operation was found and flagged.
#[tauri::command]
async fn cancel_operation(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    let flags = state.cancellations.lock().await;
    match flags.get(&name) {
        Some(flag) => {
            flag.store(true, Ordering::Relaxed);
            Ok(true)
        }
        None => Ok(false),
    }
}

#[tauri::command]
async fn list_configs(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<String>, String> {
    state
        .config_manager
        .list_configs()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config(
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<ConfigInfo, String> {
    let config = state
        .config_manager
        .load_config(&agent_id)
        .map_err(|e| e.to_string())?;

    let settings: HashMap<String, String> = config
        .settings
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    Ok(ConfigInfo {
        agent_id: config.agent_id,
        environment: config.environment.to_string(),
        settings,
        updated_at: config.metadata.updated_at.to_rfc3339(),
    })
}

#[tauri::command]
async fn get_native_config(agent_id: String) -> std::result::Result<NativeConfig, String> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let app_data = dirs::config_dir().ok_or("Could not find app data directory")?;

    // Known config file mappings (dir, file)
    // For home dir: (".codex", "config.toml")
    // For app data: ("ai.opencode.desktop", "opencode.settings")
    let config_files: HashMap<&str, (&str, &str, bool)> = HashMap::from([
        ("codex", (".codex", "config.toml", false)),
        ("codex-desktop", (".codex", "config.toml", false)),
        ("claude-code", (".claude", "settings.json", false)),
        ("claude-desktop", (".claude", "settings.json", false)),
        ("cursor", (".cursor", "argv.json", false)),
        ("windsurf", (".windsurf", "settings.json", false)),
        ("kimi-code", (".kimi", "config.toml", false)),
        ("kimi-desktop", (".kimi", "config.toml", false)),
        ("qwen-code", (".qwen", "settings.json", false)),
        ("mimo-code", (".local/share/mimocode", "auth.json", false)),
        ("reasonix", (".reasonix", "config.json", false)),
        ("reasonix-cli", (".reasonix", "config.json", false)),
        ("trae", (".trae", "argv.json", false)),
        ("trae-solo", (".trae", "argv.json", false)),
        ("antigravity", (".antigravity", "argv.json", false)),
        ("antigravity-ide", (".antigravity-ide", "argv.json", false)),
        ("qoder", (".qoder", "argv.json", false)),
        ("qoder-work", (".qoder", "argv.json", false)),
        ("minimax-agent", (".minimax-agent", "config.json", false)),
        ("zcode", (".zcode", "config.json", false)),
        ("workbuddy", (".workbuddy", ".mcp.json", false)),
        ("codebuddy", (".codebuddy", "config.json", false)),
        ("openwork", (".openwork", "config.json", false)),
        (
            "opencode",
            ("ai.opencode.desktop", "opencode.settings", true),
        ),
        ("grok-cli", (".grok", "auth.json", false)),
    ]);

    let (dir_name, file_name, use_app_data) = config_files
        .get(agent_id.as_str())
        .ok_or_else(|| format!("No known config for agent: {}", agent_id))?;

    let base_dir = if *use_app_data { &app_data } else { &home_dir };
    let config_path = base_dir.join(dir_name).join(file_name);

    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let format = if file_name.ends_with(".toml") {
        "toml"
    } else if file_name.ends_with(".json") {
        "json"
    } else if file_name.ends_with(".yaml") || file_name.ends_with(".yml") {
        "yaml"
    } else {
        "text"
    };

    // Parse the content based on format
    let parsed = match format {
        "json" => serde_json::from_str::<serde_json::Value>(&content).ok(),
        "toml" => {
            // Parse TOML to JSON Value
            match toml::from_str::<toml::Value>(&content) {
                Ok(toml_val) => {
                    // Convert TOML Value to JSON Value
                    let json_str = serde_json::to_string(&toml_val).unwrap_or_default();
                    serde_json::from_str(&json_str).ok()
                }
                Err(_) => None,
            }
        }
        _ => None,
    };

    Ok(NativeConfig {
        agent_id,
        config_path: config_path.to_string_lossy().to_string(),
        config_content: content,
        config_format: format.to_string(),
        parsed,
    })
}

#[tauri::command]
async fn save_native_config(agent_id: String, content: String) -> std::result::Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let app_data = dirs::config_dir().ok_or("Could not find app data directory")?;

    let config_files: HashMap<&str, (&str, &str, bool)> = HashMap::from([
        ("codex", (".codex", "config.toml", false)),
        ("codex-desktop", (".codex", "config.toml", false)),
        ("claude-code", (".claude", "settings.json", false)),
        ("claude-desktop", (".claude", "settings.json", false)),
        ("cursor", (".cursor", "argv.json", false)),
        ("windsurf", (".windsurf", "settings.json", false)),
        ("kimi-code", (".kimi", "config.toml", false)),
        ("kimi-desktop", (".kimi", "config.toml", false)),
        ("qwen-code", (".qwen", "settings.json", false)),
        ("mimo-code", (".local/share/mimocode", "auth.json", false)),
        ("reasonix", (".reasonix", "config.json", false)),
        ("reasonix-cli", (".reasonix", "config.json", false)),
        ("trae", (".trae", "argv.json", false)),
        ("trae-solo", (".trae", "argv.json", false)),
        ("antigravity", (".antigravity", "argv.json", false)),
        ("antigravity-ide", (".antigravity-ide", "argv.json", false)),
        ("qoder", (".qoder", "argv.json", false)),
        ("qoder-work", (".qoder", "argv.json", false)),
        ("minimax-agent", (".minimax-agent", "config.json", false)),
        ("zcode", (".zcode", "config.json", false)),
        ("workbuddy", (".workbuddy", ".mcp.json", false)),
        ("codebuddy", (".codebuddy", "config.json", false)),
        ("openwork", (".openwork", "config.json", false)),
        (
            "opencode",
            ("ai.opencode.desktop", "opencode.settings", true),
        ),
        ("grok-cli", (".grok", "auth.json", false)),
    ]);

    let (dir_name, file_name, use_app_data) = config_files
        .get(agent_id.as_str())
        .ok_or_else(|| format!("No known config for agent: {}", agent_id))?;

    let base_dir = if *use_app_data { &app_data } else { &home_dir };
    let config_path = base_dir.join(dir_name).join(file_name);

    std::fs::write(&config_path, &content).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InstalledAgent {
    id: String,
    name: String,
    installed: bool,
    version: Option<String>,
}

#[tauri::command]
async fn list_installed_agents(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<InstalledAgent>, String> {
    let catalog = state.catalog.read().await;
    let platform = state.platform;
    let agents: Vec<Agent> = catalog.agents().to_vec();

    // Run batch check in blocking thread
    let results = tokio::task::spawn_blocking(move || {
        let detector = agenthub_core::StatusDetector::new(platform);
        detector.check_agents(&agents)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?;

    let installed_agents: Vec<InstalledAgent> = results
        .into_iter()
        .map(|status| InstalledAgent {
            id: status.agent_id,
            name: String::new(), // Will be filled from catalog
            installed: status.installed,
            version: status.version,
        })
        .collect();

    // Fill in names from catalog
    let catalog = state.catalog.read().await;
    let mut result = Vec::new();
    for mut agent in installed_agents {
        if let Some(catalog_agent) = catalog.agents().iter().find(|a| a.id == agent.id) {
            agent.name = catalog_agent.name.clone();
        }
        result.push(agent);
    }

    Ok(result)
}

#[tauri::command]
async fn set_config_value(
    agent_id: String,
    key: String,
    value: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .config_manager
        .set_setting(&agent_id, &key, ConfigValue::String(value))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn validate_agent_config(
    agent_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::ConfigIssue>, String> {
    let manager = &state.config_manager;
    let mut all = Vec::new();
    match agent_id {
        Some(id) => {
            let config = manager.load_config(&id).map_err(|e| e.to_string())?;
            all.extend(agenthub_core::validate_config(&config));
        }
        None => {
            for id in manager.list_configs().map_err(|e| e.to_string())? {
                if let Ok(config) = manager.load_config(&id) {
                    let issues = agenthub_core::validate_config(&config);
                    for mut issue in issues {
                        issue.key = format!("{id}.{}", issue.key);
                        all.push(issue);
                    }
                }
            }
        }
    }
    Ok(all)
}

#[tauri::command]
async fn repair_agent_config(
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::ConfigIssue>, String> {
    state
        .config_manager
        .repair_config(&agent_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_config_history(
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::AgentConfig>, String> {
    state
        .config_manager
        .list_history(&agent_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rollback_agent_config(
    agent_id: String,
    version: u32,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::AgentConfig, String> {
    state
        .config_manager
        .rollback_config(&agent_id, version)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_config(
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .config_manager
        .delete_config(&agent_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_config(
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .config_manager
        .create_config(&agent_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============ Skill Commands ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SkillInfo {
    name: String,
    description: String,
    version: String,
    enabled: bool,
    tags: Vec<String>,
    category: Option<String>,
    source: String,
}

#[tauri::command]
async fn list_skills(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<SkillInfo>, String> {
    let skills = state
        .skill_manager
        .list_skills()
        .map_err(|e| e.to_string())?;

    // Get the codex skills directory for comparison
    let codex_skills_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("skills");

    Ok(skills
        .iter()
        .map(|s| {
            let source = if s.skill_dir.starts_with(&codex_skills_dir) {
                "codex".to_string()
            } else {
                "local".to_string()
            };
            SkillInfo {
                name: s.manifest.name.clone(),
                description: s.manifest.description.clone(),
                version: s.manifest.version.clone(),
                enabled: s.enabled,
                tags: s.manifest.tags.clone(),
                category: s.manifest.category.clone(),
                source,
            }
        })
        .collect())
}

#[tauri::command]
async fn create_skill(
    name: String,
    description: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SkillInfo, String> {
    let skill = state
        .skill_manager
        .create_skill(&name, &description)
        .map_err(|e| e.to_string())?;
    Ok(SkillInfo {
        name: skill.manifest.name,
        description: skill.manifest.description,
        version: skill.manifest.version,
        enabled: skill.enabled,
        tags: skill.manifest.tags,
        category: skill.manifest.category,
        source: "local".to_string(),
    })
}

#[tauri::command]
async fn enable_skill(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .skill_manager
        .enable_skill(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn disable_skill(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .skill_manager
        .disable_skill(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_skill(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .skill_manager
        .uninstall_skill(&name)
        .map_err(|e| e.to_string())
}

// ============ Diagnostic Commands ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DiagnosticResult {
    summary: String,
    checks: Vec<CheckResult>,
    passed: usize,
    warnings: usize,
    failed: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CheckResult {
    name: String,
    category: String,
    status: String,
    message: String,
}

#[tauri::command]
async fn run_diagnostics() -> std::result::Result<DiagnosticResult, String> {
    let mut manager = DiagnosticManager::new();
    let report = manager.run_all_checks();

    let checks: Vec<CheckResult> = report
        .checks
        .iter()
        .map(|c| CheckResult {
            name: c.name.clone(),
            category: c.category.clone(),
            status: format!("{:?}", c.status),
            message: c.message.clone(),
        })
        .collect();

    Ok(DiagnosticResult {
        summary: DiagnosticManager::format_report(&report),
        checks,
        passed: report.summary.passed,
        warnings: report.summary.warnings,
        failed: report.summary.failed,
    })
}

// ============ Prompt Commands ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptInfo {
    id: String,
    name: String,
    description: String,
    template: String,
    tags: Vec<String>,
    category: Option<String>,
    version: u32,
}

#[tauri::command]
async fn list_prompts(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<PromptInfo>, String> {
    let prompts = state
        .prompt_manager
        .list_prompts()
        .map_err(|e| e.to_string())?;
    Ok(prompts
        .iter()
        .map(|p| PromptInfo {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            template: p.template.clone(),
            tags: p.tags.clone(),
            category: p.category.clone(),
            version: p.version,
        })
        .collect())
}

#[tauri::command]
async fn create_prompt(
    id: String,
    name: String,
    description: String,
    template: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<PromptInfo, String> {
    let prompt = state
        .prompt_manager
        .create_prompt(&id, &name, &description, &template)
        .map_err(|e| e.to_string())?;
    Ok(PromptInfo {
        id: prompt.id,
        name: prompt.name,
        description: prompt.description,
        template: prompt.template,
        tags: prompt.tags,
        category: prompt.category,
        version: prompt.version,
    })
}

#[tauri::command]
async fn render_prompt(
    id: String,
    vars: HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    state
        .prompt_manager
        .render_prompt(&id, &vars)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_prompt(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .prompt_manager
        .delete_prompt(&id)
        .map_err(|e| e.to_string())
}

// ============ Session Commands ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SessionInfo {
    id: String,
    title: String,
    agent: String,
    status: String,
    started_at: String,
    ended_at: Option<String>,
    message_count: usize,
    tags: Vec<String>,
}

#[tauri::command]
async fn list_sessions(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<SessionInfo>, String> {
    let sessions = state
        .session_manager
        .list_sessions()
        .map_err(|e| e.to_string())?;
    Ok(sessions
        .iter()
        .map(|s| SessionInfo {
            id: s.id.clone(),
            title: s.title.clone(),
            agent: s.agent.clone(),
            status: s.status.to_string(),
            started_at: s.started_at.to_rfc3339(),
            ended_at: s.ended_at.map(|dt| dt.to_rfc3339()),
            message_count: s.messages.len(),
            tags: s.tags.clone(),
        })
        .collect())
}

#[tauri::command]
async fn create_session(
    title: String,
    agent: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionInfo, String> {
    let session = state
        .session_manager
        .create_session(&title, &agent)
        .map_err(|e| e.to_string())?;
    Ok(SessionInfo {
        id: session.id,
        title: session.title,
        agent: session.agent,
        status: session.status.to_string(),
        started_at: session.started_at.to_rfc3339(),
        ended_at: None,
        message_count: 0,
        tags: session.tags,
    })
}

#[tauri::command]
async fn get_session(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionInfo, String> {
    let session = state
        .session_manager
        .get_session(&id)
        .map_err(|e| e.to_string())?;
    Ok(SessionInfo {
        id: session.id,
        title: session.title,
        agent: session.agent,
        status: session.status.to_string(),
        started_at: session.started_at.to_rfc3339(),
        ended_at: session.ended_at.map(|dt| dt.to_rfc3339()),
        message_count: session.messages.len(),
        tags: session.tags,
    })
}

#[tauri::command]
async fn delete_session(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .session_manager
        .delete_session(&id)
        .map_err(|e| e.to_string())
}

// ============ Memory Commands ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MemoryInfo {
    path: String,
    title: String,
    content: String,
    scope: String,
    memory_type: String,
    tags: Vec<String>,
    updated_at: String,
}

#[tauri::command]
async fn list_memories(
    scope: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<MemoryInfo>, String> {
    let scope_enum = scope.and_then(|s| match s.as_str() {
        "global" => Some(MemoryScope::Global),
        "project" => Some(MemoryScope::Project),
        "session" => Some(MemoryScope::Session),
        _ => None,
    });

    let entries = state
        .memory_manager
        .list_entries(scope_enum)
        .map_err(|e| e.to_string())?;
    Ok(entries
        .iter()
        .map(|e| MemoryInfo {
            path: e.path.clone(),
            title: e.title.clone(),
            content: e.content.clone(),
            scope: e.scope.to_string(),
            memory_type: e.memory_type.to_string(),
            tags: e.tags.clone(),
            updated_at: e.updated_at.to_rfc3339(),
        })
        .collect())
}

#[tauri::command]
async fn create_memory(
    title: String,
    content: String,
    scope: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<MemoryInfo, String> {
    let scope_enum = match scope.as_str() {
        "global" => MemoryScope::Global,
        "project" => MemoryScope::Project,
        "session" => MemoryScope::Session,
        _ => return Err("Invalid scope".to_string()),
    };

    let entry = state
        .memory_manager
        .create_entry(scope_enum, None, &title, &content, MemoryType::Free)
        .map_err(|e| e.to_string())?;

    Ok(MemoryInfo {
        path: entry.path,
        title: entry.title,
        content: entry.content,
        scope: entry.scope.to_string(),
        memory_type: entry.memory_type.to_string(),
        tags: entry.tags,
        updated_at: entry.updated_at.to_rfc3339(),
    })
}

#[tauri::command]
async fn search_memories(
    query: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<MemoryInfo>, String> {
    let entries = state
        .memory_manager
        .search_entries(&query)
        .map_err(|e| e.to_string())?;
    Ok(entries
        .iter()
        .map(|e| MemoryInfo {
            path: e.path.clone(),
            title: e.title.clone(),
            content: e.content.clone(),
            scope: e.scope.to_string(),
            memory_type: e.memory_type.to_string(),
            tags: e.tags.clone(),
            updated_at: e.updated_at.to_rfc3339(),
        })
        .collect())
}

#[tauri::command]
async fn delete_memory(
    path: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .memory_manager
        .delete_entry(&path)
        .map_err(|e| e.to_string())
}

// ============ Management Commands (dashboard / audit / backup) ============

#[tauri::command]
async fn get_status_overview(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<StatusOverview, String> {
    let catalog = state.catalog.read().await;
    state
        .overview_report
        .overview(&catalog)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuditInfo {
    id: String,
    timestamp: String,
    actor: String,
    action: String,
    target: String,
    details: Option<String>,
    success: bool,
}

#[tauri::command]
async fn list_audit(
    action: Option<String>,
    target: Option<String>,
    since_days: Option<i64>,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<AuditInfo>, String> {
    let since = since_days.map(|d| chrono::Utc::now() - chrono::Duration::days(d));
    let query = AuditQuery {
        action,
        target,
        since,
        limit,
        ..Default::default()
    };

    let events = state
        .audit_manager
        .query(&query)
        .map_err(|e| e.to_string())?;
    Ok(events
        .into_iter()
        .map(|e| AuditInfo {
            id: e.id,
            timestamp: e.timestamp.to_rfc3339(),
            actor: e.actor,
            action: e.action,
            target: e.target,
            details: e.details,
            success: e.success,
        })
        .collect())
}

#[tauri::command]
async fn clear_audit(state: tauri::State<'_, AppState>) -> std::result::Result<(), String> {
    state.audit_manager.clear().map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_backup(
    output_path: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::BackupManifest, String> {
    let manifest = state
        .backup_manager
        .create_backup(std::path::Path::new(&output_path))
        .map_err(|e| e.to_string())?;
    Ok(manifest)
}

#[tauri::command]
async fn restore_backup(
    input_path: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::BackupManifest, String> {
    let manifest = state
        .backup_manager
        .restore_backup(std::path::Path::new(&input_path))
        .map_err(|e| e.to_string())?;
    Ok(manifest)
}

// ============ Session: usage / replay / templates ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SessionDetail {
    id: String,
    title: String,
    agent: String,
    model: Option<String>,
    status: String,
    started_at: String,
    ended_at: Option<String>,
    message_count: usize,
    total_tokens: u32,
    estimated_cost_usd: f64,
    tags: Vec<String>,
}

fn session_to_detail(session: &agenthub_core::Session) -> SessionDetail {
    SessionDetail {
        id: session.id.clone(),
        title: session.title.clone(),
        agent: session.agent.clone(),
        model: session.model.clone(),
        status: session.status.to_string(),
        started_at: session.started_at.to_rfc3339(),
        ended_at: session.ended_at.map(|dt| dt.to_rfc3339()),
        message_count: session.messages.len(),
        total_tokens: session.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
        estimated_cost_usd: session
            .usage
            .as_ref()
            .map(|u| u.estimated_cost_usd)
            .unwrap_or(0.0),
        tags: session.tags.clone(),
    }
}

#[tauri::command]
async fn replay_session(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    state
        .session_manager
        .replay_session(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn record_session_usage(
    id: String,
    input_tokens: u32,
    output_tokens: u32,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionDetail, String> {
    state
        .session_manager
        .record_usage(&id, input_tokens, output_tokens, &state.pricing_table)
        .map_err(|e| e.to_string())?;
    let session = state
        .session_manager
        .get_session(&id)
        .map_err(|e| e.to_string())?;
    Ok(session_to_detail(&session))
}

#[tauri::command]
async fn set_session_model(
    id: String,
    model: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionDetail, String> {
    state
        .session_manager
        .set_model(&id, &model)
        .map_err(|e| e.to_string())?;
    let session = state
        .session_manager
        .get_session(&id)
        .map_err(|e| e.to_string())?;
    Ok(session_to_detail(&session))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SessionTemplateInfo {
    id: String,
    name: String,
    description: String,
    agent: Option<String>,
    message_count: usize,
    tags: Vec<String>,
}

#[tauri::command]
async fn list_session_templates(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<SessionTemplateInfo>, String> {
    let templates = state
        .session_manager
        .list_templates()
        .map_err(|e| e.to_string())?;
    Ok(templates
        .iter()
        .map(|t| SessionTemplateInfo {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            agent: t.agent.clone(),
            message_count: t.messages.len(),
            tags: t.tags.clone(),
        })
        .collect())
}

#[tauri::command]
async fn create_session_template(
    id: String,
    name: String,
    description: String,
    agent: Option<String>,
    messages: Vec<(String, String)>,
    tags: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionTemplateInfo, String> {
    let messages = messages
        .into_iter()
        .map(|(role, content)| agenthub_core::TemplateMessage { role, content })
        .collect();
    let template = state
        .session_manager
        .create_template(&id, &name, &description, agent.as_deref(), messages, tags)
        .map_err(|e| e.to_string())?;
    Ok(SessionTemplateInfo {
        id: template.id,
        name: template.name,
        description: template.description,
        agent: template.agent,
        message_count: template.messages.len(),
        tags: template.tags,
    })
}

#[tauri::command]
async fn create_session_from_template(
    template_id: String,
    title: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionDetail, String> {
    let session = state
        .session_manager
        .create_session_from_template(&template_id, &title)
        .map_err(|e| e.to_string())?;
    Ok(session_to_detail(&session))
}

#[tauri::command]
async fn delete_session_template(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .session_manager
        .delete_template(&id)
        .map_err(|e| e.to_string())
}

// ============ Prompt: versions / usage / checked render ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptVersionInfo {
    version: u32,
    name: String,
    description: String,
    template: String,
    updated_at: Option<String>,
}

fn prompt_version_to_info(p: &agenthub_core::PromptTemplate) -> PromptVersionInfo {
    PromptVersionInfo {
        version: p.version,
        name: p.name.clone(),
        description: p.description.clone(),
        template: p.template.clone(),
        updated_at: p.updated_at.map(|dt| dt.to_rfc3339()),
    }
}

#[tauri::command]
async fn list_prompt_versions(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<PromptVersionInfo>, String> {
    let versions = state
        .prompt_manager
        .list_versions(&id)
        .map_err(|e| e.to_string())?;
    Ok(versions.iter().map(prompt_version_to_info).collect())
}

#[tauri::command]
async fn rollback_prompt(
    id: String,
    version: u32,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<PromptInfo, String> {
    let prompt = state
        .prompt_manager
        .rollback(&id, version)
        .map_err(|e| e.to_string())?;
    Ok(PromptInfo {
        id: prompt.id,
        name: prompt.name,
        description: prompt.description,
        template: prompt.template,
        tags: prompt.tags,
        category: prompt.category,
        version: prompt.version,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptUsageInfo {
    id: String,
    name: String,
    usage_count: u64,
    last_used_at: Option<String>,
}

#[tauri::command]
async fn get_prompt_usage(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<PromptUsageInfo>, String> {
    let usage = state
        .prompt_manager
        .list_usage()
        .map_err(|e| e.to_string())?;
    Ok(usage
        .into_iter()
        .map(|u| PromptUsageInfo {
            id: u.id,
            name: u.name,
            usage_count: u.usage_count,
            last_used_at: u.last_used_at.map(|dt| dt.to_rfc3339()),
        })
        .collect())
}

#[tauri::command]
async fn render_prompt_checked(
    id: String,
    vars: HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    state
        .prompt_manager
        .render_prompt_checked(&id, &vars)
        .map_err(|e| e.to_string())
}

// ============ Memory: semantic search / decay ============

#[tauri::command]
async fn search_memories_semantic(
    query: String,
    top_k: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<MemoryInfo>, String> {
    let entries = state
        .memory_manager
        .search_entries_bm25(&query, top_k.unwrap_or(20))
        .map_err(|e| e.to_string())?;
    Ok(entries
        .iter()
        .map(|e| MemoryInfo {
            path: e.path.clone(),
            title: e.title.clone(),
            content: e.content.clone(),
            scope: e.scope.to_string(),
            memory_type: e.memory_type.to_string(),
            tags: e.tags.clone(),
            updated_at: e.updated_at.to_rfc3339(),
        })
        .collect())
}

#[tauri::command]
async fn apply_memory_decay(
    older_than_days: i64,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<usize, String> {
    state
        .memory_manager
        .apply_decay(older_than_days, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_memory_importance(
    path: String,
    importance: u8,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .memory_manager
        .set_importance(&path, importance)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn revive_memory(
    path: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .memory_manager
        .revive(&path)
        .map_err(|e| e.to_string())
}

// ============ Wave 2: templates / import-export / budget / trend / monitor ============

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ConfigTemplateInfo {
    id: String,
    name: String,
    description: String,
    setting_count: usize,
    env_var_count: usize,
    secret_keys: Vec<String>,
}

fn config_template_to_info(t: &agenthub_core::ConfigTemplate) -> ConfigTemplateInfo {
    ConfigTemplateInfo {
        id: t.id.clone(),
        name: t.name.clone(),
        description: t.description.clone(),
        setting_count: t.settings.len(),
        env_var_count: t.environment_variables.len(),
        secret_keys: t.secret_keys.clone(),
    }
}

#[tauri::command]
async fn list_config_templates(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<ConfigTemplateInfo>, String> {
    let manager = &state.config_manager;
    let ids = manager.list_templates().map_err(|e| e.to_string())?;
    let mut templates = Vec::new();
    for id in ids {
        if let Ok(t) = manager.get_template(&id) {
            templates.push(config_template_to_info(&t));
        }
    }
    Ok(templates)
}

#[tauri::command]
async fn create_config_template(
    id: String,
    name: String,
    description: String,
    sets: Vec<(String, String)>,
    envs: Vec<String>,
    secrets: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<ConfigTemplateInfo, String> {
    let settings = sets
        .into_iter()
        .map(|(k, v)| (k, ConfigValue::from(v)))
        .collect();
    let env_vars = envs
        .into_iter()
        .map(|k| (k.clone(), String::new()))
        .collect();
    let template = state
        .config_manager
        .create_template(
            &id,
            &name,
            &description,
            settings,
            env_vars,
            secrets,
            HashMap::new(),
        )
        .map_err(|e| e.to_string())?;
    Ok(config_template_to_info(&template))
}

#[tauri::command]
async fn apply_config_template(
    agent: String,
    template: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::AgentConfig, String> {
    state
        .config_manager
        .apply_template(&agent, &template)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_config_template(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .config_manager
        .delete_template(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_prompts_json(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    state
        .prompt_manager
        .export_prompts_json(None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_prompts_json(
    json: String,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::ImportSummary, String> {
    state
        .prompt_manager
        .import_prompts(&json, force)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_memories_json(
    scope: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    let scope_enum = scope.and_then(|s| match s.as_str() {
        "global" => Some(MemoryScope::Global),
        "project" => Some(MemoryScope::Project),
        "session" => Some(MemoryScope::Session),
        _ => None,
    });
    state
        .memory_manager
        .export_memories_json(scope_enum)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_memories_json(
    json: String,
    merge: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::ImportSummary, String> {
    state
        .memory_manager
        .import_memories(&json, merge)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_session_budget(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::BudgetReport, String> {
    state
        .session_manager
        .check_budget(chrono::Utc::now())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_session_budget(
    daily: Option<f64>,
    monthly: Option<f64>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::BudgetReport, String> {
    let manager = &state.session_manager;
    let mut budget = manager.get_budget().map_err(|e| e.to_string())?;
    budget.daily_usd = daily;
    budget.monthly_usd = monthly;
    manager.set_budget(&budget).map_err(|e| e.to_string())?;
    manager
        .check_budget(chrono::Utc::now())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn fork_session(
    id: String,
    agent: Option<String>,
    title: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<SessionDetail, String> {
    let session = state
        .session_manager
        .fork_session(&id, agent.as_deref(), title.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(session_to_detail(&session))
}

#[tauri::command]
async fn get_session_usage_summary(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::SessionUsageAggregate, String> {
    state
        .session_manager
        .usage_summary()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_session_usage_trend(
    days: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::UsageTrendPoint>, String> {
    state
        .session_manager
        .usage_trend(days.unwrap_or(30) as usize)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_session_usage_json(
    days: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    state
        .session_manager
        .export_usage_json(days.unwrap_or(30) as usize)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SkillCompatibilityInfo {
    skill: String,
    skill_version: String,
    requires_agenthub: Option<String>,
    current_agenthub: String,
    compatible: bool,
    message: String,
}

#[tauri::command]
async fn check_skill_compatibility(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<SkillCompatibilityInfo>, String> {
    let manager = &state.skill_manager;
    let results = if name == "*" {
        manager
            .check_all_compatibility()
            .map_err(|e| e.to_string())?
    } else {
        vec![manager
            .check_compatibility(&name)
            .map_err(|e| e.to_string())?]
    };
    Ok(results
        .into_iter()
        .map(|c| SkillCompatibilityInfo {
            skill: c.skill,
            skill_version: c.skill_version,
            requires_agenthub: c.requires_agenthub,
            current_agenthub: c.current_agenthub,
            compatible: c.compatible,
            message: c.message,
        })
        .collect())
}

#[tauri::command]
async fn get_trend(
    days: usize,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::TrendPoint>, String> {
    state.overview_report.trend(days).map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_monitor(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::MonitorReport, String> {
    let catalog = state.catalog.read().await;
    state.monitor.run(&catalog).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Wave 3 commands: secret keystore, memory vector/graph, workflows, prompt
// extraction, HTML dashboard
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_secret(
    agent: String,
    key: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Option<String>, String> {
    state
        .config_manager
        .get_secret(&agent, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_secret(
    agent: String,
    key: String,
    value: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .config_manager
        .set_secret(&agent, &key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_secret(
    agent: String,
    key: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .config_manager
        .delete_secret(&agent, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_secrets(
    agent: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::SecretInfo>, String> {
    state
        .config_manager
        .list_secrets(agent.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rotate_secret(
    agent: String,
    key: String,
    new_value: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::SecretRotationResult, String> {
    state
        .config_manager
        .rotate_secret(&agent, &key, &new_value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn migrate_secret(
    agent: String,
    key: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .config_manager
        .migrate_secret(&agent, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_memories_vector(
    query: String,
    top_k: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::MemoryMatch>, String> {
    state
        .memory_manager
        .search_entries_vector(&query, top_k.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_memories_hybrid(
    query: String,
    top_k: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::MemoryMatch>, String> {
    state
        .memory_manager
        .hybrid_search(&query, top_k.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn build_memory_graph(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::GraphSummary, String> {
    let graph = state
        .memory_manager
        .build_graph()
        .map_err(|e| e.to_string())?;
    Ok(graph.summary())
}

#[tauri::command]
async fn get_memory_graph(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::KnowledgeGraph, String> {
    state.memory_manager.load_graph().map_err(|e| e.to_string())
}

#[tauri::command]
async fn graph_neighbors(
    entity: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::GraphEdge>, String> {
    let graph = state
        .memory_manager
        .load_graph()
        .map_err(|e| e.to_string())?;
    Ok(graph.neighbors(&entity.to_lowercase(), limit.unwrap_or(10)))
}

#[tauri::command]
async fn list_workflows(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::Workflow>, String> {
    let manager = WorkflowManager::new(state.skill_manager.skills_dir().to_path_buf());
    manager.list_workflows().map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_workflow(
    id: String,
    name: String,
    description: String,
    steps: Vec<WorkflowStep>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::Workflow, String> {
    let manager = WorkflowManager::new(state.skill_manager.skills_dir().to_path_buf());
    manager
        .create_workflow(&id, &name, &description, steps)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_workflow(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    let manager = WorkflowManager::new(state.skill_manager.skills_dir().to_path_buf());
    manager.delete_workflow(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_workflow(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::WorkflowRunReport, String> {
    let manager = WorkflowManager::new(state.skill_manager.skills_dir().to_path_buf());
    manager
        .run_workflow(&state.skill_manager, &id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn extract_prompt_from_session(
    session_id: String,
    message_index: Option<usize>,
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::PromptExtraction, String> {
    let fallback_id = format!("{}-prompt", session_id);
    let fallback_name = fallback_id.clone();
    state
        .prompt_manager
        .extract_from_session(
            &state.session_manager,
            &session_id,
            message_index,
            id.as_deref().unwrap_or(&fallback_id),
            name.as_deref().unwrap_or(&fallback_name),
            description.as_deref().unwrap_or("Extracted from a session"),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_dashboard_html(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<String, String> {
    let catalog = state.catalog.read().await;
    state
        .overview_report
        .render_dashboard_html(&catalog, 14)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Wave 4: users & permissions, prompt community, skill marketplace, plugins,
// notification channels
// ---------------------------------------------------------------------------

fn user_manager(state: &AppState) -> UserManager {
    UserManager::new(state.config_manager.config_dir().to_path_buf())
}

fn community_manager(state: &AppState) -> CommunityManager {
    CommunityManager::new(state.prompt_manager.prompts_dir().to_path_buf())
}

fn marketplace_manager(state: &AppState) -> MarketplaceManager {
    MarketplaceManager::new(state.skill_manager.skills_dir().to_path_buf())
}

fn plugin_manager(state: &AppState) -> PluginManager {
    PluginManager::new(state.skill_manager.skills_dir().to_path_buf())
}

fn notifier(state: &AppState) -> Notifier {
    Notifier::new(state.config_manager.config_dir().to_path_buf())
}

// ---- users & permissions ----

#[tauri::command]
async fn list_users(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::User>, String> {
    user_manager(&state).list_users().map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_user(
    id: String,
    name: String,
    email: Option<String>,
    roles: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::User, String> {
    user_manager(&state)
        .create_user(&id, &name, email.as_deref(), roles.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_user(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    if id == "admin" {
        return Err("Refusing to delete the built-in 'admin' user".to_string());
    }
    user_manager(&state)
        .delete_user(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_user_role(
    id: String,
    role: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::User, String> {
    user_manager(&state)
        .add_role(&id, &role)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_user_role(
    id: String,
    role: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::User, String> {
    user_manager(&state)
        .remove_role(&id, &role)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn grant_permission(
    user: String,
    action: String,
    module: Option<String>,
    agent: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::Permission, String> {
    user_manager(&state)
        .grant_permission(&user, &action, module.as_deref(), agent.as_deref(), None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn revoke_permission(
    user: String,
    action: String,
    module: Option<String>,
    agent: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    user_manager(&state)
        .revoke_permission(&user, &action, module.as_deref(), agent.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_permissions(
    user: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::Permission>, String> {
    user_manager(&state)
        .list_permissions(user.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_permission(
    user: String,
    action: String,
    module: Option<String>,
    agent: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    user_manager(&state)
        .check_permission(&user, &action, module.as_deref(), agent.as_deref())
        .map_err(|e| e.to_string())
}

// ---- prompt community ----

#[tauri::command]
async fn publish_prompt(
    id: String,
    publisher: Option<String>,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::CommunityPrompt, String> {
    community_manager(&state)
        .publish_by_id(
            &state.prompt_manager,
            &id,
            publisher.as_deref().unwrap_or("local"),
            force,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_community_prompts(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::CommunityPrompt>, String> {
    community_manager(&state).list().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_community_prompt(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::CommunityPrompt, String> {
    community_manager(&state)
        .get(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn install_community_prompt(
    id: String,
    new_id: Option<String>,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::PromptTemplate, String> {
    community_manager(&state)
        .install(&state.prompt_manager, &id, new_id.as_deref(), force)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_community_prompt(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    community_manager(&state)
        .delete(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pull_community_prompts(
    url: String,
    token: Option<String>,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::RemoteSyncReport, String> {
    community_manager(&state)
        .pull_remote(&url, token.as_deref(), force)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn push_community_prompts(
    url: String,
    token: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::RemoteSyncReport, String> {
    community_manager(&state)
        .push_remote(&url, token.as_deref())
        .map_err(|e| e.to_string())
}

// ---- skill marketplace ----

#[tauri::command]
async fn market_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::MarketplaceSkill>, String> {
    marketplace_manager(&state)
        .search(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_info(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::MarketplaceSkill, String> {
    marketplace_manager(&state)
        .info(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_install(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    marketplace_manager(&state)
        .install(&state.skill_manager, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_rate(
    name: String,
    rating: u8,
    rater: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::SkillRating, String> {
    marketplace_manager(&state)
        .rate(&name, rating, rater.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_stats(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::MarketplaceStats, String> {
    marketplace_manager(&state)
        .stats()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_refresh(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::MarketplaceStats, String> {
    marketplace_manager(&state)
        .refresh()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_pull(
    url: String,
    token: Option<String>,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::RemoteSyncReport, String> {
    marketplace_manager(&state)
        .pull_remote(&url, token.as_deref(), force)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn market_push(
    url: String,
    token: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::RemoteSyncReport, String> {
    marketplace_manager(&state)
        .push_remote(&url, token.as_deref())
        .map_err(|e| e.to_string())
}

// ---- plugins ----

#[tauri::command]
async fn list_plugins(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::Plugin>, String> {
    plugin_manager(&state)
        .list_plugins()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn register_plugin(
    name: String,
    dir: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::Plugin, String> {
    plugin_manager(&state)
        .register_plugin(&name, std::path::Path::new(&dir))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn unregister_plugin(
    name: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    plugin_manager(&state)
        .unregister_plugin(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_plugin_enabled(
    name: String,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<(), String> {
    if enabled {
        plugin_manager(&state).enable_plugin(&name)
    } else {
        plugin_manager(&state).disable_plugin(&name)
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_plugin_hook(
    event: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::PluginRunResult>, String> {
    plugin_manager(&state)
        .run_hook(&event)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn plugin_pull(
    url: String,
    token: Option<String>,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::RemoteSyncReport, String> {
    plugin_manager(&state)
        .pull_remote(&url, token.as_deref(), force)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn plugin_push(
    url: String,
    token: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::RemoteSyncReport, String> {
    plugin_manager(&state)
        .push_remote(&url, token.as_deref())
        .map_err(|e| e.to_string())
}

// ---- notification channels ----

#[tauri::command]
async fn list_notify_channels(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::NotifyChannel>, String> {
    notifier(&state).list_channels().map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn add_notify_channel(
    id: String,
    kind: String,
    target: String,
    from: Option<String>,
    subject_prefix: Option<String>,
    min_severity: Option<String>,
    dedup_minutes: Option<u64>,
    smtp_host: Option<String>,
    smtp_port: Option<u16>,
    smtp_user: Option<String>,
    smtp_password: Option<String>,
    smtp_tls: Option<String>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::NotifyChannel, String> {
    let config = match kind.as_str() {
        "webhook" => agenthub_core::ChannelConfig::Webhook {
            url: target,
            headers: Vec::new(),
        },
        "email" => agenthub_core::ChannelConfig::Email {
            to: target,
            from: from.unwrap_or_else(|| "agenthub@localhost".to_string()),
            subject_prefix,
            smtp: smtp_host.map(|host| agenthub_core::SmtpConfig {
                host,
                port: smtp_port.unwrap_or(587),
                username: smtp_user,
                password: smtp_password,
                tls: smtp_tls.unwrap_or_else(|| "starttls".to_string()),
            }),
        },
        "file" => agenthub_core::ChannelConfig::File { path: target },
        other => return Err(format!("Invalid channel kind '{}'", other)),
    };
    notifier(&state)
        .add_channel_with_options(&id, config, min_severity.as_deref(), dedup_minutes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_notify_channel(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    notifier(&state)
        .remove_channel(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_notify_channel_enabled(
    id: String,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::NotifyChannel, String> {
    notifier(&state)
        .set_channel_enabled(&id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_notification(
    force: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::ChannelResult>, String> {
    let catalog = state.catalog.read().await;
    let report = state.monitor.run(&catalog).map_err(|e| e.to_string())?;
    notifier(&state)
        .send(&report, force.unwrap_or(false))
        .map_err(|e| e.to_string())
}

// ---- wave 5: prompt effects, memory reindex ----

#[tauri::command]
async fn get_prompt_effects(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::PromptEffects, String> {
    state
        .prompt_manager
        .get_effects(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_prompt_effects(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<Vec<agenthub_core::PromptEffects>, String> {
    state
        .prompt_manager
        .list_effects()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn record_prompt_outcome(
    prompt_id: String,
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::PromptOutcome, String> {
    let session = state
        .session_manager
        .get_session(&session_id)
        .map_err(|e| e.to_string())?;
    state
        .prompt_manager
        .record_outcome_from_session(&prompt_id, &session)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_prompt_effects(
    id: String,
    state: tauri::State<'_, AppState>,
) -> std::result::Result<bool, String> {
    state
        .prompt_manager
        .clear_effects(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn build_vector_index(
    state: tauri::State<'_, AppState>,
) -> std::result::Result<agenthub_core::VectorIndexSummary, String> {
    state
        .memory_manager
        .build_vector_index()
        .map_err(|e| e.to_string())
}

fn main() {
    tracing_subscriber::fmt::init();

    let catalog = load_catalog().expect("Failed to load agent catalog");
    let platform = get_current_platform();

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agenthub");
    let config_manager = ConfigManager::new(config_dir.clone());

    // Initialize skill manager with codex skills directory
    let codex_skills_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("skills");
    let skill_manager = SkillManager::new(config_dir.join("skills"))
        .with_extra_dir(codex_skills_dir)
        .with_project_dir(
            std::env::current_dir()
                .unwrap_or_default()
                .join(".agenthub")
                .join("skills"),
        )
        .with_global_dir(PathBuf::from("/etc/agenthub/skills"));

    let prompt_manager = PromptManager::new(config_dir.join("prompts"));
    let session_manager = SessionManager::new(config_dir.join("sessions"));
    let memory_manager = MemoryManager::new(config_dir.join("memory"));
    let audit_manager = AuditManager::new(config_dir.join("audit"));
    let backup_manager = BackupManager::new(config_dir.clone());
    let overview_report = OverviewReport::new(config_dir.clone(), platform);
    let monitor = Monitor::new(config_dir.clone(), platform);
    let pricing_table = PricingTable::builtin();

    let state = AppState {
        catalog: Arc::new(RwLock::new(catalog)),
        platform,
        config_manager: Arc::new(config_manager),
        skill_manager: Arc::new(skill_manager),
        prompt_manager: Arc::new(prompt_manager),
        session_manager: Arc::new(session_manager),
        memory_manager: Arc::new(memory_manager),
        audit_manager: Arc::new(audit_manager),
        backup_manager: Arc::new(backup_manager),
        overview_report: Arc::new(overview_report),
        monitor: Arc::new(monitor),
        pricing_table: Arc::new(pricing_table),
        cancellations: Arc::new(Mutex::new(HashMap::new())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_agents,
            search_agents,
            install_agent,
            uninstall_agent,
            batch_install_agents,
            batch_uninstall_agents,
            cancel_operation,
            list_configs,
            get_config,
            get_native_config,
            save_native_config,
            set_config_value,
            validate_agent_config,
            repair_agent_config,
            list_config_history,
            rollback_agent_config,
            delete_config,
            create_config,
            list_installed_agents,
            list_skills,
            create_skill,
            enable_skill,
            disable_skill,
            delete_skill,
            run_diagnostics,
            list_prompts,
            create_prompt,
            render_prompt,
            delete_prompt,
            list_sessions,
            create_session,
            get_session,
            delete_session,
            list_memories,
            create_memory,
            search_memories,
            search_memories_semantic,
            apply_memory_decay,
            set_memory_importance,
            revive_memory,
            delete_memory,
            get_status_overview,
            list_audit,
            clear_audit,
            create_backup,
            restore_backup,
            replay_session,
            record_session_usage,
            set_session_model,
            list_session_templates,
            create_session_template,
            create_session_from_template,
            delete_session_template,
            list_prompt_versions,
            rollback_prompt,
            get_prompt_usage,
            render_prompt_checked,
            list_config_templates,
            create_config_template,
            apply_config_template,
            delete_config_template,
            export_prompts_json,
            import_prompts_json,
            export_memories_json,
            import_memories_json,
            get_session_budget,
            set_session_budget,
            get_session_usage_summary,
            get_session_usage_trend,
            export_session_usage_json,
            fork_session,
            check_skill_compatibility,
            get_trend,
            run_monitor,
            get_secret,
            set_secret,
            delete_secret,
            list_secrets,
            rotate_secret,
            migrate_secret,
            search_memories_vector,
            search_memories_hybrid,
            build_memory_graph,
            get_memory_graph,
            graph_neighbors,
            list_workflows,
            create_workflow,
            delete_workflow,
            run_workflow,
            extract_prompt_from_session,
            get_dashboard_html,
            list_users,
            create_user,
            delete_user,
            add_user_role,
            remove_user_role,
            grant_permission,
            revoke_permission,
            list_permissions,
            check_permission,
            publish_prompt,
            list_community_prompts,
            get_community_prompt,
            install_community_prompt,
            delete_community_prompt,
            pull_community_prompts,
            push_community_prompts,
            market_search,
            market_info,
            market_install,
            market_rate,
            market_stats,
            market_refresh,
            market_pull,
            market_push,
            list_plugins,
            register_plugin,
            unregister_plugin,
            set_plugin_enabled,
            run_plugin_hook,
            plugin_pull,
            plugin_push,
            list_notify_channels,
            add_notify_channel,
            remove_notify_channel,
            set_notify_channel_enabled,
            send_notification,
            get_prompt_effects,
            list_prompt_effects,
            record_prompt_outcome,
            clear_prompt_effects,
            build_vector_index
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use agenthub_core::agent::PackageManager;
    use agenthub_core::{Agent, AgentKind, InstallerConfig, Platform, SupportStatus};
    use std::collections::HashMap;

    fn test_agent() -> Agent {
        let mut installers = HashMap::new();
        installers.insert(
            Platform::Windows,
            InstallerConfig {
                manager: PackageManager::Npm,
                package: Some("@test/cli".to_string()),
            },
        );
        installers.insert(
            Platform::MacOS,
            InstallerConfig {
                manager: PackageManager::BrewCask,
                package: Some("test-package".to_string()),
            },
        );

        Agent {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            kind: AgentKind::CLI,
            provider: "Test Corp".to_string(),
            description: "A test agent".to_string(),
            homepage: "https://test.com".to_string(),
            installers,
            status: SupportStatus::Verified,
            catalog_verified_at: None,
            installer_verified_at: None,
        }
    }

    #[test]
    fn test_agent_to_info_converts_correctly() {
        let agent = test_agent();
        let info = agent_to_info(&agent);

        assert_eq!(info.id, "test-agent");
        assert_eq!(info.name, "Test Agent");
        assert_eq!(info.kind, "CLI");
        assert_eq!(info.provider, "Test Corp");
        assert_eq!(info.description, "A test agent");
        assert_eq!(info.homepage, "https://test.com");
        assert_eq!(info.status, "Verified");
        assert_eq!(info.installers.len(), 2);
        assert!(info.catalog_verified_at.is_none());
        assert!(info.installer_verified_at.is_none());
    }

    #[test]
    fn test_agent_to_info_no_verification_dates() {
        let mut agent = test_agent();
        agent.catalog_verified_at = None;
        agent.installer_verified_at = None;
        let info = agent_to_info(&agent);

        assert!(info.catalog_verified_at.is_none());
        assert!(info.installer_verified_at.is_none());
    }

    #[test]
    fn test_get_current_platform_returns_platform() {
        let platform = get_current_platform();
        match platform {
            Platform::Windows | Platform::MacOS | Platform::Linux => {}
        }
    }

    #[test]
    fn test_install_result_serialization() {
        let result = InstallResult {
            success: true,
            message: "Done".to_string(),
            agent_name: "test".to_string(),
            command: "npm install -g @test/cli".to_string(),
            exit_code: Some(0),
            stdout: "installed".to_string(),
            stderr: String::new(),
            duration_ms: 1500,
            timed_out: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"agent_name\":\"test\""));
        assert!(json.contains("\"exit_code\":0"));
    }

    #[test]
    fn test_batch_result_serialization() {
        let results = vec![
            InstallResult {
                success: true,
                message: "Done".to_string(),
                agent_name: "agent-a".to_string(),
                command: String::new(),
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 100,
                timed_out: false,
            },
            InstallResult {
                success: false,
                message: "Failed".to_string(),
                agent_name: "agent-b".to_string(),
                command: String::new(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "error".to_string(),
                duration_ms: 50,
                timed_out: false,
            },
        ];

        let batch = BatchResult {
            total: 2,
            success: 1,
            failed: 1,
            results,
        };

        let json = serde_json::to_string(&batch).unwrap();
        assert!(json.contains("\"total\":2"));
        assert!(json.contains("\"success\":1"));
        assert!(json.contains("\"failed\":1"));
    }
}
