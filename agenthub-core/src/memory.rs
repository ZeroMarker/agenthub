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
    /// Importance 0-10 (default 5). Entries below 5 are eligible for decay.
    #[serde(default = "default_importance")]
    pub importance: u8,
    /// Set to true when the entry has been decayed/archived by age.
    #[serde(default)]
    pub decayed: bool,
    #[serde(default)]
    pub last_accessed_at: Option<DateTime<Utc>>,
}

fn default_importance() -> u8 {
    5
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

        entries.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
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
            importance: default_importance(),
            decayed: false,
            last_accessed_at: None,
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
            importance: default_importance(),
            decayed: false,
            last_accessed_at: None,
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
            .filter(|e| !e.decayed)
            .filter(|e| {
                e.title.to_lowercase().contains(&query_lower)
                    || e.content.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    /// BM25 semantic search. Decayed entries are excluded. Results are scored
    /// with title tokens weighted 3x, tags 2x and content 1x, and returned most
    /// relevant first.
    pub fn search_entries_bm25(&self, query: &str, top_k: usize) -> Result<Vec<MemoryEntry>> {
        let entries: Vec<MemoryEntry> = self
            .list_entries(None)?
            .into_iter()
            .filter(|e| !e.decayed)
            .collect();
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let query_terms = tokenize(query);
        if query_terms.is_empty() {
            return Ok(Vec::new());
        }

        // Build weighted token streams for each document.
        let docs: Vec<Vec<String>> = entries
            .iter()
            .map(|e| {
                let mut tokens = Vec::new();
                // Title x3
                for _ in 0..3 {
                    tokens.extend(tokenize(&e.title));
                }
                // Tags x2
                for tag in &e.tags {
                    for _ in 0..2 {
                        tokens.extend(tokenize(tag));
                    }
                }
                // Content x1
                tokens.extend(tokenize(&e.content));
                tokens
            })
            .collect();

        let n = docs.len();
        let avgdl = docs.iter().map(|d| d.len() as f64).sum::<f64>() / n as f64;
        let avgdl = if avgdl <= 0.0 { 1.0 } else { avgdl };

        const K1: f64 = 1.5;
        const B: f64 = 0.75;

        // Term frequency per document.
        let mut tfs: Vec<std::collections::HashMap<String, f64>> = Vec::with_capacity(n);
        for doc in &docs {
            let mut tf: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
            for token in doc {
                *tf.entry(token.clone()).or_insert(0.0) += 1.0;
            }
            tfs.push(tf);
        }

        let mut scored: Vec<(f64, usize)> = Vec::with_capacity(n);
        for (idx, doc) in docs.iter().enumerate() {
            let dl = doc.len() as f64;
            let mut score = 0.0;
            for term in &query_terms {
                let tf = tfs[idx].get(term).copied().unwrap_or(0.0);
                if tf <= 0.0 {
                    continue;
                }
                let df = docs.iter().filter(|d| d.contains(term)).count() as f64;
                let idf = ((n as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
                score += idf * (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * dl / avgdl));
            }
            scored.push((score, idx));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored
            .into_iter()
            .filter(|(score, _)| *score > 0.0)
            .map(|(_, idx)| entries[idx].clone())
            .collect())
    }

    /// Mark an entry as recently accessed (used to keep it from decaying).
    pub fn touch(&self, path: &str) -> Result<()> {
        let mut entry = self.load_entry_from_file(&self.memory_dir.join(path))?;
        entry.last_accessed_at = Some(Utc::now());
        self.save_entry(&entry)
    }

    /// Set the importance (0-10) of an entry.
    pub fn set_importance(&self, path: &str, importance: u8) -> Result<()> {
        let mut entry = self.load_entry_from_file(&self.memory_dir.join(path))?;
        entry.importance = importance.min(10);
        entry.updated_at = Utc::now();
        self.save_entry(&entry)
    }

    /// Revive a decayed entry and mark it as recently accessed.
    pub fn revive(&self, path: &str) -> Result<()> {
        let mut entry = self.load_entry_from_file(&self.memory_dir.join(path))?;
        entry.decayed = false;
        entry.last_accessed_at = Some(Utc::now());
        self.save_entry(&entry)
    }

    /// Decay (archive) entries that have not been accessed for `older_than_days`
    /// and have importance below 5. Returns the number of entries decayed.
    pub fn apply_decay(&self, older_than_days: i64, now: Option<DateTime<Utc>>) -> Result<usize> {
        let now = now.unwrap_or_else(Utc::now);
        let cutoff = now - chrono::Duration::days(older_than_days);
        let mut decayed = 0usize;

        let entries = self.list_entries(None)?;
        for mut entry in entries {
            if entry.decayed {
                continue;
            }
            let last_accessed = entry.last_accessed_at.unwrap_or(entry.updated_at);
            if last_accessed < cutoff && entry.importance < 5 {
                entry.decayed = true;
                self.save_entry(&entry)?;
                decayed += 1;
            }
        }

        Ok(decayed)
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
            decayed: entries.iter().filter(|e| e.decayed).count(),
        })
    }
}

/// Split text into lowercase alphanumeric tokens of length >= 2.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total: usize,
    pub global: usize,
    pub project: usize,
    pub session: usize,
    #[serde(default)]
    pub decayed: usize,
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

    // ---- BM25 semantic search ----

    #[test]
    fn test_bm25_ranks_relevant_entries_first() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Rust Notes",
                "Rust ownership and borrow checker details for systems programming.",
                MemoryType::Learning,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Python Notes",
                "Python is a dynamically typed language used for scripting.",
                MemoryType::Learning,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Rust Async",
                "Async runtime and tokio usage in Rust projects.",
                MemoryType::Learning,
            )
            .unwrap();

        // Searching "rust ownership" should rank the Rust Notes entry first
        let results = manager.search_entries_bm25("rust ownership", 10).unwrap();
        assert_eq!(results[0].title, "Rust Notes");

        // Tokenize filter keeps only length >= 2 tokens
        let tokens = tokenize("Rust, async! tokio");
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"async".to_string()));
        assert!(tokens.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_bm25_top_k() {
        let (manager, _temp) = create_test_manager();

        for i in 0..5 {
            manager
                .create_entry(
                    MemoryScope::Global,
                    None,
                    &format!("Note {}", i),
                    "rust content here",
                    MemoryType::Free,
                )
                .unwrap();
        }

        let results = manager.search_entries_bm25("rust", 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_bm25_empty_or_unknown() {
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

        assert!(manager.search_entries_bm25("", 10).unwrap().is_empty());
        // Non-matching terms yield no scored results
        assert!(manager
            .search_entries_bm25("zzzznomatch", 10)
            .unwrap()
            .is_empty());
    }

    // ---- Importance & decay ----

    #[test]
    fn test_importance_and_touch() {
        let (manager, _temp) = create_test_manager();

        let entry = manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Important",
                "Content",
                MemoryType::Pinned,
            )
            .unwrap();

        assert_eq!(entry.importance, 5);

        manager.set_importance(&entry.path, 10).unwrap();
        manager.touch(&entry.path).unwrap();

        let updated = manager.list_entries(None).unwrap().remove(0);
        assert_eq!(updated.importance, 10);
        assert!(updated.last_accessed_at.is_some());
    }

    #[test]
    fn test_apply_decay_only_old_low_importance_entries() {
        let (manager, temp) = create_test_manager();

        // Entry 1: low importance, stale (written in the past)
        let stale_path = temp.path().join("global").join("stale.md");
        std::fs::create_dir_all(stale_path.parent().unwrap()).unwrap();
        let old = Utc::now() - chrono::Duration::days(90);
        let stale = MemoryEntry {
            path: "global/stale.md".to_string(),
            scope: MemoryScope::Global,
            scope_id: None,
            title: "Stale".to_string(),
            content: "Old content".to_string(),
            memory_type: MemoryType::Free,
            tags: Vec::new(),
            created_at: old,
            updated_at: old,
            importance: 2,
            decayed: false,
            last_accessed_at: None,
        };
        manager.save_entry(&stale).unwrap();

        // Entry 2: high importance, stale -> must NOT decay
        let pinned = MemoryEntry {
            path: "global/pinned.md".to_string(),
            scope: MemoryScope::Global,
            scope_id: None,
            title: "Pinned".to_string(),
            content: "Important content".to_string(),
            memory_type: MemoryType::Pinned,
            tags: Vec::new(),
            created_at: old,
            updated_at: old,
            importance: 10,
            decayed: false,
            last_accessed_at: None,
        };
        manager.save_entry(&pinned).unwrap();

        // Entry 3: fresh, low importance -> must NOT decay
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Fresh",
                "New content",
                MemoryType::Free,
            )
            .unwrap();

        let decayed = manager.apply_decay(30, None).unwrap();
        assert_eq!(decayed, 1);

        let entries = manager.list_entries(None).unwrap();
        let stale_updated = entries.iter().find(|e| e.title == "Stale").unwrap();
        assert!(stale_updated.decayed);
        let pinned_updated = entries.iter().find(|e| e.title == "Pinned").unwrap();
        assert!(!pinned_updated.decayed);
        let fresh_updated = entries.iter().find(|e| e.title == "Fresh").unwrap();
        assert!(!fresh_updated.decayed);

        // Stats report the decayed count
        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.decayed, 1);
    }

    #[test]
    fn test_decayed_entries_excluded_from_search() {
        let (manager, _temp) = create_test_manager();

        let old = Utc::now() - chrono::Duration::days(90);
        let stale = MemoryEntry {
            path: "global/stale.md".to_string(),
            scope: MemoryScope::Global,
            scope_id: None,
            title: "Stale Rust".to_string(),
            content: "rust is old".to_string(),
            memory_type: MemoryType::Free,
            tags: Vec::new(),
            created_at: old,
            updated_at: old,
            importance: 1,
            decayed: false,
            last_accessed_at: None,
        };
        manager.save_entry(&stale).unwrap();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Fresh Rust",
                "rust is fresh",
                MemoryType::Free,
            )
            .unwrap();

        manager.apply_decay(30, None).unwrap();

        // Substring search excludes decayed
        let results = manager.search_entries("rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Fresh Rust");

        // BM25 excludes decayed too
        let results = manager.search_entries_bm25("rust", 10).unwrap();
        assert_eq!(results.len(), 1);

        // Revive brings it back
        manager.revive("global/stale.md").unwrap();
        let results = manager.search_entries("rust").unwrap();
        assert_eq!(results.len(), 2);
    }
}
