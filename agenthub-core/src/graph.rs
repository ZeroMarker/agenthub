use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{AgentHubError, Result};
use crate::memory::{MemoryEntry, MemoryManager};

/// Entity kind: derived from a memory tag, a title token, or a quoted phrase
/// found in the content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Tag,
    Title,
    Phrase,
}

/// A node in the knowledge graph: one entity plus where it occurs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Canonical (lowercased) entity id.
    pub id: String,
    /// Human-readable label (first-seen casing).
    pub label: String,
    pub kind: EntityKind,
    /// Number of memories mentioning this entity.
    pub occurrences: usize,
    /// Memory paths that mention this entity.
    pub memories: Vec<String>,
}

/// An undirected relation between two entities (co-occurrence in a memory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    /// Number of memories where both entities occur.
    pub weight: usize,
}

/// The whole graph: nodes + co-occurrence edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub built_at: DateTime<Utc>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl KnowledgeGraph {
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to serialize graph: {}", e)))
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to parse graph: {}", e)))
    }

    /// Entities directly connected to `id`, most strongly related first.
    pub fn neighbors(&self, id: &str, limit: usize) -> Vec<GraphEdge> {
        let mut edges: Vec<GraphEdge> = self
            .edges
            .iter()
            .filter(|e| e.source == id || e.target == id)
            .cloned()
            .collect();
        edges.sort_by_key(|e| std::cmp::Reverse(e.weight));
        edges.truncate(limit);
        edges
    }

    pub fn summary(&self) -> GraphSummary {
        let mut top: Vec<(usize, String)> = self
            .nodes
            .iter()
            .map(|n| (n.occurrences, n.label.clone()))
            .collect();
        top.sort_by_key(|x| std::cmp::Reverse(x.0));
        GraphSummary {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            built_at: self.built_at,
            top_entities: top.into_iter().take(10).map(|(_, l)| l).collect(),
        }
    }
}

/// Aggregated stats for the graph (no heavy data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub built_at: DateTime<Utc>,
    pub top_entities: Vec<String>,
}

/// Extracts entities and co-occurrence relations from memories.
pub struct KnowledgeGraphBuilder;

impl KnowledgeGraphBuilder {
    pub fn build(entries: &[MemoryEntry]) -> KnowledgeGraph {
        // entity id -> node
        let mut nodes: HashMap<String, GraphNode> = HashMap::new();
        // (entity id pair) -> weight
        let mut edge_weights: HashMap<(String, String), usize> = HashMap::new();

        for entry in entries {
            let mut present: Vec<String> = Vec::new();

            // Tags -> entities
            for tag in &entry.tags {
                let id = tag.trim().to_lowercase();
                if id.is_empty() {
                    continue;
                }
                present.push(id.clone());
                let node = nodes.entry(id.clone()).or_insert_with(|| GraphNode {
                    id: id.clone(),
                    label: tag.clone(),
                    kind: EntityKind::Tag,
                    occurrences: 0,
                    memories: Vec::new(),
                });
                node.occurrences += 1;
                if !node.memories.contains(&entry.path) {
                    node.memories.push(entry.path.clone());
                }
            }

            // Title tokens -> entities
            for token in tokenize(&entry.title) {
                if STOPWORDS.contains(&token.as_str()) {
                    continue;
                }
                present.push(token.clone());
                let node = nodes.entry(token.clone()).or_insert_with(|| GraphNode {
                    id: token.clone(),
                    label: entry
                        .title
                        .split_whitespace()
                        .find(|w| w.to_lowercase() == token)
                        .unwrap_or(&token)
                        .to_string(),
                    kind: EntityKind::Title,
                    occurrences: 0,
                    memories: Vec::new(),
                });
                node.occurrences += 1;
                if !node.memories.contains(&entry.path) {
                    node.memories.push(entry.path.clone());
                }
            }

            // Quoted phrases in content -> entities
            for phrase in quoted_phrases(&entry.content) {
                present.push(phrase.clone());
                let node = nodes.entry(phrase.clone()).or_insert_with(|| GraphNode {
                    id: phrase.clone(),
                    label: phrase.clone(),
                    kind: EntityKind::Phrase,
                    occurrences: 0,
                    memories: Vec::new(),
                });
                node.occurrences += 1;
                if !node.memories.contains(&entry.path) {
                    node.memories.push(entry.path.clone());
                }
            }

            // Co-occurrence edges within this memory
            present.sort();
            present.dedup();
            for i in 0..present.len() {
                for j in (i + 1)..present.len() {
                    let key = if present[i] < present[j] {
                        (present[i].clone(), present[j].clone())
                    } else {
                        (present[j].clone(), present[i].clone())
                    };
                    *edge_weights.entry(key).or_insert(0) += 1;
                }
            }
        }

        let mut nodes: Vec<GraphNode> = nodes.into_values().collect();
        nodes.sort_by(|a, b| b.occurrences.cmp(&a.occurrences).then(a.id.cmp(&b.id)));

        let mut edges: Vec<GraphEdge> = edge_weights
            .into_iter()
            .map(|((source, target), weight)| GraphEdge {
                source,
                target,
                weight,
            })
            .collect();
        edges.sort_by_key(|e| std::cmp::Reverse(e.weight));

        KnowledgeGraph {
            built_at: Utc::now(),
            nodes,
            edges,
        }
    }
}

impl MemoryManager {
    /// Build the knowledge graph from all non-decayed memories and persist it
    /// to `<memory_dir>/graph.json`.
    pub fn build_graph(&self) -> Result<KnowledgeGraph> {
        let entries: Vec<MemoryEntry> = self
            .list_entries(None)?
            .into_iter()
            .filter(|e| !e.decayed)
            .collect();
        let graph = KnowledgeGraphBuilder::build(&entries);
        std::fs::create_dir_all(self.memory_dir()).map_err(|e| {
            AgentHubError::MemoryError(format!("Failed to create memory dir: {}", e))
        })?;
        std::fs::write(self.memory_dir().join("graph.json"), graph.to_json()?)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to write graph: {}", e)))?;
        Ok(graph)
    }

    /// Load the last persisted knowledge graph.
    pub fn load_graph(&self) -> Result<KnowledgeGraph> {
        let path = self.memory_dir().join("graph.json");
        if !path.exists() {
            return Err(AgentHubError::MemoryError(
                "No knowledge graph found; run `agenthub memory graph build` first".to_string(),
            ));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::MemoryError(format!("Failed to read graph: {}", e)))?;
        KnowledgeGraph::from_json(&content)
    }
}

/// Tokenize into lowercased alphanumeric tokens (length >= 2).
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

/// Extract `"..."` / `'...'` quoted phrases (length >= 2, <= 80).
fn quoted_phrases(content: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    let bytes: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote == '"' || quote == '\'' {
            let mut phrase = String::new();
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != quote {
                phrase.push(bytes[j]);
                j += 1;
            }
            if j < bytes.len() && (2..=80).contains(&phrase.chars().count()) {
                let id = phrase.trim().to_lowercase();
                if !id.is_empty() && id.chars().all(|c| c.is_alphanumeric() || c.is_whitespace()) {
                    phrases.push(id);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    phrases
}

/// Small stopword list to keep title-derived entities meaningful.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "this", "that", "how", "what", "when", "your", "you",
    "our", "are", "was", "were", "have", "has", "had", "not", "but", "all", "can", "will", "use",
    "using", "used", "about", "into", "than", "then", "them", "they", "their", "there", "also",
    "more", "most", "some", "any", "each", "other", "such", "only", "should", "would", "could",
    "may", "might", "must", "shall", "just", "very", "where", "which", "who", "whom",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryManager, MemoryScope, MemoryType};

    fn create_manager() -> (MemoryManager, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        (MemoryManager::new(temp.path().to_path_buf()), temp)
    }

    fn seed(manager: &MemoryManager) {
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Database schema design",
                "Use postgres with indexes on the \"users table\". Foreign keys link projects.",
                MemoryType::Reference,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Postgres tuning",
                "postgres needs proper indexing for \"users table\" lookups.",
                MemoryType::Learning,
            )
            .unwrap();
        manager
            .create_entry(
                MemoryScope::Global,
                None,
                "Deployment pipeline",
                "Deploy via github actions to production.",
                MemoryType::Learning,
            )
            .unwrap();
    }

    #[test]
    fn test_build_graph_nodes_and_edges() {
        let (manager, _temp) = create_manager();
        seed(&manager);
        let graph = manager.build_graph().unwrap();

        // postgres appears in a title token and a tag
        let pg = graph
            .nodes
            .iter()
            .find(|n| n.id == "postgres")
            .expect("postgres node");
        assert_eq!(pg.occurrences, 1);
        assert_eq!(pg.memories.len(), 1);

        // quoted phrase extracted from both entries
        let phrase = graph
            .nodes
            .iter()
            .find(|n| n.kind == EntityKind::Phrase && n.id == "users table")
            .expect("phrase node");
        assert_eq!(phrase.occurrences, 2);

        // "users table" co-occurs with postgres in one entry
        let edge = graph
            .edges
            .iter()
            .find(|e| {
                (e.source == "postgres" && e.target == "users table")
                    || (e.source == "users table" && e.target == "postgres")
            })
            .expect("co-occurrence edge");
        assert_eq!(edge.weight, 1);
    }

    #[test]
    fn test_graph_persists_and_reloads() {
        let (manager, _temp) = create_manager();
        seed(&manager);
        let graph = manager.build_graph().unwrap();
        let json = graph.to_json().unwrap();
        let reloaded = KnowledgeGraph::from_json(&json).unwrap();
        assert_eq!(reloaded.nodes.len(), graph.nodes.len());
        assert_eq!(reloaded.edges.len(), graph.edges.len());

        let loaded = manager.load_graph().unwrap();
        assert_eq!(loaded.nodes.len(), graph.nodes.len());
    }

    #[test]
    fn test_neighbors_returns_weighted_relations() {
        let (manager, _temp) = create_manager();
        seed(&manager);
        let graph = manager.build_graph().unwrap();
        let neighbors = graph.neighbors("users table", 10);
        assert!(!neighbors.is_empty());
        // strongest relation first
        assert!(neighbors[0].weight >= neighbors[neighbors.len() - 1].weight);
        // "users table" co-occurs with postgres (title entity)
        assert!(neighbors
            .iter()
            .any(|e| e.source == "postgres" || e.target == "postgres"));
    }

    #[test]
    fn test_summary() {
        let (manager, _temp) = create_manager();
        seed(&manager);
        let graph = manager.build_graph().unwrap();
        let summary = graph.summary();
        assert!(summary.node_count >= 5);
        assert!(!summary.top_entities.is_empty());
        // top entity is the most frequent: "users table" appears in two entries
        assert_eq!(summary.top_entities[0], "users table");
    }
}
