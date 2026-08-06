use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

/// Name of the append-only audit log file inside the audit directory.
pub const AUDIT_LOG_FILE: &str = "events.jsonl";

/// A single auditable operation record.
///
/// The audit log is append-only: every record carries a monotonic timestamp and
/// a unique id, and existing records are never mutated in place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    /// Who performed the action, e.g. `cli`, `gui` or `user:alice`.
    pub actor: String,
    /// Machine-readable action name, e.g. `install`, `config.set`, `backup.create`.
    pub action: String,
    /// The subject of the action, e.g. an agent id, config id or session id.
    pub target: String,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default = "default_true")]
    pub success: bool,
}

fn default_true() -> bool {
    true
}

/// Filter for [`AuditManager::query`]. All filters are ANDed together; `None`
/// fields are ignored.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Substring match on action (case-insensitive).
    pub action: Option<String>,
    /// Substring match on target (case-insensitive).
    pub target: Option<String>,
    /// Substring match on actor (case-insensitive).
    pub actor: Option<String>,
    /// Only events at or after this instant.
    pub since: Option<DateTime<Utc>>,
    /// Only events at or before this instant.
    pub until: Option<DateTime<Utc>>,
    /// Maximum number of results (most recent first).
    pub limit: Option<usize>,
}

/// Append-only JSONL audit log.
pub struct AuditManager {
    audit_dir: PathBuf,
}

impl AuditManager {
    pub fn new(audit_dir: PathBuf) -> Self {
        Self { audit_dir }
    }

    pub fn audit_dir(&self) -> &Path {
        &self.audit_dir
    }

    fn log_path(&self) -> PathBuf {
        self.audit_dir.join(AUDIT_LOG_FILE)
    }

    /// Append a new audit event and return it.
    pub fn record(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        details: Option<&str>,
        success: bool,
    ) -> Result<AuditEvent> {
        std::fs::create_dir_all(&self.audit_dir)
            .map_err(|e| AgentHubError::AuditError(format!("Failed to create audit dir: {}", e)))?;

        let event = AuditEvent {
            id: format!(
                "evt_{}_{}",
                Utc::now().timestamp_millis(),
                rand::random::<u32>()
            ),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            details: details.map(|s| s.to_string()),
            success,
        };

        let mut line = serde_json::to_string(&event).map_err(|e| {
            AgentHubError::AuditError(format!("Failed to serialize audit event: {}", e))
        })?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())
            .map_err(|e| AgentHubError::AuditError(format!("Failed to open audit log: {}", e)))?;

        file.write_all(line.as_bytes())
            .map_err(|e| AgentHubError::AuditError(format!("Failed to write audit log: {}", e)))?;

        Ok(event)
    }

    /// Load every recorded event (most recent first).
    pub fn load_all(&self) -> Result<Vec<AuditEvent>> {
        let path = self.log_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::AuditError(format!("Failed to read audit log: {}", e)))?;

        let mut events = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEvent>(line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse audit event on line {}: {}",
                        idx + 1,
                        e
                    );
                }
            }
        }

        events.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
        Ok(events)
    }

    /// Query events matching all provided filters, most recent first.
    pub fn query(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>> {
        let mut events = self.load_all()?;

        events.retain(|e| {
            if let Some(action) = &query.action {
                if !e.action.to_lowercase().contains(&action.to_lowercase()) {
                    return false;
                }
            }
            if let Some(target) = &query.target {
                if !e.target.to_lowercase().contains(&target.to_lowercase()) {
                    return false;
                }
            }
            if let Some(actor) = &query.actor {
                if !e.actor.to_lowercase().contains(&actor.to_lowercase()) {
                    return false;
                }
            }
            if let Some(since) = query.since {
                if e.timestamp < since {
                    return false;
                }
            }
            if let Some(until) = query.until {
                if e.timestamp > until {
                    return false;
                }
            }
            true
        });

        if let Some(limit) = query.limit {
            events.truncate(limit);
        }

        Ok(events)
    }

    /// Total number of recorded events.
    pub fn count(&self) -> Result<usize> {
        Ok(self.load_all()?.len())
    }

    /// Count events grouped by action, most frequent first.
    pub fn action_counts(&self, limit: usize) -> Result<Vec<(String, usize)>> {
        let events = self.load_all()?;
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for event in &events {
            *counts.entry(event.action.clone()).or_default() += 1;
        }

        let mut items: Vec<(String, usize)> = counts.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        items.truncate(limit);
        Ok(items)
    }

    /// Erase the entire audit log.
    pub fn clear(&self) -> Result<()> {
        let path = self.log_path();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::AuditError(format!("Failed to clear audit log: {}", e))
            })?;
        }
        Ok(())
    }

    /// Bulk-append events (used by backup restore). Returns the number of events imported.
    pub fn import_events(&self, events: &[AuditEvent]) -> Result<usize> {
        std::fs::create_dir_all(&self.audit_dir)
            .map_err(|e| AgentHubError::AuditError(format!("Failed to create audit dir: {}", e)))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())
            .map_err(|e| AgentHubError::AuditError(format!("Failed to open audit log: {}", e)))?;

        for event in events {
            let mut line = serde_json::to_string(event).map_err(|e| {
                AgentHubError::AuditError(format!("Failed to serialize audit event: {}", e))
            })?;
            line.push('\n');
            file.write_all(line.as_bytes()).map_err(|e| {
                AgentHubError::AuditError(format!("Failed to write audit log: {}", e))
            })?;
        }

        Ok(events.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (AuditManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = AuditManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_record_and_load() {
        let (manager, _temp) = create_test_manager();

        manager
            .record("cli", "install", "claude-code", Some("dry_run=false"), true)
            .unwrap();
        manager
            .record("cli", "uninstall", "codex", None, false)
            .unwrap();

        let events = manager.load_all().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].action, "uninstall");
        assert!(!events[0].success);
        assert_eq!(events[1].action, "install");
        assert!(events[1].details.is_some());
    }

    #[test]
    fn test_query_filters() {
        let (manager, _temp) = create_test_manager();

        manager
            .record("cli", "install", "claude-code", None, true)
            .unwrap();
        manager
            .record("cli", "config.set", "claude-code", None, true)
            .unwrap();
        manager
            .record("gui", "install", "codex", None, true)
            .unwrap();

        let query = AuditQuery {
            action: Some("install".to_string()),
            ..Default::default()
        };
        let events = manager.query(&query).unwrap();
        assert_eq!(events.len(), 2);

        let query = AuditQuery {
            action: Some("install".to_string()),
            actor: Some("gui".to_string()),
            ..Default::default()
        };
        let events = manager.query(&query).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, "codex");

        let query = AuditQuery {
            limit: Some(2),
            ..Default::default()
        };
        let events = manager.query(&query).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_count_and_action_counts() {
        let (manager, _temp) = create_test_manager();

        manager.record("cli", "install", "a", None, true).unwrap();
        manager.record("cli", "install", "b", None, true).unwrap();
        manager
            .record("cli", "config.set", "c", None, true)
            .unwrap();

        assert_eq!(manager.count().unwrap(), 3);

        let counts = manager.action_counts(10).unwrap();
        assert_eq!(counts[0], ("install".to_string(), 2));
        assert_eq!(counts[1], ("config.set".to_string(), 1));
    }

    #[test]
    fn test_clear() {
        let (manager, _temp) = create_test_manager();

        manager.record("cli", "install", "a", None, true).unwrap();
        assert_eq!(manager.count().unwrap(), 1);

        manager.clear().unwrap();
        assert_eq!(manager.count().unwrap(), 0);
    }

    #[test]
    fn test_import_events() {
        let (manager, _temp) = create_test_manager();

        let events = vec![
            AuditEvent {
                id: "evt_1".to_string(),
                timestamp: Utc::now(),
                actor: "cli".to_string(),
                action: "install".to_string(),
                target: "a".to_string(),
                details: None,
                success: true,
            },
            AuditEvent {
                id: "evt_2".to_string(),
                timestamp: Utc::now(),
                actor: "gui".to_string(),
                action: "backup.restore".to_string(),
                target: "all".to_string(),
                details: None,
                success: true,
            },
        ];

        let imported = manager.import_events(&events).unwrap();
        assert_eq!(imported, 2);
        assert_eq!(manager.count().unwrap(), 2);
    }

    #[test]
    fn test_query_since_until() {
        let (manager, _temp) = create_test_manager();

        manager.record("cli", "install", "a", None, true).unwrap();

        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);

        let query = AuditQuery {
            since: Some(past),
            until: Some(future),
            ..Default::default()
        };
        assert_eq!(manager.query(&query).unwrap().len(), 1);

        let query = AuditQuery {
            since: Some(future),
            ..Default::default()
        };
        assert_eq!(manager.query(&query).unwrap().len(), 0);
    }
}
