use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

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
    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            extra_dirs: Vec::new(),
        }
    }

    pub fn with_extra_dir(mut self, dir: PathBuf) -> Self {
        self.extra_dirs.push(dir);
        self
    }

    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    fn installed_dir(&self) -> PathBuf {
        self.skills_dir.join("installed")
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
        let mut skills = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        // Load from installed directory
        let installed_dir = self.installed_dir();
        if installed_dir.exists() {
            for entry in std::fs::read_dir(&installed_dir).map_err(|e| {
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
                                seen_names.insert(skill.manifest.name.clone());
                                skills.push(skill);
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to load skill at {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        // Load from extra directories (e.g., codex skills)
        for extra_dir in &self.extra_dirs {
            if extra_dir.exists() {
                for entry in std::fs::read_dir(extra_dir).map_err(|e| {
                    AgentHubError::SkillError(format!("Failed to read extra skills dir: {}", e))
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
                                    // Only add if not already seen
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
        let skill_dir = self.installed_dir().join(skill_name);
        if !skill_dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Skill not found: {}",
                skill_name
            )));
        }

        self.load_skill_from_dir(&skill_dir)
    }

    pub fn install_skill(&self, skill_name: &str, source_dir: &Path) -> Result<Skill> {
        let dest_dir = self.installed_dir().join(skill_name);
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

    pub fn uninstall_skill(&self, skill_name: &str) -> Result<bool> {
        let skill_dir = self.installed_dir().join(skill_name);
        if !skill_dir.exists() {
            return Ok(false);
        }

        std::fs::remove_dir_all(&skill_dir)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to uninstall skill: {}", e)))?;

        Ok(true)
    }

    pub fn enable_skill(&self, skill_name: &str) -> Result<()> {
        let skill_dir = self.installed_dir().join(skill_name);
        if !skill_dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Skill not found: {}",
                skill_name
            )));
        }

        let enabled_path = skill_dir.join(".enabled");
        std::fs::write(&enabled_path, "")
            .map_err(|e| AgentHubError::SkillError(format!("Failed to enable skill: {}", e)))?;

        Ok(())
    }

    pub fn disable_skill(&self, skill_name: &str) -> Result<()> {
        let skill_dir = self.installed_dir().join(skill_name);
        if !skill_dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Skill not found: {}",
                skill_name
            )));
        }

        let enabled_path = skill_dir.join(".enabled");
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
}
