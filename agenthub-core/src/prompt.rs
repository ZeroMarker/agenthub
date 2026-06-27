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
                .map_or(false, |ext| ext == "yaml" || ext == "yml")
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

        if let Some(n) = name {
            prompt.name = n.to_string();
        }
        if let Some(d) = description {
            prompt.description = d.to_string();
        }
        if let Some(t) = template {
            prompt.template = t.to_string();
        }

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
        let mut rendered = prompt.template.clone();

        for (key, value) in vars {
            rendered = rendered.replace(&format!("{{{{{}}}}}", key), value);
        }

        Ok(rendered)
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
}
