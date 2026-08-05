use agenthub_core::{Agent, Catalog, DiagnosticManager, Installer, Platform, RealCommandRunner};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Clap CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "agenthub", about = "AI coding agent management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all agents
    List {
        /// Filter by type: cli or desktop
        #[arg(long)]
        kind: Option<String>,
    },
    /// Search agents by query
    Search {
        /// Search query
        query: String,
        /// Filter by type: cli or desktop
        #[arg(long)]
        kind: Option<String>,
    },
    /// Show agent details
    Info {
        /// Agent name or ID
        name: String,
    },
    /// Install an agent
    Install {
        /// Agent name or ID
        name: String,
        /// Preview installation without executing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Uninstall an agent
    Uninstall {
        /// Agent name or ID
        name: String,
        /// Preview uninstallation without executing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Run environment diagnostics
    Doctor,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn get_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Linux
    }
}

pub fn load_catalog() -> Result<Catalog, String> {
    let paths = [
        PathBuf::from("agents.json"),
        PathBuf::from("../agents.json"),
    ];

    for path in &paths {
        if path.exists() {
            return Catalog::from_file(path).map_err(|e| e.to_string());
        }
    }

    Err("Could not find agents.json".to_string())
}

// ---------------------------------------------------------------------------
// Command implementations (return strings for testability)
// ---------------------------------------------------------------------------

pub fn cmd_list(kind: Option<String>, catalog: &Catalog) -> String {
    let agents = match kind.as_deref() {
        Some("cli") => catalog.filter_by_kind(agenthub_core::AgentKind::CLI),
        Some("desktop") => catalog.filter_by_kind(agenthub_core::AgentKind::Desktop),
        _ => catalog.agents().iter().collect(),
    };

    if agents.is_empty() {
        return "No agents found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<20} {:<25} {:<8} {:<20} {:<10}\n",
        "ID", "Name", "Type", "Provider", "Status"
    ));
    out.push_str(&format!("{}\n", "-".repeat(85)));

    for agent in &agents {
        out.push_str(&format!(
            "{:<20} {:<25} {:<8} {:<20} {:<10}\n",
            agent.id,
            agent.name,
            format!("{:?}", agent.kind),
            agent.provider,
            format!("{:?}", agent.status)
        ));
    }

    let (cli, desktop) = catalog.count_by_kind();
    out.push_str(&format!(
        "\nTotal: {} agents ({} CLI, {} Desktop)",
        agents.len(),
        cli,
        desktop
    ));
    out
}

pub fn cmd_search(query: &str, kind: Option<String>, catalog: &Catalog) -> String {
    let results = match kind.as_deref() {
        Some("cli") => catalog
            .search(query)
            .into_iter()
            .filter(|a| a.kind == agenthub_core::AgentKind::CLI)
            .collect::<Vec<_>>(),
        Some("desktop") => catalog
            .search(query)
            .into_iter()
            .filter(|a| a.kind == agenthub_core::AgentKind::Desktop)
            .collect::<Vec<_>>(),
        _ => catalog.search(query),
    };

    if results.is_empty() {
        return format!("No agents found matching '{}'", query);
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<20} {:<25} {:<8} {:<20}\n",
        "ID", "Name", "Type", "Provider"
    ));
    out.push_str(&format!("{}\n", "-".repeat(75)));

    for agent in &results {
        out.push_str(&format!(
            "{:<20} {:<25} {:<8} {:<20}\n",
            agent.id,
            agent.name,
            format!("{:?}", agent.kind),
            agent.provider
        ));
    }
    out.push_str(&format!("\nFound {} agent(s)", results.len()));
    out
}

pub fn cmd_info(name: &str, catalog: &Catalog) -> String {
    let agent = match catalog.find_by_name(name) {
        Some(a) => a,
        None => return format!("Agent '{}' not found", name),
    };

    let mut out = String::new();
    out.push_str(&format!("Name:        {}\n", agent.name));
    out.push_str(&format!("ID:          {}\n", agent.id));
    out.push_str(&format!("Type:        {:?}\n", agent.kind));
    out.push_str(&format!("Provider:    {}\n", agent.provider));
    out.push_str(&format!("Description: {}\n", agent.description));
    out.push_str(&format!("Homepage:    {}\n", agent.homepage));
    out.push_str(&format!("Status:      {:?}\n", agent.status));

    if let Some(date) = agent.catalog_verified_at {
        out.push_str(&format!("Catalog Verified: {}\n", date));
    }
    if let Some(date) = agent.installer_verified_at {
        out.push_str(&format!("Installer Verified: {}\n", date));
    }

    out.push_str("\nInstallers:\n");
    let platform = get_platform();
    for (p, config) in &agent.installers {
        let current = if *p == platform { " (current)" } else { "" };
        let pkg = config.package.as_deref().unwrap_or("N/A");
        out.push_str(&format!(
            "  {:?}{}: {:?} {}\n",
            p, current, config.manager, pkg
        ));
    }
    out
}

pub fn cmd_install(_name: &str, dry_run: bool, agent: &Agent, platform: Platform) -> String {
    let runner = RealCommandRunner::new(platform);
    let installer = Installer::new(platform, Box::new(runner));

    let preview = match installer.get_command_preview(agent, false) {
        Some(cmd) => format!(
            "Will execute: {}\nDescription:  {}",
            cmd.command, cmd.description
        ),
        None => return format!("No installer available for {} on this platform", agent.name),
    };

    let result = match installer.execute_install(agent, dry_run, None) {
        Ok(r) => r,
        Err(e) => return format!("Installation failed: {}", e),
    };

    if result.success {
        format!(
            "{}\n\n✅ {} installed successfully ({})",
            preview, agent.name, result.duration_ms
        )
    } else {
        let mut out = format!("{}\n\n❌ {} installation failed", preview, agent.name);
        if !result.stderr.is_empty() {
            out.push_str(&format!("\n{}", result.stderr));
        }
        out
    }
}

pub fn cmd_uninstall(_name: &str, dry_run: bool, agent: &Agent, platform: Platform) -> String {
    let runner = RealCommandRunner::new(platform);
    let installer = Installer::new(platform, Box::new(runner));

    let preview = match installer.get_command_preview(agent, true) {
        Some(cmd) => format!(
            "Will execute: {}\nDescription:  {}",
            cmd.command, cmd.description
        ),
        None => {
            return format!(
                "No uninstaller available for {} on this platform",
                agent.name
            )
        }
    };

    let result = match installer.execute_uninstall(agent, dry_run, None) {
        Ok(r) => r,
        Err(e) => return format!("Uninstallation failed: {}", e),
    };

    if result.success {
        format!(
            "{}\n\n✅ {} uninstalled successfully ({})",
            preview, agent.name, result.duration_ms
        )
    } else {
        let mut out = format!("{}\n\n❌ {} uninstallation failed", preview, agent.name);
        if !result.stderr.is_empty() {
            out.push_str(&format!("\n{}", result.stderr));
        }
        out
    }
}

pub fn cmd_doctor() -> String {
    let mut manager = DiagnosticManager::new();
    let report = manager.run_all_checks();

    format!(
        "{}\n\nSummary: {} passed, {} warnings, {} failed",
        DiagnosticManager::format_report(&report),
        report.summary.passed,
        report.summary.warnings,
        report.summary.failed
    )
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();
    let platform = get_platform();

    let result = match cli.command {
        Commands::List { kind } => match load_catalog() {
            Ok(catalog) => cmd_list(kind, &catalog),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Search { query, kind } => match load_catalog() {
            Ok(catalog) => cmd_search(&query, kind, &catalog),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Info { name } => match load_catalog() {
            Ok(catalog) => cmd_info(&name, &catalog),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Install { name, dry_run, yes } => {
            let catalog = match load_catalog() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let agent = match catalog.find_by_name(&name) {
                Some(a) => a.clone(),
                None => {
                    eprintln!("Agent '{}' not found", name);
                    std::process::exit(1);
                }
            };
            if !yes && !dry_run {
                print!("\nProceed with installation? [y/N]: ");
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                if input.trim().to_lowercase() != "y" {
                    println!("Installation cancelled.");
                    return;
                }
            }
            cmd_install(&name, dry_run, &agent, platform)
        }
        Commands::Uninstall { name, dry_run, yes } => {
            let catalog = match load_catalog() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let agent = match catalog.find_by_name(&name) {
                Some(a) => a.clone(),
                None => {
                    eprintln!("Agent '{}' not found", name);
                    std::process::exit(1);
                }
            };
            if !yes && !dry_run {
                print!("\nProceed with uninstallation? [y/N]: ");
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                if input.trim().to_lowercase() != "y" {
                    println!("Uninstallation cancelled.");
                    return;
                }
            }
            cmd_uninstall(&name, dry_run, &agent, platform)
        }
        Commands::Doctor => cmd_doctor(),
    };

    println!("{}", result);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use agenthub_core::Catalog;

    fn test_catalog() -> Catalog {
        Catalog::from_json(TEST_AGENTS_JSON).unwrap()
    }

    #[test]
    fn test_get_platform_returns_platform() {
        let platform = get_platform();
        // Should always return a valid platform variant
        match platform {
            Platform::Windows | Platform::MacOS | Platform::Linux => {}
        }
    }

    #[test]
    fn test_cmd_list_all_agents() {
        let catalog = test_catalog();
        let output = cmd_list(None, &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Test Desktop"));
        assert!(output.contains("Total:"));
    }

    #[test]
    fn test_cmd_list_cli_agents() {
        let catalog = test_catalog();
        let output = cmd_list(Some("cli".to_string()), &catalog);
        assert!(output.contains("Test CLI"));
        assert!(!output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_list_desktop_agents() {
        let catalog = test_catalog();
        let output = cmd_list(Some("desktop".to_string()), &catalog);
        assert!(output.contains("Test Desktop"));
        assert!(!output.contains("Test CLI"));
    }

    #[test]
    fn test_cmd_list_empty_kind() {
        let catalog = test_catalog();
        let output = cmd_list(Some("".to_string()), &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_search_found() {
        let catalog = test_catalog();
        let output = cmd_search("CLI", None, &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Found"));
    }

    #[test]
    fn test_cmd_search_not_found() {
        let catalog = test_catalog();
        let output = cmd_search("nonexistent", None, &catalog);
        assert!(output.contains("No agents found"));
    }

    #[test]
    fn test_cmd_search_case_insensitive() {
        let catalog = test_catalog();
        let output = cmd_search("test cli", None, &catalog);
        assert!(output.contains("Test CLI"));
    }

    #[test]
    fn test_cmd_search_by_provider() {
        let catalog = test_catalog();
        let output = cmd_search("Test Provider", None, &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_search_filter_cli() {
        let catalog = test_catalog();
        let output = cmd_search("test", Some("cli".to_string()), &catalog);
        assert!(output.contains("Test CLI"));
        assert!(!output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_info_found() {
        let catalog = test_catalog();
        let output = cmd_info("Test CLI", &catalog);
        assert!(output.contains("Name:        Test CLI"));
        assert!(output.contains("ID:          test-cli"));
        assert!(output.contains("test-cli.com"));
    }

    #[test]
    fn test_cmd_info_not_found() {
        let catalog = test_catalog();
        let output = cmd_info("nonexistent", &catalog);
        assert!(output.contains("not found"));
    }

    #[test]
    fn test_cmd_info_by_id() {
        let catalog = test_catalog();
        let output = cmd_info("test-cli", &catalog);
        assert!(output.contains("test-cli"));
    }

    #[test]
    fn test_cmd_doctor_returns_report() {
        let output = cmd_doctor();
        assert!(output.contains("Summary:"));
        assert!(output.contains("passed"));
    }

    #[test]
    fn test_load_catalog_from_project_root() {
        // Running from project root, agents.json should be found
        let result = load_catalog();
        assert!(
            result.is_ok(),
            "expected catalog to load: {:?}",
            result.err()
        );
    }

    const TEST_AGENTS_JSON: &str = r#"{
        "version": "1.0.0",
        "last_updated": "2026-06-27",
        "agents": [
            {
                "id": "test-cli",
                "name": "Test CLI",
                "kind": "cli",
                "provider": "Test Provider",
                "description": "A test CLI agent",
                "homepage": "https://test-cli.com",
                "installers": {
                    "windows": { "manager": "npm", "package": "@test/cli" }
                },
                "status": "verified",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": "2026-06-27"
            },
            {
                "id": "test-desktop",
                "name": "Test Desktop",
                "kind": "desktop",
                "provider": "Test Provider",
                "description": "A test desktop agent",
                "homepage": "https://test-desktop.com",
                "installers": {
                    "windows": { "manager": "winget", "package": "Test.Desktop" }
                },
                "status": "community",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": null
            }
        ]
    }"#;
}
