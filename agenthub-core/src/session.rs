use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Paused => write!(f, "paused"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUsage {
    #[serde(default)]
    pub total_tokens: u32,
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default)]
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub agent: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub duration_minutes: Option<u32>,
    #[serde(default)]
    pub messages: Vec<SessionMessage>,
    #[serde(default)]
    pub usage: Option<SessionUsage>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub rating: Option<u32>,
    #[serde(default)]
    pub notes: Option<String>,
}

pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    pub fn sessions_dir(&self) -> &Path {
        &self.sessions_dir
    }

    fn data_dir(&self) -> PathBuf {
        self.sessions_dir.join("data")
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.data_dir().join(format!("{}.yaml", id))
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let data_dir = self.data_dir();
        if !data_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&data_dir).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to read sessions dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| AgentHubError::SessionError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                match self.load_session_from_file(&path) {
                    Ok(session) => sessions.push(session),
                    Err(e) => {
                        eprintln!("Warning: Failed to load session at {:?}: {}", path, e);
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(sessions)
    }

    fn load_session_from_file(&self, path: &Path) -> Result<Session> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to read session: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to parse session: {}", e)))
    }

    pub fn get_session(&self, id: &str) -> Result<Session> {
        let path = self.session_path(id);
        if !path.exists() {
            return Err(AgentHubError::SessionError(format!(
                "Session not found: {}",
                id
            )));
        }

        self.load_session_from_file(&path)
    }

    pub fn create_session(&self, title: &str, agent: &str) -> Result<Session> {
        let id = format!(
            "ses_{}_{}",
            Utc::now().timestamp_millis(),
            rand::random::<u32>()
        );
        let now = Utc::now();

        let session = Session {
            id: id.clone(),
            title: title.to_string(),
            agent: agent.to_string(),
            model: None,
            project: None,
            status: SessionStatus::Active,
            started_at: now,
            ended_at: None,
            duration_minutes: None,
            messages: Vec::new(),
            usage: None,
            tags: Vec::new(),
            rating: None,
            notes: None,
        };

        self.save_session(&session)?;
        Ok(session)
    }

    pub fn save_session(&self, session: &Session) -> Result<()> {
        std::fs::create_dir_all(self.data_dir()).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to create sessions dir: {}", e))
        })?;

        let path = self.session_path(&session.id);
        let content = serde_yaml::to_string(session).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to serialize session: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to write session: {}", e)))?;

        Ok(())
    }

    pub fn update_status(&self, id: &str, status: SessionStatus) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.status = status.clone();

        if status == SessionStatus::Completed || status == SessionStatus::Failed {
            let now = Utc::now();
            session.ended_at = Some(now);
            session.duration_minutes = Some((now - session.started_at).num_minutes() as u32);
        }

        self.save_session(&session)
    }

    pub fn add_message(&self, id: &str, role: &str, content: &str) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens: None,
        });
        self.save_session(&session)
    }

    pub fn add_tag(&self, id: &str, tag: &str) -> Result<()> {
        let mut session = self.get_session(id)?;
        if !session.tags.contains(&tag.to_string()) {
            session.tags.push(tag.to_string());
            self.save_session(&session)?;
        }
        Ok(())
    }

    pub fn remove_tag(&self, id: &str, tag: &str) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.tags.retain(|t| t != tag);
        self.save_session(&session)
    }

    pub fn set_rating(&self, id: &str, rating: u32) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.rating = Some(rating.min(5));
        self.save_session(&session)
    }

    pub fn set_notes(&self, id: &str, notes: &str) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.notes = Some(notes.to_string());
        self.save_session(&session)
    }

    pub fn delete_session(&self, id: &str) -> Result<bool> {
        let path = self.session_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::SessionError(format!("Failed to delete session: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn search_sessions(&self, query: &str) -> Result<Vec<Session>> {
        let sessions = self.list_sessions()?;
        let query_lower = query.to_lowercase();

        Ok(sessions
            .into_iter()
            .filter(|s| {
                s.title.to_lowercase().contains(&query_lower)
                    || s.agent.to_lowercase().contains(&query_lower)
                    || s.messages
                        .iter()
                        .any(|m| m.content.to_lowercase().contains(&query_lower))
                    || s.notes
                        .as_ref()
                        .is_some_and(|n| n.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    pub fn get_stats(&self) -> Result<SessionStats> {
        let sessions = self.list_sessions()?;
        let total = sessions.len();
        let active = sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Active)
            .count();
        let completed = sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Completed)
            .count();
        let failed = sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Failed)
            .count();

        let total_tokens: u32 = sessions
            .iter()
            .filter_map(|s| s.usage.as_ref())
            .map(|u| u.total_tokens)
            .sum();

        let total_cost: f64 = sessions
            .iter()
            .filter_map(|s| s.usage.as_ref())
            .map(|u| u.estimated_cost_usd)
            .sum();

        Ok(SessionStats {
            total,
            active,
            completed,
            failed,
            total_tokens,
            total_cost,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub total_tokens: u32,
    pub total_cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (SessionManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_create_session() {
        let (manager, _temp) = create_test_manager();

        let session = manager
            .create_session("Test Session", "claude-code")
            .unwrap();
        assert_eq!(session.title, "Test Session");
        assert_eq!(session.agent, "claude-code");
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[test]
    fn test_list_sessions() {
        let (manager, _temp) = create_test_manager();

        manager.create_session("Session 1", "codex").unwrap();
        manager.create_session("Session 2", "claude-code").unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_update_status() {
        let (manager, _temp) = create_test_manager();

        let session = manager.create_session("Test", "codex").unwrap();
        manager
            .update_status(&session.id, SessionStatus::Completed)
            .unwrap();

        let updated = manager.get_session(&session.id).unwrap();
        assert_eq!(updated.status, SessionStatus::Completed);
        assert!(updated.ended_at.is_some());
    }

    #[test]
    fn test_add_message() {
        let (manager, _temp) = create_test_manager();

        let session = manager.create_session("Test", "codex").unwrap();
        manager.add_message(&session.id, "user", "Hello!").unwrap();
        manager
            .add_message(&session.id, "assistant", "Hi there!")
            .unwrap();

        let updated = manager.get_session(&session.id).unwrap();
        assert_eq!(updated.messages.len(), 2);
    }

    #[test]
    fn test_search_sessions() {
        let (manager, _temp) = create_test_manager();

        manager.create_session("Auth Refactor", "codex").unwrap();
        manager.create_session("Bug Fix", "claude-code").unwrap();

        let results = manager.search_sessions("auth").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Auth Refactor");
    }

    #[test]
    fn test_session_stats() {
        let (manager, _temp) = create_test_manager();

        manager.create_session("Session 1", "codex").unwrap();
        manager.create_session("Session 2", "claude-code").unwrap();

        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.active, 2);
    }
}
