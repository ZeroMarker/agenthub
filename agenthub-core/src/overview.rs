use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::agent::{AgentKind, Platform, SupportStatus};
use crate::audit::AuditManager;
use crate::catalog::Catalog;
use crate::config::ConfigManager;
use crate::error::Result;
use crate::memory::{MemoryManager, MemoryStats};
use crate::prompt::PromptManager;
use crate::session::{SessionManager, SessionStats};
use crate::skill::SkillManager;
use crate::status::{AgentStatus, StatusDetector};

/// Catalog-level counts for the status overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogOverview {
    pub total: usize,
    pub cli: usize,
    pub desktop: usize,
    pub verified: usize,
    pub community: usize,
    pub manual: usize,
    pub deprecated: usize,
}

/// A point-in-time snapshot of the whole AgentHub workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusOverview {
    pub generated_at: DateTime<Utc>,
    pub platform: String,
    pub agenthub_version: String,
    pub catalog: CatalogOverview,
    /// Number of catalog agents detected as installed on this machine.
    pub installed_agents: usize,
    pub configs: usize,
    pub prompts: usize,
    pub sessions: SessionStats,
    pub memories: MemoryStats,
    pub skills_total: usize,
    pub skills_enabled: usize,
    pub audit_events: usize,
}

/// One day bucket in a [`TrendReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    /// UTC date `YYYY-MM-DD`.
    pub date: String,
    pub sessions_started: usize,
    pub sessions_completed: usize,
    pub tokens: u64,
    pub cost_usd: f64,
    pub memories_created: usize,
    pub audit_events: usize,
}

/// Aggregates data across every manager to produce a unified overview
/// (dashboard / `agenthub status`).
pub struct OverviewReport {
    base_dir: PathBuf,
    platform: Platform,
}

impl OverviewReport {
    pub fn new(base_dir: PathBuf, platform: Platform) -> Self {
        Self { base_dir, platform }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Build an overview, detecting installed agents with [`StatusDetector`].
    pub fn overview(&self, catalog: &Catalog) -> Result<StatusOverview> {
        let statuses = StatusDetector::new(self.platform).check_agents(catalog.agents());
        self.overview_with_status(catalog, &statuses)
    }

    /// Build an overview from pre-computed agent statuses (no subprocess calls;
    /// used by tests and by callers that already have status data).
    pub fn overview_with_status(
        &self,
        catalog: &Catalog,
        statuses: &[AgentStatus],
    ) -> Result<StatusOverview> {
        let config_manager = ConfigManager::new(self.base_dir.clone());
        let prompt_manager = PromptManager::new(self.base_dir.join("prompts"));
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let memory_manager = MemoryManager::new(self.base_dir.join("memory"));
        let skill_manager = SkillManager::new(self.base_dir.join("skills"));
        let audit_manager = AuditManager::new(self.base_dir.join("audit"));

        let agents = catalog.agents();
        let catalog_overview = CatalogOverview {
            total: agents.len(),
            cli: agents.iter().filter(|a| a.kind == AgentKind::CLI).count(),
            desktop: agents
                .iter()
                .filter(|a| a.kind == AgentKind::Desktop)
                .count(),
            verified: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Verified)
                .count(),
            community: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Community)
                .count(),
            manual: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Manual)
                .count(),
            deprecated: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Deprecated)
                .count(),
        };

        let skills = skill_manager.list_skills()?;

        Ok(StatusOverview {
            generated_at: Utc::now(),
            platform: format!("{:?}", self.platform),
            agenthub_version: env!("CARGO_PKG_VERSION").to_string(),
            catalog: catalog_overview,
            installed_agents: statuses.iter().filter(|s| s.installed).count(),
            configs: config_manager.list_configs()?.len(),
            prompts: prompt_manager.list_prompts()?.len(),
            sessions: session_manager.get_stats()?,
            memories: memory_manager.get_stats()?,
            skills_total: skills.len(),
            skills_enabled: skills.iter().filter(|s| s.enabled).count(),
            audit_events: audit_manager.count()?,
        })
    }

    /// Daily trend for the last `days` days (oldest first, includes today),
    /// bucketed by UTC date.
    pub fn trend(&self, days: usize) -> Result<Vec<TrendPoint>> {
        self.trend_with_now(days, Utc::now())
    }

    pub fn trend_with_now(&self, days: usize, now: DateTime<Utc>) -> Result<Vec<TrendPoint>> {
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let memory_manager = MemoryManager::new(self.base_dir.join("memory"));
        let audit_manager = AuditManager::new(self.base_dir.join("audit"));

        let today = now.date_naive();
        let mut points: Vec<TrendPoint> = (0..days)
            .map(|i| {
                let date = today - chrono::Duration::days((days - 1 - i) as i64);
                TrendPoint {
                    date: date.to_string(),
                    sessions_started: 0,
                    sessions_completed: 0,
                    tokens: 0,
                    cost_usd: 0.0,
                    memories_created: 0,
                    audit_events: 0,
                }
            })
            .collect();
        let index: std::collections::HashMap<String, usize> = points
            .iter()
            .enumerate()
            .map(|(i, p)| (p.date.clone(), i))
            .collect();

        for session in session_manager.list_sessions()? {
            if let Some(&i) = index.get(&session.started_at.date_naive().to_string()) {
                points[i].sessions_started += 1;
                if session.status == crate::session::SessionStatus::Completed {
                    points[i].sessions_completed += 1;
                }
                if let Some(usage) = &session.usage {
                    points[i].tokens += usage.total_tokens as u64;
                    points[i].cost_usd += usage.estimated_cost_usd;
                }
            }
        }

        for memory in memory_manager.list_entries(None)? {
            if let Some(&i) = index.get(&memory.created_at.date_naive().to_string()) {
                points[i].memories_created += 1;
            }
        }

        for event in audit_manager.load_all()? {
            if let Some(&i) = index.get(&event.timestamp.date_naive().to_string()) {
                points[i].audit_events += 1;
            }
        }

        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
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

    #[test]
    fn test_overview_with_status() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().to_path_buf();

        // Populate some data
        ConfigManager::new(base.clone())
            .create_config("test-cli")
            .unwrap();
        PromptManager::new(base.join("prompts"))
            .create_prompt("p1", "P1", "d", "t")
            .unwrap();
        SessionManager::new(base.join("sessions"))
            .create_session("S1", "codex")
            .unwrap();
        MemoryManager::new(base.join("memory"))
            .create_entry(
                crate::memory::MemoryScope::Global,
                None,
                "Note",
                "c",
                crate::memory::MemoryType::Free,
            )
            .unwrap();
        AuditManager::new(base.join("audit"))
            .record("cli", "install", "test-cli", None, true)
            .unwrap();

        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();
        let report = OverviewReport::new(base, Platform::Windows);

        let statuses = vec![
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
        ];

        let overview = report.overview_with_status(&catalog, &statuses).unwrap();
        assert_eq!(overview.catalog.total, 2);
        assert_eq!(overview.catalog.cli, 1);
        assert_eq!(overview.catalog.desktop, 1);
        assert_eq!(overview.catalog.verified, 1);
        assert_eq!(overview.catalog.community, 1);
        assert_eq!(overview.installed_agents, 1);
        assert_eq!(overview.configs, 1);
        assert_eq!(overview.prompts, 1);
        assert_eq!(overview.sessions.total, 1);
        assert_eq!(overview.memories.total, 1);
        assert_eq!(overview.audit_events, 1);
        assert_eq!(overview.platform, "Windows");
    }

    #[test]
    fn test_overview_empty_workspace() {
        let temp = TempDir::new().unwrap();
        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();
        let report = OverviewReport::new(temp.path().to_path_buf(), Platform::Linux);

        let overview = report.overview_with_status(&catalog, &[]).unwrap();
        assert_eq!(overview.installed_agents, 0);
        assert_eq!(overview.configs, 0);
        assert_eq!(overview.sessions.total, 0);
        assert_eq!(overview.audit_events, 0);
    }

    // ---- Trend ----

    #[test]
    fn test_trend_buckets_by_day() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().to_path_buf();
        let report = OverviewReport::new(base.clone(), Platform::Linux);
        let now = Utc::now();

        // Sessions started today and 2 days ago
        let sm = SessionManager::new(base.join("sessions"));
        let today_session = sm.create_session("Today", "codex").unwrap();
        sm.record_usage(
            &today_session.id,
            1_000_000,
            0,
            &crate::session::PricingTable::builtin(),
        )
        .unwrap();
        sm.update_status(&today_session.id, crate::session::SessionStatus::Completed)
            .unwrap();

        let old = now - chrono::Duration::days(2);
        let mut old_session = sm.create_session("Old", "codex").unwrap();
        old_session.started_at = old;
        old_session.usage = Some(crate::session::SessionUsage {
            total_tokens: 500_000,
            input_tokens: 500_000,
            output_tokens: 0,
            estimated_cost_usd: 1.0,
        });
        sm.save_session(&old_session).unwrap();

        // Memory created today
        MemoryManager::new(base.join("memory"))
            .create_entry(
                crate::memory::MemoryScope::Global,
                None,
                "Note",
                "c",
                crate::memory::MemoryType::Free,
            )
            .unwrap();

        // Audit event today
        AuditManager::new(base.join("audit"))
            .record("cli", "install", "x", None, true)
            .unwrap();

        let points = report.trend_with_now(7, now).unwrap();
        assert_eq!(points.len(), 7);

        let today_idx = 6; // last bucket = today
        assert_eq!(points[today_idx].sessions_started, 1);
        assert_eq!(points[today_idx].sessions_completed, 1);
        assert!((points[today_idx].cost_usd - 3.0).abs() < 1e-9); // fallback pricing $3/1M
        assert_eq!(points[today_idx].memories_created, 1);
        assert_eq!(points[today_idx].audit_events, 1);

        // 2 days ago: old session
        let old_idx = 4; // 7 buckets: idx 0..6, old is 2 days before today -> idx 4
        assert_eq!(points[old_idx].sessions_started, 1);
        assert!((points[old_idx].cost_usd - 1.0).abs() < 1e-9);
        assert_eq!(points[old_idx].tokens, 500_000);

        // Other days empty
        assert_eq!(points[0].sessions_started, 0);
    }
}
