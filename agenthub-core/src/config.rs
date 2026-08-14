use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::secrets::{RotationResult, SecretInfo, SecretStore};
use crate::storage::{atomic_write, is_safe_id};

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
    #[serde(default = "default_config_version")]
    pub version: u32,
    pub environment: Environment,
    #[serde(default)]
    pub settings: HashMap<String, ConfigValue>,
    #[serde(default)]
    pub secrets: HashMap<String, String>,
    #[serde(default)]
    pub environment_variables: HashMap<String, String>,
    #[serde(default)]
    pub custom: HashMap<String, ConfigValue>,
    #[serde(default)]
    pub metadata: ConfigMetadata,
}

/// Fallback version used when an imported/legacy config omits it.
fn default_config_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            created_by: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Severity of a config validation finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
}

/// A single validation finding on an agent config (or one of its settings).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigIssue {
    /// Dotted path of the offending field, e.g. `settings.temperature`.
    pub key: String,
    /// Human-readable description of the problem.
    pub message: String,
    pub severity: IssueSeverity,
}

/// Validation rule + fallback default for a known setting key.
struct SettingRule {
    /// Default applied when the key is missing or its value is out of range.
    /// `None` means the key cannot be defaulted (only reported).
    default: Option<ConfigValue>,
    /// Returns an error message when `value` violates the rule.
    check: fn(&ConfigValue) -> Option<String>,
}

fn known_setting_rules() -> [(&'static str, SettingRule); 6] {
    [
        (
            "temperature",
            SettingRule {
                default: Some(ConfigValue::Number(0.7)),
                check: |v| match v.as_f64() {
                    Some(n) if (0.0..=2.0).contains(&n) => None,
                    Some(n) => Some(format!("must be in [0.0, 2.0], got {n}")),
                    None => Some("must be a number".to_string()),
                },
            },
        ),
        (
            "top_p",
            SettingRule {
                default: Some(ConfigValue::Number(1.0)),
                check: |v| match v.as_f64() {
                    Some(n) if n > 0.0 && n <= 1.0 => None,
                    Some(n) => Some(format!("must be in (0.0, 1.0], got {n}")),
                    None => Some("must be a number".to_string()),
                },
            },
        ),
        (
            "max_tokens",
            SettingRule {
                default: Some(ConfigValue::Number(4096.0)),
                check: |v| match v.as_f64() {
                    Some(n) if n >= 1.0 && n.fract() == 0.0 => None,
                    Some(n) => Some(format!("must be a positive integer, got {n}")),
                    None => Some("must be a number".to_string()),
                },
            },
        ),
        (
            "frequency_penalty",
            SettingRule {
                default: Some(ConfigValue::Number(0.0)),
                check: |v| match v.as_f64() {
                    Some(n) if (-2.0..=2.0).contains(&n) => None,
                    Some(n) => Some(format!("must be in [-2.0, 2.0], got {n}")),
                    None => Some("must be a number".to_string()),
                },
            },
        ),
        (
            "presence_penalty",
            SettingRule {
                default: Some(ConfigValue::Number(0.0)),
                check: |v| match v.as_f64() {
                    Some(n) if (-2.0..=2.0).contains(&n) => None,
                    Some(n) => Some(format!("must be in [-2.0, 2.0], got {n}")),
                    None => Some("must be a number".to_string()),
                },
            },
        ),
        (
            "model",
            SettingRule {
                // No safe default: an empty model would break every query.
                default: None,
                check: |v| match v.as_str() {
                    Some(s) if !s.trim().is_empty() => None,
                    _ => Some("must be a non-empty string".to_string()),
                },
            },
        ),
    ]
}

/// Validate the known settings of an agent config without mutating anything.
/// Out-of-range values are reported as errors; missing settings that have no
/// safe default are reported as warnings.
pub fn validate_settings(settings: &HashMap<String, ConfigValue>) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();
    for (key, rule) in known_setting_rules() {
        match settings.get(key) {
            None => {
                if rule.default.is_none() {
                    issues.push(ConfigIssue {
                        key: format!("settings.{key}"),
                        message: format!("missing required setting `{key}`"),
                        severity: IssueSeverity::Warning,
                    });
                }
            }
            Some(value) => {
                if let Some(msg) = (rule.check)(value) {
                    issues.push(ConfigIssue {
                        key: format!("settings.{key}"),
                        message: msg,
                        severity: IssueSeverity::Error,
                    });
                }
            }
        }
    }
    issues
}

/// Apply default-value fallbacks for known settings in place: missing keys are
/// filled with their defaults and out-of-range values are replaced. Returns the
/// list of corrections applied.
pub fn normalize_settings(settings: &mut HashMap<String, ConfigValue>) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();
    for (key, rule) in known_setting_rules() {
        match settings.get(key) {
            None => {
                if let Some(default) = &rule.default {
                    settings.insert(key.to_string(), default.clone());
                    issues.push(ConfigIssue {
                        key: format!("settings.{key}"),
                        message: format!("missing, defaulted to {default}"),
                        severity: IssueSeverity::Warning,
                    });
                }
            }
            Some(value) => {
                if let Some(msg) = (rule.check)(value) {
                    match &rule.default {
                        Some(default) => {
                            settings.insert(key.to_string(), default.clone());
                            issues.push(ConfigIssue {
                                key: format!("settings.{key}"),
                                message: format!("{msg}; replaced with default {default}"),
                                severity: IssueSeverity::Error,
                            });
                        }
                        None => issues.push(ConfigIssue {
                            key: format!("settings.{key}"),
                            message: msg,
                            severity: IssueSeverity::Error,
                        }),
                    }
                }
            }
        }
    }
    issues
}

/// Validate a whole agent config (known settings + env var names).
pub fn validate_config(config: &AgentConfig) -> Vec<ConfigIssue> {
    let mut issues = validate_settings(&config.settings);
    for key in config.environment_variables.keys() {
        if key.trim().is_empty() {
            issues.push(ConfigIssue {
                key: "environment_variables".to_string(),
                message: "environment variable name must not be empty".to_string(),
                severity: IssueSeverity::Error,
            });
        }
    }
    issues
}

pub struct ConfigManager {
    config_dir: PathBuf,
    current_environment: Environment,
    /// Serializes read-modify-write mutations so concurrent callers cannot
    /// overwrite each other's in-flight changes (lost updates).
    write_lock: std::sync::Mutex<()>,
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
            write_lock: std::sync::Mutex::new(()),
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

        atomic_write(&path, &content)
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        if config.settings.get(key) == Some(&value) {
            return Ok(());
        }
        self.snapshot_current(&config)?;
        config.settings.insert(key.to_string(), value);
        self.bump_and_save(&mut config)
    }

    pub fn unset_setting(&self, agent_id: &str, key: &str) -> Result<bool> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut config = self.load_config(agent_id)?;
        if !config.settings.contains_key(key) {
            return Ok(false);
        }
        self.snapshot_current(&config)?;
        config.settings.remove(key);
        self.bump_and_save(&mut config)?;
        Ok(true)
    }

    pub fn get_custom(&self, agent_id: &str, key: &str) -> Result<Option<ConfigValue>> {
        let config = self.load_config(agent_id)?;
        Ok(config.custom.get(key).cloned())
    }

    pub fn set_custom(&self, agent_id: &str, key: &str, value: ConfigValue) -> Result<()> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        if config.custom.get(key) == Some(&value) {
            return Ok(());
        }
        self.snapshot_current(&config)?;
        config.custom.insert(key.to_string(), value);
        self.bump_and_save(&mut config)
    }

    pub fn set_env_var(&self, agent_id: &str, key: &str, value: &str) -> Result<()> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        if config.environment_variables.get(key).map(String::as_str) == Some(value) {
            return Ok(());
        }
        self.snapshot_current(&config)?;
        config
            .environment_variables
            .insert(key.to_string(), value.to_string());
        self.bump_and_save(&mut config)
    }

    pub fn get_env_var(&self, agent_id: &str, key: &str) -> Result<Option<String>> {
        let config = self.load_config(agent_id)?;
        Ok(config.environment_variables.get(key).cloned())
    }

    // ---------------------------------------------------------------------
    // Validation & default-value fallback
    // ---------------------------------------------------------------------

    /// Load a config and apply default-value fallbacks in memory only (nothing
    /// is persisted). Invalid known settings are replaced with their defaults.
    pub fn load_config_normalized(&self, agent_id: &str) -> Result<AgentConfig> {
        let mut config = self.load_config(agent_id)?;
        let _ = normalize_settings(&mut config.settings);
        Ok(config)
    }

    /// Validate and repair an agent config on disk: apply default-value
    /// fallbacks, bump the version and record the change in history. Returns
    /// the corrections applied (empty when the config was already valid).
    pub fn repair_config(&self, agent_id: &str) -> Result<Vec<ConfigIssue>> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut config = self.load_config(agent_id)?;
        let pre_repair = config.clone();
        let issues = normalize_settings(&mut config.settings);
        if !issues.is_empty() {
            self.snapshot_current(&pre_repair)?;
            self.bump_and_save(&mut config)?;
        }
        Ok(issues)
    }

    // -----------------------------------------------------------------------
    // Change history & rollback
    // -----------------------------------------------------------------------

    fn history_dir(&self, agent_id: &str) -> PathBuf {
        self.config_dir.join("history").join(agent_id)
    }

    fn history_path(&self, agent_id: &str, version: u32) -> PathBuf {
        self.history_dir(agent_id)
            .join(format!("v{}.yaml", version))
    }

    /// Snapshot the current state of a config before it is mutated. Inline
    /// secret values are redacted (keys preserved, values blanked) so
    /// plaintext secrets never land in the change history.
    fn snapshot_current(&self, config: &AgentConfig) -> Result<()> {
        let dir = self.history_dir(&config.agent_id);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to create history dir: {}", e))
        })?;

        let mut redacted = config.clone();
        for value in redacted.secrets.values_mut() {
            *value = String::new();
        }

        let path = self.history_path(&config.agent_id, config.version);
        let content = serde_yaml::to_string(&redacted).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize history version: {}", e))
        })?;

        atomic_write(&path, &content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write history: {}", e)))?;

        Ok(())
    }

    /// Bump the version and persist via [`Self::save_config`]. Callers must
    /// snapshot the pre-mutation state with [`Self::snapshot_current`] first.
    fn bump_and_save(&self, config: &mut AgentConfig) -> Result<()> {
        config.version += 1;
        config.metadata.updated_at = Utc::now();
        self.save_config(config)
    }

    /// List the change history of an agent config, oldest first.
    pub fn list_history(&self, agent_id: &str) -> Result<Vec<AgentConfig>> {
        Self::validate_id("agent", agent_id)?;
        let dir = self.history_dir(agent_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read history dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::ConfigError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(config) = serde_yaml::from_str::<AgentConfig>(&content) {
                        versions.push(config);
                    }
                }
            }
        }

        versions.sort_by_key(|v| v.version);
        Ok(versions)
    }

    /// Load a specific historical version of an agent config.
    pub fn get_history(&self, agent_id: &str, version: u32) -> Result<AgentConfig> {
        Self::validate_id("agent", agent_id)?;
        let path = self.history_path(agent_id, version);
        if !path.exists() {
            return Err(AgentHubError::ConfigError(format!(
                "Config version {} not found for {}",
                version, agent_id
            )));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read history: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse history: {}", e)))
    }

    /// Roll back an agent config to a previous version. The current state is
    /// snapshotted first (no history is lost) and the version number keeps
    /// increasing. Live secret values are carried over because history
    /// snapshots redact them.
    pub fn rollback_config(&self, agent_id: &str, version: u32) -> Result<AgentConfig> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let current = self.load_config(agent_id)?;
        let historical = self.get_history(agent_id, version)?;

        self.snapshot_current(&current)?;

        let mut restored = historical;
        restored.agent_id = agent_id.to_string();
        restored.version = current.version + 1;
        for (key, value) in &current.secrets {
            if !value.is_empty() {
                restored
                    .secrets
                    .entry(key.clone())
                    .or_insert_with(|| value.clone());
            }
        }
        restored.secrets.retain(|_, value| !value.is_empty());
        restored.metadata.updated_at = Utc::now();

        self.save_config(&restored)?;
        Ok(restored)
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        self.secret_store().set(agent_id, key, value)?;
        // Drop any inline plaintext copy from the config file.
        if self.agent_config_path(agent_id).exists() {
            let mut config = self.load_config(agent_id).ok();
            if let Some(config) = config.as_mut() {
                if config.secrets.contains_key(key) {
                    self.snapshot_current(config)?;
                    config.secrets.remove(key);
                    config.version += 1;
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        self.secret_store().rotate(agent_id, key, new_value)
    }

    /// Move a legacy inline secret value from the agent config file into the
    /// keystore, then blank it in the file. Returns true when a value moved.
    pub fn migrate_secret(&self, agent_id: &str, key: &str) -> Result<bool> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        if !self.agent_config_path(agent_id).exists() {
            return Ok(false);
        }
        let mut config = self.load_config(agent_id)?;
        match config.secrets.remove(key) {
            Some(value) if !value.is_empty() => {
                self.secret_store().set(agent_id, key, &value)?;
                self.snapshot_current(&config)?;
                config.version += 1;
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let content = std::fs::read_to_string(input_path)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read import: {}", e)))?;

        let mut config: AgentConfig = serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse import: {}", e)))?;

        if let Some(id) = agent_id {
            config.agent_id = id.to_string();
        }

        // Snapshot the existing state before overwriting it (if any).
        if let Ok(existing) = self.load_config(&config.agent_id) {
            self.snapshot_current(&existing)?;
            config.version = existing.version.max(config.version) + 1;
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

        atomic_write(&path, &content)
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
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let template = self.get_template(template_id)?;
        let mut config = self
            .load_config(agent_id)
            .or_else(|_| self.create_config(agent_id))?;

        // Snapshot the pre-application state before merging template values.
        self.snapshot_current(&config)?;
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

    // ---- Validation & default-value fallback ----

    #[test]
    fn test_validate_settings_reports_out_of_range() {
        let mut settings = HashMap::new();
        settings.insert("temperature".to_string(), ConfigValue::from(3.5f64));
        settings.insert("top_p".to_string(), ConfigValue::from(0.0f64));
        settings.insert("max_tokens".to_string(), ConfigValue::from(-10.0f64));
        settings.insert("model".to_string(), ConfigValue::from(""));

        let issues = validate_settings(&settings);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert_eq!(errors.len(), 4, "issues: {:?}", issues);
    }

    #[test]
    fn test_validate_settings_ok_and_missing_required() {
        let mut settings = HashMap::new();
        settings.insert("temperature".to_string(), ConfigValue::from(1.0f64));
        settings.insert("model".to_string(), ConfigValue::from("gpt-4o"));
        assert!(validate_settings(&settings).is_empty());

        // Missing model has no safe default -> warning, not error.
        let mut settings = HashMap::new();
        settings.insert("temperature".to_string(), ConfigValue::from(1.0f64));
        let issues = validate_settings(&settings);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn test_normalize_settings_fills_defaults() {
        let mut settings = HashMap::new();
        settings.insert("temperature".to_string(), ConfigValue::from(9.9f64));
        settings.insert("model".to_string(), ConfigValue::from("gpt-4o"));

        let issues = normalize_settings(&mut settings);
        assert_eq!(
            settings.get("temperature").unwrap().as_f64(),
            Some(0.7) // out-of-range replaced with default
        );
        assert_eq!(settings.get("top_p").unwrap().as_f64(), Some(1.0)); // missing filled
        assert_eq!(settings.get("max_tokens").unwrap().as_f64(), Some(4096.0));
        // Unaffected key untouched.
        assert_eq!(settings.get("model").unwrap().as_str(), Some("gpt-4o"));
        // 1 replacement (temperature) + 4 missing defaults (top_p, max_tokens,
        // frequency_penalty, presence_penalty).
        assert_eq!(issues.len(), 5);
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Error));
    }

    #[test]
    fn test_load_config_normalized_does_not_persist() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "temperature", ConfigValue::from(5.0f64))
            .unwrap();

        let normalized = manager.load_config_normalized("agent-a").unwrap();
        assert_eq!(
            normalized.settings.get("temperature").unwrap().as_f64(),
            Some(0.7)
        );

        // On-disk config is untouched until repair runs.
        let raw = manager.load_config("agent-a").unwrap();
        assert_eq!(raw.settings.get("temperature").unwrap().as_f64(), Some(5.0));
    }

    #[test]
    fn test_repair_config_persists_and_bumps_version() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        let before = manager.load_config("agent-a").unwrap();
        assert_eq!(before.version, 1);

        let issues = manager.repair_config("agent-a").unwrap();
        // temperature + top_p + max_tokens + frequency_penalty + presence_penalty
        assert_eq!(issues.len(), 5);

        let after = manager.load_config("agent-a").unwrap();
        assert_eq!(after.version, 2);
        assert_eq!(
            after.settings.get("temperature").unwrap().as_f64(),
            Some(0.7)
        );
        assert_eq!(after.settings.get("top_p").unwrap().as_f64(), Some(1.0));

        // A second repair is a no-op (already valid).
        assert!(manager.repair_config("agent-a").unwrap().is_empty());
        assert_eq!(manager.load_config("agent-a").unwrap().version, 2);
    }

    #[test]
    fn test_legacy_config_with_missing_fields_parses() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        let path = manager.agent_config_path("agent-a");

        // Hand-written legacy config: no version, settings, metadata or secrets.
        std::fs::write(&path, "agent_id: agent-a\nenvironment: development\n").unwrap();

        let config = manager.load_config("agent-a").unwrap();
        assert_eq!(config.version, 1);
        assert!(config.settings.is_empty());
        assert!(config.secrets.is_empty());
        assert!(!config.metadata.created_at.to_rfc3339().is_empty());
    }

    // ---- Change history & rollback ----

    #[test]
    fn test_mutations_record_history() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();

        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        manager
            .set_setting("agent-a", "temperature", ConfigValue::from(0.5f64))
            .unwrap();
        manager.unset_setting("agent-a", "model").unwrap();

        let history = manager.list_history("agent-a").unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].version, 1);
        assert!(history[0].settings.is_empty());
        assert_eq!(history[1].version, 2);
        assert_eq!(
            history[1].settings.get("model").unwrap().as_str(),
            Some("gpt-4o")
        );
        assert_eq!(history[2].version, 3);
        assert!(history[2].settings.contains_key("temperature"));

        let live = manager.load_config("agent-a").unwrap();
        assert_eq!(live.version, 4);
        assert!(!live.settings.contains_key("model"));
    }

    #[test]
    fn test_identical_set_is_noop() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        let version_after_first = manager.load_config("agent-a").unwrap().version;

        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        assert_eq!(
            manager.load_config("agent-a").unwrap().version,
            version_after_first
        );
        assert_eq!(manager.list_history("agent-a").unwrap().len(), 1);
    }

    #[test]
    fn test_history_redacts_inline_secrets() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        let mut with_secret = manager.load_config("agent-a").unwrap();
        with_secret
            .secrets
            .insert("api_key".to_string(), "super-secret".to_string());
        manager.save_config(&with_secret).unwrap();

        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();

        let history = manager.list_history("agent-a").unwrap();
        for version in &history {
            let raw = serde_yaml::to_string(version).unwrap();
            assert!(!raw.contains("super-secret"));
            if version.secrets.contains_key("api_key") {
                assert_eq!(version.secrets.get("api_key").unwrap(), "");
            }
        }
    }

    #[test]
    fn test_rollback_config_restores_and_bumps() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("claude-3.5"))
            .unwrap();

        let current = manager.load_config("agent-a").unwrap();
        assert_eq!(
            current.settings.get("model").unwrap().as_str(),
            Some("claude-3.5")
        );
        let current_version = current.version;

        let restored = manager.rollback_config("agent-a", 2).unwrap();
        assert_eq!(restored.version, current_version + 1);
        assert_eq!(
            restored.settings.get("model").unwrap().as_str(),
            Some("gpt-4o")
        );

        // History grew: current state preserved before rollback.
        let history = manager.list_history("agent-a").unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history.last().unwrap().version, current_version);
        assert_eq!(
            history
                .last()
                .unwrap()
                .settings
                .get("model")
                .unwrap()
                .as_str(),
            Some("claude-3.5")
        );
    }

    #[test]
    fn test_rollback_preserves_live_secrets() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        manager
            .set_secret("agent-a", "api_key", "live-secret")
            .unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("claude-3.5"))
            .unwrap();

        let restored = manager.rollback_config("agent-a", 1).unwrap();
        // Secret lives in the keystore; the restored config must not blank it.
        assert_eq!(
            manager.get_secret("agent-a", "api_key").unwrap().as_deref(),
            Some("live-secret")
        );
        assert!(restored.secrets.values().all(|v| v.is_empty()));
    }

    #[test]
    fn test_get_history_missing_version() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        assert!(manager.get_history("agent-a", 99).is_err());
    }

    #[test]
    fn test_import_config_snapshots_existing() {
        let (manager, temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();

        // Craft an import file with a different model.
        let import_path = temp.path().join("import.yaml");
        std::fs::write(
            &import_path,
            "agent_id: agent-a\nenvironment: production\nsettings:\n  model: claude-3.5\n",
        )
        .unwrap();

        let imported = manager.import_config(&import_path, None).unwrap();
        assert_eq!(
            imported.settings.get("model").unwrap().as_str(),
            Some("claude-3.5")
        );
        assert_eq!(imported.environment, Environment::Production);

        // Prior live state was snapshotted before overwrite.
        let history = manager.list_history("agent-a").unwrap();
        assert!(history.iter().any(|v| v
            .settings
            .get("model")
            .is_some_and(|m| m.as_str() == Some("gpt-4o"))));
    }

    // ---- Negative & concurrency tests ----

    #[test]
    fn test_load_config_corrupt_yaml_errors() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        // Unclosed YAML quote -> parse error, never a panic.
        std::fs::write(
            manager.agent_config_path("agent-a"),
            "agent_id: \"unterminated",
        )
        .unwrap();
        assert!(manager.load_config("agent-a").is_err());

        // Invalid UTF-8 bytes -> read error.
        std::fs::write(
            manager.agent_config_path("agent-a"),
            [0xff, 0xfe, 0x00, 0x01],
        )
        .unwrap();
        assert!(manager.load_config("agent-a").is_err());

        // Corrupt configs must not crash directory listing.
        assert_eq!(manager.list_configs().unwrap(), vec!["agent-a".to_string()]);
    }

    #[test]
    fn test_import_config_corrupt_file_errors() {
        let (manager, temp) = create_test_manager();
        let bad = temp.path().join("bad.yaml");
        std::fs::write(&bad, "settings: [unterminated").unwrap();
        assert!(manager.import_config(&bad, None).is_err());
    }

    #[test]
    fn test_corrupt_history_entry_skipped_not_fatal() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        manager
            .set_setting("agent-a", "model", ConfigValue::from("gpt-4o"))
            .unwrap();
        manager
            .set_setting("agent-a", "temperature", ConfigValue::from(0.3f64))
            .unwrap();
        assert_eq!(manager.list_history("agent-a").unwrap().len(), 2);

        // Corrupt one history file in place; listing must skip it, not fail.
        std::fs::write(
            manager.history_path("agent-a", 1),
            "agent_id: \"unterminated",
        )
        .unwrap();
        let history = manager.list_history("agent-a").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version, 2);
    }

    #[test]
    fn test_concurrent_distinct_key_writes_no_lost_update() {
        use std::sync::{Arc, Barrier};
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        let manager = Arc::new(manager);

        // 8 threads × 25 distinct keys each, released simultaneously so the
        // read-modify-write cycles genuinely interleave.
        const THREADS: usize = 8;
        const WRITES: usize = 25;
        let barrier = Arc::new(Barrier::new(THREADS));
        let mut handles = Vec::new();
        for t in 0..THREADS {
            let manager = manager.clone();
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                for i in 0..WRITES {
                    let key = format!("k{}_{}", t, i);
                    let value = format!("v{}_{}", t, i);
                    manager
                        .set_setting("agent-a", &key, ConfigValue::from(value))
                        .unwrap();
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }

        // Every write survived (no lost updates) and the file parses cleanly.
        let config = manager.load_config("agent-a").unwrap();
        for t in 0..THREADS {
            for i in 0..WRITES {
                let key = format!("k{}_{}", t, i);
                assert_eq!(
                    config.settings.get(&key).and_then(|v| v.as_str()),
                    Some(format!("v{}_{}", t, i).as_str()),
                    "lost update for {}",
                    key
                );
            }
        }
        assert_eq!(config.version, 1 + (THREADS * WRITES) as u32);
    }

    #[test]
    fn test_concurrent_secret_sets_no_lost_update() {
        let (manager, _temp) = create_test_manager();
        manager.create_config("agent-a").unwrap();
        let manager = std::sync::Arc::new(manager);

        let mut handles = Vec::new();
        for i in 0..8 {
            let manager = manager.clone();
            handles.push(std::thread::spawn(move || {
                manager
                    .set_secret("agent-a", &format!("k{}", i), &format!("secret-{}", i))
                    .unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }

        for i in 0..8 {
            assert_eq!(
                manager
                    .get_secret("agent-a", &format!("k{}", i))
                    .unwrap()
                    .as_deref(),
                Some(format!("secret-{}", i).as_str())
            );
        }
    }
}
