use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template: String,
    #[serde(default)]
    pub variables: Vec<PromptVariable>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    /// Number of times the prompt has been rendered.
    #[serde(default)]
    pub usage_count: u64,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Usage statistics for a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUsage {
    pub id: String,
    pub name: String,
    pub usage_count: u64,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// A single historical snapshot of a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    pub version: u32,
    pub name: String,
    pub description: String,
    pub template: String,
    pub updated_at: Option<DateTime<Utc>>,
}

pub struct PromptManager {
    prompts_dir: PathBuf,
}

impl PromptManager {
    pub fn new(prompts_dir: PathBuf) -> Self {
        Self { prompts_dir }
    }

    pub fn prompts_dir(&self) -> &Path {
        &self.prompts_dir
    }

    fn templates_dir(&self) -> PathBuf {
        self.prompts_dir.join("templates")
    }

    fn versions_dir(&self, id: &str) -> PathBuf {
        self.templates_dir().join("versions").join(id)
    }

    fn version_path(&self, id: &str, version: u32) -> PathBuf {
        self.versions_dir(id).join(format!("v{}.yaml", version))
    }

    /// Snapshot the current version of a prompt before it is modified.
    fn snapshot_current(&self, prompt: &PromptTemplate) -> Result<()> {
        let dir = self.versions_dir(&prompt.id);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create versions dir: {}", e))
        })?;

        let path = self.version_path(&prompt.id, prompt.version);
        let content = serde_yaml::to_string(prompt).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to serialize version: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to write version: {}", e)))?;

        Ok(())
    }

    /// List the historical snapshots of a prompt, oldest first.
    pub fn list_versions(&self, id: &str) -> Result<Vec<PromptTemplate>> {
        let dir = self.versions_dir(id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to read versions dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| AgentHubError::PromptError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                if let Ok(version) = self.load_prompt_from_file(&path) {
                    versions.push(version);
                }
            }
        }

        versions.sort_by_key(|v| v.version);
        Ok(versions)
    }

    /// Load a specific historical snapshot of a prompt.
    pub fn get_version(&self, id: &str, version: u32) -> Result<PromptTemplate> {
        let path = self.version_path(id, version);
        if !path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Prompt version {} not found for {}",
                version, id
            )));
        }

        self.load_prompt_from_file(&path)
    }

    /// Restore a prompt to a previous version. The current state is snapshotted
    /// first so no history is lost, and the version number keeps increasing.
    pub fn rollback(&self, id: &str, version: u32) -> Result<PromptTemplate> {
        let current = self.get_prompt(id)?;
        let historical = self.get_version(id, version)?;

        self.snapshot_current(&current)?;

        let mut restored = historical;
        restored.id = id.to_string();
        restored.version = current.version + 1;
        restored.updated_at = Some(Utc::now());

        self.save_prompt(&restored)?;
        Ok(restored)
    }

    /// Import historical snapshots (used by backup restore).
    pub fn import_versions(&self, id: &str, versions: &[PromptTemplate]) -> Result<()> {
        for version in versions {
            let dir = self.versions_dir(id);
            std::fs::create_dir_all(&dir).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to create versions dir: {}", e))
            })?;

            let path = self.version_path(id, version.version);
            let content = serde_yaml::to_string(version).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to serialize version: {}", e))
            })?;

            std::fs::write(&path, content).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to write version: {}", e))
            })?;
        }
        Ok(())
    }

    /// List prompts sorted by usage count (most used first).
    pub fn list_usage(&self) -> Result<Vec<PromptUsage>> {
        let prompts = self.list_prompts()?;
        let mut usage: Vec<PromptUsage> = prompts
            .iter()
            .map(|p| PromptUsage {
                id: p.id.clone(),
                name: p.name.clone(),
                usage_count: p.usage_count,
                last_used_at: p.last_used_at,
            })
            .collect();
        usage.sort_by_key(|b| std::cmp::Reverse(b.usage_count));
        Ok(usage)
    }

    fn prompt_path(&self, id: &str) -> PathBuf {
        self.templates_dir().join(format!("{}.yaml", id))
    }

    pub fn list_prompts(&self) -> Result<Vec<PromptTemplate>> {
        let templates_dir = self.templates_dir();
        if !templates_dir.exists() {
            return Ok(Vec::new());
        }

        let mut prompts = Vec::new();
        for entry in std::fs::read_dir(&templates_dir)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to read prompts dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::PromptError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                match self.load_prompt_from_file(&path) {
                    Ok(prompt) => prompts.push(prompt),
                    Err(e) => {
                        eprintln!("Warning: Failed to load prompt at {:?}: {}", path, e);
                    }
                }
            }
        }

        prompts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(prompts)
    }

    fn load_prompt_from_file(&self, path: &Path) -> Result<PromptTemplate> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to read prompt: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to parse prompt: {}", e)))
    }

    pub fn get_prompt(&self, id: &str) -> Result<PromptTemplate> {
        let path = self.prompt_path(id);
        if !path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Prompt not found: {}",
                id
            )));
        }

        self.load_prompt_from_file(&path)
    }

    pub fn create_prompt(
        &self,
        id: &str,
        name: &str,
        description: &str,
        template: &str,
    ) -> Result<PromptTemplate> {
        let path = self.prompt_path(id);
        if path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Prompt already exists: {}",
                id
            )));
        }

        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create prompts dir: {}", e))
        })?;

        let now = Utc::now();
        let prompt = PromptTemplate {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            template: template.to_string(),
            variables: Vec::new(),
            tags: Vec::new(),
            category: None,
            version: 1,
            author: None,
            created_at: Some(now),
            updated_at: Some(now),
            usage_count: 0,
            last_used_at: None,
        };

        self.save_prompt(&prompt)?;
        Ok(prompt)
    }

    pub fn save_prompt(&self, prompt: &PromptTemplate) -> Result<()> {
        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create prompts dir: {}", e))
        })?;

        let path = self.prompt_path(&prompt.id);
        let content = serde_yaml::to_string(prompt).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to serialize prompt: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to write prompt: {}", e)))?;

        Ok(())
    }

    pub fn update_prompt(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        template: Option<&str>,
    ) -> Result<PromptTemplate> {
        let mut prompt = self.get_prompt(id)?;

        // Snapshot the current state before modifying it.
        self.snapshot_current(&prompt)?;

        if let Some(n) = name {
            prompt.name = n.to_string();
        }
        if let Some(d) = description {
            prompt.description = d.to_string();
        }
        if let Some(t) = template {
            prompt.template = t.to_string();
        }

        prompt.version += 1;
        prompt.updated_at = Some(Utc::now());
        self.save_prompt(&prompt)?;
        Ok(prompt)
    }

    pub fn delete_prompt(&self, id: &str) -> Result<bool> {
        let path = self.prompt_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to delete prompt: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn render_prompt(&self, id: &str, vars: &HashMap<String, String>) -> Result<String> {
        let prompt = self.get_prompt(id)?;
        let rendered = Self::render(&prompt, vars);
        self.record_usage(&prompt)?;
        Ok(rendered)
    }

    /// Render with validation: required variables must be supplied (falling back
    /// to their declared defaults), otherwise an error is returned.
    pub fn render_prompt_checked(
        &self,
        id: &str,
        vars: &HashMap<String, String>,
    ) -> Result<String> {
        let prompt = self.get_prompt(id)?;

        let mut resolved = vars.clone();
        for variable in &prompt.variables {
            if !resolved.contains_key(&variable.name) {
                if let Some(default) = &variable.default {
                    resolved.insert(variable.name.clone(), default.clone());
                } else if variable.required {
                    return Err(AgentHubError::PromptError(format!(
                        "Missing required variable: {}",
                        variable.name
                    )));
                }
            }
        }

        let rendered = Self::render(&prompt, &resolved);
        self.record_usage(&prompt)?;
        Ok(rendered)
    }

    fn render(prompt: &PromptTemplate, vars: &HashMap<String, String>) -> String {
        let mut rendered = prompt.template.clone();
        for (key, value) in vars {
            rendered = rendered.replace(&format!("{{{{{}}}}}", key), value);
        }
        rendered
    }

    /// Increment the usage counters for a prompt after a render.
    fn record_usage(&self, prompt: &PromptTemplate) -> Result<()> {
        let mut updated = prompt.clone();
        updated.usage_count += 1;
        updated.last_used_at = Some(Utc::now());
        self.save_prompt(&updated)
    }

    pub fn add_tag(&self, id: &str, tag: &str) -> Result<()> {
        let mut prompt = self.get_prompt(id)?;
        if !prompt.tags.contains(&tag.to_string()) {
            prompt.tags.push(tag.to_string());
            prompt.updated_at = Some(Utc::now());
            self.save_prompt(&prompt)?;
        }
        Ok(())
    }

    pub fn remove_tag(&self, id: &str, tag: &str) -> Result<()> {
        let mut prompt = self.get_prompt(id)?;
        prompt.tags.retain(|t| t != tag);
        prompt.updated_at = Some(Utc::now());
        self.save_prompt(&prompt)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (PromptManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = PromptManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_create_and_get_prompt() {
        let (manager, _temp) = create_test_manager();

        let prompt = manager
            .create_prompt("test", "Test Prompt", "A test prompt", "Hello {{name}}!")
            .unwrap();

        assert_eq!(prompt.id, "test");
        assert_eq!(prompt.name, "Test Prompt");

        let loaded = manager.get_prompt("test").unwrap();
        assert_eq!(loaded.id, "test");
    }

    #[test]
    fn test_list_prompts() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("p1", "Prompt 1", "First", "Template 1")
            .unwrap();
        manager
            .create_prompt("p2", "Prompt 2", "Second", "Template 2")
            .unwrap();

        let prompts = manager.list_prompts().unwrap();
        assert_eq!(prompts.len(), 2);
    }

    #[test]
    fn test_render_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt(
                "greeting",
                "Greeting",
                "A greeting",
                "Hello {{name}}, welcome to {{place}}!",
            )
            .unwrap();

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("place".to_string(), "Wonderland".to_string());

        let rendered = manager.render_prompt("greeting", &vars).unwrap();
        assert_eq!(rendered, "Hello Alice, welcome to Wonderland!");
    }

    #[test]
    fn test_update_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Template")
            .unwrap();

        let updated = manager
            .update_prompt("test", Some("New Name"), None, None)
            .unwrap();
        assert_eq!(updated.name, "New Name");
    }

    #[test]
    fn test_delete_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Template")
            .unwrap();

        let deleted = manager.delete_prompt("test").unwrap();
        assert!(deleted);

        let result = manager.get_prompt("test");
        assert!(result.is_err());
    }

    #[test]
    fn test_tags() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Template")
            .unwrap();

        manager.add_tag("test", "review").unwrap();
        manager.add_tag("test", "code").unwrap();

        let prompt = manager.get_prompt("test").unwrap();
        assert_eq!(prompt.tags.len(), 2);

        manager.remove_tag("test", "review").unwrap();
        let prompt = manager.get_prompt("test").unwrap();
        assert_eq!(prompt.tags.len(), 1);
    }

    // ---- Version control ----

    #[test]
    fn test_update_prompt_creates_versions() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "v1 content")
            .unwrap();

        let updated = manager
            .update_prompt("test", None, None, Some("v2 content"))
            .unwrap();
        assert_eq!(updated.version, 2);
        assert_eq!(updated.template, "v2 content");

        // History contains the original v1
        let versions = manager.list_versions("test").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[0].template, "v1 content");

        let v1 = manager.get_version("test", 1).unwrap();
        assert_eq!(v1.template, "v1 content");
    }

    #[test]
    fn test_rollback_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "v1 content")
            .unwrap();
        manager
            .update_prompt("test", None, None, Some("v2 content"))
            .unwrap();
        manager
            .update_prompt("test", None, None, Some("v3 content"))
            .unwrap();

        let rolled = manager.rollback("test", 1).unwrap();
        assert_eq!(rolled.template, "v1 content");
        assert_eq!(rolled.version, 4);

        // The v3 state was snapshotted before rollback
        let versions = manager.list_versions("test").unwrap();
        assert_eq!(versions.len(), 3);
        assert!(versions
            .iter()
            .any(|v| v.version == 3 && v.template == "v3 content"));
    }

    #[test]
    fn test_rollback_missing_version() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "v1 content")
            .unwrap();

        assert!(manager.rollback("test", 99).is_err());
    }

    #[test]
    fn test_import_versions() {
        let (manager, _temp) = create_test_manager();

        let snapshots = vec![PromptTemplate {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Desc".to_string(),
            template: "old content".to_string(),
            variables: Vec::new(),
            tags: Vec::new(),
            category: None,
            version: 1,
            author: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            usage_count: 0,
            last_used_at: None,
        }];

        manager.import_versions("test", &snapshots).unwrap();
        let versions = manager.list_versions("test").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].template, "old content");
    }

    // ---- Usage stats ----

    #[test]
    fn test_render_records_usage() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Hello {{name}}")
            .unwrap();

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());

        manager.render_prompt("test", &vars).unwrap();
        manager.render_prompt("test", &vars).unwrap();

        let prompt = manager.get_prompt("test").unwrap();
        assert_eq!(prompt.usage_count, 2);
        assert!(prompt.last_used_at.is_some());

        let usage = manager.list_usage().unwrap();
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].usage_count, 2);
    }

    // ---- Variable validation ----

    #[test]
    fn test_render_prompt_checked() {
        let (manager, _temp) = create_test_manager();

        let prompt = manager
            .create_prompt("checked", "Checked", "Desc", "Hi {{name}} in {{city}}")
            .unwrap();
        // Attach variable declarations directly for validation
        let mut p = prompt;
        p.variables = vec![
            PromptVariable {
                name: "name".to_string(),
                var_type: "string".to_string(),
                required: true,
                description: None,
                default: None,
            },
            PromptVariable {
                name: "city".to_string(),
                var_type: "string".to_string(),
                required: false,
                description: None,
                default: Some("Shanghai".to_string()),
            },
        ];
        manager.save_prompt(&p).unwrap();

        // Missing required variable -> error
        let mut vars = HashMap::new();
        assert!(manager.render_prompt_checked("checked", &vars).is_err());

        // Required present, optional uses default
        vars.insert("name".to_string(), "Alice".to_string());
        let rendered = manager.render_prompt_checked("checked", &vars).unwrap();
        assert_eq!(rendered, "Hi Alice in Shanghai");

        // Explicit override wins over default
        vars.insert("city".to_string(), "Beijing".to_string());
        let rendered = manager.render_prompt_checked("checked", &vars).unwrap();
        assert_eq!(rendered, "Hi Alice in Beijing");
    }
}
