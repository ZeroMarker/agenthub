//! Prompt community sharing.
//!
//! Prompts can be *published* to a local community directory
//! (`prompts/community/`) — a snapshot that carries provenance (publisher,
//! publish time, source prompt id). Community prompts can be *installed* back
//! as local templates. The community directory itself can be a git repo or
//! synced folder, so sharing works fully offline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::prompt::{PromptManager, PromptTemplate, PromptVariable};
use crate::storage::is_safe_id;

/// A prompt published to the community directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityPrompt {
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
    pub version: u32,
    #[serde(default)]
    pub author: Option<String>,
    /// Identity of the publisher (e.g. the local user).
    pub publisher: String,
    pub published_at: DateTime<Utc>,
    /// Original local prompt id this snapshot came from.
    #[serde(default)]
    pub source: Option<String>,
}

/// Manages the prompt community directory.
pub struct CommunityManager {
    prompts_dir: PathBuf,
}

impl CommunityManager {
    fn validate_id(id: &str) -> Result<()> {
        if !is_safe_id(id) {
            return Err(AgentHubError::PromptError(format!(
                "Invalid community prompt id: {id}"
            )));
        }
        Ok(())
    }

    pub fn new(prompts_dir: PathBuf) -> Self {
        Self { prompts_dir }
    }

    pub fn prompts_dir(&self) -> &Path {
        &self.prompts_dir
    }

    pub fn community_dir(&self) -> PathBuf {
        self.prompts_dir.join("community")
    }

    fn community_path(&self, id: &str) -> PathBuf {
        self.community_dir().join(format!("{}.yaml", id))
    }

    /// Publish a local prompt template as a community snapshot.
    ///
    /// Errors when the community id already exists unless `force` is set (in
    /// which case the snapshot is updated in place).
    pub fn publish(
        &self,
        prompt: &PromptTemplate,
        publisher: &str,
        force: bool,
    ) -> Result<CommunityPrompt> {
        Self::validate_id(&prompt.id)?;
        let path = self.community_path(&prompt.id);
        if path.exists() && !force {
            return Err(AgentHubError::PromptError(format!(
                "Community prompt '{}' already exists (use --force to overwrite)",
                prompt.id
            )));
        }
        std::fs::create_dir_all(self.community_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create community dir: {}", e))
        })?;

        let community = CommunityPrompt {
            id: prompt.id.clone(),
            name: prompt.name.clone(),
            description: prompt.description.clone(),
            template: prompt.template.clone(),
            variables: prompt.variables.clone(),
            tags: prompt.tags.clone(),
            category: prompt.category.clone(),
            version: prompt.version,
            author: prompt.author.clone(),
            publisher: publisher.to_string(),
            published_at: Utc::now(),
            source: Some(prompt.id.clone()),
        };

        let content = serde_yaml::to_string(&community).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to serialize community prompt: {}", e))
        })?;
        std::fs::write(&path, content).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to write community prompt: {}", e))
        })?;
        Ok(community)
    }

    /// Publish a local prompt by id (loads the current template).
    pub fn publish_by_id(
        &self,
        prompt_manager: &PromptManager,
        prompt_id: &str,
        publisher: &str,
        force: bool,
    ) -> Result<CommunityPrompt> {
        let prompt = prompt_manager.get_prompt(prompt_id)?;
        self.publish(&prompt, publisher, force)
    }

    pub fn list(&self) -> Result<Vec<CommunityPrompt>> {
        let dir = self.community_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut prompts = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to read community dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| AgentHubError::PromptError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                match self.load(&path) {
                    Ok(p) => prompts.push(p),
                    Err(e) => {
                        eprintln!("Warning: failed to load community prompt {:?}: {}", path, e);
                    }
                }
            }
        }
        prompts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(prompts)
    }

    pub fn get(&self, id: &str) -> Result<CommunityPrompt> {
        Self::validate_id(id)?;
        let path = self.community_path(id);
        if !path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Community prompt not found: {}",
                id
            )));
        }
        self.load(&path)
    }

    fn load(&self, path: &Path) -> Result<CommunityPrompt> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to read community prompt: {}", e))
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to parse community prompt: {}", e))
        })
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        Self::validate_id(id)?;
        let path = self.community_path(id);
        if !path.exists() {
            return Ok(false);
        }
        std::fs::remove_file(&path).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to delete community prompt: {}", e))
        })?;
        Ok(true)
    }

    /// Import a full community snapshot set (used by backup restore).
    pub fn import(&self, prompts: &[CommunityPrompt]) -> Result<()> {
        std::fs::create_dir_all(self.community_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create community dir: {}", e))
        })?;
        for prompt in prompts {
            Self::validate_id(&prompt.id)?;
            let content = serde_yaml::to_string(prompt).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to serialize community prompt: {}", e))
            })?;
            std::fs::write(self.community_path(&prompt.id), content).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to write community prompt: {}", e))
            })?;
        }
        Ok(())
    }

    /// Install a community prompt as a local template.
    ///
    /// By default the local template keeps the same id; `new_id` allows
    /// installing under a different name. Fails if the target id exists unless
    /// `force` is set.
    pub fn install(
        &self,
        prompt_manager: &PromptManager,
        id: &str,
        new_id: Option<&str>,
        force: bool,
    ) -> Result<PromptTemplate> {
        let community = self.get(id)?;
        let target_id = new_id.unwrap_or(&community.id);
        if prompt_manager.get_prompt(target_id).is_ok() && !force {
            return Err(AgentHubError::PromptError(format!(
                "Prompt '{}' already exists (use --force to overwrite)",
                target_id
            )));
        }
        prompt_manager.create_prompt(
            target_id,
            &community.name,
            &community.description,
            &community.template,
        )?;
        // Carry over variables/tags/category/author by saving a full template.
        let mut template = prompt_manager.get_prompt(target_id)?;
        template.variables = community.variables.clone();
        template.tags = community.tags.clone();
        template.category = community.category.clone();
        template.author = community.author.clone();
        template.version = community.version.max(1);
        prompt_manager.save_prompt(&template)?;
        Ok(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_publish_list_get_delete() {
        let temp = TempDir::new().unwrap();
        let pm = PromptManager::new(temp.path().join("prompts"));
        let cm = CommunityManager::new(temp.path().join("prompts"));

        pm.create_prompt("review", "Review", "desc", "review {{code}}")
            .unwrap();
        let published = cm.publish_by_id(&pm, "review", "alice", false).unwrap();
        assert_eq!(published.id, "review");
        assert_eq!(published.publisher, "alice");
        assert_eq!(published.source.as_deref(), Some("review"));

        // Duplicate publish rejected unless forced
        assert!(cm.publish_by_id(&pm, "review", "alice", false).is_err());
        assert!(cm.publish_by_id(&pm, "review", "alice", true).is_ok());

        let list = cm.list().unwrap();
        assert_eq!(list.len(), 1);

        let got = cm.get("review").unwrap();
        assert_eq!(got.template, "review {{code}}");

        assert!(cm.delete("review").unwrap());
        assert!(cm.get("review").is_err());
        assert!(!cm.delete("review").unwrap());
    }

    #[test]
    fn test_install_community_prompt() {
        let temp = TempDir::new().unwrap();
        let pm = PromptManager::new(temp.path().join("prompts"));
        let cm = CommunityManager::new(temp.path().join("prompts"));

        pm.create_prompt("src", "Src", "desc", "hello {{name}}")
            .unwrap();
        // Add variables + tags before publishing
        let mut t = pm.get_prompt("src").unwrap();
        t.tags = vec!["demo".to_string()];
        t.category = Some("testing".to_string());
        t.variables = vec![PromptVariable {
            name: "name".to_string(),
            var_type: "string".to_string(),
            required: true,
            description: None,
            default: None,
        }];
        pm.save_prompt(&t).unwrap();
        cm.publish_by_id(&pm, "src", "bob", false).unwrap();

        // Install into a fresh prompts dir (same community dir, new template store)
        let pm2 = PromptManager::new(temp.path().join("fresh"));
        let installed = cm.install(&pm2, "src", None, false).unwrap();
        assert_eq!(installed.id, "src");
        assert_eq!(installed.template, "hello {{name}}");
        assert_eq!(installed.tags, vec!["demo".to_string()]);
        assert_eq!(installed.variables.len(), 1);

        // Installing again without force errors
        assert!(cm.install(&pm2, "src", None, false).is_err());
        // With a new id it succeeds
        assert!(cm.install(&pm2, "src", Some("src-clone"), false).is_ok());
        assert!(pm2.get_prompt("src-clone").is_ok());
    }

    #[test]
    fn test_import_rejects_unsafe_ids() {
        let temp = TempDir::new().unwrap();
        let cm = CommunityManager::new(temp.path().join("prompts"));
        let prompt = CommunityPrompt {
            id: "../escape".to_string(),
            name: "Unsafe".to_string(),
            description: String::new(),
            template: String::new(),
            variables: Vec::new(),
            tags: Vec::new(),
            category: None,
            version: 1,
            author: None,
            publisher: "test".to_string(),
            published_at: Utc::now(),
            source: None,
        };

        assert!(cm.import(&[prompt]).is_err());
        assert!(!temp.path().join("prompts/escape.yaml").exists());
    }
}
