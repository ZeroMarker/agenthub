use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::error::{AgentHubError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Passed => write!(f, "✅ Passed"),
            CheckStatus::Warning => write!(f, "⚠️ Warning"),
            CheckStatus::Failed => write!(f, "❌ Failed"),
            CheckStatus::Skipped => write!(f, "⏭️ Skipped"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub category: String,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub timestamp: String,
    pub platform: String,
    pub checks: Vec<DiagnosticCheck>,
    pub summary: DiagnosticSummary,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub total: usize,
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub rust_version: Option<String>,
    pub node_version: Option<String>,
    pub npm_version: Option<String>,
    pub cargo_version: Option<String>,
}

pub struct DiagnosticManager {
    checks: Vec<DiagnosticCheck>,
    start_time: Instant,
    /// Injectable tool probe so tests can simulate missing/failing tools.
    tool_probe: Option<fn(&str, &[&str]) -> Option<String>>,
}

impl Default for DiagnosticManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticManager {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            start_time: Instant::now(),
            tool_probe: None,
        }
    }

    fn probe_tool(&self, cmd: &str, args: &[&str]) -> Option<String> {
        match self.tool_probe {
            Some(probe) => probe(cmd, args),
            None => Self::get_tool_version(cmd, args),
        }
    }

    pub fn run_all_checks(&mut self) -> DiagnosticReport {
        self.check_system_info();
        self.check_package_managers();
        self.check_rust_toolchain();
        self.check_node_toolchain();
        self.check_catalog_integrity();
        self.check_config_directory();
        self.check_skills_directory();
        self.check_disk_space();
        self.check_network_connectivity();

        self.build_report()
    }

    fn add_check(&mut self, check: DiagnosticCheck) {
        self.checks.push(check);
    }

    fn run_check<F>(&mut self, name: &str, category: &str, f: F)
    where
        F: FnOnce() -> (CheckStatus, String, Option<String>),
    {
        let start = Instant::now();
        let (status, message, details) = f();
        let duration = start.elapsed();

        self.add_check(DiagnosticCheck {
            name: name.to_string(),
            category: category.to_string(),
            status,
            message,
            details,
            duration_ms: duration.as_millis() as u64,
        });
    }

    fn check_system_info(&mut self) {
        self.run_check("Operating System", "system", || {
            let os = std::env::consts::OS;
            let arch = std::env::consts::ARCH;
            (CheckStatus::Passed, format!("{} {}", os, arch), None)
        });

        self.run_check("Hostname", "system", || {
            hostname::get()
                .map(|h| (CheckStatus::Passed, h.to_string_lossy().to_string(), None))
                .unwrap_or_else(|_| {
                    (
                        CheckStatus::Warning,
                        "Could not detect hostname".to_string(),
                        None,
                    )
                })
        });
    }

    fn check_package_managers(&mut self) {
        // npm
        let npm = self.probe_tool("npm", &["--version"]);
        self.run_check("npm", "package_manager", move || match npm {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Failed,
                "npm not found".to_string(),
                Some("Install Node.js from https://nodejs.org".to_string()),
            ),
        });

        // pip
        let pip = self.probe_tool("pip", &["--version"]);
        self.run_check("pip", "package_manager", move || match pip {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Warning,
                "pip not found".to_string(),
                Some("Install Python from https://python.org".to_string()),
            ),
        });

        // winget (Windows only)
        if cfg!(target_os = "windows") {
            let winget = self.probe_tool("cmd", &["/C", "winget", "--version"]);
            self.run_check("winget", "package_manager", move || match winget {
                Some(version) => (CheckStatus::Passed, version, None),
                None => (
                    CheckStatus::Warning,
                    "winget not found".to_string(),
                    Some("Install App Installer from Microsoft Store".to_string()),
                ),
            });
        }

        // brew (macOS only)
        if cfg!(target_os = "macos") {
            let brew = self.probe_tool("brew", &["--version"]);
            self.run_check("brew", "package_manager", move || match brew {
                Some(version) => (CheckStatus::Passed, version, None),
                None => (
                    CheckStatus::Warning,
                    "brew not found".to_string(),
                    Some("Install from https://brew.sh".to_string()),
                ),
            });
        }
    }

    fn check_rust_toolchain(&mut self) {
        let rustc = self.probe_tool("rustc", &["--version"]);
        self.run_check("Rust Compiler", "toolchain", move || match rustc {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Warning,
                "rustc not found".to_string(),
                Some("Install from https://rustup.rs".to_string()),
            ),
        });

        let cargo = self.probe_tool("cargo", &["--version"]);
        self.run_check("Cargo", "toolchain", move || match cargo {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Warning,
                "cargo not found".to_string(),
                Some("Install Rust from https://rustup.rs".to_string()),
            ),
        });

        let clippy = self.probe_tool("cargo", &["clippy", "--version"]);
        self.run_check("Clippy", "toolchain", move || match clippy {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Warning,
                "clippy not found".to_string(),
                Some("Run: rustup component add clippy".to_string()),
            ),
        });

        let rustfmt = self.probe_tool("rustfmt", &["--version"]);
        self.run_check("rustfmt", "toolchain", move || match rustfmt {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Warning,
                "rustfmt not found".to_string(),
                Some("Run: rustup component add rustfmt".to_string()),
            ),
        });
    }

    fn check_node_toolchain(&mut self) {
        let node = self.probe_tool("node", &["--version"]);
        self.run_check("Node.js", "toolchain", move || match node {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Failed,
                "node not found".to_string(),
                Some("Install from https://nodejs.org".to_string()),
            ),
        });

        let tsc = self.probe_tool("npx", &["tsc", "--version"]);
        self.run_check("TypeScript", "toolchain", move || match tsc {
            Some(version) => (CheckStatus::Passed, version, None),
            None => (
                CheckStatus::Warning,
                "TypeScript not found".to_string(),
                Some("Run: npm install -g typescript".to_string()),
            ),
        });
    }

    fn check_catalog_integrity(&mut self) {
        self.run_check("Agent Catalog", "catalog", || {
            let catalog_paths = vec![
                PathBuf::from("agents.json"),
                PathBuf::from("../agents.json"),
                PathBuf::from("../../agents.json"),
            ];

            for path in &catalog_paths {
                if path.exists() {
                    match std::fs::read_to_string(path) {
                        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(json) => {
                                if let Some(agents) = json.get("agents").and_then(|a| a.as_array())
                                {
                                    return (
                                        CheckStatus::Passed,
                                        format!(
                                            "{} agents found at {}",
                                            agents.len(),
                                            path.display()
                                        ),
                                        None,
                                    );
                                } else {
                                    return (
                                        CheckStatus::Warning,
                                        format!("Invalid catalog format at {}", path.display()),
                                        Some("Missing 'agents' array".to_string()),
                                    );
                                }
                            }
                            Err(e) => {
                                return (
                                    CheckStatus::Failed,
                                    format!("Invalid JSON at {}", path.display()),
                                    Some(e.to_string()),
                                );
                            }
                        },
                        Err(e) => {
                            return (
                                CheckStatus::Failed,
                                format!("Cannot read {}", path.display()),
                                Some(e.to_string()),
                            );
                        }
                    }
                }
            }

            (
                CheckStatus::Failed,
                "agents.json not found".to_string(),
                Some("Create agents.json in project root".to_string()),
            )
        });
    }

    fn check_config_directory(&mut self) {
        self.run_check("Config Directory", "storage", || {
            let config_dir = Self::get_config_dir();
            if config_dir.exists() {
                match std::fs::read_dir(&config_dir) {
                    Ok(entries) => {
                        let count = entries.count();
                        (
                            CheckStatus::Passed,
                            format!("{} configs in {}", count, config_dir.display()),
                            None,
                        )
                    }
                    Err(e) => (
                        CheckStatus::Warning,
                        format!("Cannot read config dir: {}", e),
                        None,
                    ),
                }
            } else {
                (
                    CheckStatus::Skipped,
                    "Config directory not created yet".to_string(),
                    None,
                )
            }
        });
    }

    fn check_skills_directory(&mut self) {
        self.run_check("Skills Directory", "storage", || {
            let skills_dir = Self::get_skills_dir();
            if skills_dir.exists() {
                match std::fs::read_dir(skills_dir.join("installed")) {
                    Ok(entries) => {
                        let count = entries.count();
                        (
                            CheckStatus::Passed,
                            format!("{} skills installed in {}", count, skills_dir.display()),
                            None,
                        )
                    }
                    Err(_) => (
                        CheckStatus::Skipped,
                        "No skills installed yet".to_string(),
                        None,
                    ),
                }
            } else {
                (
                    CheckStatus::Skipped,
                    "Skills directory not created yet".to_string(),
                    None,
                )
            }
        });
    }

    fn check_disk_space(&mut self) {
        self.run_check("Disk Space", "system", || match std::fs::metadata(".") {
            Ok(_) => (CheckStatus::Passed, "Disk accessible".to_string(), None),
            Err(e) => (
                CheckStatus::Warning,
                format!("Cannot check disk: {}", e),
                None,
            ),
        });
    }

    fn check_network_connectivity(&mut self) {
        self.run_check("Network", "connectivity", || {
            let start = Instant::now();
            let result = Command::new("ping")
                .args(if cfg!(target_os = "windows") {
                    vec!["-n", "1", "-w", "3000", "8.8.8.8"]
                } else {
                    vec!["-c", "1", "-W", "3", "8.8.8.8"]
                })
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();

            let duration = start.elapsed();
            match result {
                Ok(status) if status.success() => (
                    CheckStatus::Passed,
                    format!("Connected ({}ms)", duration.as_millis()),
                    None,
                ),
                Ok(_) => (
                    CheckStatus::Warning,
                    "Network unreachable".to_string(),
                    Some("Check internet connection".to_string()),
                ),
                Err(e) => (CheckStatus::Warning, format!("Ping failed: {}", e), None),
            }
        });
    }

    fn build_report(&self) -> DiagnosticReport {
        let passed = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Passed)
            .count();
        let warnings = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warning)
            .count();
        let failed = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Failed)
            .count();
        let skipped = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Skipped)
            .count();

        DiagnosticReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
            checks: self.checks.clone(),
            summary: DiagnosticSummary {
                total: self.checks.len(),
                passed,
                warnings,
                failed,
                skipped,
                duration_ms: self.start_time.elapsed().as_millis() as u64,
            },
            system_info: self.gather_system_info(),
        }
    }

    fn gather_system_info(&self) -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            rust_version: self.probe_tool("rustc", &["--version"]),
            node_version: self.probe_tool("node", &["--version"]),
            npm_version: self.probe_tool("npm", &["--version"]),
            cargo_version: self.probe_tool("cargo", &["--version"]),
        }
    }

    pub fn get_tool_version(cmd: &str, args: &[&str]) -> Option<String> {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
    }

    fn get_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agenthub")
    }

    fn get_skills_dir() -> PathBuf {
        Self::get_config_dir().join("skills")
    }

    pub fn format_report(report: &DiagnosticReport) -> String {
        let mut output = String::new();

        output.push_str("\n🏥 AgentHub Diagnostic Report\n");
        output.push_str(&format!("{:=<60}\n", ""));
        output.push_str(&format!("Platform: {}\n", report.platform));
        output.push_str(&format!("Timestamp: {}\n", report.timestamp));
        output.push('\n');

        // Group checks by category
        let mut categories: HashMap<String, Vec<&DiagnosticCheck>> = HashMap::new();
        for check in &report.checks {
            categories
                .entry(check.category.clone())
                .or_default()
                .push(check);
        }

        for (category, checks) in &categories {
            output.push_str(&format!("{}:\n", Self::format_category_name(category)));
            for check in checks {
                output.push_str(&format!(
                    "  {} {} - {}\n",
                    check.status, check.name, check.message
                ));
                if let Some(details) = &check.details {
                    output.push_str(&format!("      💡 {}\n", details));
                }
            }
            output.push('\n');
        }

        output.push_str(&format!("{:=<60}\n", ""));
        output.push_str("Summary:\n");
        output.push_str(&format!(
            "  ✅ Passed: {} | ⚠️ Warnings: {} | ❌ Failed: {} | ⏭️ Skipped: {}\n",
            report.summary.passed,
            report.summary.warnings,
            report.summary.failed,
            report.summary.skipped
        ));
        output.push_str(&format!(
            "  Total checks: {} | Duration: {}ms\n",
            report.summary.total, report.summary.duration_ms
        ));

        if report.summary.failed > 0 {
            output.push_str("\n❌ Some checks failed. Please fix the issues above.\n");
        } else if report.summary.warnings > 0 {
            output.push_str("\n⚠️ Some warnings detected. Review recommended.\n");
        } else {
            output.push_str("\n✅ All checks passed!\n");
        }

        output
    }

    fn format_category_name(category: &str) -> String {
        match category {
            "system" => "🖥️  System",
            "package_manager" => "📦 Package Managers",
            "toolchain" => "🔧 Toolchain",
            "catalog" => "📋 Catalog",
            "storage" => "💾 Storage",
            "connectivity" => "🌐 Connectivity",
            _ => category,
        }
        .to_string()
    }

    pub fn export_report(report: &DiagnosticReport, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(report).map_err(|e| {
            AgentHubError::DiagnosticError(format!("Failed to serialize report: {}", e))
        })?;

        std::fs::write(path, json).map_err(|e| {
            AgentHubError::DiagnosticError(format!("Failed to write report: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_manager_creation() {
        let manager = DiagnosticManager::new();
        assert!(manager.checks.is_empty());
    }

    #[test]
    fn test_run_check() {
        let mut manager = DiagnosticManager::new();
        manager.run_check("Test Check", "test", || {
            (CheckStatus::Passed, "Test passed".to_string(), None)
        });

        assert_eq!(manager.checks.len(), 1);
        assert_eq!(manager.checks[0].name, "Test Check");
        assert_eq!(manager.checks[0].status, CheckStatus::Passed);
    }

    #[test]
    fn test_check_status_display() {
        assert_eq!(format!("{}", CheckStatus::Passed), "✅ Passed");
        assert_eq!(format!("{}", CheckStatus::Warning), "⚠️ Warning");
        assert_eq!(format!("{}", CheckStatus::Failed), "❌ Failed");
        assert_eq!(format!("{}", CheckStatus::Skipped), "⏭️ Skipped");
    }

    #[test]
    fn test_get_tool_version() {
        // This should work on most systems
        let version = DiagnosticManager::get_tool_version("cargo", &["--version"]);
        assert!(version.is_some());
    }

    #[test]
    fn test_build_report() {
        let mut manager = DiagnosticManager::new();
        manager.run_check("Check 1", "test", || {
            (CheckStatus::Passed, "OK".to_string(), None)
        });
        manager.run_check("Check 2", "test", || {
            (CheckStatus::Warning, "Warning".to_string(), None)
        });

        let report = manager.build_report();
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.warnings, 1);
    }

    #[test]
    fn test_format_report() {
        let mut manager = DiagnosticManager::new();
        manager.run_check("Test", "system", || {
            (CheckStatus::Passed, "OK".to_string(), None)
        });

        let report = manager.build_report();
        let formatted = DiagnosticManager::format_report(&report);
        assert!(formatted.contains("Diagnostic Report"));
        assert!(formatted.contains("Test"));
    }

    #[test]
    fn test_run_all_checks_smoke() {
        // Runs every check including real tool probes (rustc/cargo/node/npm),
        // catalog lookup, config/skills dirs, disk and network. Must never
        // panic and must produce a coherent report.
        let mut manager = DiagnosticManager::new();
        let report = manager.run_all_checks();

        assert!(!report.checks.is_empty());
        assert!(!report.platform.is_empty());
        assert!(!report.system_info.os.is_empty());
        assert!(!report.system_info.arch.is_empty());
        assert_eq!(report.summary.total, report.checks.len());
        assert_eq!(
            report.summary.passed + report.summary.warnings + report.summary.failed,
            report.summary.total - report.summary.skipped
        );
        // Every check carries a status and a message.
        for check in &report.checks {
            assert!(!check.name.is_empty());
            assert!(!check.message.is_empty());
        }
    }

    #[test]
    fn test_run_all_checks_covers_all_categories() {
        let mut manager = DiagnosticManager::new();
        let report = manager.run_all_checks();
        let categories: Vec<&str> = report.checks.iter().map(|c| c.category.as_str()).collect();
        for wanted in [
            "system",
            "package_manager",
            "toolchain",
            "catalog",
            "storage",
        ] {
            assert!(
                categories.contains(&wanted),
                "missing category {wanted} in {:?}",
                categories
            );
        }
    }

    #[test]
    fn test_get_tool_version_missing_tool_returns_none() {
        assert_eq!(
            DiagnosticManager::get_tool_version("agenthub-no-such-tool-xyz", &["--version"]),
            None
        );
    }

    #[test]
    fn test_get_tool_version_failing_command_returns_none() {
        // `cargo` exists but the bogus flag makes the command fail.
        assert_eq!(
            DiagnosticManager::get_tool_version("cargo", &["--definitely-not-a-flag"]),
            None
        );
    }

    #[test]
    fn test_run_all_checks_with_no_tools_degrades() {
        // Simulate every tool probe failing: npm/node must fail, pip/rust
        // family must warn, and the report must stay coherent.
        let mut manager = DiagnosticManager::new();
        manager.tool_probe = Some(|_, _| None);
        let report = manager.run_all_checks();

        assert_eq!(report.summary.total, report.checks.len());
        let npm = report
            .checks
            .iter()
            .find(|c| c.name == "npm")
            .expect("npm check");
        assert_eq!(npm.status, CheckStatus::Failed);
        assert!(npm.message.contains("not found"));
        let rustc = report
            .checks
            .iter()
            .find(|c| c.name == "Rust Compiler")
            .expect("rustc check");
        assert_eq!(rustc.status, CheckStatus::Warning);
        // System-info probes degrade to None, never panic.
        assert!(report.system_info.rust_version.is_none());
    }

    #[test]
    fn test_default_constructs_empty_manager() {
        let manager = DiagnosticManager::default();
        assert!(manager.checks.is_empty());
    }

    #[test]
    fn test_export_report_writes_json() {
        let temp = tempfile::tempdir().unwrap();
        let mut manager = DiagnosticManager::new();
        manager.run_check("Test", "system", || {
            (
                CheckStatus::Passed,
                "OK".to_string(),
                Some("detail".to_string()),
            )
        });
        let report = manager.build_report();

        let out = temp.path().join("report.json");
        DiagnosticManager::export_report(&report, &out).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(parsed["summary"]["total"], 1);
    }

    #[test]
    fn test_format_report_summary_variants() {
        let mut manager = DiagnosticManager::new();
        manager.run_check("A", "catalog", || {
            (
                CheckStatus::Failed,
                "bad".to_string(),
                Some("fix it".to_string()),
            )
        });
        manager.run_check("B", "connectivity", || {
            (CheckStatus::Warning, "warn".to_string(), None)
        });
        manager.run_check("C", "storage", || {
            (CheckStatus::Skipped, "skip".to_string(), None)
        });
        let report = manager.build_report();

        let formatted = DiagnosticManager::format_report(&report);
        assert!(formatted.contains("📋 Catalog"));
        assert!(formatted.contains("🌐 Connectivity"));
        assert!(formatted.contains("💾 Storage"));
        assert!(formatted.contains("💡 fix it"));
        assert!(formatted.contains("Some checks failed"));

        // All-passed variant.
        let mut manager = DiagnosticManager::new();
        manager.run_check("A", "system", || {
            (CheckStatus::Passed, "ok".to_string(), None)
        });
        let report = manager.build_report();
        assert!(DiagnosticManager::format_report(&report).contains("All checks passed"));

        // Warning-only variant.
        let mut manager = DiagnosticManager::new();
        manager.run_check("A", "system", || {
            (CheckStatus::Warning, "warn".to_string(), None)
        });
        let report = manager.build_report();
        assert!(DiagnosticManager::format_report(&report).contains("warnings detected"));
    }
}
