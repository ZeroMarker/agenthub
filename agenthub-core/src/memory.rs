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

/// A scored search result from vector or hybrid search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMatch {
    pub entry: MemoryEntry,
    /// Normalized relevance score in 0..1.
    pub score: f64,
    /// Search method used: "vector" or "hybrid".
    pub method: String,
}

/// Cached weighted embedding for one memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexEntry {
    pub path: String,
    /// Combined weighted embedding (title 3x / tags 2x / content 1x).
    pub embedding: Vec<f32>,
    /// When the embedding was computed; entries edited after this are stale.
    pub indexed_at: DateTime<Utc>,
}

/// Persisted vector index (`memory/vector_index.json`). Speeds up repeated
/// semantic searches by avoiding recomputing embeddings on every query.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorIndex {
    #[serde(default)]
    pub entries: std::collections::HashMap<String, VectorIndexEntry>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Result of (re)building the vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexSummary {
    pub indexed: usize,
    pub skipped_decayed: usize,
    pub built_at: DateTime<Utc>,
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
            // Drop the cached embedding if present.
            if let Ok(mut index) = self.load_vector_index() {
                if index.entries.remove(path).is_some() {
                    let _ = self.save_vector_index(&index);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // ---- vector index persistence -----------------------------------------

    fn vector_index_path(&self) -> PathBuf {
        self.memory_dir.join("vector_index.json")
    }

    /// Load the persisted vector index (empty when missing or unreadable).
    pub fn load_vector_index(&self) -> Result<VectorIndex> {
        let path = self.vector_index_path();
        if !path.exists() {
            return Ok(VectorIndex::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentHubError::MemoryError(format!("Failed to read vector index: {}", e))
        })?;
        serde_json::from_str(&content)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to parse vector index: {}", e)))
    }

    pub fn save_vector_index(&self, index: &VectorIndex) -> Result<()> {
        std::fs::create_dir_all(&self.memory_dir).map_err(|e| {
            AgentHubError::MemoryError(format!("Failed to create memory dir: {}", e))
        })?;
        let content = serde_json::to_string_pretty(index).map_err(|e| {
            AgentHubError::MemoryError(format!("Failed to serialize vector index: {}", e))
        })?;
        std::fs::write(self.vector_index_path(), content).map_err(|e| {
            AgentHubError::MemoryError(format!("Failed to write vector index: {}", e))
        })?;
        Ok(())
    }

    /// Rebuild the vector index for all non-decayed entries.
    pub fn build_vector_index(&self) -> Result<VectorIndexSummary> {
        let entries = self.list_entries(None)?;
        let mut index = VectorIndex::default();
        let mut skipped_decayed = 0usize;
        for entry in &entries {
            if entry.decayed {
                skipped_decayed += 1;
                continue;
            }
            index.entries.insert(
                entry.path.clone(),
                VectorIndexEntry {
                    path: entry.path.clone(),
                    embedding: weighted_embedding(entry),
                    indexed_at: Utc::now(),
                },
            );
        }
        index.updated_at = Some(Utc::now());
        self.save_vector_index(&index)?;
        Ok(VectorIndexSummary {
            indexed: index.entries.len(),
            skipped_decayed,
            built_at: Utc::now(),
        })
    }

    /// Get the cached embedding for an entry if it is still fresh, otherwise
    /// compute and (lazily) update the index. Returns (embedding, index_dirty).
    fn embedding_for(&self, entry: &MemoryEntry, index: &mut VectorIndex) -> (Vec<f32>, bool) {
        if let Some(cached) = index.entries.get(&entry.path) {
            if cached.indexed_at >= entry.updated_at {
                return (cached.embedding.clone(), false);
            }
        }
        let embedding = weighted_embedding(entry);
        index.entries.insert(
            entry.path.clone(),
            VectorIndexEntry {
                path: entry.path.clone(),
                embedding: embedding.clone(),
                indexed_at: Utc::now(),
            },
        );
        (embedding, true)
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
        if entries.is_empty() || tokenize(query).is_empty() {
            return Ok(Vec::new());
        }

        let scored = self.bm25_scores(query, &entries);

        Ok(scored
            .into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.0)
            .map(|(_, idx)| entries[idx].clone())
            .collect())
    }

    /// Vector (embedding) semantic search. Decayed entries are excluded.
    /// Uses local feature-hashed character n-gram embeddings (no network), with
    /// the same title 3x / tags 2x / content 1x weighting as BM25. Returns
    /// scored matches with cosine similarity in descending order.
    ///
    /// Embeddings are served from the persisted `vector_index.json` cache and
    /// recomputed incrementally for edited entries.
    pub fn search_entries_vector(&self, query: &str, top_k: usize) -> Result<Vec<MemoryMatch>> {
        let entries: Vec<MemoryEntry> = self
            .list_entries(None)?
            .into_iter()
            .filter(|e| !e.decayed)
            .collect();
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let mut index = self.load_vector_index()?;
        let mut dirty = false;
        let query_vec = embed_text(query);
        let mut scored: Vec<(f64, &MemoryEntry)> = Vec::with_capacity(entries.len());
        for entry in &entries {
            let (combined, changed) = self.embedding_for(entry, &mut index);
            dirty |= changed;
            let score = cosine_similarity(&query_vec, &combined) as f64;
            scored.push((score, entry));
        }
        if dirty {
            self.save_vector_index(&index)?;
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored
            .into_iter()
            .filter(|(score, _)| *score > 0.0)
            .map(|(score, entry)| MemoryMatch {
                entry: entry.clone(),
                score,
                method: "vector".to_string(),
            })
            .collect())
    }

    /// Hybrid search: BM25 and vector scores are independently normalized to
    /// 0..1 and blended 50/50. Decayed entries are excluded.
    pub fn hybrid_search(&self, query: &str, top_k: usize) -> Result<Vec<MemoryMatch>> {
        let entries: Vec<MemoryEntry> = self
            .list_entries(None)?
            .into_iter()
            .filter(|e| !e.decayed)
            .collect();
        if entries.is_empty() || tokenize(query).is_empty() {
            return Ok(Vec::new());
        }

        let bm25 = self.bm25_scores(query, &entries);
        let max_bm25 = bm25.iter().map(|(s, _)| *s).fold(0.0f64, f64::max);
        let bm25_map: std::collections::HashMap<usize, f64> = bm25
            .into_iter()
            .map(|(s, i)| (i, if max_bm25 > 0.0 { s / max_bm25 } else { 0.0 }))
            .collect();

        let query_vec = embed_text(query);
        let mut index = self.load_vector_index()?;
        let mut dirty = false;
        let mut vector_scores: Vec<f64> = Vec::with_capacity(entries.len());
        for entry in &entries {
            let (combined, changed) = self.embedding_for(entry, &mut index);
            dirty |= changed;
            vector_scores.push(cosine_similarity(&query_vec, &combined) as f64);
        }
        if dirty {
            self.save_vector_index(&index)?;
        }
        let max_vec = vector_scores.iter().fold(0.0f64, |a, &b| a.max(b));

        let mut scored: Vec<(f64, usize)> = entries
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let nv = if max_vec > 0.0 {
                    vector_scores[idx] / max_vec
                } else {
                    0.0
                };
                let nb = bm25_map.get(&idx).copied().unwrap_or(0.0);
                (0.5 * nb + 0.5 * nv, idx)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored
            .into_iter()
            .filter(|(score, _)| *score > 0.0)
            .map(|(score, idx)| MemoryMatch {
                entry: entries[idx].clone(),
                score,
                method: "hybrid".to_string(),
            })
            .collect())
    }

    /// Internal BM25 scorer shared by `search_entries_bm25` and `hybrid_search`.
    fn bm25_scores(&self, query: &str, entries: &[MemoryEntry]) -> Vec<(f64, usize)> {
        let query_terms = tokenize(query);
        if query_terms.is_empty() || entries.is_empty() {
            return Vec::new();
        }

        // Build weighted token streams for each document.
        let docs: Vec<Vec<String>> = entries
            .iter()
            .map(|e| {
                let mut tokens = Vec::new();
                for _ in 0..3 {
                    tokens.extend(tokenize(&e.title));
                }
                for tag in &e.tags {
                    for _ in 0..2 {
                        tokens.extend(tokenize(tag));
                    }
                }
                tokens.extend(tokenize(&e.content));
                tokens
            })
            .collect();

        let n = docs.len();
        let avgdl = docs.iter().map(|d| d.len() as f64).sum::<f64>() / n as f64;
        let avgdl = if avgdl <= 0.0 { 1.0 } else { avgdl };

        const K1: f64 = 1.5;
        const B: f64 = 0.75;

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
        scored
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

    // -----------------------------------------------------------------------
    // Export / import
    // -----------------------------------------------------------------------

    /// Export entries as JSON (optionally restricted to a scope).
    pub fn export_memories_json(&self, scope: Option<MemoryScope>) -> Result<String> {
        let entries = self.list_entries(scope)?;
        serde_json::to_string_pretty(&entries)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to serialize memories: {}", e)))
    }

    /// Import entries from JSON. With `merge` set, entries whose path already
    /// exists are skipped; otherwise they are overwritten. Returns the number
    /// imported / skipped.
    pub fn import_memories(&self, json: &str, merge: bool) -> Result<crate::prompt::ImportSummary> {
        let entries: Vec<MemoryEntry> = serde_json::from_str(json).map_err(|e| {
            AgentHubError::MemoryError(format!("Failed to parse memory export: {}", e))
        })?;

        let mut summary = crate::prompt::ImportSummary::default();
        for entry in &entries {
            let target = self.memory_dir.join(&entry.path);
            if merge && target.exists() {
                summary.skipped += 1;
                continue;
            }
            self.save_entry(entry)?;
            summary.imported += 1;
        }

        Ok(summary)
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

// ---------------------------------------------------------------------------
// Local embedding: feature-hashed character n-grams -> fixed-dim vector.
//
// No network, no model weights: each text is lowercased, split into
// overlapping 3-character windows, each window is FNV-1a hashed and votes for
// one bucket of a fixed-size vector, which is then L2-normalized. Two texts
// with many shared n-grams get high cosine similarity. Dimensionality is
// deterministic across runs and platforms.
// ---------------------------------------------------------------------------

/// Dimensionality of the local embedding space.
pub const EMBEDDING_DIM: usize = 256;

/// FNV-1a 64-bit hash (deterministic, no std dependency beyond u64 ops).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Embed text into the local embedding space (L2-normalized vector).
/// Character 3-grams (or individual chars for very short text).
pub fn embed_text(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; EMBEDDING_DIM];
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() < 3 {
        for c in chars {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            let h = fnv1a(s.as_bytes());
            vec[(h as usize) % EMBEDDING_DIM] += 1.0;
        }
    } else {
        for w in chars.windows(3) {
            let s: String = w.iter().collect();
            let h = fnv1a(s.as_bytes());
            vec[(h as usize) % EMBEDDING_DIM] += 1.0;
        }
    }
    normalize(&mut vec);
    vec
}

fn normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}

/// Cosine similarity between two equal-length vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Weighted embedding for a memory entry: title 3x, tags 2x, content 1x.
fn weighted_embedding(entry: &MemoryEntry) -> Vec<f32> {
    let mut combined = vec![0.0f32; EMBEDDING_DIM];
    let title = embed_text(&entry.title);
    let tags = embed_text(&entry.tags.join(" "));
    let content = embed_text(&entry.content);
    for i in 0..EMBEDDING_DIM {
        combined[i] = 3.0 * title[i] + 2.0 * tags[i] + content[i];
    }
    normalize(&mut combined);
    combined
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

    // ---- Export / import ----

    #[test]
    fn test_export_import_memories_roundtrip() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Note A",
                "content A",
                MemoryType::Learning,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Project,
                Some("proj"),
                "Note B",
                "content B",
                MemoryType::Decision,
            )
            .unwrap();

        let json = manager.export_memories_json(None).unwrap();

        let (target, _temp2) = create_test_manager();
        let summary = target.import_memories(&json, false).unwrap();
        assert_eq!(summary.imported, 2);
        assert_eq!(summary.skipped, 0);
        assert_eq!(target.list_entries(None).unwrap().len(), 2);

        // Scope-restricted export
        let global_json = manager
            .export_memories_json(Some(MemoryScope::Global))
            .unwrap();
        let entries: Vec<MemoryEntry> = serde_json::from_str(&global_json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scope, MemoryScope::Global);
    }

    #[test]
    fn test_import_memories_merge_skips_existing() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Note",
                "original",
                MemoryType::Free,
            )
            .unwrap();

        // Export, then modify the file directly
        let json = manager.export_memories_json(None).unwrap();
        let mut entries: Vec<MemoryEntry> = serde_json::from_str(&json).unwrap();
        entries[0].content = "changed".to_string();
        let changed = serde_json::to_string(&entries).unwrap();

        // Merge: existing path skipped, original content kept
        let summary = manager.import_memories(&changed, true).unwrap();
        assert_eq!(summary.imported, 0);
        assert_eq!(summary.skipped, 1);
        let entries = manager.list_entries(None).unwrap();
        assert_eq!(entries[0].content, "original");

        // No merge: overwritten
        let summary = manager.import_memories(&changed, false).unwrap();
        assert_eq!(summary.imported, 1);
        let entries = manager.list_entries(None).unwrap();
        assert_eq!(entries[0].content, "changed");
    }

    // -------------------------------------------------------------------
    // Vector & hybrid search
    // -------------------------------------------------------------------

    #[test]
    fn test_embedding_deterministic_and_normalized() {
        let a = embed_text("rust project build system");
        let b = embed_text("rust project build system");
        assert_eq!(a.len(), EMBEDDING_DIM);
        assert_eq!(a, b);
        let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_same_vs_different() {
        let a = embed_text("deployment pipeline configuration");
        let b = embed_text("deployment pipeline configuration");
        let c = embed_text("banana bread recipe");
        assert!(cosine_similarity(&a, &b) > 0.99);
        assert!(cosine_similarity(&a, &c) < cosine_similarity(&a, &b));
    }

    #[test]
    fn test_vector_search_ranks_relevant_first() {
        let (manager, _temp) = create_test_manager();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Database schema design",
                "Use postgres with indexes on the users table and foreign keys.",
                MemoryType::Reference,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Pasta cooking notes",
                "Boil spaghetti for ten minutes with salt.",
                MemoryType::Free,
            )
            .unwrap();

        let matches = manager
            .search_entries_vector("postgres database schema", 5)
            .unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0].entry.title, "Database schema design");
        assert_eq!(matches[0].method, "vector");
        assert!(matches[0].score >= matches[1].score);
    }

    #[test]
    fn test_hybrid_search_combines_methods() {
        let (manager, _temp) = create_test_manager();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "API rate limiting",
                "Apply token bucket at the gateway for public endpoints.",
                MemoryType::Learning,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Unrelated recipe",
                "Mix flour eggs and milk for pancakes.",
                MemoryType::Free,
            )
            .unwrap();

        let matches = manager.hybrid_search("rate limiting gateway", 5).unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0].entry.title, "API rate limiting");
        assert_eq!(matches[0].method, "hybrid");
        // Scores normalized to 0..1
        assert!((0.0..=1.0).contains(&matches[0].score));
    }

    #[test]
    fn test_vector_index_persistence_and_staleness() {
        let (manager, _temp) = create_test_manager();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Postgres schema",
                "users table with indexes",
                MemoryType::Reference,
            )
            .unwrap();

        // First search builds and persists the index.
        let matches = manager.search_entries_vector("postgres", 5).unwrap();
        assert_eq!(matches.len(), 1);
        let index = manager.load_vector_index().unwrap();
        assert_eq!(index.entries.len(), 1);
        assert!(manager.memory_dir().join("vector_index.json").exists());

        // Editing the entry invalidates the cached embedding (updated_at newer
        // than indexed_at) and search recomputes it.
        let mut entry = manager.list_entries(None).unwrap().remove(0);
        entry.content = "postgres database with replication and sharding".to_string();
        entry.updated_at = Utc::now();
        manager.save_entry(&entry).unwrap();

        let matches = manager.search_entries_vector("sharding", 5).unwrap();
        assert!(!matches.is_empty());

        // Rebuild refreshes everything.
        let summary = manager.build_vector_index().unwrap();
        assert_eq!(summary.indexed, 1);
        assert_eq!(summary.skipped_decayed, 0);

        // Deleting an entry drops its cached embedding.
        manager.delete_entry(&entry.path).unwrap();
        let index = manager.load_vector_index().unwrap();
        assert!(index.entries.is_empty());
    }

    #[test]
    fn test_vector_search_excludes_decayed() {
        let (manager, _temp) = create_test_manager();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Old project notes",
                "Some details about the legacy monorepo setup.",
                MemoryType::Free,
            )
            .unwrap();
        let mut entry = manager.list_entries(None).unwrap().remove(0);
        entry.decayed = true;
        manager.save_entry(&entry).unwrap();

        let matches = manager.search_entries_vector("legacy monorepo", 5).unwrap();
        assert!(matches.is_empty());
    }
}
