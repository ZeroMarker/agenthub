use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::AppHandle;
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum AgentType {
    CLI,
    Desktop,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Agent {
    name: String,
    description: String,
    package_name: String,
    manager: String,
    agent_type: AgentType,
    install_source: String,
    download_url: String,
    version: Option<String>,
    installed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InstallResult {
    success: bool,
    message: String,
    agent_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BatchResult {
    total: usize,
    success: usize,
    failed: usize,
    results: Vec<InstallResult>,
}

fn get_known_agents() -> Vec<Agent> {
    vec![
        // ==================== CLI Agents ====================
        Agent {
            name: "codex".to_string(),
            description: "OpenAI Codex CLI - AI coding assistant powered by GPT-4".to_string(),
            package_name: "@openai/codex".to_string(),
            manager: "npm".to_string(),
            agent_type: AgentType::CLI,
            install_source: "npmjs.com".to_string(),
            download_url: "".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "claude-code".to_string(),
            description: "Anthropic Claude Code - AI pair programmer with Claude".to_string(),
            package_name: "@anthropic-ai/claude-code".to_string(),
            manager: "npm".to_string(),
            agent_type: AgentType::CLI,
            install_source: "npmjs.com".to_string(),
            download_url: "".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "kimi-code".to_string(),
            description: "Moonshot Kimi Code - AI coding assistant with long context".to_string(),
            package_name: "@moonshot-ai/kimi-code".to_string(),
            manager: "npm".to_string(),
            agent_type: AgentType::CLI,
            install_source: "npmjs.com".to_string(),
            download_url: "".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "qwen-code".to_string(),
            description: "Alibaba Qwen Coder - AI coding assistant".to_string(),
            package_name: "@qwen-code/qwen-code".to_string(),
            manager: "npm".to_string(),
            agent_type: AgentType::CLI,
            install_source: "npmjs.com".to_string(),
            download_url: "".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "reasonix-cli".to_string(),
            description: "Reasonix CLI - AI reasoning and coding agent".to_string(),
            package_name: "ESEngine.ReasonixCLI".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::CLI,
            install_source: "winget".to_string(),
            download_url: "reasonix.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "mimo-code".to_string(),
            description: "Xiaomi MiMo Code - AI coding assistant powered by MiMo".to_string(),
            package_name: "@mimo-ai/cli".to_string(),
            manager: "npm".to_string(),
            agent_type: AgentType::CLI,
            install_source: "npmjs.com".to_string(),
            download_url: "".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "grok-cli".to_string(),
            description: "Grok CLI - AI coding agent by xAI".to_string(),
            package_name: "xAI.GrokBuild".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::CLI,
            install_source: "winget".to_string(),
            download_url: "x.ai".to_string(),
            version: None,
            installed: false,
        },

        // ==================== Desktop Agents ====================
        Agent {
            name: "cursor".to_string(),
            description: "Cursor - AI-first code editor built on VS Code".to_string(),
            package_name: "Anysphere.Cursor".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "cursor.com".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "windsurf".to_string(),
            description: "Windsurf - AI-powered IDE by Codeium".to_string(),
            package_name: "Codeium.Windsurf".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "codeium.com".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "trae".to_string(),
            description: "Trae - AI coding companion by ByteDance".to_string(),
            package_name: "ByteDance.Trae".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "trae.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "trae-solo".to_string(),
            description: "Trae Solo - Standalone AI coding agent by ByteDance".to_string(),
            package_name: "ByteDance.TraeWork".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "trae.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "codex-desktop".to_string(),
            description: "OpenAI Codex Desktop - AI coding assistant desktop app".to_string(),
            package_name: "9N8CJ4W95TBZ".to_string(),
            manager: "msstore".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "msstore".to_string(),
            download_url: "openai.com".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "claude-desktop".to_string(),
            description: "Anthropic Claude Desktop - AI assistant desktop app".to_string(),
            package_name: "Anthropic.Claude".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "claude.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "kimi-desktop".to_string(),
            description: "Moonshot Kimi Desktop - AI assistant desktop app".to_string(),
            package_name: "MoonshotAI.Kimi".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "kimi.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "workbuddy".to_string(),
            description: "WorkBuddy - AI work assistant".to_string(),
            package_name: "Tencent.WorkBuddy".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "workbuddy.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "codebuddy".to_string(),
            description: "CodeBuddy - AI coding assistant by Tencent".to_string(),
            package_name: "Tencent.CodeBuddy".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "codebuddy.com".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "qoder".to_string(),
            description: "Qoder - AI coding agent".to_string(),
            package_name: "Alibaba.Qoder".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "qoder.com".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "qoder-work".to_string(),
            description: "Qoder Work - AI coding agent for work".to_string(),
            package_name: "Alibaba.QoderWork".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "qoder.com".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "minimax-agent".to_string(),
            description: "MiniMax Agent - AI coding assistant by MiniMax".to_string(),
            package_name: "MiniMax.MiniMaxCode".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "minimax.chat".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "zcode".to_string(),
            description: "ZCode - AI coding assistant".to_string(),
            package_name: "ZhipuAI.ZCode".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "zcode.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "antigravity".to_string(),
            description: "Google Antigravity - AI coding assistant by Google".to_string(),
            package_name: "Google.Antigravity".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "antigravity.google".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "antigravity-ide".to_string(),
            description: "Google Antigravity IDE - AI-powered IDE by Google".to_string(),
            package_name: "Google.AntigravityIDE".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "antigravity.google".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "reasonix".to_string(),
            description: "Reasonix - AI reasoning and coding agent".to_string(),
            package_name: "reasonix".to_string(),
            manager: "npm".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget / brew".to_string(),
            download_url: "reasonix.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "opencode".to_string(),
            description: "OpenCode - Open source AI coding assistant".to_string(),
            package_name: "SST.OpenCodeDesktop".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget / brew".to_string(),
            download_url: "opencode.ai".to_string(),
            version: None,
            installed: false,
        },
        Agent {
            name: "openwork".to_string(),
            description: "OpenWork - AI-powered work assistant".to_string(),
            package_name: "DifferentAI.OpenWork".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
            download_url: "openwork.ai".to_string(),
            version: None,
            installed: false,
        },
    ]
}

fn check_installed(agents: &mut Vec<Agent>) {
    // 只执行一次npm list命令
    let npm_output = {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "npm", "list", "-g", "--depth=0"])
                .output()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("npm")
                .args(["list", "-g", "--depth=0"])
                .output()
        }
    };

    let npm_stdout = match npm_output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    };

    // 只执行一次pip list命令
    let pip_output = Command::new("pip")
        .args(["list", "--format=columns"])
        .output();

    let pip_stdout = match pip_output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    };

    // Windows: 执行一次winget list命令
    #[cfg(target_os = "windows")]
    let winget_output = Command::new("cmd")
        .args(["/C", "winget", "list"])
        .output();

    #[cfg(target_os = "windows")]
    let winget_stdout = match winget_output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    };

    #[cfg(not(target_os = "windows"))]
    let winget_stdout = String::new();

    for agent in agents {
        agent.installed = match agent.manager.as_str() {
            "npm" => npm_stdout.contains(&agent.package_name),
            "pip" => pip_stdout.contains(&agent.package_name),
            "winget" | "msstore" => winget_stdout.contains(&agent.package_name),
            _ => false,
        };
    }
}

#[tauri::command]
fn list_agents(agent_type: Option<String>) -> Vec<Agent> {
    let mut agents = get_known_agents();
    check_installed(&mut agents);
    
    match agent_type.as_deref() {
        Some("cli") => agents.into_iter().filter(|a| a.agent_type == AgentType::CLI).collect(),
        Some("desktop") => agents.into_iter().filter(|a| a.agent_type == AgentType::Desktop).collect(),
        _ => agents,
    }
}

#[tauri::command]
fn search_agents(query: String, agent_type: Option<String>) -> Vec<Agent> {
    let mut agents = get_known_agents();
    let query_lower = query.to_lowercase();

    check_installed(&mut agents);

    let filtered = agents
        .into_iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&query_lower)
                || a.description.to_lowercase().contains(&query_lower)
                || a.package_name.to_lowercase().contains(&query_lower)
        });

    match agent_type.as_deref() {
        Some("cli") => filtered.filter(|a| a.agent_type == AgentType::CLI).collect(),
        Some("desktop") => filtered.filter(|a| a.agent_type == AgentType::Desktop).collect(),
        _ => filtered.collect(),
    }
}

#[tauri::command]
async fn install_agent(name: String, app: AppHandle) -> Result<InstallResult, String> {
    let agents = get_known_agents();
    let agent = agents.iter().find(|a| a.name == name).cloned();

    match agent {
        Some(agent) => {
            // Emit progress: preparing
            let _ = app.emit("install-progress", serde_json::json!({
                "name": agent.name,
                "step": 1,
                "total_steps": 3,
                "message": "Preparing installation..."
            }));
            
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            
            // Emit progress: downloading
            let _ = app.emit("install-progress", serde_json::json!({
                "name": agent.name,
                "step": 2,
                "total_steps": 3,
                "message": "Downloading and installing..."
            }));
            
            let package_name = agent.package_name.clone();
            let manager = agent.manager.clone();
            let agent_name = agent.name.clone();
            
            let result = tokio::task::spawn_blocking(move || {
                let status = match manager.as_str() {
                    "npm" => {
                        #[cfg(target_os = "windows")]
                        {
                            Command::new("cmd")
                                .args(["/C", "npm", "install", "-g", &package_name])
                                .status()
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            Command::new("npm")
                                .args(["install", "-g", &package_name])
                                .status()
                        }
                    }
                    "pip" => Command::new("pip")
                        .args(["install", &package_name])
                        .status(),
                    "cargo" => Command::new("cargo")
                        .args(["install", &package_name])
                        .status(),
                    "winget" | "msstore" => {
                        #[cfg(target_os = "windows")]
                        {
                            Command::new("cmd")
                                .args(["/C", "winget", "install", &package_name, "--accept-package-agreements", "--accept-source-agreements"])
                                .status()
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            Command::new("brew")
                                .args(["install", "--cask", &package_name])
                                .status()
                        }
                    }
                    _ => return Err(format!("Unsupported package manager: {}", manager)),
                };

                match status {
                    Ok(status) => {
                        if status.success() {
                            Ok(())
                        } else {
                            Err("Installation failed".to_string())
                        }
                    }
                    Err(e) => Err(format!("Error running installer: {}", e)),
                }
            }).await.unwrap_or_else(|e| Err(format!("Task failed: {}", e)));

            match result {
                Ok(()) => {
                    let _ = app.emit("install-progress", serde_json::json!({
                        "name": agent_name,
                        "step": 3,
                        "total_steps": 3,
                        "message": "Completed"
                    }));
                    Ok(InstallResult {
                        success: true,
                        message: format!("{} installed successfully", agent_name),
                        agent_name,
                    })
                }
                Err(e) => {
                    let _ = app.emit("install-progress", serde_json::json!({
                        "name": agent_name,
                        "step": 3,
                        "total_steps": 3,
                        "message": format!("Failed: {}", e)
                    }));
                    Ok(InstallResult {
                        success: false,
                        message: e,
                        agent_name,
                    })
                }
            }
        }
        None => Err(format!("Agent '{}' not found", name)),
    }
}

#[tauri::command]
async fn uninstall_agent(name: String, app: AppHandle) -> Result<InstallResult, String> {
    let agents = get_known_agents();
    let agent = agents.iter().find(|a| a.name == name).cloned();

    match agent {
        Some(agent) => {
            // Emit progress: preparing
            let _ = app.emit("uninstall-progress", serde_json::json!({
                "name": agent.name,
                "step": 1,
                "total_steps": 3,
                "message": "Preparing uninstallation..."
            }));
            
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            
            // Emit progress: removing
            let _ = app.emit("uninstall-progress", serde_json::json!({
                "name": agent.name,
                "step": 2,
                "total_steps": 3,
                "message": "Removing package..."
            }));
            
            let package_name = agent.package_name.clone();
            let manager = agent.manager.clone();
            let agent_name = agent.name.clone();
            
            let result = tokio::task::spawn_blocking(move || {
                let status = match manager.as_str() {
                    "npm" => {
                        #[cfg(target_os = "windows")]
                        {
                            Command::new("cmd")
                                .args(["/C", "npm", "uninstall", "-g", &package_name])
                                .status()
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            Command::new("npm")
                                .args(["uninstall", "-g", &package_name])
                                .status()
                        }
                    }
                    "pip" => Command::new("pip")
                        .args(["uninstall", "-y", &package_name])
                        .status(),
                    "cargo" => Command::new("cargo")
                        .args(["uninstall", &package_name])
                        .status(),
                    "winget" | "msstore" => {
                        #[cfg(target_os = "windows")]
                        {
                            Command::new("cmd")
                                .args(["/C", "winget", "uninstall", &package_name])
                                .status()
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            Command::new("brew")
                                .args(["uninstall", "--cask", &package_name])
                                .status()
                        }
                    }
                    _ => return Err(format!("Unsupported package manager: {}", manager)),
                };

                match status {
                    Ok(status) => {
                        if status.success() {
                            Ok(())
                        } else {
                            Err("Uninstallation failed".to_string())
                        }
                    }
                    Err(e) => Err(format!("Error running uninstaller: {}", e)),
                }
            }).await.unwrap_or_else(|e| Err(format!("Task failed: {}", e)));

            match result {
                Ok(()) => {
                    let _ = app.emit("uninstall-progress", serde_json::json!({
                        "name": agent_name,
                        "step": 3,
                        "total_steps": 3,
                        "message": "Completed"
                    }));
                    Ok(InstallResult {
                        success: true,
                        message: format!("{} uninstalled successfully", agent_name),
                        agent_name,
                    })
                }
                Err(e) => {
                    let _ = app.emit("uninstall-progress", serde_json::json!({
                        "name": agent_name,
                        "step": 3,
                        "total_steps": 3,
                        "message": format!("Failed: {}", e)
                    }));
                    Ok(InstallResult {
                        success: false,
                        message: e,
                        agent_name,
                    })
                }
            }
        }
        None => Err(format!("Agent '{}' not found", name)),
    }
}

#[tauri::command]
async fn batch_install_agents(names: Vec<String>, app: AppHandle) -> Result<BatchResult, String> {
    let agents = get_known_agents();
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    for (index, name) in names.iter().enumerate() {
        let agent = agents.iter().find(|a| a.name == *name).cloned();

        match agent {
            Some(agent) => {
                let _ = app.emit("batch-progress", serde_json::json!({
                    "current": index + 1,
                    "total": names.len(),
                    "agent": agent.name,
                    "action": "install"
                }));

                let package_name = agent.package_name.clone();
                let manager = agent.manager.clone();
                let agent_name = agent.name.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let status = match manager.as_str() {
                        "npm" => {
                            #[cfg(target_os = "windows")]
                            {
                                Command::new("cmd")
                                    .args(["/C", "npm", "install", "-g", &package_name])
                                    .status()
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                Command::new("npm")
                                    .args(["install", "-g", &package_name])
                                    .status()
                            }
                        }
                        "pip" => Command::new("pip")
                            .args(["install", &package_name])
                            .status(),
                        "cargo" => Command::new("cargo")
                            .args(["install", &package_name])
                            .status(),
                        "winget" | "msstore" => {
                            #[cfg(target_os = "windows")]
                            {
                                Command::new("cmd")
                                    .args(["/C", "winget", "install", &package_name, "--accept-package-agreements", "--accept-source-agreements"])
                                    .status()
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                Command::new("brew")
                                    .args(["install", "--cask", &package_name])
                                    .status()
                            }
                        }
                        _ => return Err(format!("Unsupported package manager: {}", manager)),
                    };

                    match status {
                        Ok(status) => {
                            if status.success() {
                                Ok(())
                            } else {
                                Err("Installation failed".to_string())
                            }
                        }
                        Err(e) => Err(format!("Error: {}", e)),
                    }
                }).await.unwrap_or_else(|e| Err(format!("Task failed: {}", e)));

                match result {
                    Ok(()) => {
                        results.push(InstallResult {
                            success: true,
                            message: "Installed successfully".to_string(),
                            agent_name: agent_name.clone(),
                        });
                        success_count += 1;
                    }
                    Err(e) => {
                        results.push(InstallResult {
                            success: false,
                            message: e,
                            agent_name: agent_name.clone(),
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
async fn batch_uninstall_agents(names: Vec<String>, app: AppHandle) -> Result<BatchResult, String> {
    let agents = get_known_agents();
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    for (index, name) in names.iter().enumerate() {
        let agent = agents.iter().find(|a| a.name == *name).cloned();

        match agent {
            Some(agent) => {
                let _ = app.emit("batch-progress", serde_json::json!({
                    "current": index + 1,
                    "total": names.len(),
                    "agent": agent.name,
                    "action": "uninstall"
                }));

                let package_name = agent.package_name.clone();
                let manager = agent.manager.clone();
                let agent_name = agent.name.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let status = match manager.as_str() {
                        "npm" => {
                            #[cfg(target_os = "windows")]
                            {
                                Command::new("cmd")
                                    .args(["/C", "npm", "uninstall", "-g", &package_name])
                                    .status()
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                Command::new("npm")
                                    .args(["uninstall", "-g", &package_name])
                                    .status()
                            }
                        }
                        "pip" => Command::new("pip")
                            .args(["uninstall", "-y", &package_name])
                            .status(),
                        "cargo" => Command::new("cargo")
                            .args(["uninstall", &package_name])
                            .status(),
                        "winget" | "msstore" => {
                            #[cfg(target_os = "windows")]
                            {
                                Command::new("cmd")
                                    .args(["/C", "winget", "uninstall", &package_name])
                                    .status()
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                Command::new("brew")
                                    .args(["uninstall", "--cask", &package_name])
                                    .status()
                            }
                        }
                        _ => return Err(format!("Unsupported package manager: {}", manager)),
                    };

                    match status {
                        Ok(status) => {
                            if status.success() {
                                Ok(())
                            } else {
                                Err("Uninstallation failed".to_string())
                            }
                        }
                        Err(e) => Err(format!("Error: {}", e)),
                    }
                }).await.unwrap_or_else(|e| Err(format!("Task failed: {}", e)));

                match result {
                    Ok(()) => {
                        results.push(InstallResult {
                            success: true,
                            message: "Uninstalled successfully".to_string(),
                            agent_name: agent_name.clone(),
                        });
                        success_count += 1;
                    }
                    Err(e) => {
                        results.push(InstallResult {
                            success: false,
                            message: e,
                            agent_name: agent_name.clone(),
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
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
