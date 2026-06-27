use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
        }
    }
}

impl std::str::FromStr for Environment {
    type Err = AgentHubError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "staging" | "stage" => Ok(Environment::Staging),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(AgentHubError::ConfigError(format!(
                "Invalid environment: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub version: u32,
    pub environment: Environment,
    pub settings: HashMap<String, ConfigValue>,
    #[serde(default)]
    pub secrets: HashMap<String, String>,
    #[serde(default)]
    pub environment_variables: HashMap<String, String>,
    #[serde(default)]
    pub custom: HashMap<String, ConfigValue>,
    pub metadata: ConfigMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Map(HashMap<String, ConfigValue>),
    Null,
}

impl std::fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValue::String(s) => write!(f, "{}", s),
            ConfigValue::Number(n) => write!(f, "{}", n),
            ConfigValue::Boolean(b) => write!(f, "{}", b),
            ConfigValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            ConfigValue::Map(_) => write!(f, "{{object}}"),
            ConfigValue::Null => write!(f, "null"),
        }
    }
}

impl ConfigValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        ConfigValue::String(s)
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        ConfigValue::String(s.to_string())
    }
}

impl From<f64> for ConfigValue {
    fn from(n: f64) -> Self {
        ConfigValue::Number(n)
    }
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        ConfigValue::Boolean(b)
    }
}

pub struct ConfigManager {
    config_dir: PathBuf,
    current_environment: Environment,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            current_environment: Environment::Development,
        }
    }

    pub fn with_environment(mut self, env: Environment) -> Self {
        self.current_environment = env;
        self
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn current_environment(&self) -> &Environment {
        &self.current_environment
    }

    fn agent_config_path(&self, agent_id: &str) -> PathBuf {
        self.config_dir
            .join("agents")
            .join(format!("{}.yaml", agent_id))
    }

    pub fn list_configs(&self) -> Result<Vec<String>> {
        let agents_dir = self.config_dir.join("agents");
        if !agents_dir.exists() {
            return Ok(Vec::new());
        }

        let mut configs = Vec::new();
        for entry in std::fs::read_dir(&agents_dir)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read config dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::ConfigError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .map_or(false, |ext| ext == "yaml" || ext == "yml")
            {
                if let Some(stem) = path.file_stem() {
                    configs.push(stem.to_string_lossy().to_string());
                }
            }
        }

        configs.sort();
        Ok(configs)
    }

    pub fn load_config(&self, agent_id: &str) -> Result<AgentConfig> {
        let path = self.agent_config_path(agent_id);
        if !path.exists() {
            return Err(AgentHubError::ConfigError(format!(
                "Config not found for agent: {}",
                agent_id
            )));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read config: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse config: {}", e)))
    }

    pub fn save_config(&self, config: &AgentConfig) -> Result<()> {
        let agents_dir = self.config_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to create config dir: {}", e))
        })?;

        let path = self.agent_config_path(&config.agent_id);
        let content = serde_yaml::to_string(config).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    pub fn create_config(&self, agent_id: &str) -> Result<AgentConfig> {
        let now = Utc::now();
        let config = AgentConfig {
            agent_id: agent_id.to_string(),
            version: 1,
            environment: self.current_environment.clone(),
            settings: HashMap::new(),
            secrets: HashMap::new(),
            environment_variables: HashMap::new(),
            custom: HashMap::new(),
            metadata: ConfigMetadata {
                created_at: now,
                updated_at: now,
                created_by: None,
            },
        };

        self.save_config(&config)?;
        Ok(config)
    }

    pub fn get_setting(&self, agent_id: &str, key: &str) -> Result<Option<ConfigValue>> {
        let config = self.load_config(agent_id)?;
        Ok(config.settings.get(key).cloned())
    }

    pub fn set_setting(&self, agent_id: &str, key: &str, value: ConfigValue) -> Result<()> {
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        config.settings.insert(key.to_string(), value);
        config.metadata.updated_at = Utc::now();
        self.save_config(&config)
    }

    pub fn unset_setting(&self, agent_id: &str, key: &str) -> Result<bool> {
        let mut config = self.load_config(agent_id)?;
        let removed = config.settings.remove(key).is_some();
        if removed {
            config.metadata.updated_at = Utc::now();
            self.save_config(&config)?;
        }
        Ok(removed)
    }

    pub fn get_custom(&self, agent_id: &str, key: &str) -> Result<Option<ConfigValue>> {
        let config = self.load_config(agent_id)?;
        Ok(config.custom.get(key).cloned())
    }

    pub fn set_custom(&self, agent_id: &str, key: &str, value: ConfigValue) -> Result<()> {
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        config.custom.insert(key.to_string(), value);
        config.metadata.updated_at = Utc::now();
        self.save_config(&config)
    }

    pub fn set_env_var(&self, agent_id: &str, key: &str, value: &str) -> Result<()> {
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        config
            .environment_variables
            .insert(key.to_string(), value.to_string());
        config.metadata.updated_at = Utc::now();
        self.save_config(&config)
    }

    pub fn get_env_var(&self, agent_id: &str, key: &str) -> Result<Option<String>> {
        let config = self.load_config(agent_id)?;
        Ok(config.environment_variables.get(key).cloned())
    }

    pub fn delete_config(&self, agent_id: &str) -> Result<bool> {
        let path = self.agent_config_path(agent_id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::ConfigError(format!("Failed to delete config: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn reset_config(&self, agent_id: &str) -> Result<AgentConfig> {
        self.delete_config(agent_id)?;
        self.create_config(agent_id)
    }

    pub fn export_config(&self, agent_id: &str, output_path: &Path) -> Result<()> {
        let config = self.load_config(agent_id)?;
        let content = serde_yaml::to_string(&config).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(output_path, content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write export: {}", e)))?;

        Ok(())
    }

    pub fn import_config(&self, input_path: &Path, agent_id: Option<&str>) -> Result<AgentConfig> {
        let content = std::fs::read_to_string(input_path)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read import: {}", e)))?;

        let mut config: AgentConfig = serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse import: {}", e)))?;

        if let Some(id) = agent_id {
            config.agent_id = id.to_string();
        }

        config.metadata.updated_at = Utc::now();
        self.save_config(&config)?;
        Ok(config)
    }

    pub fn export_all(&self, output_path: &Path) -> Result<()> {
        let configs = self.list_configs()?;
        let mut all_configs: HashMap<String, AgentConfig> = HashMap::new();

        for agent_id in configs {
            if let Ok(config) = self.load_config(&agent_id) {
                all_configs.insert(agent_id, config);
            }
        }

        let content = serde_yaml::to_string(&all_configs).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize configs: {}", e))
        })?;

        std::fs::write(output_path, content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write export: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (ConfigManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_create_and_load_config() {
        let (manager, _temp) = create_test_manager();
        let config = manager.create_config("test-agent").unwrap();

        assert_eq!(config.agent_id, "test-agent");
        assert_eq!(config.version, 1);
        assert_eq!(config.environment, Environment::Development);

        let loaded = manager.load_config("test-agent").unwrap();
        assert_eq!(loaded.agent_id, "test-agent");
    }

    #[test]
    fn test_set_and_get_setting() {
        let (manager, _temp) = create_test_manager();

        manager
            .set_setting("test-agent", "model", ConfigValue::from("gpt-4"))
            .unwrap();

        let value = manager.get_setting("test-agent", "model").unwrap();
        assert_eq!(value.unwrap().as_str(), Some("gpt-4"));
    }

    #[test]
    fn test_list_configs() {
        let (manager, _temp) = create_test_manager();

        manager.create_config("agent-a").unwrap();
        manager.create_config("agent-b").unwrap();
        manager.create_config("agent-c").unwrap();

        let configs = manager.list_configs().unwrap();
        assert_eq!(configs.len(), 3);
        assert!(configs.contains(&"agent-a".to_string()));
        assert!(configs.contains(&"agent-b".to_string()));
        assert!(configs.contains(&"agent-c".to_string()));
    }

    #[test]
    fn test_unset_setting() {
        let (manager, _temp) = create_test_manager();

        manager
            .set_setting("test-agent", "model", ConfigValue::from("gpt-4"))
            .unwrap();

        let removed = manager.unset_setting("test-agent", "model").unwrap();
        assert!(removed);

        let value = manager.get_setting("test-agent", "model").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_delete_config() {
        let (manager, _temp) = create_test_manager();

        manager.create_config("test-agent").unwrap();
        let deleted = manager.delete_config("test-agent").unwrap();
        assert!(deleted);

        let result = manager.load_config("test-agent");
        assert!(result.is_err());
    }

    #[test]
    fn test_environment_parsing() {
        assert_eq!(
            "development".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "dev".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "production".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            "prod".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert!("invalid".parse::<Environment>().is_err());
    }

    #[test]
    fn test_export_import_config() {
        let (manager, temp) = create_test_manager();

        manager.create_config("test-agent").unwrap();
        manager
            .set_setting("test-agent", "model", ConfigValue::from("gpt-4"))
            .unwrap();

        let export_path = temp.path().join("export.yaml");
        manager.export_config("test-agent", &export_path).unwrap();

        manager.delete_config("test-agent").unwrap();

        let imported = manager.import_config(&export_path, None).unwrap();
        assert_eq!(imported.agent_id, "test-agent");
        assert_eq!(
            imported.settings.get("model").unwrap().as_str(),
            Some("gpt-4")
        );
    }
}
