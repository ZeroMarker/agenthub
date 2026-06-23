use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "agenthub")]
#[command(about = "Agent Hub - Manage your AI coding agents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available agents
    List {
        /// Filter by agent type (cli or desktop)
        #[arg(short, long)]
        r#type: Option<String>,
    },
    /// Search for agents
    Search {
        /// Search query
        query: String,
        /// Filter by agent type (cli or desktop)
        #[arg(short, long)]
        r#type: Option<String>,
    },
    /// Install an agent
    Install {
        /// Agent name(s) - can be multiple for batch install
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,
        /// Preview commands without executing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Uninstall an agent
    Uninstall {
        /// Agent name(s) - can be multiple for batch uninstall
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,
        /// Preview commands without executing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Show agent details
    Info {
        /// Agent name
        name: String,
    },
    /// Check environment and dependencies
    Doctor,
}

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
            package_name: "ESEngine.ReasonixDesktop".to_string(),
            manager: "winget".to_string(),
            agent_type: AgentType::Desktop,
            install_source: "winget".to_string(),
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

fn list_agents(agent_type: Option<String>) {
    let mut agents = get_known_agents();
    check_installed(&mut agents);

    let filtered = match agent_type.as_deref() {
        Some("cli") => agents.into_iter().filter(|a| a.agent_type == AgentType::CLI).collect::<Vec<_>>(),
        Some("desktop") => agents.into_iter().filter(|a| a.agent_type == AgentType::Desktop).collect::<Vec<_>>(),
        _ => agents,
    };

    println!("\nAvailable Agents:");
    println!("{:-<70}", "");
    
    let cli_agents: Vec<&Agent> = filtered.iter().filter(|a| a.agent_type == AgentType::CLI).collect();
    let desktop_agents: Vec<&Agent> = filtered.iter().filter(|a| a.agent_type == AgentType::Desktop).collect();
    
    if !cli_agents.is_empty() {
        println!("\n  CLI Agents ({}):", cli_agents.len());
        for agent in &cli_agents {
            let status = if agent.installed { "✓" } else { " " };
            println!("    [{}] {} - {}", status, agent.name, agent.description);
        }
    }
    
    if !desktop_agents.is_empty() {
        println!("\n  Desktop Agents ({}):", desktop_agents.len());
        for agent in &desktop_agents {
            let status = if agent.installed { "✓" } else { " " };
            println!("    [{}] {} - {}", status, agent.name, agent.description);
        }
    }
    
    println!("\n{:-<70}", "");
    println!("Total: {} agents ({} CLI, {} Desktop)", 
        filtered.len(), cli_agents.len(), desktop_agents.len());
}

fn search_agents(query: &str, agent_type: Option<String>) {
    let mut agents = get_known_agents();
    let query_lower = query.to_lowercase();

    check_installed(&mut agents);

    let filtered: Vec<&Agent> = agents
        .iter()
        .filter(|a| {
            (a.name.to_lowercase().contains(&query_lower)
                || a.description.to_lowercase().contains(&query_lower)
                || a.package_name.to_lowercase().contains(&query_lower))
                && match agent_type.as_deref() {
                    Some("cli") => a.agent_type == AgentType::CLI,
                    Some("desktop") => a.agent_type == AgentType::Desktop,
                    _ => true,
                }
        })
        .collect();

    if filtered.is_empty() {
        println!("No agents found matching '{}'", query);
    } else {
        println!("\nSearch results for '{}':", query);
        println!("{:-<70}", "");
        for agent in filtered {
            let status = if agent.installed { "✓" } else { " " };
            let type_label = match agent.agent_type {
                AgentType::CLI => "[CLI]",
                AgentType::Desktop => "[Desktop]",
            };
            println!("    [{}] {} {} - {}", status, type_label, agent.name, agent.description);
        }
    }
}

fn show_progress(step: &str, progress: u8, total_steps: u8) {
    let bar_width = 30;
    let filled = (progress as f64 / total_steps as f64 * bar_width as f64) as usize;
    let empty = bar_width - filled;
    
    print!("\r  [");
    for _ in 0..filled {
        print!("=");
    }
    for _ in 0..empty {
        print!("-");
    }
    print!("] {}/{} {}", progress, total_steps, step);
    io::stdout().flush().unwrap();
}

fn show_install_step(step: &str, step_num: u8, total_steps: u8) {
    println!("\n  Step {}/{}: {}", step_num, total_steps, step);
    show_progress("Starting...", step_num - 1, total_steps);
}

fn get_install_command(agent: &Agent) -> (String, Vec<String>) {
    match agent.manager.as_str() {
        "npm" => {
            #[cfg(target_os = "windows")]
            { ("cmd".to_string(), vec!["/C".to_string(), "npm".to_string(), "install".to_string(), "-g".to_string(), agent.package_name.clone()]) }
            #[cfg(not(target_os = "windows"))]
            { ("npm".to_string(), vec!["install".to_string(), "-g".to_string(), agent.package_name.clone()]) }
        }
        "pip" => ("pip".to_string(), vec!["install".to_string(), agent.package_name.clone()]),
        "cargo" => ("cargo".to_string(), vec!["install".to_string(), agent.package_name.clone()]),
        "winget" | "msstore" => {
            #[cfg(target_os = "windows")]
            { ("cmd".to_string(), vec!["/C".to_string(), "winget".to_string(), "install".to_string(), agent.package_name.clone(), "--accept-package-agreements".to_string(), "--accept-source-agreements".to_string()]) }
            #[cfg(not(target_os = "windows"))]
            { ("brew".to_string(), vec!["install".to_string(), "--cask".to_string(), agent.package_name.clone()]) }
        }
        _ => ("echo".to_string(), vec![format!("Unsupported package manager: {}", agent.manager)]),
    }
}

fn get_uninstall_command(agent: &Agent) -> (String, Vec<String>) {
    match agent.manager.as_str() {
        "npm" => {
            #[cfg(target_os = "windows")]
            { ("cmd".to_string(), vec!["/C".to_string(), "npm".to_string(), "uninstall".to_string(), "-g".to_string(), agent.package_name.clone()]) }
            #[cfg(not(target_os = "windows"))]
            { ("npm".to_string(), vec!["uninstall".to_string(), "-g".to_string(), agent.package_name.clone()]) }
        }
        "pip" => ("pip".to_string(), vec!["uninstall".to_string(), "-y".to_string(), agent.package_name.clone()]),
        "cargo" => ("cargo".to_string(), vec!["uninstall".to_string(), agent.package_name.clone()]),
        "winget" | "msstore" => {
            #[cfg(target_os = "windows")]
            { ("cmd".to_string(), vec!["/C".to_string(), "winget".to_string(), "uninstall".to_string(), agent.package_name.clone()]) }
            #[cfg(not(target_os = "windows"))]
            { ("brew".to_string(), vec!["uninstall".to_string(), "--cask".to_string(), agent.package_name.clone()]) }
        }
        _ => ("echo".to_string(), vec![format!("Unsupported package manager: {}", agent.manager)]),
    }
}

fn confirm_action(prompt: &str, auto_yes: bool) -> bool {
    if auto_yes {
        return true;
    }
    print!("{} [y/N] ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    input.trim().to_lowercase() == "y"
}

fn install_agent(names: &[String], dry_run: bool, yes: bool) {
    let agents = get_known_agents();
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for name in names {
        let agent = agents.iter().find(|a| &a.name == name);

        match agent {
            Some(agent) => {
                let (cmd, args) = get_install_command(agent);
                let cmd_display = format!("{} {}", cmd, args.join(" "));
                
                println!("\n🔧 Installing {}...", agent.name);
                println!("   Package: {}", agent.package_name);
                println!("   Manager: {}", agent.manager);
                println!("   Command: {}", cmd_display);
                
                if dry_run {
                    println!("   ⏭️  Dry run - command not executed");
                    success_count += 1;
                    continue;
                }
                
                if !confirm_action(&format!("   Proceed with installation of {}?", agent.name), yes) {
                    println!("   ⏭️  Skipped");
                    continue;
                }
                
                let total_steps = 3;
                
                show_install_step("Preparing installation...", 1, total_steps);
                std::thread::sleep(std::time::Duration::from_millis(200));
                show_progress("Prepared", 1, total_steps);
                println!();
                
                show_install_step("Downloading and installing...", 2, total_steps);
                
                let status = Command::new(&cmd).args(&args).status();

                match status {
                    Ok(status) => {
                        if status.success() {
                            show_progress("Completed", 3, total_steps);
                            println!();
                            println!("✅ {} installed successfully!", agent.name);
                            success_count += 1;
                        } else {
                            show_progress("Failed", 3, total_steps);
                            println!();
                            println!("❌ Failed to install {}", agent.name);
                            fail_count += 1;
                        }
                    }
                    Err(e) => {
                        show_progress("Error", 3, total_steps);
                        println!();
                        println!("❌ Error running installer: {}", e);
                        fail_count += 1;
                    }
                }
            }
            None => {
                println!("Agent '{}' not found", name);
                fail_count += 1;
            }
        }
        println!();
    }
    
    if names.len() > 1 {
        println!("Batch install complete: {} succeeded, {} failed", success_count, fail_count);
    }
}

fn uninstall_agent(names: &[String], dry_run: bool, yes: bool) {
    let agents = get_known_agents();
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for name in names {
        let agent = agents.iter().find(|a| &a.name == name);

        match agent {
            Some(agent) => {
                let (cmd, args) = get_uninstall_command(agent);
                let cmd_display = format!("{} {}", cmd, args.join(" "));
                
                println!("\n🔧 Uninstalling {}...", agent.name);
                println!("   Package: {}", agent.package_name);
                println!("   Manager: {}", agent.manager);
                println!("   Command: {}", cmd_display);
                
                if dry_run {
                    println!("   ⏭️  Dry run - command not executed");
                    success_count += 1;
                    continue;
                }
                
                if !confirm_action(&format!("   Proceed with uninstallation of {}?", agent.name), yes) {
                    println!("   ⏭️  Skipped");
                    continue;
                }
                
                let total_steps = 3;
                
                show_install_step("Preparing uninstallation...", 1, total_steps);
                std::thread::sleep(std::time::Duration::from_millis(200));
                show_progress("Prepared", 1, total_steps);
                println!();
                
                show_install_step("Removing package...", 2, total_steps);
                
                let status = Command::new(&cmd).args(&args).status();

                match status {
                    Ok(status) => {
                        if status.success() {
                            show_progress("Completed", 3, total_steps);
                            println!();
                            println!("✅ {} uninstalled successfully!", agent.name);
                            success_count += 1;
                        } else {
                            show_progress("Failed", 3, total_steps);
                            println!();
                            println!("❌ Failed to uninstall {}", agent.name);
                            fail_count += 1;
                        }
                    }
                    Err(e) => {
                        show_progress("Error", 3, total_steps);
                        println!();
                        println!("❌ Error running uninstaller: {}", e);
                        fail_count += 1;
                    }
                }
            }
            None => {
                println!("Agent '{}' not found", name);
                fail_count += 1;
            }
        }
        println!();
    }
    
    if names.len() > 1 {
        println!("Batch uninstall complete: {} succeeded, {} failed", success_count, fail_count);
    }
}

fn show_agent_info(name: &str) {
    let mut agents = get_known_agents();
    check_installed(&mut agents);
    
    let agent = agents.iter().find(|a| a.name == name);

    match agent {
        Some(agent) => {
            println!("\nAgent Information:");
            println!("{:-<40}", "");
            println!("Name:           {}", agent.name);
            println!("Description:    {}", agent.description);
            println!("Package:        {}", agent.package_name);
            println!("Manager:        {}", agent.manager);
            println!("Type:           {:?}", agent.agent_type);
            println!("Install Source: {}", agent.install_source);
            if !agent.download_url.is_empty() {
                println!("Download URL:   {}", agent.download_url);
            }
            println!(
                "Status:         {}",
                if agent.installed {
                    "Installed"
                } else {
                    "Not installed"
                }
            );
            
            // Show install command
            let (cmd, args) = get_install_command(agent);
            println!("Install Cmd:    {} {}", cmd, args.join(" "));
            
            // Show uninstall command
            let (cmd, args) = get_uninstall_command(agent);
            println!("Uninstall Cmd:  {} {}", cmd, args.join(" "));
        }
        None => println!("Agent '{}' not found", name),
    }
}

fn check_tool(name: &str, cmd: &str, args: &[&str]) -> bool {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout).to_string();
            let version_line = version.lines().next().unwrap_or("unknown").trim();
            println!("  ✅ {} - {}", name, version_line);
            true
        }
        Err(_) => {
            println!("  ❌ {} - not found", name);
            false
        }
    }
}

fn run_doctor() {
    println!("\n🏥 AgentHub Environment Check");
    println!("{:-<40}", "");
    
    let mut all_ok = true;
    
    // Check package managers
    println!("\nPackage Managers:");
    all_ok &= check_tool("npm", "npm", &["--version"]);
    all_ok &= check_tool("pip", "pip", &["--version"]);
    
    #[cfg(target_os = "windows")]
    { all_ok &= check_tool("winget", "cmd", &["/C", "winget", "--version"]); }
    
    #[cfg(target_os = "macos")]
    { all_ok &= check_tool("brew", "brew", &["--version"]); }
    
    // Check agents
    let agents = get_known_agents();
    let cli_count = agents.iter().filter(|a| a.agent_type == AgentType::CLI).count();
    let desktop_count = agents.iter().filter(|a| a.agent_type == AgentType::Desktop).count();
    
    println!("\nAgent Catalog:");
    println!("  📦 {} CLI agents", cli_count);
    println!("  🖥️  {} Desktop agents", desktop_count);
    println!("  📊 {} total agents", agents.len());
    
    // Check installed agents
    let mut agents_checked = agents.clone();
    check_installed(&mut agents_checked);
    let installed = agents_checked.iter().filter(|a| a.installed).count();
    
    println!("\nInstalled Agents:");
    println!("  ✅ {} agents installed", installed);
    
    if installed > 0 {
        println!("  Installed: {}", agents_checked.iter()
            .filter(|a| a.installed)
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", "));
    }
    
    println!("\n{:-<40}", "");
    if all_ok {
        println!("✅ Environment looks good!");
    } else {
        println!("⚠️  Some tools are missing. Install them for full functionality.");
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { r#type } => list_agents(r#type),
        Commands::Search { query, r#type } => search_agents(&query, r#type),
        Commands::Install { names, dry_run, yes } => install_agent(&names, dry_run, yes),
        Commands::Uninstall { names, dry_run, yes } => uninstall_agent(&names, dry_run, yes),
        Commands::Info { name } => show_agent_info(&name),
        Commands::Doctor => run_doctor(),
    }
}
