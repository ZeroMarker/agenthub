use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::agent::Platform;
use crate::catalog::Catalog;
use crate::diagnostic::{DiagnosticManager, DiagnosticReport};
use crate::error::{AgentHubError, Result};
use crate::session::{BudgetReport, SessionManager};
use crate::skill::SkillManager;
use crate::status::{AgentStatus, StatusDetector};

/// Alert severity derived from a monitor report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for AlertSeverity {
    type Err = AgentHubError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(AlertSeverity::Info),
            "warning" | "warn" => Ok(AlertSeverity::Warning),
            "critical" | "crit" => Ok(AlertSeverity::Critical),
            _ => Err(AgentHubError::ManagementError(format!(
                "Invalid severity: {} (expected info|warning|critical)",
                s
            ))),
        }
    }
}

/// A point-in-time health/monitoring report across all modules.
///
/// This is the first slice of the cross-cutting monitoring capability: it
/// aggregates diagnostics, installed-agent status, cost budget alerts and skill
/// version compatibility into a single pass/fail report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorReport {
    pub generated_at: DateTime<Utc>,
    pub agenthub_version: String,
    pub platform: String,
    /// True when there are no warnings.
    pub healthy: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub installed_agents: usize,
    /// Verified catalog agents not detected as installed.
    #[serde(default)]
    pub missing_agents: Vec<String>,
    pub budget: BudgetReport,
    /// Skills whose `min_agenthub_version` is not satisfied.
    #[serde(default)]
    pub incompatible_skills: Vec<String>,
    pub diagnostics_passed: usize,
    pub diagnostics_warnings: usize,
    pub diagnostics_failed: usize,
}

impl MonitorReport {
    /// Serialize the report as pretty JSON (for cron/systemd integration and
    /// machine-readable alerting).
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to serialize report: {}", e))
        })
    }

    /// One-line alert summary, empty when healthy. Suitable for watch/cron logs.
    pub fn alert_summary(&self) -> String {
        if self.healthy {
            format!(
                "OK: {} installed agents, budget within limits",
                self.installed_agents
            )
        } else {
            format!(
                "WARN ({}): {}",
                self.warnings.len(),
                self.warnings.join("; ")
            )
        }
    }

    /// Derived alert severity: critical when diagnostics fail or the budget is
    /// exceeded; warning when anything else is flagged; otherwise info.
    pub fn severity(&self) -> AlertSeverity {
        if self.diagnostics_failed > 0
            || self.budget.alerts.iter().any(|a| a.contains("exceeded"))
            || !self.incompatible_skills.is_empty()
        {
            AlertSeverity::Critical
        } else if !self.healthy {
            AlertSeverity::Warning
        } else {
            AlertSeverity::Info
        }
    }
}

/// Cross-cutting monitor: runs the checks that feed the report.
pub struct Monitor {
    base_dir: PathBuf,
    platform: Platform,
}

impl Monitor {
    pub fn new(base_dir: PathBuf, platform: Platform) -> Self {
        Self { base_dir, platform }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Run the full monitoring pass (diagnostics + status + budget + skills).
    pub fn run(&self, catalog: &Catalog) -> Result<MonitorReport> {
        let mut diagnostic_manager = DiagnosticManager::new();
        let diagnostics = diagnostic_manager.run_all_checks();
        let statuses = StatusDetector::new(self.platform).check_agents(catalog.agents());
        self.assemble(catalog, &diagnostics, &statuses)
    }

    /// Assemble a report from pre-computed inputs (no subprocess calls; used by
    /// tests and callers that already have the data).
    pub fn assemble(
        &self,
        catalog: &Catalog,
        diagnostics: &DiagnosticReport,
        statuses: &[AgentStatus],
    ) -> Result<MonitorReport> {
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let skill_manager = SkillManager::new(self.base_dir.join("skills"));

        let mut warnings: Vec<String> = Vec::new();

        if diagnostics.summary.failed > 0 {
            warnings.push(format!(
                "{} diagnostic check(s) failed",
                diagnostics.summary.failed
            ));
        }

        let installed_agents = statuses.iter().filter(|s| s.installed).count();
        let missing_agents: Vec<String> = catalog
            .agents()
            .iter()
            .filter(|a| {
                a.status == crate::agent::SupportStatus::Verified
                    && !statuses.iter().any(|s| s.agent_id == a.id && s.installed)
            })
            .map(|a| a.name.clone())
            .collect();
        if !missing_agents.is_empty() {
            warnings.push(format!(
                "{} verified agent(s) not installed",
                missing_agents.len()
            ));
        }

        let budget = session_manager.check_budget(Utc::now())?;
        for alert in &budget.alerts {
            warnings.push(alert.clone());
        }

        let incompatible_skills: Vec<String> = skill_manager
            .check_all_compatibility()?
            .into_iter()
            .filter(|c| !c.compatible)
            .map(|c| c.skill)
            .collect();
        if !incompatible_skills.is_empty() {
            warnings.push(format!(
                "{} skill(s) require a newer AgentHub",
                incompatible_skills.len()
            ));
        }

        Ok(MonitorReport {
            generated_at: Utc::now(),
            agenthub_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: format!("{:?}", self.platform),
            healthy: warnings.is_empty(),
            warnings,
            installed_agents,
            missing_agents,
            budget,
            incompatible_skills,
            diagnostics_passed: diagnostics.summary.passed,
            diagnostics_warnings: diagnostics.summary.warnings,
            diagnostics_failed: diagnostics.summary.failed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::config::ConfigManager;
    use crate::diagnostic::{CheckStatus, DiagnosticCheck, DiagnosticSummary, SystemInfo};
    use tempfile::TempDir;

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
                    "windows": { "manager": "manual", "package": null }
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
                    "windows": { "manager": "manual", "package": null }
                },
                "status": "community",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": null
            }
        ]
    }"#;

    fn diag(passed: usize, failed: usize) -> DiagnosticReport {
        DiagnosticReport {
            timestamp: "2026-08-06T00:00:00Z".to_string(),
            platform: "Linux".to_string(),
            checks: vec![DiagnosticCheck {
                name: "catalog".to_string(),
                category: "catalog".to_string(),
                status: if failed > 0 {
                    CheckStatus::Failed
                } else {
                    CheckStatus::Passed
                },
                message: "ok".to_string(),
                details: None,
                duration_ms: 0,
            }],
            summary: DiagnosticSummary {
                total: passed + failed,
                passed,
                warnings: 0,
                failed,
                skipped: 0,
                duration_ms: 0,
            },
            system_info: SystemInfo {
                os: "Linux".to_string(),
                arch: "x86_64".to_string(),
                hostname: "test".to_string(),
                rust_version: None,
                node_version: None,
                npm_version: None,
                cargo_version: None,
            },
        }
    }

    fn statuses() -> Vec<AgentStatus> {
        vec![
            AgentStatus {
                agent_id: "test-cli".to_string(),
                installed: true,
                version: Some("1.2.3".to_string()),
                detection_method: "npm".to_string(),
            },
            AgentStatus {
                agent_id: "test-desktop".to_string(),
                installed: false,
                version: None,
                detection_method: "winget".to_string(),
            },
        ]
    }

    #[test]
    fn test_monitor_healthy() {
        let temp = TempDir::new().unwrap();
        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();
        let monitor = Monitor::new(temp.path().to_path_buf(), Platform::Linux);

        let report = monitor
            .assemble(&catalog, &diag(5, 0), &statuses())
            .unwrap();

        assert!(report.healthy);
        assert!(report.warnings.is_empty());
        assert_eq!(report.installed_agents, 1);
        // test-desktop is community -> not flagged as missing
        assert!(report.missing_agents.is_empty());
    }

    #[test]
    fn test_monitor_flags_issues() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().to_path_buf();
        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();

        // Diagnostics failing
        let failing = diag(0, 2);

        // Verified agent not installed
        let statuses = vec![AgentStatus {
            agent_id: "test-cli".to_string(),
            installed: false,
            version: None,
            detection_method: "manual".to_string(),
        }];

        // Budget exceeded today
        ConfigManager::new(base.clone()).create_config("x").unwrap();
        let sm = SessionManager::new(base.join("sessions"));
        let session = sm.create_session("Cost", "codex").unwrap();
        sm.set_model(&session.id, "gpt-4o-mini").unwrap();
        sm.record_usage(
            &session.id,
            40_000_000,
            0,
            &crate::session::PricingTable::builtin(),
        )
        .unwrap(); // $6.00
        sm.set_budget(&crate::session::BudgetConfig {
            daily_usd: Some(5.0),
            monthly_usd: None,
        })
        .unwrap();

        // Incompatible skill
        let skill_dir = base.join("skills").join("installed").join("old-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: old-skill\ndescription: \"x\"\nversion: 1.0.0\nmin_agenthub_version: 99.0.0\n---\n\n# x\n",
        )
        .unwrap();

        let monitor = Monitor::new(base, Platform::Linux);
        let report = monitor.assemble(&catalog, &failing, &statuses).unwrap();

        assert!(!report.healthy);
        assert_eq!(report.diagnostics_failed, 2);
        assert!(report.missing_agents.contains(&"Test CLI".to_string()));
        assert!(report
            .budget
            .alerts
            .iter()
            .any(|a| a.contains("Daily budget exceeded")));
        assert!(report
            .incompatible_skills
            .contains(&"old-skill".to_string()));
        assert!(report.warnings.len() >= 4);
    }
}
