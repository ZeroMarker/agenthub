use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::storage::is_safe_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub min_agenthub_version: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<SkillDependency>,
    #[serde(default)]
    pub config: HashMap<String, SkillConfigValue>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// Skill visibility scope. Resolution precedence (highest first):
/// `Project` > `User` > `Global`. Extra directories load after the user
/// scope but before the global one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillScope {
    Project,
    User,
    Global,
}

impl std::fmt::Display for SkillScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillScope::Project => write!(f, "project"),
            SkillScope::User => write!(f, "user"),
            SkillScope::Global => write!(f, "global"),
        }
    }
}

impl std::str::FromStr for SkillScope {
    type Err = AgentHubError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "project" => Ok(SkillScope::Project),
            "user" => Ok(SkillScope::User),
            "global" => Ok(SkillScope::Global),
            other => Err(AgentHubError::SkillError(format!(
                "Invalid skill scope '{}' (expected project|user|global)",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub required: bool,
    pub check: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SkillConfigValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

impl std::fmt::Display for SkillConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillConfigValue::String(s) => write!(f, "{}", s),
            SkillConfigValue::Number(n) => write!(f, "{}", n),
            SkillConfigValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub manifest: SkillManifest,
    pub installed: bool,
    pub enabled: bool,
    pub installed_at: Option<DateTime<Utc>>,
    pub skill_dir: PathBuf,
}

pub struct SkillManager {
    skills_dir: PathBuf,
    extra_dirs: Vec<PathBuf>,
    /// Project-scope skills root (e.g. `<repo>/.agenthub/skills`).
    project_dir: Option<PathBuf>,
    /// Global-scope skills root (e.g. `/etc/agenthub/skills`).
    global_dir: Option<PathBuf>,
}

/// Result of a skill/AgentHub version compatibility check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCompatibility {
    pub skill: String,
    pub skill_version: String,
    pub requires_agenthub: Option<String>,
    pub current_agenthub: String,
    pub compatible: bool,
    pub message: String,
}

/// Parse the first three numeric components of a semver-ish string.
fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .filter_map(|p| p.parse::<u64>().ok());
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Compare two version strings; returns >0 if `a` is newer than `b`.
fn compare_versions(a: &str, b: &str) -> i32 {
    let (ma, pa, ra) = parse_version(a);
    let (mb, pb, rb) = parse_version(b);
    if ma != mb {
        return (ma as i64 - mb as i64).signum() as i32;
    }
    if pa != pb {
        return (pa as i64 - pb as i64).signum() as i32;
    }
    (ra as i64 - rb as i64).signum() as i32
}

impl SkillManager {
    fn validate_name(name: &str) -> Result<()> {
        if !is_safe_id(name) {
            return Err(AgentHubError::SkillError(format!(
                "Invalid skill name: {name}"
            )));
        }
        Ok(())
    }

    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            extra_dirs: Vec::new(),
            project_dir: None,
            global_dir: None,
        }
    }

    pub fn with_extra_dir(mut self, dir: PathBuf) -> Self {
        self.extra_dirs.push(dir);
        self
    }

    /// Configure the project-scope skills root (e.g. `<repo>/.agenthub/skills`).
    pub fn with_project_dir(mut self, dir: PathBuf) -> Self {
        self.project_dir = Some(dir);
        self
    }

    /// Configure the global-scope skills root (e.g. `/etc/agenthub/skills`).
    pub fn with_global_dir(mut self, dir: PathBuf) -> Self {
        self.global_dir = Some(dir);
        self
    }

    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    fn installed_dir(&self) -> PathBuf {
        self.skills_dir.join("installed")
    }

    /// Root directory for a scope, when configured.
    pub fn scope_dir(&self, scope: SkillScope) -> Option<PathBuf> {
        match scope {
            SkillScope::User => Some(self.skills_dir.clone()),
            SkillScope::Project => self.project_dir.clone(),
            SkillScope::Global => self.global_dir.clone(),
        }
    }

    /// Installed-dir for a scope (`<scope_root>/installed`).
    pub fn scope_installed_dir(&self, scope: SkillScope) -> Option<PathBuf> {
        self.scope_dir(scope).map(|root| root.join("installed"))
    }

    /// Ordered candidate skill directories across scopes, highest precedence
    /// first: project, user (installed), extra dirs, global.
    fn candidate_installed_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(dir) = self.scope_installed_dir(SkillScope::Project) {
            dirs.push(dir);
        }
        dirs.push(self.installed_dir());
        dirs.extend(self.extra_dirs.iter().cloned());
        if let Some(dir) = self.scope_installed_dir(SkillScope::Global) {
            dirs.push(dir);
        }
        dirs
    }

    /// Parse a SKILL.md manifest (public: used by the marketplace module).
    pub fn parse_manifest_pub(content: &str) -> Result<SkillManifest> {
        Self::parse_manifest(content)
    }

    /// Recursively copy a skill directory (public: used by the marketplace module).
    pub fn copy_dir_recursive_pub(src: &Path, dst: &Path) -> Result<()> {
        Self::copy_dir_recursive(src, dst)
    }

    fn parse_manifest(content: &str) -> Result<SkillManifest> {
        // Extract YAML frontmatter between --- markers
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 2 {
            return Err(AgentHubError::SkillError(
                "Invalid SKILL.md format: missing frontmatter".to_string(),
            ));
        }

        serde_yaml::from_str(parts[1]).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to parse skill manifest: {}", e))
        })
    }

    pub fn list_skills(&self) -> Result<Vec<Skill>> {
        // Load in precedence order (project > user > extra > global); the
        // first occurrence of a name wins (project shadows user/global).
        let mut skills = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        for dir in self.candidate_installed_dirs() {
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(&dir).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to read skills dir: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    AgentHubError::SkillError(format!("Failed to read entry: {}", e))
                })?;

                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("SKILL.md");
                    if manifest_path.exists() {
                        match self.load_skill_from_dir(&path) {
                            Ok(skill) => {
                                if !seen_names.contains(&skill.manifest.name) {
                                    seen_names.insert(skill.manifest.name.clone());
                                    skills.push(skill);
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to load skill at {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        skills.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        Ok(skills)
    }

    fn load_skill_from_dir(&self, skill_dir: &Path) -> Result<Skill> {
        let manifest_path = skill_dir.join("SKILL.md");
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to read SKILL.md: {}", e)))?;

        let manifest = Self::parse_manifest(&content)?;
        let enabled_path = skill_dir.join(".enabled");
        let enabled = enabled_path.exists();

        let metadata = std::fs::metadata(&manifest_path).ok();
        let installed_at = metadata.and_then(|m| m.modified().ok().map(DateTime::from));

        Ok(Skill {
            manifest,
            installed: true,
            enabled,
            installed_at,
            skill_dir: skill_dir.to_path_buf(),
        })
    }

    pub fn get_skill(&self, skill_name: &str) -> Result<Skill> {
        Self::validate_name(skill_name)?;
        for dir in self.candidate_installed_dirs() {
            let skill_dir = dir.join(skill_name);
            if skill_dir.exists() {
                return self.load_skill_from_dir(&skill_dir);
            }
        }
        Err(AgentHubError::SkillError(format!(
            "Skill not found: {}",
            skill_name
        )))
    }

    /// Load a skill directly from an installed directory path.
    pub fn get_skill_from_dir(&self, skill_dir: &Path) -> Result<Skill> {
        self.load_skill_from_dir(skill_dir)
    }

    /// Locate a skill and report which scope it came from.
    pub fn resolve_skill_scope(&self, skill_name: &str) -> Result<Option<(Skill, SkillScope)>> {
        Self::validate_name(skill_name)?;
        let mut candidates = Vec::new();
        if let Some(dir) = self.scope_installed_dir(SkillScope::Project) {
            candidates.push((SkillScope::Project, dir.join(skill_name)));
        }
        candidates.push((SkillScope::User, self.installed_dir().join(skill_name)));
        if let Some(dir) = self.scope_installed_dir(SkillScope::Global) {
            candidates.push((SkillScope::Global, dir.join(skill_name)));
        }
        for (scope, skill_dir) in candidates {
            if skill_dir.exists() {
                return Ok(Some((self.load_skill_from_dir(&skill_dir)?, scope)));
            }
        }
        Ok(None)
    }

    /// Install a skill into the user scope (default).
    pub fn install_skill(&self, skill_name: &str, source_dir: &Path) -> Result<Skill> {
        self.install_skill_to_scope(skill_name, source_dir, SkillScope::User)
    }

    /// Install a skill into a specific scope. The scope root must be
    /// configured (user always is; project/global via the builder).
    pub fn install_skill_to_scope(
        &self,
        skill_name: &str,
        source_dir: &Path,
        scope: SkillScope,
    ) -> Result<Skill> {
        Self::validate_name(skill_name)?;
        let installed = self.scope_installed_dir(scope).ok_or_else(|| {
            AgentHubError::SkillError(format!(
                "Scope '{}' is not configured on this manager",
                scope
            ))
        })?;
        let dest_dir = installed.join(skill_name);
        if dest_dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Skill already installed: {}",
                skill_name
            )));
        }

        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to create skill dir: {}", e)))?;

        // Copy skill files
        Self::copy_dir_recursive(source_dir, &dest_dir)?;

        // Enable by default
        let enabled_path = dest_dir.join(".enabled");
        std::fs::write(&enabled_path, "")
            .map_err(|e| AgentHubError::SkillError(format!("Failed to enable skill: {}", e)))?;

        self.get_skill(skill_name)
    }

    /// Uninstall a skill from whichever scope it currently lives in.
    pub fn uninstall_skill(&self, skill_name: &str) -> Result<bool> {
        Self::validate_name(skill_name)?;
        for dir in self.candidate_installed_dirs() {
            let skill_dir = dir.join(skill_name);
            if skill_dir.exists() {
                std::fs::remove_dir_all(&skill_dir).map_err(|e| {
                    AgentHubError::SkillError(format!("Failed to uninstall skill: {}", e))
                })?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Uninstall a skill from one specific scope.
    pub fn uninstall_skill_from_scope(&self, skill_name: &str, scope: SkillScope) -> Result<bool> {
        Self::validate_name(skill_name)?;
        let Some(installed) = self.scope_installed_dir(scope) else {
            return Ok(false);
        };
        let skill_dir = installed.join(skill_name);
        if !skill_dir.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(&skill_dir)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to uninstall skill: {}", e)))?;
        Ok(true)
    }

    pub fn enable_skill(&self, skill_name: &str) -> Result<()> {
        Self::validate_name(skill_name)?;
        let skill = self.get_skill(skill_name)?;
        std::fs::write(skill.skill_dir.join(".enabled"), "")
            .map_err(|e| AgentHubError::SkillError(format!("Failed to enable skill: {}", e)))?;
        Ok(())
    }

    pub fn disable_skill(&self, skill_name: &str) -> Result<()> {
        Self::validate_name(skill_name)?;
        let skill = self.get_skill(skill_name)?;
        let enabled_path = skill.skill_dir.join(".enabled");
        if enabled_path.exists() {
            std::fs::remove_file(&enabled_path).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to disable skill: {}", e))
            })?;
        }
        Ok(())
    }

    pub fn get_skill_config(&self, skill_name: &str) -> Result<HashMap<String, SkillConfigValue>> {
        let skill = self.get_skill(skill_name)?;
        Ok(skill.manifest.config)
    }

    pub fn check_dependencies(&self, skill_name: &str) -> Result<Vec<(String, bool)>> {
        let skill = self.get_skill(skill_name)?;
        let mut results = Vec::new();

        for dep in &skill.manifest.dependencies {
            let available = self.check_command(&dep.check);
            results.push((dep.name.clone(), available));
        }

        Ok(results)
    }

    /// Check whether a skill's `min_agenthub_version` is satisfied by the
    /// running AgentHub version.
    pub fn check_compatibility(&self, skill_name: &str) -> Result<SkillCompatibility> {
        let skill = self.get_skill(skill_name)?;
        let current = env!("CARGO_PKG_VERSION").to_string();
        let required = skill.manifest.min_agenthub_version.clone();

        let compatible = match &required {
            None => true,
            Some(min) => compare_versions(&current, min) >= 0,
        };
        let message = match &required {
            None => "No version constraint".to_string(),
            Some(min) if compatible => format!("Requires agenthub >= {}, running {}", min, current),
            Some(min) => format!(
                "Requires agenthub >= {} but running {} — please upgrade",
                min, current
            ),
        };

        Ok(SkillCompatibility {
            skill: skill.manifest.name.clone(),
            skill_version: skill.manifest.version.clone(),
            requires_agenthub: required,
            current_agenthub: current,
            compatible,
            message,
        })
    }

    /// Check compatibility for every installed skill.
    pub fn check_all_compatibility(&self) -> Result<Vec<SkillCompatibility>> {
        let skills = self.list_skills()?;
        let mut results = Vec::new();
        for skill in &skills {
            let compat = self.check_compatibility(&skill.manifest.name)?;
            if compat.requires_agenthub.is_some() {
                results.push(compat);
            }
        }
        results.sort_by(|a, b| a.skill.cmp(&b.skill));
        Ok(results)
    }

    fn check_command(&self, command: &str) -> bool {
        std::process::Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        })
        .args(if cfg!(target_os = "windows") {
            vec!["/C", command]
        } else {
            vec!["-c", command]
        })
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to create directory: {}", e)))?;

        for entry in std::fs::read_dir(src)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::SkillError(format!("Failed to read entry: {}", e)))?;

            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path).map_err(|e| {
                    AgentHubError::SkillError(format!("Failed to copy file: {}", e))
                })?;
            }
        }

        Ok(())
    }

    pub fn create_skill(&self, skill_name: &str, description: &str) -> Result<Skill> {
        let skill_dir = self.installed_dir().join(skill_name);
        if skill_dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Skill already exists: {}",
                skill_name
            )));
        }

        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to create skill dir: {}", e)))?;

        let manifest_content = format!(
            r#"---
name: {}
description: "{}"
version: 0.1.0
author: ""
triggers: []
tags: []
category: general
dependencies: []
config: {{}}
---

# {}

{}
"#,
            skill_name, description, skill_name, description
        );

        let manifest_path = skill_dir.join("SKILL.md");
        std::fs::write(&manifest_path, manifest_content)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to write manifest: {}", e)))?;

        // Enable by default
        let enabled_path = skill_dir.join(".enabled");
        std::fs::write(&enabled_path, "")
            .map_err(|e| AgentHubError::SkillError(format!("Failed to enable skill: {}", e)))?;

        self.get_skill(skill_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_skill(manager: &SkillManager, name: &str) -> PathBuf {
        let skill_dir = manager.installed_dir().join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();

        let manifest = format!(
            r#"---
name: {}
description: "Test skill"
version: 1.0.0
triggers:
  - "*.test"
tags:
  - test
category: testing
dependencies: []
config:
  key: value
---

# Test Skill

This is a test skill.
"#,
            name
        );

        std::fs::write(skill_dir.join("SKILL.md"), manifest).unwrap();
        std::fs::write(skill_dir.join(".enabled"), "").unwrap();
        skill_dir
    }

    #[test]
    fn test_list_skills() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        create_test_skill(&manager, "skill-a");
        create_test_skill(&manager, "skill-b");

        let skills = manager.list_skills().unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].manifest.name, "skill-a");
        assert_eq!(skills[1].manifest.name, "skill-b");
    }

    #[test]
    fn test_get_skill() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        create_test_skill(&manager, "test-skill");

        let skill = manager.get_skill("test-skill").unwrap();
        assert_eq!(skill.manifest.name, "test-skill");
        assert_eq!(skill.manifest.version, "1.0.0");
        assert!(skill.enabled);
    }

    #[test]
    fn test_enable_disable_skill() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        create_test_skill(&manager, "test-skill");

        let skill = manager.get_skill("test-skill").unwrap();
        assert!(skill.enabled);

        manager.disable_skill("test-skill").unwrap();
        let skill = manager.get_skill("test-skill").unwrap();
        assert!(!skill.enabled);

        manager.enable_skill("test-skill").unwrap();
        let skill = manager.get_skill("test-skill").unwrap();
        assert!(skill.enabled);
    }

    #[test]
    fn test_create_skill() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        let skill = manager.create_skill("new-skill", "A new skill").unwrap();
        assert_eq!(skill.manifest.name, "new-skill");
        assert_eq!(skill.manifest.description, "A new skill");
        assert!(skill.enabled);
    }

    #[test]
    fn test_uninstall_skill() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        create_test_skill(&manager, "test-skill");

        let removed = manager.uninstall_skill("test-skill").unwrap();
        assert!(removed);

        let result = manager.get_skill("test-skill");
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_config() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        create_test_skill(&manager, "test-skill");

        let config = manager.get_skill_config("test-skill").unwrap();
        assert!(config.contains_key("key"));
    }

    // ---- Version compatibility ----

    #[test]
    fn test_parse_and_compare_versions() {
        assert_eq!(parse_version("1.2.3"), (1, 2, 3));
        assert_eq!(parse_version("v2.0.0"), (2, 0, 0));
        assert_eq!(parse_version("1.0.0-beta.1"), (1, 0, 0));
        assert_eq!(parse_version("garbage"), (0, 0, 0));

        assert!(compare_versions("1.2.3", "1.2.2") > 0);
        assert!(compare_versions("1.2.3", "1.2.3") == 0);
        assert!(compare_versions("1.0.0", "2.0.0") < 0);
        assert!(compare_versions("2.0.0", "1.9.9") > 0);
    }

    #[test]
    fn test_check_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SkillManager::new(temp_dir.path().to_path_buf());

        // No constraint -> compatible
        create_test_skill(&manager, "unconstrained");
        let compat = manager.check_compatibility("unconstrained").unwrap();
        assert!(compat.compatible);
        assert!(compat.requires_agenthub.is_none());

        // Low constraint -> compatible
        let dir = manager.installed_dir().join("constrained-ok");
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = r#"---
name: constrained-ok
description: "x"
version: 1.0.0
min_agenthub_version: 0.1.0
triggers: []
---

# x
"#;
        std::fs::write(dir.join("SKILL.md"), manifest).unwrap();
        let compat = manager.check_compatibility("constrained-ok").unwrap();
        assert!(compat.compatible);
        assert_eq!(compat.requires_agenthub.as_deref(), Some("0.1.0"));

        // Unreachable constraint -> incompatible
        let dir = manager.installed_dir().join("constrained-no");
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = r#"---
name: constrained-no
description: "x"
version: 1.0.0
min_agenthub_version: 99.0.0
triggers: []
---

# x
"#;
        std::fs::write(dir.join("SKILL.md"), manifest).unwrap();
        let compat = manager.check_compatibility("constrained-no").unwrap();
        assert!(!compat.compatible);
        assert!(compat.message.contains("upgrade"));

        // Bulk check includes only constrained skills
        let all = manager.check_all_compatibility().unwrap();
        let names: Vec<&str> = all.iter().map(|c| c.skill.as_str()).collect();
        assert!(names.contains(&"constrained-ok"));
        assert!(names.contains(&"constrained-no"));
        assert!(!names.contains(&"unconstrained"));
    }

    #[test]
    fn test_rejects_unsafe_skill_names() {
        let temp = TempDir::new().unwrap();
        let manager = SkillManager::new(temp.path().to_path_buf());
        assert!(manager.get_skill("../escape").is_err());
        assert!(manager.uninstall_skill("../escape").is_err());
    }

    #[test]
    fn test_get_skill_corrupt_manifest_errors() {
        let temp = TempDir::new().unwrap();
        let manager = SkillManager::new(temp.path().to_path_buf());
        let skill_dir = manager.installed_dir().join("demo");
        std::fs::create_dir_all(&skill_dir).unwrap();

        // No frontmatter at all -> manifest parse error.
        std::fs::write(skill_dir.join("SKILL.md"), "# no frontmatter\n").unwrap();
        assert!(manager.get_skill("demo").is_err());

        // Binary garbage -> same, no panic.
        std::fs::write(skill_dir.join("SKILL.md"), [0xff, 0x00, 0x01]).unwrap();
        assert!(manager.get_skill("demo").is_err());
    }

    #[test]
    fn test_skill_config_value_display() {
        assert_eq!(format!("{}", SkillConfigValue::String("x".into())), "x");
        assert_eq!(format!("{}", SkillConfigValue::Number(0.5)), "0.5");
        assert_eq!(format!("{}", SkillConfigValue::Boolean(true)), "true");
    }

    #[test]
    fn test_compare_versions_more_cases() {
        assert!(compare_versions("1.0.0", "1.0.0") == 0);
        assert!(compare_versions("1.2.0", "1.10.0") < 0);
        assert!(compare_versions("2.0.0", "1.9.9") > 0);
        assert!(compare_versions("1.0.1", "1.0.0") > 0);
        assert!(compare_versions("garbage", "0.1.0") < 0); // unparseable -> 0.0.0
    }

    #[test]
    fn test_parse_manifest_missing_frontmatter() {
        assert!(SkillManager::parse_manifest_pub("no markers at all").is_err());
        // Frontmatter exists but the YAML body is corrupt.
        assert!(SkillManager::parse_manifest_pub("---\nname: [unterminated\n---\n").is_err());
    }

    #[test]
    fn test_list_skills_with_extra_dir_and_corrupt_entries() {
        let temp = TempDir::new().unwrap();
        let manager = SkillManager::new(temp.path().to_path_buf());
        create_test_skill(&manager, "installed-one");

        // A corrupt skill directory must be skipped with a warning, not fatal.
        let corrupt = manager.installed_dir().join("corrupt");
        std::fs::create_dir_all(&corrupt).unwrap();
        std::fs::write(corrupt.join("SKILL.md"), "# no frontmatter").unwrap();

        // A directory without SKILL.md is ignored entirely.
        let empty = manager.installed_dir().join("empty");
        std::fs::create_dir_all(&empty).unwrap();

        // Extra dir (e.g. codex skills) contributes skills, deduplicated.
        let extra = temp.path().join("extra");
        let extra_manager =
            SkillManager::new(temp.path().to_path_buf()).with_extra_dir(extra.clone());
        let extra_skill_dir = extra.join("extra-skill");
        std::fs::create_dir_all(&extra_skill_dir).unwrap();
        std::fs::write(
            extra_skill_dir.join("SKILL.md"),
            r#"---
name: extra-skill
description: "extra"
version: 1.0.0
author: ""
triggers: []
tags: []
category: general
---
Extra skill.
"#,
        )
        .unwrap();
        std::fs::write(extra_skill_dir.join(".enabled"), "").unwrap();

        let skills = extra_manager.list_skills().unwrap();
        let names: Vec<&str> = skills.iter().map(|s| s.manifest.name.as_str()).collect();
        assert!(names.contains(&"installed-one"));
        assert!(names.contains(&"extra-skill"));
        assert!(!names.contains(&"corrupt"));
        assert!(!names.contains(&"empty"));
    }

    // ---- Scope resolution (project > user > global) ----

    fn scoped_manager(temp: &TempDir) -> SkillManager {
        SkillManager::new(temp.path().join("config").join("skills"))
            .with_project_dir(temp.path().join("project").join(".agenthub").join("skills"))
            .with_global_dir(temp.path().join("etc").join("agenthub").join("skills"))
    }

    #[test]
    fn test_install_to_scope_and_resolve() {
        let temp = TempDir::new().unwrap();
        let manager = scoped_manager(&temp);

        // Same skill name installed into three scopes with distinct versions.
        let src = temp.path().join("src");
        let write = |name: &str, version: &str| {
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(
                src.join("SKILL.md"),
                format!(
                    "---\nname: {name}\ndescription: \"x\"\nversion: {version}\nauthor: \"\"\ntriggers: []\ntags: []\ncategory: general\n---\n# {name}\n"
                ),
            )
            .unwrap();
        };

        write("demo", "1.0.0");
        manager
            .install_skill_to_scope("demo", &src, SkillScope::Global)
            .unwrap();
        write("demo", "2.0.0");
        manager
            .install_skill_to_scope("demo", &src, SkillScope::User)
            .unwrap();
        write("demo", "3.0.0");
        manager
            .install_skill_to_scope("demo", &src, SkillScope::Project)
            .unwrap();

        // Project wins.
        let (skill, scope) = manager.resolve_skill_scope("demo").unwrap().unwrap();
        assert_eq!(scope, SkillScope::Project);
        assert_eq!(skill.manifest.version, "3.0.0");
        assert_eq!(manager.get_skill("demo").unwrap().manifest.version, "3.0.0");

        // Listing shows the project copy only (dedup by name).
        let skills = manager.list_skills().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].manifest.version, "3.0.0");

        // Scoped removal surfaces the user copy next.
        assert!(manager
            .uninstall_skill_from_scope("demo", SkillScope::Project)
            .unwrap());
        let (_, scope) = manager.resolve_skill_scope("demo").unwrap().unwrap();
        assert_eq!(scope, SkillScope::User);
        assert_eq!(manager.get_skill("demo").unwrap().manifest.version, "2.0.0");

        // enable/disable act on the resolved (user) copy.
        manager.disable_skill("demo").unwrap();
        assert!(!manager.get_skill("demo").unwrap().enabled);
        manager.enable_skill("demo").unwrap();
        assert!(manager.get_skill("demo").unwrap().enabled);

        // uninstall removes from the resolved scope.
        assert!(manager.uninstall_skill("demo").unwrap());
        let (_, scope) = manager.resolve_skill_scope("demo").unwrap().unwrap();
        assert_eq!(scope, SkillScope::Global);
    }

    #[test]
    fn test_unconfigured_scope_rejected() {
        let temp = TempDir::new().unwrap();
        let manager = SkillManager::new(temp.path().join("skills"));
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: demo\ndescription: \"x\"\nversion: 1.0.0\n---\n# demo\n",
        )
        .unwrap();

        // Project/global roots not configured -> explicit error.
        assert!(manager
            .install_skill_to_scope("demo", &src, SkillScope::Project)
            .is_err());
        assert!(manager
            .install_skill_to_scope("demo", &src, SkillScope::Global)
            .is_err());
        // User scope always works.
        assert!(manager
            .install_skill_to_scope("demo", &src, SkillScope::User)
            .is_ok());
    }

    #[test]
    fn test_scope_parsing() {
        assert_eq!(
            "project".parse::<SkillScope>().unwrap(),
            SkillScope::Project
        );
        assert_eq!("user".parse::<SkillScope>().unwrap(), SkillScope::User);
        assert_eq!("global".parse::<SkillScope>().unwrap(), SkillScope::Global);
        assert!("system".parse::<SkillScope>().is_err());
        assert_eq!(SkillScope::Project.to_string(), "project");
    }
}
