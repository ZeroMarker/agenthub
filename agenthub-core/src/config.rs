use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::secrets::{RotationResult, SecretInfo, SecretStore};
use crate::storage::is_safe_id;

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

/// A reusable configuration template. Secrets are never stored in templates;
/// only their key names are reserved via [`ConfigTemplate::secret_keys`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub settings: HashMap<String, ConfigValue>,
    #[serde(default)]
    pub environment_variables: HashMap<String, String>,
    /// Names of secret keys to reserve when applying the template.
    #[serde(default)]
    pub secret_keys: Vec<String>,
    #[serde(default)]
    pub custom: HashMap<String, ConfigValue>,
    pub metadata: ConfigMetadata,
}

impl ConfigManager {
    fn validate_id(kind: &str, id: &str) -> Result<()> {
        if !is_safe_id(id) {
            return Err(AgentHubError::ConfigError(format!(
                "Invalid {kind} id: {id}"
            )));
        }
        Ok(())
    }

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
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
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
        Self::validate_id("agent", agent_id)?;
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
        Self::validate_id("agent", &config.agent_id)?;
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

    // ---------------------------------------------------------------------
    // Secret keystore (values never stored in agent config files)
    // ---------------------------------------------------------------------

    /// Open the file-backed secret keystore for this workspace.
    pub fn secret_store(&self) -> SecretStore {
        SecretStore::new(self.config_dir.clone())
    }

    /// Store a secret in the keystore and remove any inline copy from the
    /// agent config file (values live only in `secrets.yaml`, never in the
    /// agent YAML).
    pub fn set_secret(&self, agent_id: &str, key: &str, value: &str) -> Result<()> {
        self.secret_store().set(agent_id, key, value)?;
        // Drop any inline plaintext copy from the config file.
        if self.agent_config_path(agent_id).exists() {
            let mut config = self.load_config(agent_id).ok();
            if let Some(config) = config.as_mut() {
                if config.secrets.remove(key).is_some() {
                    config.metadata.updated_at = Utc::now();
                    self.save_config(config)?;
                }
            }
        }
        Ok(())
    }

    /// Read a secret from the keystore, falling back to a legacy inline value
    /// in the agent config file (if any).
    pub fn get_secret(&self, agent_id: &str, key: &str) -> Result<Option<String>> {
        let store = self.secret_store();
        if let Some(value) = store.get(agent_id, key) {
            return Ok(Some(value));
        }
        // Legacy fallback: inline secret in the config file.
        if self.agent_config_path(agent_id).exists() {
            if let Ok(config) = self.load_config(agent_id) {
                if let Some(value) = config.secrets.get(key) {
                    if !value.is_empty() {
                        return Ok(Some(value.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn delete_secret(&self, agent_id: &str, key: &str) -> Result<bool> {
        self.secret_store().delete(agent_id, key)
    }

    /// List stored secret keys (redacted values only), optionally for one agent.
    pub fn list_secrets(&self, agent_id: Option<&str>) -> Result<Vec<SecretInfo>> {
        Ok(self.secret_store().list(agent_id))
    }

    /// Rotate a secret: archive the current value, activate the new one.
    pub fn rotate_secret(
        &self,
        agent_id: &str,
        key: &str,
        new_value: &str,
    ) -> Result<RotationResult> {
        self.secret_store().rotate(agent_id, key, new_value)
    }

    /// Move a legacy inline secret value from the agent config file into the
    /// keystore, then blank it in the file. Returns true when a value moved.
    pub fn migrate_secret(&self, agent_id: &str, key: &str) -> Result<bool> {
        if !self.agent_config_path(agent_id).exists() {
            return Ok(false);
        }
        let mut config = self.load_config(agent_id)?;
        match config.secrets.remove(key) {
            Some(value) if !value.is_empty() => {
                self.secret_store().set(agent_id, key, &value)?;
                config.metadata.updated_at = Utc::now();
                self.save_config(&config)?;
                Ok(true)
            }
            _ => Ok(false),
        }
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

    // -----------------------------------------------------------------------
    // Config templates
    // -----------------------------------------------------------------------

    fn templates_dir(&self) -> PathBuf {
        self.config_dir.join("templates")
    }

    fn template_path(&self, id: &str) -> PathBuf {
        self.templates_dir().join(format!("{}.yaml", id))
    }

    /// List config template ids, sorted.
    pub fn list_templates(&self) -> Result<Vec<String>> {
        let dir = self.templates_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to read templates dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| AgentHubError::ConfigError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                if let Some(stem) = path.file_stem() {
                    templates.push(stem.to_string_lossy().to_string());
                }
            }
        }

        templates.sort();
        Ok(templates)
    }

    pub fn get_template(&self, id: &str) -> Result<ConfigTemplate> {
        Self::validate_id("template", id)?;
        let path = self.template_path(id);
        if !path.exists() {
            return Err(AgentHubError::ConfigError(format!(
                "Template not found: {}",
                id
            )));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read template: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse template: {}", e)))
    }

    pub fn save_template(&self, template: &ConfigTemplate) -> Result<()> {
        Self::validate_id("template", &template.id)?;
        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to create templates dir: {}", e))
        })?;

        let path = self.template_path(&template.id);
        let content = serde_yaml::to_string(template).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize template: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write template: {}", e)))?;

        Ok(())
    }

    /// Create a template from scratch.
    #[allow(clippy::too_many_arguments)]
    pub fn create_template(
        &self,
        id: &str,
        name: &str,
        description: &str,
        settings: HashMap<String, ConfigValue>,
        environment_variables: HashMap<String, String>,
        secret_keys: Vec<String>,
        custom: HashMap<String, ConfigValue>,
    ) -> Result<ConfigTemplate> {
        Self::validate_id("template", id)?;
        let path = self.template_path(id);
        if path.exists() {
            return Err(AgentHubError::ConfigError(format!(
                "Template already exists: {}",
                id
            )));
        }

        let now = Utc::now();
        let template = ConfigTemplate {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            settings,
            environment_variables,
            secret_keys,
            custom,
            metadata: ConfigMetadata {
                created_at: now,
                updated_at: now,
                created_by: None,
            },
        };

        self.save_template(&template)?;
        Ok(template)
    }

    pub fn delete_template(&self, id: &str) -> Result<bool> {
        Self::validate_id("template", id)?;
        let path = self.template_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::ConfigError(format!("Failed to delete template: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Create a template from an existing agent config. Secret values are never
    /// copied — only their key names are reserved.
    pub fn save_config_as_template(
        &self,
        agent_id: &str,
        template_id: &str,
        name: &str,
        description: &str,
    ) -> Result<ConfigTemplate> {
        let config = self.load_config(agent_id)?;
        let now = Utc::now();
        let template = ConfigTemplate {
            id: template_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            settings: config.settings,
            environment_variables: config.environment_variables,
            secret_keys: config.secrets.keys().cloned().collect(),
            custom: config.custom,
            metadata: ConfigMetadata {
                created_at: now,
                updated_at: now,
                created_by: None,
            },
        };

        self.save_template(&template)?;
        Ok(template)
    }

    /// Apply a template to an agent config (creating it if needed). Template
    /// values win over existing ones; secret keys keep existing values or are
    /// reserved as empty placeholders.
    pub fn apply_template(&self, agent_id: &str, template_id: &str) -> Result<AgentConfig> {
        let template = self.get_template(template_id)?;
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        for (key, value) in &template.settings {
            config.settings.insert(key.clone(), value.clone());
        }
        for (key, value) in &template.environment_variables {
            config
                .environment_variables
                .insert(key.clone(), value.clone());
        }
        for key in &template.secret_keys {
            config.secrets.entry(key.clone()).or_default();
        }
        for (key, value) in &template.custom {
            config.custom.insert(key.clone(), value.clone());
        }

        config.version += 1;
        config.metadata.updated_at = Utc::now();
        self.save_config(&config)?;
        Ok(config)
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

    // ---- Config templates ----

    fn sample_template(manager: &ConfigManager) -> ConfigTemplate {
        let mut settings = HashMap::new();
        settings.insert("model".to_string(), ConfigValue::from("gpt-4o"));
        settings.insert("temperature".to_string(), ConfigValue::from(0.7f64));
        let mut env = HashMap::new();
        env.insert("OPENAI_API_KEY".to_string(), "".to_string());
        let mut custom = HashMap::new();
        custom.insert("lang".to_string(), ConfigValue::from("rust"));

        manager
            .create_template(
                "llm-default",
                "LLM Default",
                "Standard model settings",
                settings,
                env,
                vec!["api_key".to_string()],
                custom,
            )
            .unwrap()
    }

    #[test]
    fn test_template_crud() {
        let (manager, _temp) = create_test_manager();

        sample_template(&manager);

        let ids = manager.list_templates().unwrap();
        assert_eq!(ids, vec!["llm-default".to_string()]);

        let template = manager.get_template("llm-default").unwrap();
        assert_eq!(template.name, "LLM Default");
        assert_eq!(template.secret_keys, vec!["api_key".to_string()]);
        assert!(template.settings.contains_key("model"));

        // Duplicate id rejected
        assert!(manager
            .create_template(
                "llm-default",
                "X",
                "",
                HashMap::new(),
                HashMap::new(),
                Vec::new(),
                HashMap::new()
            )
            .is_err());

        assert!(manager.delete_template("llm-default").unwrap());
        assert!(manager.get_template("llm-default").is_err());
    }

    #[test]
    fn test_apply_template() {
        let (manager, _temp) = create_test_manager();
        sample_template(&manager);

        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("old-model"))
            .unwrap();
        manager
            .set_setting("agent-a", "keep", ConfigValue::from("value"))
            .unwrap();

        let config = manager.apply_template("agent-a", "llm-default").unwrap();
        // Template wins for model
        assert_eq!(
            config.settings.get("model").unwrap().as_str(),
            Some("gpt-4o")
        );
        // Unrelated settings preserved
        assert_eq!(config.settings.get("keep").unwrap().as_str(), Some("value"));
        // Secret key reserved
        assert!(config.secrets.contains_key("api_key"));
        // Version bumped
        assert!(config.version >= 2);

        // Applying to a nonexistent agent creates it
        let created = manager.apply_template("new-agent", "llm-default").unwrap();
        assert_eq!(created.agent_id, "new-agent");
        assert_eq!(
            created.settings.get("model").unwrap().as_str(),
            Some("gpt-4o")
        );
    }

    #[test]
    fn test_save_config_as_template_omits_secrets() {
        let (manager, _temp) = create_test_manager();

        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        manager
            .set_env_var("agent-a", "OPENAI_API_KEY", "sk-real")
            .unwrap();
        // Inject a secret directly (reload first to pick up the setting above)
        let mut with_secret = manager.load_config("agent-a").unwrap();
        with_secret
            .secrets
            .insert("api_key".to_string(), "super-secret".to_string());
        manager.save_config(&with_secret).unwrap();

        let template = manager
            .save_config_as_template("agent-a", "from-agent", "From Agent", "desc")
            .unwrap();

        // Secret VALUES never stored; only key names reserved
        assert_eq!(template.secret_keys, vec!["api_key".to_string()]);
        assert!(!serde_yaml::to_string(&template)
            .unwrap()
            .contains("super-secret"));
        assert_eq!(
            template.settings.get("model").unwrap().as_str(),
            Some("gpt-4o")
        );
        assert!(template
            .environment_variables
            .contains_key("OPENAI_API_KEY"));
    }

    #[test]
    fn test_secret_keystore_roundtrip_and_rotation() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();

        manager
            .set_secret("agent-a", "api_key", "sk-top-secret")
            .unwrap();
        assert_eq!(
            manager.get_secret("agent-a", "api_key").unwrap().as_deref(),
            Some("sk-top-secret")
        );

        // Secret value must NOT appear in the agent config file.
        let config = manager.load_config("agent-a").unwrap();
        assert!(!serde_yaml::to_string(&config)
            .unwrap()
            .contains("sk-top-secret"));

        // Listing returns redacted values only.
        let infos = manager.list_secrets(Some("agent-a")).unwrap();
        assert_eq!(infos.len(), 1);
        assert!(!infos[0].redacted_value.contains("top-secret"));

        // Rotation archives the old value.
        let rotation = manager
            .rotate_secret("agent-a", "api_key", "sk-new-key")
            .unwrap();
        assert!(rotation.rotated);
        assert_eq!(
            manager.get_secret("agent-a", "api_key").unwrap().as_deref(),
            Some("sk-new-key")
        );
        let infos = manager.list_secrets(Some("agent-a")).unwrap();
        assert_eq!(infos[0].rotated_count, 1);

        // Delete.
        assert!(manager.delete_secret("agent-a", "api_key").unwrap());
        assert_eq!(manager.get_secret("agent-a", "api_key").unwrap(), None);
    }

    #[test]
    fn test_secret_migrate_from_inline_config() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        let mut config = manager.load_config("agent-a").unwrap();
        config
            .secrets
            .insert("api_key".to_string(), "legacy-inline".to_string());
        manager.save_config(&config).unwrap();

        // Legacy fallback read works.
        assert_eq!(
            manager.get_secret("agent-a", "api_key").unwrap().as_deref(),
            Some("legacy-inline")
        );

        // Migration moves the value into the keystore and blanks the config.
        assert!(manager.migrate_secret("agent-a", "api_key").unwrap());
        assert_eq!(
            manager.get_secret("agent-a", "api_key").unwrap().as_deref(),
            Some("legacy-inline")
        );
        let config = manager.load_config("agent-a").unwrap();
        assert!(!serde_yaml::to_string(&config)
            .unwrap()
            .contains("legacy-inline"));

        // Migrating again is a no-op.
        assert!(!manager.migrate_secret("agent-a", "api_key").unwrap());
    }
}
