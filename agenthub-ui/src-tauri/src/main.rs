use agenthub_core::{
    Agent, AgentKind, Catalog, ConfigManager, ConfigValue, DiagnosticManager, Installer,
    MemoryManager, MemoryScope, MemoryType, Platform, PromptManager, Result, SessionManager,
    SkillManager,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::RwLock;

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

            let result = tokio::task::spawn_blocking(move || {
                let installer = Installer::new(platform);
                installer.execute_install(&agent_clone, false)
            })
            .await
            .map_err(|e| format!("Task failed: {}", e))?;

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

            let result = tokio::task::spawn_blocking(move || {
                let installer = Installer::new(platform);
                installer.execute_uninstall(&agent_clone, false)
            })
            .await
            .map_err(|e| format!("Task failed: {}", e))?;

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

                let agent_clone = agent.clone();
                let agent_name = agent.name.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let installer = Installer::new(platform);
                    installer.execute_install(&agent_clone, false)
                })
                .await
                .map_err(|e| format!("Task failed: {}", e))?;

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

                let agent_clone = agent.clone();
                let agent_name = agent.name.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let installer = Installer::new(platform);
                    installer.execute_uninstall(&agent_clone, false)
                })
                .await
                .map_err(|e| format!("Task failed: {}", e))?;

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
    let skill_manager =
        SkillManager::new(config_dir.join("skills")).with_extra_dir(codex_skills_dir);

    let prompt_manager = PromptManager::new(config_dir.join("prompts"));
    let session_manager = SessionManager::new(config_dir.join("sessions"));
    let memory_manager = MemoryManager::new(config_dir.join("memory"));

    let state = AppState {
        catalog: Arc::new(RwLock::new(catalog)),
        platform,
        config_manager: Arc::new(config_manager),
        skill_manager: Arc::new(skill_manager),
        prompt_manager: Arc::new(prompt_manager),
        session_manager: Arc::new(session_manager),
        memory_manager: Arc::new(memory_manager),
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
            list_configs,
            get_config,
            get_native_config,
            save_native_config,
            set_config_value,
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
            delete_memory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
