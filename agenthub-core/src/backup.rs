use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::audit::{AuditEvent, AuditManager};
use crate::config::{AgentConfig, ConfigManager};
use crate::error::{AgentHubError, Result};
use crate::graph::KnowledgeGraph;
use crate::memory::{MemoryEntry, MemoryManager};
use crate::prompt::{PromptManager, PromptTemplate};
use crate::session::{Session, SessionManager, SessionTemplate};
use crate::workflow::{Workflow, WorkflowManager};

/// Current backup file format version.
pub const BACKUP_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCounts {
    pub configs: usize,
    pub prompts: usize,
    pub prompt_versions: usize,
    pub sessions: usize,
    pub session_templates: usize,
    pub memories: usize,
    #[serde(default)]
    pub workflows: usize,
    #[serde(default)]
    pub memory_graph: bool,
    pub audit_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub format_version: u32,
    pub created_at: DateTime<Utc>,
    pub agenthub_version: String,
    pub counts: BackupCounts,
}

/// A complete snapshot of all user data managed by AgentHub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupData {
    pub manifest: BackupManifest,
    pub configs: Vec<AgentConfig>,
    pub prompts: Vec<PromptTemplate>,
    /// Historical prompt snapshots keyed by prompt id.
    #[serde(default)]
    pub prompt_versions: HashMap<String, Vec<PromptTemplate>>,
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub session_templates: Vec<SessionTemplate>,
    pub memories: Vec<MemoryEntry>,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
    #[serde(default)]
    pub memory_graph: Option<KnowledgeGraph>,
    #[serde(default)]
    pub audit_events: Vec<AuditEvent>,
}

/// Creates and restores whole-workspace backups.
///
/// The base directory matches the layout used by the other managers, i.e. the
/// AgentHub config directory (configs under `agents/`, prompts under `prompts/`,
/// sessions under `sessions/`, memories under `memory/`, audit under `audit/`).
pub struct BackupManager {
    base_dir: PathBuf,
}

impl BackupManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Snapshot all user data into a single JSON backup file.
    pub fn create_backup(&self, output_path: &Path) -> Result<BackupManifest> {
        let config_manager = ConfigManager::new(self.base_dir.clone());
        let prompt_manager = PromptManager::new(self.base_dir.join("prompts"));
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let memory_manager = MemoryManager::new(self.base_dir.join("memory"));
        let audit_manager = AuditManager::new(self.base_dir.join("audit"));

        let mut configs = Vec::new();
        for id in config_manager.list_configs()? {
            if let Ok(config) = config_manager.load_config(&id) {
                configs.push(config);
            }
        }

        let prompts = prompt_manager.list_prompts()?;
        let mut prompt_versions: HashMap<String, Vec<PromptTemplate>> = HashMap::new();
        for prompt in &prompts {
            let versions = prompt_manager.list_versions(&prompt.id)?;
            if !versions.is_empty() {
                prompt_versions.insert(prompt.id.clone(), versions);
            }
        }

        let sessions = session_manager.list_sessions()?;
        let session_templates = session_manager.list_templates()?;
        let memories = memory_manager.list_entries(None)?;
        let workflows = WorkflowManager::new(self.base_dir.join("skills")).list_workflows()?;
        let memory_graph = memory_manager.load_graph().ok();
        let audit_events = audit_manager.load_all()?;

        let counts = BackupCounts {
            configs: configs.len(),
            prompts: prompts.len(),
            prompt_versions: prompt_versions.values().map(|v| v.len()).sum(),
            sessions: sessions.len(),
            session_templates: session_templates.len(),
            memories: memories.len(),
            workflows: workflows.len(),
            memory_graph: memory_graph.is_some(),
            audit_events: audit_events.len(),
        };

        let summary = format!(
            "configs={}, prompts={}, prompt_versions={}, sessions={}, session_templates={}, memories={}, audit={}",
            counts.configs,
            counts.prompts,
            counts.prompt_versions,
            counts.sessions,
            counts.session_templates,
            counts.memories,
            counts.audit_events
        );

        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            created_at: Utc::now(),
            agenthub_version: env!("CARGO_PKG_VERSION").to_string(),
            counts,
        };

        let data = BackupData {
            manifest,
            configs,
            prompts,
            prompt_versions,
            sessions,
            session_templates,
            memories,
            workflows,
            memory_graph,
            audit_events,
        };

        // Note: secret VALUES are intentionally never included in backups; only
        // key names live in the config files themselves. Restoring a backup
        // does not recreate keystore entries.

        let content = serde_json::to_string_pretty(&data).map_err(|e| {
            AgentHubError::BackupError(format!("Failed to serialize backup: {}", e))
        })?;

        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AgentHubError::BackupError(format!("Failed to create output dir: {}", e))
                })?;
            }
        }

        std::fs::write(output_path, content)
            .map_err(|e| AgentHubError::BackupError(format!("Failed to write backup: {}", e)))?;

        audit_manager.record(
            "cli",
            "backup.create",
            output_path.to_string_lossy().as_ref(),
            Some(&summary),
            true,
        )?;

        Ok(data.manifest)
    }

    /// Restore all user data from a backup file. The audit log is replaced by
    /// the backup's events, then a `backup.restore` event is appended.
    pub fn restore_backup(&self, input_path: &Path) -> Result<BackupManifest> {
        let content = std::fs::read_to_string(input_path)
            .map_err(|e| AgentHubError::BackupError(format!("Failed to read backup: {}", e)))?;

        let data: BackupData = serde_json::from_str(&content)
            .map_err(|e| AgentHubError::BackupError(format!("Failed to parse backup: {}", e)))?;

        if data.manifest.format_version > BACKUP_FORMAT_VERSION {
            return Err(AgentHubError::BackupError(format!(
                "Backup format version {} is newer than supported version {}",
                data.manifest.format_version, BACKUP_FORMAT_VERSION
            )));
        }

        let config_manager = ConfigManager::new(self.base_dir.clone());
        let prompt_manager = PromptManager::new(self.base_dir.join("prompts"));
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let memory_manager = MemoryManager::new(self.base_dir.join("memory"));
        let audit_manager = AuditManager::new(self.base_dir.join("audit"));

        for config in &data.configs {
            config_manager.save_config(config)?;
        }
        for prompt in &data.prompts {
            prompt_manager.save_prompt(prompt)?;
        }
        for (id, versions) in &data.prompt_versions {
            prompt_manager.import_versions(id, versions)?;
        }
        for session in &data.sessions {
            session_manager.save_session(session)?;
        }
        for template in &data.session_templates {
            session_manager.save_template(template)?;
        }
        for memory in &data.memories {
            memory_manager.save_entry(memory)?;
        }
        let workflow_manager = WorkflowManager::new(self.base_dir.join("skills"));
        for workflow in &data.workflows {
            workflow_manager.save_workflow(workflow)?;
        }
        if let Some(graph) = &data.memory_graph {
            std::fs::create_dir_all(memory_manager.memory_dir()).map_err(|e| {
                AgentHubError::BackupError(format!("Failed to create memory dir: {}", e))
            })?;
            std::fs::write(
                memory_manager.memory_dir().join("graph.json"),
                graph.to_json()?,
            )
            .map_err(|e| AgentHubError::BackupError(format!("Failed to write graph: {}", e)))?;
        }

        // Replace the audit log with the backup's events.
        audit_manager.clear()?;
        audit_manager.import_events(&data.audit_events)?;

        let summary = format!(
            "configs={}, prompts={}, sessions={}, session_templates={}, memories={}, audit={}",
            data.configs.len(),
            data.prompts.len(),
            data.sessions.len(),
            data.session_templates.len(),
            data.memories.len(),
            data.audit_events.len()
        );
        audit_manager.record(
            "cli",
            "backup.restore",
            input_path.to_string_lossy().as_ref(),
            Some(&summary),
            true,
        )?;

        Ok(data.manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigValue;
    use crate::memory::{MemoryScope, MemoryType};
    use tempfile::TempDir;

    fn populate(base_dir: &Path) {
        let config_manager = ConfigManager::new(base_dir.to_path_buf());
        config_manager.create_config("agent-a").unwrap();
        config_manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();

        let prompt_manager = PromptManager::new(base_dir.join("prompts"));
        prompt_manager
            .create_prompt("review", "Review", "desc", "review {{code}}")
            .unwrap();
        prompt_manager
            .update_prompt("review", None, None, Some("review v2 {{code}}"))
            .unwrap();

        let session_manager = SessionManager::new(base_dir.join("sessions"));
        let session = session_manager.create_session("S1", "codex").unwrap();
        session_manager
            .add_message(&session.id, "user", "hi")
            .unwrap();
        session_manager
            .create_template("tpl", "T", "", None, Vec::new(), Vec::new())
            .unwrap();

        let memory_manager = MemoryManager::new(base_dir.join("memory"));
        memory_manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Note",
                "content",
                MemoryType::Free,
            )
            .unwrap();

        let audit_manager = AuditManager::new(base_dir.join("audit"));
        audit_manager
            .record("cli", "install", "agent-a", None, true)
            .unwrap();

        let workflow_manager = WorkflowManager::new(base_dir.join("skills"));
        workflow_manager
            .create_workflow(
                "ci",
                "CI",
                "checks",
                vec![crate::workflow::WorkflowStep {
                    skill: "rust-dev".to_string(),
                    args: std::collections::HashMap::new(),
                    optional: false,
                }],
            )
            .unwrap();
    }

    #[test]
    fn test_create_backup_manifest_counts() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("data");
        populate(&base);

        let manager = BackupManager::new(base.clone());
        let out = temp.path().join("backup.json");
        let manifest = manager.create_backup(&out).unwrap();

        assert_eq!(manifest.format_version, BACKUP_FORMAT_VERSION);
        assert_eq!(manifest.counts.configs, 1);
        assert_eq!(manifest.counts.prompts, 1);
        assert_eq!(manifest.counts.prompt_versions, 1);
        assert_eq!(manifest.counts.sessions, 1);
        assert_eq!(manifest.counts.memories, 1);
        assert_eq!(manifest.counts.workflows, 1);
        assert_eq!(manifest.counts.audit_events, 1);

        // Backup operation itself is audited
        let audit = AuditManager::new(base.join("audit"));
        assert_eq!(audit.count().unwrap(), 2);
        assert!(out.exists());
    }

    #[test]
    fn test_restore_backup_roundtrip() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        populate(&source);

        let backup_path = temp.path().join("backup.json");
        BackupManager::new(source.clone())
            .create_backup(&backup_path)
            .unwrap();

        // Restore into a fresh directory
        let target = temp.path().join("target");
        let manifest = BackupManager::new(target.clone())
            .restore_backup(&backup_path)
            .unwrap();
        assert_eq!(manifest.counts.configs, 1);

        // Config restored
        let config_manager = ConfigManager::new(target.clone());
        let config = config_manager.load_config("agent-a").unwrap();
        assert_eq!(
            config.settings.get("model").unwrap().as_str(),
            Some("gpt-4o")
        );

        // Prompt + version history restored
        let prompt_manager = PromptManager::new(target.join("prompts"));
        let prompt = prompt_manager.get_prompt("review").unwrap();
        assert_eq!(prompt.version, 2);
        assert_eq!(prompt_manager.list_versions("review").unwrap().len(), 1);

        // Session + template restored
        let session_manager = SessionManager::new(target.join("sessions"));
        assert_eq!(session_manager.list_sessions().unwrap().len(), 1);
        assert_eq!(session_manager.list_templates().unwrap().len(), 1);

        // Memory restored
        let memory_manager = MemoryManager::new(target.join("memory"));
        assert_eq!(memory_manager.list_entries(None).unwrap().len(), 1);

        // Audit replaced + restore event appended
        let audit_manager = AuditManager::new(target.join("audit"));
        let events = audit_manager.load_all().unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().any(|e| e.action == "backup.restore"));
    }

    #[test]
    fn test_restore_rejects_newer_format() {
        let temp = TempDir::new().unwrap();
        let backup_path = temp.path().join("future.json");

        let data = BackupData {
            manifest: BackupManifest {
                format_version: BACKUP_FORMAT_VERSION + 1,
                created_at: Utc::now(),
                agenthub_version: "9.9.9".to_string(),
                counts: BackupCounts {
                    configs: 0,
                    prompts: 0,
                    prompt_versions: 0,
                    sessions: 0,
                    session_templates: 0,
                    memories: 0,
                    workflows: 0,
                    memory_graph: false,
                    audit_events: 0,
                },
            },
            configs: Vec::new(),
            prompts: Vec::new(),
            prompt_versions: HashMap::new(),
            sessions: Vec::new(),
            session_templates: Vec::new(),
            memories: Vec::new(),
            workflows: Vec::new(),
            memory_graph: None,
            audit_events: Vec::new(),
        };
        std::fs::write(&backup_path, serde_json::to_string_pretty(&data).unwrap()).unwrap();

        let manager = BackupManager::new(temp.path().join("target"));
        let result = manager.restore_backup(&backup_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_missing_file() {
        let temp = TempDir::new().unwrap();
        let manager = BackupManager::new(temp.path().to_path_buf());
        assert!(manager
            .restore_backup(&temp.path().join("nope.json"))
            .is_err());
    }
}
