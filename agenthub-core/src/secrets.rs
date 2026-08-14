use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::storage::atomic_write;

/// One previously active value for a secret, kept during the rotation grace
/// period so a failed rollout can be rolled back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousSecret {
    pub value: String,
    pub rotated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Rotated-out values (most recent first), kept for rollback.
    #[serde(default)]
    pub previous: Vec<PreviousSecret>,
}

impl SecretEntry {
    fn new(value: String) -> Self {
        let now = Utc::now();
        Self {
            value,
            created_at: now,
            updated_at: now,
            previous: Vec::new(),
        }
    }
}

/// Result of a secret rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    /// Composite key `agent_id.key`.
    pub key: String,
    /// True when a previous value existed and was archived.
    pub rotated: bool,
    /// Number of archived previous values (including the one just rotated out).
    pub previous_count: usize,
    pub rotated_at: DateTime<Utc>,
}

/// Non-sensitive metadata about a stored secret (never includes the value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    /// Composite key `agent_id.key`.
    pub key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rotated_count: usize,
    pub redacted_value: String,
}

/// File-backed secret keystore.
///
/// Secrets live in `<config_dir>/secrets.yaml`, a single file that is created
/// with restrictive permissions (`0600` on Unix) so values never sit inside the
/// agent config YAML files. The OS keyring was evaluated but rejected for the
/// core library because it drags in platform-specific system dependencies
/// (libsecret/Keychain/DPAPI) and fails on headless Linux; the file keystore
/// keeps the same "values never in config files or templates" property with
/// zero external dependencies. A future `keyring` backend can be slotted in
/// behind the same interface.
#[derive(Debug, Clone)]
pub struct SecretStore {
    dir: PathBuf,
    secrets: HashMap<String, SecretEntry>,
}

impl SecretStore {
    pub fn new(config_dir: PathBuf) -> Self {
        let mut store = Self {
            dir: config_dir.join("secrets"),
            secrets: HashMap::new(),
        };
        store.load();
        store
    }

    pub fn base_dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self) -> PathBuf {
        self.dir.join("secrets.yaml")
    }

    fn load(&mut self) {
        let path = self.path();
        if !path.exists() {
            return;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_yaml::from_str::<HashMap<String, SecretEntry>>(&content) {
                self.secrets = parsed;
            }
        }
    }

    fn persist(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to create secrets dir: {}", e))
        })?;
        let content = serde_yaml::to_string(&self.secrets).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize secrets: {}", e))
        })?;
        let path = self.path();
        atomic_write(&path, &content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write secrets: {}", e)))?;
        // Restrictive permissions: owner read/write only (no-op on Windows).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    fn composite(agent_id: &str, key: &str) -> String {
        format!("{}.{}", agent_id, key)
    }

    /// Store (or overwrite) a secret value. The value is never written to the
    /// agent config file; it lives only in the keystore.
    pub fn set(&mut self, agent_id: &str, key: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            return Err(AgentHubError::ConfigError(
                "Secret value must not be empty".to_string(),
            ));
        }
        let composite = Self::composite(agent_id, key);
        let now = Utc::now();
        match self.secrets.get_mut(&composite) {
            Some(entry) => {
                entry.value = value.to_string();
                entry.updated_at = now;
            }
            None => {
                self.secrets
                    .insert(composite, SecretEntry::new(value.to_string()));
            }
        }
        self.persist()
    }

    /// Read a secret value. Returns `None` when the key is not stored.
    pub fn get(&self, agent_id: &str, key: &str) -> Option<String> {
        self.secrets
            .get(&Self::composite(agent_id, key))
            .map(|e| e.value.clone())
    }

    pub fn has(&self, agent_id: &str, key: &str) -> bool {
        self.secrets.contains_key(&Self::composite(agent_id, key))
    }

    /// Remove a secret. Returns true when a value was removed.
    pub fn delete(&mut self, agent_id: &str, key: &str) -> Result<bool> {
        let removed = self
            .secrets
            .remove(&Self::composite(agent_id, key))
            .is_some();
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// Rotate a secret: the current value is archived into `previous` and the
    /// new value becomes active. Returns the rotation record.
    pub fn rotate(&mut self, agent_id: &str, key: &str, new_value: &str) -> Result<RotationResult> {
        if new_value.is_empty() {
            return Err(AgentHubError::ConfigError(
                "New secret value must not be empty".to_string(),
            ));
        }
        let composite = Self::composite(agent_id, key);
        let now = Utc::now();
        let rotated = self.secrets.contains_key(&composite);
        let entry = self
            .secrets
            .entry(composite.clone())
            .or_insert_with(|| SecretEntry::new(String::new()));
        if rotated {
            let old = std::mem::replace(&mut entry.value, new_value.to_string());
            entry.previous.insert(
                0,
                PreviousSecret {
                    value: old,
                    rotated_at: now,
                },
            );
            entry.updated_at = now;
        } else {
            entry.value = new_value.to_string();
            entry.updated_at = now;
        }
        let previous_count = entry.previous.len();
        self.persist()?;
        Ok(RotationResult {
            key: composite,
            rotated,
            previous_count,
            rotated_at: now,
        })
    }

    /// List stored secret keys (optionally restricted to one agent) with
    /// redacted values only — never the raw values.
    pub fn list(&self, agent_id: Option<&str>) -> Vec<SecretInfo> {
        let mut infos: Vec<SecretInfo> = self
            .secrets
            .iter()
            .filter(|(k, _)| match agent_id {
                Some(a) => k.starts_with(&format!("{}.", a)),
                None => true,
            })
            .map(|(k, e)| SecretInfo {
                key: k.clone(),
                created_at: e.created_at,
                updated_at: e.updated_at,
                rotated_count: e.previous.len(),
                redacted_value: Self::redact(&e.value),
            })
            .collect();
        infos.sort_by(|a, b| a.key.cmp(&b.key));
        infos
    }

    /// Redact a value for display: `sk-abc123…7890` (first 4 + last 4 chars).
    pub fn redact(value: &str) -> String {
        if value.len() <= 8 {
            return "••••".to_string();
        }
        let bytes = value.as_bytes();
        let mut out = String::new();
        out.push_str(&value[..4.min(value.len())]);
        out.push('…');
        out.push_str(&value[value.len() - 4..]);
        let _ = bytes;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_store() -> (SecretStore, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let store = SecretStore::new(temp.path().to_path_buf());
        (store, temp)
    }

    #[test]
    fn test_set_get_delete() {
        let (mut store, _temp) = create_store();
        store.set("agent-a", "api_key", "sk-secret-123").unwrap();
        assert!(store.has("agent-a", "api_key"));
        assert_eq!(
            store.get("agent-a", "api_key").as_deref(),
            Some("sk-secret-123")
        );
        // Different agent isolation
        assert!(!store.has("agent-b", "api_key"));

        assert!(store.delete("agent-a", "api_key").unwrap());
        assert!(!store.has("agent-a", "api_key"));
        assert!(!store.delete("agent-a", "api_key").unwrap());
    }

    #[test]
    fn test_empty_value_rejected() {
        let (mut store, _temp) = create_store();
        assert!(store.set("agent-a", "api_key", "").is_err());
    }

    #[test]
    fn test_rotate_archives_previous() {
        let (mut store, _temp) = create_store();
        store.set("agent-a", "api_key", "old-key").unwrap();

        let result = store.rotate("agent-a", "api_key", "new-key").unwrap();
        assert!(result.rotated);
        assert_eq!(result.previous_count, 1);
        assert_eq!(store.get("agent-a", "api_key").as_deref(), Some("new-key"));

        // Rotating again keeps the full history
        store.rotate("agent-a", "api_key", "newer-key").unwrap();
        let infos = store.list(Some("agent-a"));
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].rotated_count, 2);
    }

    #[test]
    fn test_rotate_first_time_creates() {
        let (mut store, _temp) = create_store();
        let result = store.rotate("agent-a", "api_key", "first-key").unwrap();
        assert!(!result.rotated);
        assert_eq!(result.previous_count, 0);
        assert_eq!(
            store.get("agent-a", "api_key").as_deref(),
            Some("first-key")
        );
    }

    #[test]
    fn test_list_redacts_values() {
        let (mut store, _temp) = create_store();
        store
            .set("agent-a", "api_key", "sk-long-secret-value")
            .unwrap();
        store.set("agent-a", "other", "short").unwrap();
        store.set("agent-b", "api_key", "another-secret").unwrap();

        let all = store.list(None);
        assert_eq!(all.len(), 3);
        // Never contains raw values
        for info in &all {
            assert!(!info.redacted_value.contains("secret"));
            assert!(!info.key.contains("value"));
        }

        let only_a = store.list(Some("agent-a"));
        assert_eq!(only_a.len(), 2);
    }

    #[test]
    fn test_persists_across_reload() {
        let temp = tempfile::tempdir().unwrap();
        {
            let mut store = SecretStore::new(temp.path().to_path_buf());
            store.set("agent-a", "api_key", "persisted-value").unwrap();
        }
        let reloaded = SecretStore::new(temp.path().to_path_buf());
        assert_eq!(
            reloaded.get("agent-a", "api_key").as_deref(),
            Some("persisted-value")
        );
        // Restrictive permissions on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(temp.path().join("secrets/secrets.yaml")).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn test_redact_short_and_long() {
        assert_eq!(SecretStore::redact("abcdef"), "••••");
        assert_eq!(SecretStore::redact("sk-abcdefghijklmnop"), "sk-a…mnop");
    }

    #[test]
    fn test_corrupt_keystore_file_degrades_gracefully() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("secrets.yaml"), "secrets: [unterminated").unwrap();

        // A corrupt keystore must degrade to empty, never panic.
        let store = SecretStore::new(temp.path().to_path_buf());
        assert_eq!(store.get("agent-a", "api_key"), None);
        assert!(store.list(None).is_empty());

        // And a fresh write must recover the file.
        let mut store = store;
        store.set("agent-a", "api_key", "sk-new").unwrap();
        assert_eq!(store.get("agent-a", "api_key").as_deref(), Some("sk-new"));
    }
}
