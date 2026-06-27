use agenthub_core::{Agent, AgentKind, Catalog, Installer, Platform, Result};
use serde::{Deserialize, Serialize};
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

fn main() {
    tracing_subscriber::fmt::init();

    let catalog = load_catalog().expect("Failed to load agent catalog");
    let platform = get_current_platform();

    let state = AppState {
        catalog: Arc::new(RwLock::new(catalog)),
        platform,
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
            batch_uninstall_agents
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}