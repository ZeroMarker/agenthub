//! User & permission management (config module domain).
//!
//! Users and role-based permissions live in the AgentHub config directory
//! (`users.yaml` / `permissions.yaml`). The `admin` role bypasses all checks;
//! `operator` grants write access, `viewer` grants read access. Finer-grained
//! permissions can be granted per module and per agent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

/// Built-in roles. `admin` bypasses all permission checks.
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_OPERATOR: &str = "operator";
pub const ROLE_VIEWER: &str = "viewer";

/// A user of the AgentHub workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A permission grants `action` on a module and/or agent to a user.
///
/// `module` and `agent` are optional — `None` means "all". The wildcard `"*"`
/// is accepted too. Supported actions: `read`, `write`, `admin`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub user_id: String,
    pub action: String,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    pub granted_at: DateTime<Utc>,
    #[serde(default)]
    pub granted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UsersFile {
    #[serde(default)]
    users: Vec<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PermissionsFile {
    #[serde(default)]
    permissions: Vec<Permission>,
}

/// Manages users and permissions stored in the AgentHub config directory.
pub struct UserManager {
    base_dir: PathBuf,
}

impl UserManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    fn users_path(&self) -> PathBuf {
        self.base_dir.join("users.yaml")
    }

    fn permissions_path(&self) -> PathBuf {
        self.base_dir.join("permissions.yaml")
    }

    // ---- file I/O ---------------------------------------------------------

    fn load_users_file(&self) -> Result<UsersFile> {
        let path = self.users_path();
        if !path.exists() {
            return Ok(UsersFile::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to read users: {}", e)))?;
        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse users: {}", e)))
    }

    fn save_users_file(&self, file: &UsersFile) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to create config dir: {}", e))
        })?;
        let content = serde_yaml::to_string(file)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to serialize users: {}", e)))?;
        std::fs::write(self.users_path(), content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write users: {}", e)))
    }

    fn load_permissions_file(&self) -> Result<PermissionsFile> {
        let path = self.permissions_path();
        if !path.exists() {
            return Ok(PermissionsFile::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to read permissions: {}", e))
        })?;
        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to parse permissions: {}", e)))
    }

    fn save_permissions_file(&self, file: &PermissionsFile) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to create config dir: {}", e))
        })?;
        let content = serde_yaml::to_string(file).map_err(|e| {
            AgentHubError::ConfigError(format!("Failed to serialize permissions: {}", e))
        })?;
        std::fs::write(self.permissions_path(), content)
            .map_err(|e| AgentHubError::ConfigError(format!("Failed to write permissions: {}", e)))
    }

    /// Import a full user list (used by backup restore).
    pub fn import_users(&self, users: &[User]) -> Result<()> {
        let file = UsersFile {
            users: users.to_vec(),
        };
        self.save_users_file(&file)
    }

    /// Import a full permission list (used by backup restore).
    pub fn import_permissions(&self, permissions: &[Permission]) -> Result<()> {
        let file = PermissionsFile {
            permissions: permissions.to_vec(),
        };
        self.save_permissions_file(&file)
    }

    // ---- users ------------------------------------------------------------

    /// Ensure an `admin` user exists (created on first use with default
    /// credentials). Returns the admin user id.
    pub fn ensure_default_admin(&self) -> Result<String> {
        let mut file = self.load_users_file()?;
        if !file
            .users
            .iter()
            .any(|u| u.roles.contains(&ROLE_ADMIN.to_string()))
        {
            let now = Utc::now();
            file.users.push(User {
                id: "admin".to_string(),
                name: "Administrator".to_string(),
                email: None,
                roles: vec![ROLE_ADMIN.to_string()],
                created_at: now,
                updated_at: now,
            });
            self.save_users_file(&file)?;
        }
        Ok("admin".to_string())
    }

    pub fn create_user(
        &self,
        id: &str,
        name: &str,
        email: Option<&str>,
        roles: Vec<String>,
    ) -> Result<User> {
        self.ensure_default_admin()?;
        let mut file = self.load_users_file()?;
        if file.users.iter().any(|u| u.id == id) {
            return Err(AgentHubError::ConfigError(format!(
                "User already exists: {}",
                id
            )));
        }
        let now = Utc::now();
        let user = User {
            id: id.to_string(),
            name: name.to_string(),
            email: email.map(|e| e.to_string()),
            roles,
            created_at: now,
            updated_at: now,
        };
        file.users.push(user.clone());
        self.save_users_file(&file)?;
        Ok(user)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        self.ensure_default_admin()?;
        let mut file = self.load_users_file()?;
        file.users.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(file.users)
    }

    pub fn get_user(&self, id: &str) -> Result<User> {
        self.ensure_default_admin()?;
        let file = self.load_users_file()?;
        file.users
            .into_iter()
            .find(|u| u.id == id)
            .ok_or_else(|| AgentHubError::ConfigError(format!("User not found: {}", id)))
    }

    pub fn update_user(&self, id: &str, name: Option<&str>, email: Option<&str>) -> Result<User> {
        let mut file = self.load_users_file()?;
        let user = file
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AgentHubError::ConfigError(format!("User not found: {}", id)))?;
        if let Some(name) = name {
            user.name = name.to_string();
        }
        if let Some(email) = email {
            user.email = Some(email.to_string());
        }
        user.updated_at = Utc::now();
        let user = user.clone();
        self.save_users_file(&file)?;
        Ok(user)
    }

    pub fn delete_user(&self, id: &str) -> Result<bool> {
        let mut file = self.load_users_file()?;
        let before = file.users.len();
        file.users.retain(|u| u.id != id);
        if file.users.len() == before {
            return Ok(false);
        }
        self.save_users_file(&file)?;
        // Drop permissions belonging to the user.
        let mut perms = self.load_permissions_file()?;
        perms.permissions.retain(|p| p.user_id != id);
        self.save_permissions_file(&perms)?;
        Ok(true)
    }

    pub fn add_role(&self, id: &str, role: &str) -> Result<User> {
        let mut file = self.load_users_file()?;
        let user = file
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AgentHubError::ConfigError(format!("User not found: {}", id)))?;
        if !user.roles.iter().any(|r| r == role) {
            user.roles.push(role.to_string());
            user.updated_at = Utc::now();
        }
        let user = user.clone();
        self.save_users_file(&file)?;
        Ok(user)
    }

    pub fn remove_role(&self, id: &str, role: &str) -> Result<User> {
        let mut file = self.load_users_file()?;
        let user = file
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AgentHubError::ConfigError(format!("User not found: {}", id)))?;
        user.roles.retain(|r| r != role);
        user.updated_at = Utc::now();
        let user = user.clone();
        self.save_users_file(&file)?;
        Ok(user)
    }

    // ---- permissions ------------------------------------------------------

    pub fn grant_permission(
        &self,
        user_id: &str,
        action: &str,
        module: Option<&str>,
        agent: Option<&str>,
        granted_by: Option<&str>,
    ) -> Result<Permission> {
        self.get_user(user_id)?;
        if !["read", "write", "admin"].contains(&action) {
            return Err(AgentHubError::ConfigError(format!(
                "Invalid action '{}' (expected read|write|admin)",
                action
            )));
        }
        let mut file = self.load_permissions_file()?;
        // Idempotent: replace an existing identical grant.
        file.permissions.retain(|p| {
            !(p.user_id == user_id
                && p.action == action
                && p.module.as_deref() == module
                && p.agent.as_deref() == agent)
        });
        let permission = Permission {
            user_id: user_id.to_string(),
            action: action.to_string(),
            module: module.map(|m| m.to_string()),
            agent: agent.map(|a| a.to_string()),
            granted_at: Utc::now(),
            granted_by: granted_by.map(|g| g.to_string()),
        };
        file.permissions.push(permission.clone());
        self.save_permissions_file(&file)?;
        Ok(permission)
    }

    pub fn revoke_permission(
        &self,
        user_id: &str,
        action: &str,
        module: Option<&str>,
        agent: Option<&str>,
    ) -> Result<bool> {
        let mut file = self.load_permissions_file()?;
        let before = file.permissions.len();
        file.permissions.retain(|p| {
            !(p.user_id == user_id
                && p.action == action
                && p.module.as_deref() == module
                && p.agent.as_deref() == agent)
        });
        if file.permissions.len() == before {
            return Ok(false);
        }
        self.save_permissions_file(&file)?;
        Ok(true)
    }

    /// List permissions, optionally filtered by user.
    pub fn list_permissions(&self, user_id: Option<&str>) -> Result<Vec<Permission>> {
        let file = self.load_permissions_file()?;
        let mut perms: Vec<Permission> = file
            .permissions
            .into_iter()
            .filter(|p| user_id.is_none_or(|uid| p.user_id == uid))
            .collect();
        perms.sort_by(|a, b| a.user_id.cmp(&b.user_id).then(a.action.cmp(&b.action)));
        Ok(perms)
    }

    /// Check whether `user_id` may perform `action` on `module`/`agent`.
    ///
    /// Admin role always passes. Otherwise an exact grant wins, then module
    /// (or agent) scoped grants, then fully wildcard grants.
    pub fn check_permission(
        &self,
        user_id: &str,
        action: &str,
        module: Option<&str>,
        agent: Option<&str>,
    ) -> Result<bool> {
        let user = self.get_user(user_id)?;
        if user.roles.contains(&ROLE_ADMIN.to_string()) {
            return Ok(true);
        }
        let file = self.load_permissions_file()?;
        let mut allowed = false;
        for p in &file.permissions {
            if p.user_id != user_id {
                continue;
            }
            if !action_matches(&p.action, action) {
                continue;
            }
            // Module/agent scoping: a grant must cover the requested scope.
            let module_ok = match (&p.module, module) {
                (None, _) => true,
                (Some(m), Some(req)) => m == "*" || m == req,
                (Some(_), None) => false,
            };
            if !module_ok {
                continue;
            }
            let agent_ok = match (&p.agent, agent) {
                (None, _) => true,
                (Some(a), Some(req)) => a == "*" || a == req,
                (Some(_), None) => false,
            };
            if !agent_ok {
                continue;
            }
            allowed = true;
            break;
        }
        Ok(allowed)
    }

    /// Convenience: check permission against every module for a user.
    pub fn permissions_map(&self, user_id: Option<&str>) -> Result<HashMap<String, Vec<String>>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for p in self.list_permissions(user_id)? {
            map.entry(p.user_id.clone()).or_default().push(format!(
                "{}{}{}",
                p.action,
                p.module
                    .as_deref()
                    .map(|m| format!(":{m}"))
                    .unwrap_or_default(),
                p.agent
                    .as_deref()
                    .map(|a| format!("@{}", a))
                    .unwrap_or_default()
            ));
        }
        Ok(map)
    }
}

fn action_matches(grant: &str, requested: &str) -> bool {
    if grant == "*" || grant == requested {
        return true;
    }
    // "write" implies "read"; "admin" implies "write" and "read".
    match grant {
        "write" => requested == "read",
        "admin" => matches!(requested, "read" | "write"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn manager(base: &std::path::Path) -> UserManager {
        UserManager::new(base.to_path_buf())
    }

    #[test]
    fn test_ensure_default_admin() {
        let temp = TempDir::new().unwrap();
        let m = manager(temp.path());
        assert_eq!(m.ensure_default_admin().unwrap(), "admin");
        let users = m.list_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, "admin");
        assert!(users[0].roles.contains(&ROLE_ADMIN.to_string()));
    }

    #[test]
    fn test_user_crud() {
        let temp = TempDir::new().unwrap();
        let m = manager(temp.path());

        let user = m
            .create_user(
                "alice",
                "Alice",
                Some("alice@example.com"),
                vec!["viewer".to_string()],
            )
            .unwrap();
        assert_eq!(user.id, "alice");
        assert_eq!(user.email.as_deref(), Some("alice@example.com"));

        // Duplicate rejected
        assert!(m.create_user("alice", "A", None, Vec::new()).is_err());

        // Update
        let updated = m.update_user("alice", Some("Alice B"), None).unwrap();
        assert_eq!(updated.name, "Alice B");

        // Roles
        let with_role = m.add_role("alice", "operator").unwrap();
        assert!(with_role.roles.contains(&"operator".to_string()));
        let without = m.remove_role("alice", "operator").unwrap();
        assert!(!without.roles.contains(&"operator".to_string()));

        // Delete removes the user and its permissions
        m.grant_permission("alice", "read", Some("config"), None, Some("admin"))
            .unwrap();
        assert!(m.delete_user("alice").unwrap());
        assert!(m.get_user("alice").is_err());
        assert!(m.list_permissions(Some("alice")).unwrap().is_empty());

        // Deleting again is a no-op
        assert!(!m.delete_user("alice").unwrap());
    }

    #[test]
    fn test_permission_grant_revoke_list() {
        let temp = TempDir::new().unwrap();
        let m = manager(temp.path());
        m.create_user("bob", "Bob", None, Vec::new()).unwrap();

        m.grant_permission("bob", "write", Some("config"), None, None)
            .unwrap();
        m.grant_permission("bob", "read", Some("session"), Some("codex"), None)
            .unwrap();

        let perms = m.list_permissions(Some("bob")).unwrap();
        assert_eq!(perms.len(), 2);

        // Revoke
        assert!(m
            .revoke_permission("bob", "write", Some("config"), None)
            .unwrap());
        assert!(!m
            .revoke_permission("bob", "write", Some("config"), None)
            .unwrap());
        assert_eq!(m.list_permissions(Some("bob")).unwrap().len(), 1);
    }

    #[test]
    fn test_check_permission_roles() {
        let temp = TempDir::new().unwrap();
        let m = manager(temp.path());
        m.create_user("viewer", "V", None, vec!["viewer".to_string()])
            .unwrap();
        m.create_user("operator", "O", None, vec!["operator".to_string()])
            .unwrap();

        // Roles alone grant nothing.
        assert!(!m
            .check_permission("viewer", "read", Some("config"), None)
            .unwrap());

        // Explicit grant works.
        m.grant_permission("viewer", "read", None, None, None)
            .unwrap();
        assert!(m
            .check_permission("viewer", "read", Some("config"), Some("codex"))
            .unwrap());

        // write implies read.
        m.grant_permission("operator", "write", Some("session"), None, None)
            .unwrap();
        assert!(m
            .check_permission("operator", "read", Some("session"), None)
            .unwrap());
        assert!(!m
            .check_permission("operator", "write", Some("config"), None)
            .unwrap());

        // admin bypasses everything.
        assert!(m
            .check_permission("admin", "write", Some("anything"), Some("any-agent"))
            .unwrap());
    }

    #[test]
    fn test_check_permission_scoping() {
        let temp = TempDir::new().unwrap();
        let m = manager(temp.path());
        m.create_user("carol", "Carol", None, Vec::new()).unwrap();

        // Agent-scoped grant only covers that agent.
        m.grant_permission("carol", "write", None, Some("codex"), None)
            .unwrap();
        assert!(m
            .check_permission("carol", "write", None, Some("codex"))
            .unwrap());
        assert!(!m
            .check_permission("carol", "write", None, Some("other"))
            .unwrap());

        // Module-scoped grant covers that module only.
        m.grant_permission("carol", "read", Some("prompt"), None, None)
            .unwrap();
        assert!(m
            .check_permission("carol", "read", Some("prompt"), None)
            .unwrap());
        assert!(!m
            .check_permission("carol", "read", Some("memory"), None)
            .unwrap());
    }

    #[test]
    fn test_invalid_action_rejected() {
        let temp = TempDir::new().unwrap();
        let m = manager(temp.path());
        assert!(m
            .grant_permission("admin", "delete", None, None, None)
            .is_err());
    }

    #[test]
    fn test_list_users_corrupt_file_errors() {
        let temp = TempDir::new().unwrap();
        let manager = UserManager::new(temp.path().to_path_buf());
        std::fs::write(temp.path().join("users.yaml"), "users: \"unterminated").unwrap();
        assert!(manager.list_users().is_err());
    }
}
