use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryScope {
    Global,
    Project,
    Session,
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::Global => write!(f, "global"),
            MemoryScope::Project => write!(f, "project"),
            MemoryScope::Session => write!(f, "session"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MemoryType {
    Pinned,
    Learning,
    Decision,
    Reference,
    Feedback,
    #[default]
    Free,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Pinned => write!(f, "pinned"),
            MemoryType::Learning => write!(f, "learning"),
            MemoryType::Decision => write!(f, "decision"),
            MemoryType::Reference => write!(f, "reference"),
            MemoryType::Feedback => write!(f, "feedback"),
            MemoryType::Free => write!(f, "free"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub path: String,
    pub scope: MemoryScope,
    #[serde(default)]
    pub scope_id: Option<String>,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub memory_type: MemoryType,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MemoryManager {
    memory_dir: PathBuf,
}

impl MemoryManager {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self { memory_dir }
    }

    pub fn memory_dir(&self) -> &Path {
        &self.memory_dir
    }

    fn scope_dir(&self, scope: &MemoryScope, scope_id: Option<&str>) -> PathBuf {
        match scope {
            MemoryScope::Global => self.memory_dir.join("global"),
            MemoryScope::Project => self
                .memory_dir
                .join("projects")
                .join(scope_id.unwrap_or("default")),
            MemoryScope::Session => self
                .memory_dir
                .join("sessions")
                .join(scope_id.unwrap_or("default")),
        }
    }

    pub fn list_entries(&self, scope: Option<MemoryScope>) -> Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();

        match scope {
            Some(MemoryScope::Global) => {
                let dir = self.memory_dir.join("global");
                if dir.exists() {
                    self.collect_entries(&dir, &mut entries)?;
                }
            }
            Some(MemoryScope::Project) => {
                let projects_dir = self.memory_dir.join("projects");
                if projects_dir.exists() {
                    for entry in std::fs::read_dir(&projects_dir).map_err(|e| {
                        AgentHubError::MemoryError(format!("Failed to read dir: {}", e))
                    })? {
                        let entry = entry.map_err(|e| {
                            AgentHubError::MemoryError(format!("Failed to read entry: {}", e))
                        })?;
                        if entry.path().is_dir() {
                            self.collect_entries(&entry.path(), &mut entries)?;
                        }
                    }
                }
            }
            Some(MemoryScope::Session) => {
                let sessions_dir = self.memory_dir.join("sessions");
                if sessions_dir.exists() {
                    for entry in std::fs::read_dir(&sessions_dir).map_err(|e| {
                        AgentHubError::MemoryError(format!("Failed to read dir: {}", e))
                    })? {
                        let entry = entry.map_err(|e| {
                            AgentHubError::MemoryError(format!("Failed to read entry: {}", e))
                        })?;
                        if entry.path().is_dir() {
                            self.collect_entries(&entry.path(), &mut entries)?;
                        }
                    }
                }
            }
            None => {
                // Scan all scopes
                let global_dir = self.memory_dir.join("global");
                if global_dir.exists() {
                    self.collect_entries(&global_dir, &mut entries)?;
                }

                let projects_dir = self.memory_dir.join("projects");
                if projects_dir.exists() {
                    for entry in std::fs::read_dir(&projects_dir).map_err(|e| {
                        AgentHubError::MemoryError(format!("Failed to read dir: {}", e))
                    })? {
                        let entry = entry.map_err(|e| {
                            AgentHubError::MemoryError(format!("Failed to read entry: {}", e))
                        })?;
                        if entry.path().is_dir() {
                            self.collect_entries(&entry.path(), &mut entries)?;
                        }
                    }
                }

                let sessions_dir = self.memory_dir.join("sessions");
                if sessions_dir.exists() {
                    for entry in std::fs::read_dir(&sessions_dir).map_err(|e| {
                        AgentHubError::MemoryError(format!("Failed to read dir: {}", e))
                    })? {
                        let entry = entry.map_err(|e| {
                            AgentHubError::MemoryError(format!("Failed to read entry: {}", e))
                        })?;
                        if entry.path().is_dir() {
                            self.collect_entries(&entry.path(), &mut entries)?;
                        }
                    }
                }
            }
        }

        entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(entries)
    }

    fn collect_entries(&self, dir: &Path, entries: &mut Vec<MemoryEntry>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to read dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::MemoryError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                self.collect_entries(&path, entries)?;
            } else if path.extension().is_some_and(|ext| ext == "md") {
                match self.load_entry_from_file(&path) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        eprintln!("Warning: Failed to load memory at {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn load_entry_from_file(&self, path: &Path) -> Result<MemoryEntry> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to read memory: {}", e)))?;

        // Parse frontmatter if present
        if content.starts_with("---") {
            let parts: Vec<&str> = content.splitn(3, "---").collect();
            if parts.len() >= 2 {
                if let Ok(mut entry) = serde_yaml::from_str::<MemoryEntry>(parts[1]) {
                    // Extract content after frontmatter
                    if parts.len() > 2 {
                        entry.content = parts[2].trim().to_string();
                    }
                    return Ok(entry);
                }
            }
        }

        // Fallback: create entry from raw content
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        let relative_path = path
            .strip_prefix(&self.memory_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        Ok(MemoryEntry {
            path: relative_path,
            scope: MemoryScope::Global,
            scope_id: None,
            title,
            content,
            memory_type: MemoryType::Free,
            tags: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn create_entry(
        &self,
        scope: MemoryScope,
        scope_id: Option<&str>,
        title: &str,
        content: &str,
        memory_type: MemoryType,
    ) -> Result<MemoryEntry> {
        let dir = self.scope_dir(&scope, scope_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to create dir: {}", e)))?;

        let filename = format!("{}.md", title.to_lowercase().replace(' ', "-"));
        let path = dir.join(&filename);
        let relative_path = path
            .strip_prefix(&self.memory_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let now = Utc::now();
        let entry = MemoryEntry {
            path: relative_path,
            scope: scope.clone(),
            scope_id: scope_id.map(|s| s.to_string()),
            title: title.to_string(),
            content: content.to_string(),
            memory_type,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.save_entry(&entry)?;
        Ok(entry)
    }

    pub fn save_entry(&self, entry: &MemoryEntry) -> Result<()> {
        let path = self.memory_dir.join(&entry.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AgentHubError::MemoryError(format!("Failed to create dir: {}", e)))?;
        }

        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&serde_yaml::to_string(entry).unwrap_or_default());
        output.push_str("---\n\n");
        output.push_str(&entry.content);

        std::fs::write(&path, output)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to write memory: {}", e)))?;

        Ok(())
    }

    pub fn delete_entry(&self, path: &str) -> Result<bool> {
        let full_path = self.memory_dir.join(path);
        if full_path.exists() {
            std::fs::remove_file(&full_path).map_err(|e| {
                AgentHubError::MemoryError(format!("Failed to delete memory: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn search_entries(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let entries = self.list_entries(None)?;
        let query_lower = query.to_lowercase();

        Ok(entries
            .into_iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&query_lower)
                    || e.content.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    pub fn add_tag(&self, path: &str, tag: &str) -> Result<()> {
        let mut entry = self.load_entry_from_file(&self.memory_dir.join(path))?;
        if !entry.tags.contains(&tag.to_string()) {
            entry.tags.push(tag.to_string());
            entry.updated_at = Utc::now();
            self.save_entry(&entry)?;
        }
        Ok(())
    }

    pub fn remove_tag(&self, path: &str, tag: &str) -> Result<()> {
        let mut entry = self.load_entry_from_file(&self.memory_dir.join(path))?;
        entry.tags.retain(|t| t != tag);
        entry.updated_at = Utc::now();
        self.save_entry(&entry)
    }

    pub fn get_stats(&self) -> Result<MemoryStats> {
        let entries = self.list_entries(None)?;

        let global = entries
            .iter()
            .filter(|e| e.scope == MemoryScope::Global)
            .count();
        let project = entries
            .iter()
            .filter(|e| e.scope == MemoryScope::Project)
            .count();
        let session = entries
            .iter()
            .filter(|e| e.scope == MemoryScope::Session)
            .count();

        Ok(MemoryStats {
            total: entries.len(),
            global,
            project,
            session,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total: usize,
    pub global: usize,
    pub project: usize,
    pub session: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (MemoryManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_create_entry() {
        let (manager, _temp) = create_test_manager();

        let entry = manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Test Memory",
                "This is a test memory",
                MemoryType::Learning,
            )
            .unwrap();

        assert_eq!(entry.title, "Test Memory");
        assert_eq!(entry.scope, MemoryScope::Global);
    }

    #[test]
    fn test_list_entries() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Entry 1",
                "Content 1",
                MemoryType::Free,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Entry 2",
                "Content 2",
                MemoryType::Free,
            )
            .unwrap();

        let entries = manager.list_entries(None).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_search_entries() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Rust Notes",
                "Rust is great",
                MemoryType::Learning,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Python Notes",
                "Python is cool",
                MemoryType::Learning,
            )
            .unwrap();

        let results = manager.search_entries("rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Notes");
    }

    #[test]
    fn test_delete_entry() {
        let (manager, _temp) = create_test_manager();

        let entry = manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Test",
                "Content",
                MemoryType::Free,
            )
            .unwrap();

        let deleted = manager.delete_entry(&entry.path).unwrap();
        assert!(deleted);

        let entries = manager.list_entries(None).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_stats() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Global 1",
                "Content",
                MemoryType::Free,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Project,
                Some("proj"),
                "Project 1",
                "Content",
                MemoryType::Free,
            )
            .unwrap();

        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.global, 1);
        assert_eq!(stats.project, 1);
    }
}
