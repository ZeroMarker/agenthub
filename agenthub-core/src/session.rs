use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::storage::is_safe_id;

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
    /// Number of API calls recorded for this session.
    #[serde(default)]
    pub calls: u32,
}

/// Per-model pricing in USD per 1,000,000 tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPricing {
    pub model: String,
    pub input_per_1m: f64,
    pub output_per_1m: f64,
}

impl ModelPricing {
    pub fn new(model: &str, input_per_1m: f64, output_per_1m: f64) -> Self {
        Self {
            model: model.to_string(),
            input_per_1m,
            output_per_1m,
        }
    }

    /// USD cost for the given token counts.
    pub fn cost_usd(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        (input_tokens as f64 / 1_000_000.0) * self.input_per_1m
            + (output_tokens as f64 / 1_000_000.0) * self.output_per_1m
    }
}

/// A lookup table of model pricing with a fallback for unknown models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTable {
    #[serde(default)]
    pub models: HashMap<String, ModelPricing>,
    /// Fallback USD per 1M tokens used for models not in the table.
    #[serde(default = "default_fallback_input")]
    pub fallback_input_per_1m: f64,
    #[serde(default = "default_fallback_output")]
    pub fallback_output_per_1m: f64,
}

fn default_fallback_input() -> f64 {
    3.0
}

fn default_fallback_output() -> f64 {
    15.0
}

impl Default for PricingTable {
    fn default() -> Self {
        Self {
            models: HashMap::new(),
            fallback_input_per_1m: default_fallback_input(),
            fallback_output_per_1m: default_fallback_output(),
        }
    }
}

impl PricingTable {
    /// Table of commonly used model prices (USD per 1M tokens).
    pub fn builtin() -> Self {
        let mut table = Self::default();
        for pricing in [
            ModelPricing::new("gpt-4o", 2.50, 10.00),
            ModelPricing::new("gpt-4o-mini", 0.15, 0.60),
            ModelPricing::new("gpt-4.1", 2.00, 8.00),
            ModelPricing::new("o3-mini", 1.10, 4.40),
            ModelPricing::new("o4-mini", 1.10, 4.40),
            ModelPricing::new("claude-opus-4", 15.00, 75.00),
            ModelPricing::new("claude-sonnet-4-20250514", 3.00, 15.00),
            ModelPricing::new("claude-sonnet-4", 3.00, 15.00),
            ModelPricing::new("claude-3-5-haiku", 0.80, 4.00),
            ModelPricing::new("claude-haiku-4-5", 1.00, 5.00),
            ModelPricing::new("gemini-2.5-pro", 1.25, 10.00),
            ModelPricing::new("gemini-2.5-flash", 0.30, 2.50),
            ModelPricing::new("gemini-1.5-pro", 1.25, 5.00),
            ModelPricing::new("deepseek-chat", 0.27, 1.10),
            ModelPricing::new("deepseek-reasoner", 0.55, 2.19),
            ModelPricing::new("qwen-max", 1.60, 6.40),
            ModelPricing::new("qwen-plus", 0.80, 2.00),
        ] {
            table.add(pricing);
        }
        table
    }

    pub fn add(&mut self, pricing: ModelPricing) {
        self.models.insert(pricing.model.clone(), pricing);
    }

    pub fn get(&self, model: &str) -> Option<&ModelPricing> {
        self.models.get(model)
    }

    /// Cost for a model (or the fallback if unknown) for the given token counts.
    pub fn cost_usd(&self, model: Option<&str>, input_tokens: u32, output_tokens: u32) -> f64 {
        match model.and_then(|m| self.models.get(m)) {
            Some(pricing) => pricing.cost_usd(input_tokens, output_tokens),
            None => {
                (input_tokens as f64 / 1_000_000.0) * self.fallback_input_per_1m
                    + (output_tokens as f64 / 1_000_000.0) * self.fallback_output_per_1m
            }
        }
    }

    pub fn set_fallback(&mut self, input_per_1m: f64, output_per_1m: f64) {
        self.fallback_input_per_1m = input_per_1m;
        self.fallback_output_per_1m = output_per_1m;
    }
}

/// A reusable session starter: preset messages that seed new sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTemplate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub messages: Vec<TemplateMessage>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMessage {
    pub role: String,
    pub content: String,
}

/// Workspace cost budget limits (USD). `None` means no limit.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BudgetConfig {
    #[serde(default)]
    pub daily_usd: Option<f64>,
    #[serde(default)]
    pub monthly_usd: Option<f64>,
}

/// Current spending against the budget, plus any threshold alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetReport {
    pub daily_spent_usd: f64,
    pub daily_limit_usd: Option<f64>,
    pub monthly_spent_usd: f64,
    pub monthly_limit_usd: Option<f64>,
    pub total_tokens_today: u64,
    #[serde(default)]
    pub alerts: Vec<String>,
}

/// Portable context extracted from a session, used to hand off to another
/// session / agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub source_session: String,
    pub agent: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub messages: Vec<ContextMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
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
    fn validate_id(kind: &str, id: &str) -> Result<()> {
        if !is_safe_id(id) {
            return Err(AgentHubError::SessionError(format!(
                "Invalid {kind} id: {id}"
            )));
        }
        Ok(())
    }

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

        sessions.sort_by_key(|b| std::cmp::Reverse(b.started_at));
        Ok(sessions)
    }

    fn load_session_from_file(&self, path: &Path) -> Result<Session> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to read session: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to parse session: {}", e)))
    }

    pub fn get_session(&self, id: &str) -> Result<Session> {
        Self::validate_id("session", id)?;
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
        Self::validate_id("session", &session.id)?;
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
        self.add_message_with_tokens(id, role, content, None)
    }

    /// Append a message, optionally recording the token count used by it.
    pub fn add_message_with_tokens(
        &self,
        id: &str,
        role: &str,
        content: &str,
        tokens: Option<u32>,
    ) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens,
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
        Self::validate_id("session", id)?;
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

    /// Record model usage for a session and (re)compute the estimated cost using
    /// the supplied pricing table. Tokens, calls and cost accumulate across calls.
    pub fn record_usage(
        &self,
        id: &str,
        input_tokens: u32,
        output_tokens: u32,
        pricing: &PricingTable,
    ) -> Result<()> {
        let mut session = self.get_session(id)?;
        let model = session.model.as_deref();
        let cost_added = pricing.cost_usd(model, input_tokens, output_tokens);

        let usage = session.usage.get_or_insert(SessionUsage {
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
            calls: 0,
        });
        usage.input_tokens += input_tokens;
        usage.output_tokens += output_tokens;
        usage.total_tokens += input_tokens + output_tokens;
        usage.estimated_cost_usd += cost_added;
        usage.calls += 1;

        self.save_session(&session)
    }

    /// Set the model used by a session (used for cost estimation).
    pub fn set_model(&self, id: &str, model: &str) -> Result<()> {
        let mut session = self.get_session(id)?;
        session.model = Some(model.to_string());
        self.save_session(&session)
    }

    /// Render a session as a markdown transcript (会话回放).
    pub fn replay_session(&self, id: &str) -> Result<String> {
        let session = self.get_session(id)?;

        let mut out = String::new();
        out.push_str(&format!("# {}\n\n", session.title));
        out.push_str(&format!("- **Agent**: {}\n", session.agent));
        if let Some(model) = &session.model {
            out.push_str(&format!("- **Model**: {}\n", model));
        }
        out.push_str(&format!("- **Status**: {}\n", session.status));
        out.push_str(&format!(
            "- **Started**: {}\n",
            session.started_at.to_rfc3339()
        ));
        if let Some(ended) = session.ended_at {
            out.push_str(&format!("- **Ended**: {}\n", ended.to_rfc3339()));
        }
        if let Some(usage) = &session.usage {
            out.push_str(&format!(
                "- **Tokens**: {} ({} in / {} out)\n",
                usage.total_tokens, usage.input_tokens, usage.output_tokens
            ));
            out.push_str(&format!("- **Cost**: ${:.4}\n", usage.estimated_cost_usd));
        }
        if let Some(project) = &session.project {
            out.push_str(&format!("- **Project**: {}\n", project));
        }
        out.push('\n');

        for message in &session.messages {
            out.push_str(&format!("## {}\n\n", message.role));
            out.push_str(&message.content);
            out.push_str("\n\n");
        }

        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Session templates
    // -----------------------------------------------------------------------

    fn templates_dir(&self) -> PathBuf {
        self.sessions_dir.join("templates")
    }

    fn template_path(&self, id: &str) -> PathBuf {
        self.templates_dir().join(format!("{}.yaml", id))
    }

    pub fn create_template(
        &self,
        id: &str,
        name: &str,
        description: &str,
        agent: Option<&str>,
        messages: Vec<TemplateMessage>,
        tags: Vec<String>,
    ) -> Result<SessionTemplate> {
        Self::validate_id("session template", id)?;
        let path = self.template_path(id);
        if path.exists() {
            return Err(AgentHubError::SessionError(format!(
                "Session template already exists: {}",
                id
            )));
        }

        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to create templates dir: {}", e))
        })?;

        let template = SessionTemplate {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            agent: agent.map(|s| s.to_string()),
            messages,
            tags,
            created_at: Some(Utc::now()),
        };

        self.save_template(&template)?;
        Ok(template)
    }

    pub fn save_template(&self, template: &SessionTemplate) -> Result<()> {
        Self::validate_id("session template", &template.id)?;
        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to create templates dir: {}", e))
        })?;

        let path = self.template_path(&template.id);
        let content = serde_yaml::to_string(template).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to serialize template: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to write template: {}", e)))?;

        Ok(())
    }

    pub fn list_templates(&self) -> Result<Vec<SessionTemplate>> {
        let templates_dir = self.templates_dir();
        if !templates_dir.exists() {
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();
        for entry in std::fs::read_dir(&templates_dir).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to read templates dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| AgentHubError::SessionError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                match self.load_template_from_file(&path) {
                    Ok(t) => templates.push(t),
                    Err(e) => {
                        eprintln!("Warning: Failed to load template at {:?}: {}", path, e);
                    }
                }
            }
        }

        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(templates)
    }

    fn load_template_from_file(&self, path: &Path) -> Result<SessionTemplate> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to read template: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to parse template: {}", e)))
    }

    pub fn get_template(&self, id: &str) -> Result<SessionTemplate> {
        Self::validate_id("session template", id)?;
        let path = self.template_path(id);
        if !path.exists() {
            return Err(AgentHubError::SessionError(format!(
                "Session template not found: {}",
                id
            )));
        }

        self.load_template_from_file(&path)
    }

    pub fn delete_template(&self, id: &str) -> Result<bool> {
        Self::validate_id("session template", id)?;
        let path = self.template_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::SessionError(format!("Failed to delete template: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Create a new session pre-populated from a template's messages.
    pub fn create_session_from_template(&self, template_id: &str, title: &str) -> Result<Session> {
        let template = self.get_template(template_id)?;
        let agent = template.agent.as_deref().unwrap_or("unknown");
        let mut session = self.create_session(title, agent)?;

        let now = Utc::now();
        for msg in &template.messages {
            session.messages.push(SessionMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
                timestamp: now,
                tokens: None,
            });
        }
        for tag in &template.tags {
            if !session.tags.contains(tag) {
                session.tags.push(tag.clone());
            }
        }

        self.save_session(&session)?;
        Ok(session)
    }

    // -----------------------------------------------------------------------
    // Cost budget & alerts
    // -----------------------------------------------------------------------

    fn budget_path(&self) -> PathBuf {
        self.sessions_dir.join("budget.yaml")
    }

    /// Load the workspace cost budget (defaults to no limits when unset).
    pub fn get_budget(&self) -> Result<BudgetConfig> {
        let path = self.budget_path();
        if !path.exists() {
            return Ok(BudgetConfig::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to read budget: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to parse budget: {}", e)))
    }

    pub fn set_budget(&self, budget: &BudgetConfig) -> Result<()> {
        std::fs::create_dir_all(&self.sessions_dir).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to create sessions dir: {}", e))
        })?;

        let content = serde_yaml::to_string(budget).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to serialize budget: {}", e))
        })?;

        std::fs::write(self.budget_path(), content)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to write budget: {}", e)))?;

        Ok(())
    }

    /// Compute how much was spent today / this month (UTC, by session start)
    /// against the configured budget and produce alerts when limits are hit.
    pub fn check_budget(&self, now: DateTime<Utc>) -> Result<BudgetReport> {
        let budget = self.get_budget()?;
        let sessions = self.list_sessions()?;

        let today = now.date_naive();
        let month_start = today - chrono::Duration::days(today.day() as i64 - 1);

        let mut daily_spent = 0.0f64;
        let mut monthly_spent = 0.0f64;
        let mut total_tokens_today = 0u64;
        for session in &sessions {
            let started = session.started_at.date_naive();
            let cost = session
                .usage
                .as_ref()
                .map(|u| u.estimated_cost_usd)
                .unwrap_or(0.0);
            if started == today {
                daily_spent += cost;
                total_tokens_today += session
                    .usage
                    .as_ref()
                    .map(|u| u.total_tokens as u64)
                    .unwrap_or(0);
            }
            if started >= month_start {
                monthly_spent += cost;
            }
        }

        let mut alerts = Vec::new();
        if let Some(limit) = budget.daily_usd {
            if daily_spent > limit {
                alerts.push(format!(
                    "Daily budget exceeded: ${:.2} spent > ${:.2} limit",
                    daily_spent, limit
                ));
            }
        }
        if let Some(limit) = budget.monthly_usd {
            if monthly_spent > limit {
                alerts.push(format!(
                    "Monthly budget exceeded: ${:.2} spent > ${:.2} limit",
                    monthly_spent, limit
                ));
            }
        }

        Ok(BudgetReport {
            daily_spent_usd: daily_spent,
            daily_limit_usd: budget.daily_usd,
            monthly_spent_usd: monthly_spent,
            monthly_limit_usd: budget.monthly_usd,
            total_tokens_today,
            alerts,
        })
    }

    // -----------------------------------------------------------------------
    // Cross-session context transfer
    // -----------------------------------------------------------------------

    /// Extract the trailing messages of a session as portable context.
    pub fn export_context(&self, id: &str, last_n: Option<usize>) -> Result<SessionContext> {
        let session = self.get_session(id)?;
        let mut messages: Vec<ContextMessage> = session
            .messages
            .iter()
            .map(|m| ContextMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        if let Some(n) = last_n {
            let skip = messages.len().saturating_sub(n);
            messages.drain(..skip);
        }

        Ok(SessionContext {
            source_session: id.to_string(),
            agent: session.agent,
            model: session.model,
            messages,
        })
    }

    pub fn export_context_json(&self, id: &str, last_n: Option<usize>) -> Result<String> {
        let context = self.export_context(id, last_n)?;
        serde_json::to_string_pretty(&context)
            .map_err(|e| AgentHubError::SessionError(format!("Failed to serialize context: {}", e)))
    }

    /// Start a new session carrying the messages (and optionally another
    /// agent/model) of a source session — the basis for cross-agent context
    /// handoff.
    pub fn fork_session(
        &self,
        source_id: &str,
        agent: Option<&str>,
        title: Option<&str>,
    ) -> Result<Session> {
        let source = self.get_session(source_id)?;
        let agent = agent.unwrap_or(&source.agent);
        let title = title
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!("Fork of {}", source.title));

        let mut session = self.create_session(&title, agent)?;
        session.model = source.model.clone();
        let now = Utc::now();
        for message in &source.messages {
            session.messages.push(SessionMessage {
                role: message.role.clone(),
                content: message.content.clone(),
                timestamp: now,
                tokens: message.tokens,
            });
        }
        for tag in &source.tags {
            if !session.tags.contains(tag) {
                session.tags.push(tag.clone());
            }
        }
        if source.project.is_some() {
            session.project = source.project.clone();
        }

        self.save_session(&session)?;
        Ok(session)
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
            .fold(0.0, |acc, cost| acc + cost);

        Ok(SessionStats {
            total,
            active,
            completed,
            failed,
            total_tokens,
            total_cost,
        })
    }

    // -------------------------------------------------------------------
    // API-call / cost aggregation & export
    // -------------------------------------------------------------------

    /// Aggregate API calls, tokens and cost across every session.
    pub fn usage_summary(&self) -> Result<SessionUsageAggregate> {
        let sessions = self.list_sessions()?;
        let mut agg = SessionUsageAggregate::default();
        for session in &sessions {
            if let Some(usage) = &session.usage {
                agg.sessions += 1;
                agg.api_calls += usage.calls;
                agg.total_tokens += usage.total_tokens;
                agg.input_tokens += usage.input_tokens;
                agg.output_tokens += usage.output_tokens;
                agg.cost_usd += usage.estimated_cost_usd;
            }
        }
        Ok(agg)
    }

    /// Bucket API calls / tokens / cost by UTC date over the last `days` days
    /// (oldest first). Sessions without usage are skipped.
    pub fn usage_trend(&self, days: usize) -> Result<Vec<UsageTrendPoint>> {
        self.usage_trend_with_now(days, Utc::now())
    }

    pub fn usage_trend_with_now(
        &self,
        days: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<UsageTrendPoint>> {
        if days == 0 {
            return Ok(Vec::new());
        }
        let sessions = self.list_sessions()?;
        let mut buckets: HashMap<String, UsageTrendPoint> = HashMap::new();
        for session in &sessions {
            let Some(usage) = &session.usage else {
                continue;
            };
            let date = session.started_at.format("%Y-%m-%d").to_string();
            let bucket = buckets.entry(date.clone()).or_insert(UsageTrendPoint {
                date,
                api_calls: 0,
                tokens: 0,
                cost_usd: 0.0,
            });
            bucket.api_calls += usage.calls;
            bucket.tokens += usage.total_tokens;
            bucket.cost_usd += usage.estimated_cost_usd;
        }

        let mut trend: Vec<UsageTrendPoint> = buckets.into_values().collect();
        trend.sort_by(|a, b| a.date.cmp(&b.date));
        // Keep only buckets within the requested window relative to `now`.
        let cutoff = (now - chrono::Duration::days(days as i64))
            .format("%Y-%m-%d")
            .to_string();
        trend.retain(|point| point.date >= cutoff);
        Ok(trend)
    }

    /// Per-session usage rows for export.
    pub fn usage_rows(&self) -> Result<Vec<SessionUsageRow>> {
        let sessions = self.list_sessions()?;
        let mut rows = Vec::new();
        for session in sessions {
            let usage = session.usage.unwrap_or(SessionUsage {
                total_tokens: 0,
                input_tokens: 0,
                output_tokens: 0,
                estimated_cost_usd: 0.0,
                calls: 0,
            });
            rows.push(SessionUsageRow {
                session_id: session.id,
                title: session.title,
                agent: session.agent,
                model: session.model,
                status: session.status,
                started_at: session.started_at,
                calls: usage.calls,
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
                cost_usd: usage.estimated_cost_usd,
            });
        }
        rows.sort_by_key(|b| std::cmp::Reverse(b.started_at));
        Ok(rows)
    }

    /// Serialize per-session usage plus the daily trend as pretty JSON.
    pub fn export_usage_json(&self, days: usize) -> Result<String> {
        let export = UsageExport {
            generated_at: Utc::now(),
            trend_days: days,
            sessions: self.usage_rows()?,
            daily: self.usage_trend(days)?,
            total: self.usage_summary()?,
        };
        serde_json::to_string_pretty(&export).map_err(|e| {
            AgentHubError::SessionError(format!("Failed to serialize usage export: {}", e))
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

/// Aggregated API usage across every session.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionUsageAggregate {
    /// Sessions that recorded at least one usage entry.
    pub sessions: usize,
    pub api_calls: u32,
    pub total_tokens: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
}

/// Per-day usage bucket (UTC).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageTrendPoint {
    /// UTC date `YYYY-MM-DD`.
    pub date: String,
    pub api_calls: u32,
    pub tokens: u32,
    pub cost_usd: f64,
}

/// Per-session usage row for export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionUsageRow {
    pub session_id: String,
    pub title: String,
    pub agent: String,
    pub model: Option<String>,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub calls: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: f64,
}

/// JSON export payload: per-session usage + daily trend + totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageExport {
    pub generated_at: DateTime<Utc>,
    pub trend_days: usize,
    pub sessions: Vec<SessionUsageRow>,
    pub daily: Vec<UsageTrendPoint>,
    pub total: SessionUsageAggregate,
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

    // ---- Cost tracking ----

    #[test]
    fn test_pricing_cost_usd() {
        let pricing = ModelPricing::new("gpt-4o", 2.50, 10.00);
        // 1M input + 0 output = $2.50
        assert!((pricing.cost_usd(1_000_000, 0) - 2.50).abs() < 1e-9);
        // 0 input + 1M output = $10.00
        assert!((pricing.cost_usd(0, 1_000_000) - 10.00).abs() < 1e-9);
        // 1M input + 1M output = $12.50
        assert!((pricing.cost_usd(1_000_000, 1_000_000) - 12.50).abs() < 1e-9);
    }

    #[test]
    fn test_pricing_table_builtin_and_fallback() {
        let table = PricingTable::builtin();
        assert!(table.get("gpt-4o").is_some());
        assert!(table.get("claude-sonnet-4-20250514").is_some());

        // Known model
        let cost = table.cost_usd(Some("gpt-4o-mini"), 1_000_000, 0);
        assert!((cost - 0.15).abs() < 1e-9);

        // Unknown model uses fallback (3.0/15.0)
        let cost = table.cost_usd(Some("unknown-model"), 1_000_000, 0);
        assert!((cost - 3.0).abs() < 1e-9);

        // No model at all
        let cost = table.cost_usd(None, 0, 1_000_000);
        assert!((cost - 15.0).abs() < 1e-9);

        // Custom override
        let mut custom = PricingTable::default();
        custom.add(ModelPricing::new("my-model", 1.0, 2.0));
        assert!((custom.cost_usd(Some("my-model"), 1_000_000, 0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_record_usage_accumulates() {
        let (manager, _temp) = create_test_manager();
        let table = PricingTable::builtin();

        let session = manager.create_session("Cost Test", "codex").unwrap();
        manager.set_model(&session.id, "gpt-4o-mini").unwrap();

        manager
            .record_usage(&session.id, 100_000, 20_000, &table)
            .unwrap();
        manager
            .record_usage(&session.id, 50_000, 10_000, &table)
            .unwrap();

        let updated = manager.get_session(&session.id).unwrap();
        let usage = updated.usage.unwrap();
        assert_eq!(usage.input_tokens, 150_000);
        assert_eq!(usage.output_tokens, 30_000);
        assert_eq!(usage.total_tokens, 180_000);

        // gpt-4o-mini: 0.15/1M in, 0.60/1M out
        let expected = 0.15 * 0.15 + 0.60 * 0.03;
        assert!((usage.estimated_cost_usd - expected).abs() < 1e-9);
    }

    #[test]
    fn test_record_usage_unknown_model_uses_fallback() {
        let (manager, _temp) = create_test_manager();
        let table = PricingTable::builtin();

        let session = manager.create_session("Cost Test", "codex").unwrap();
        manager
            .record_usage(&session.id, 1_000_000, 0, &table)
            .unwrap();

        let updated = manager.get_session(&session.id).unwrap();
        assert!((updated.usage.unwrap().estimated_cost_usd - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_add_message_with_tokens() {
        let (manager, _temp) = create_test_manager();

        let session = manager.create_session("Msg", "codex").unwrap();
        manager
            .add_message_with_tokens(&session.id, "user", "hi", Some(42))
            .unwrap();

        let updated = manager.get_session(&session.id).unwrap();
        assert_eq!(updated.messages[0].tokens, Some(42));
    }

    // ---- Replay ----

    #[test]
    fn test_replay_session() {
        let (manager, _temp) = create_test_manager();

        let session = manager.create_session("Replay", "codex").unwrap();
        manager.add_message(&session.id, "user", "Hello").unwrap();
        manager
            .add_message(&session.id, "assistant", "Hi!")
            .unwrap();

        let replay = manager.replay_session(&session.id).unwrap();
        assert!(replay.contains("# Replay"));
        assert!(replay.contains("**Agent**: codex"));
        assert!(replay.contains("## user"));
        assert!(replay.contains("Hello"));
        assert!(replay.contains("## assistant"));
        assert!(replay.contains("Hi!"));
    }

    // ---- Templates ----

    #[test]
    fn test_session_templates_crud() {
        let (manager, _temp) = create_test_manager();

        let template = manager
            .create_template(
                "code-review",
                "Code Review",
                "Standard code review flow",
                Some("codex"),
                vec![
                    TemplateMessage {
                        role: "user".to_string(),
                        content: "Review this diff".to_string(),
                    },
                    TemplateMessage {
                        role: "user".to_string(),
                        content: "Check for security issues".to_string(),
                    },
                ],
                vec!["review".to_string()],
            )
            .unwrap();

        assert_eq!(template.name, "Code Review");
        assert_eq!(manager.list_templates().unwrap().len(), 1);

        let loaded = manager.get_template("code-review").unwrap();
        assert_eq!(loaded.messages.len(), 2);

        // Duplicate id is rejected
        assert!(manager
            .create_template("code-review", "X", "", None, Vec::new(), Vec::new())
            .is_err());

        assert!(manager.delete_template("code-review").unwrap());
        assert_eq!(manager.list_templates().unwrap().len(), 0);
    }

    #[test]
    fn test_create_session_from_template() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_template(
                "tpl",
                "T",
                "",
                Some("codex"),
                vec![TemplateMessage {
                    role: "user".to_string(),
                    content: "Start".to_string(),
                }],
                vec!["tag1".to_string()],
            )
            .unwrap();

        let session = manager
            .create_session_from_template("tpl", "From Template")
            .unwrap();
        assert_eq!(session.agent, "codex");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Start");
        assert!(session.tags.contains(&"tag1".to_string()));
    }

    // ---- Budget ----

    #[test]
    fn test_budget_default_and_set() {
        let (manager, _temp) = create_test_manager();

        let budget = manager.get_budget().unwrap();
        assert_eq!(budget, BudgetConfig::default());

        manager
            .set_budget(&BudgetConfig {
                daily_usd: Some(5.0),
                monthly_usd: Some(50.0),
            })
            .unwrap();
        let budget = manager.get_budget().unwrap();
        assert_eq!(budget.daily_usd, Some(5.0));
        assert_eq!(budget.monthly_usd, Some(50.0));
    }

    #[test]
    fn test_check_budget_alerts() {
        let (manager, _temp) = create_test_manager();
        let table = PricingTable::builtin();
        let now = Utc::now();

        // Session started today with gpt-4o-mini cost
        let session = manager.create_session("Today", "codex").unwrap();
        manager.set_model(&session.id, "gpt-4o-mini").unwrap();
        manager
            .record_usage(&session.id, 40_000_000, 0, &table)
            .unwrap(); // $6.00

        manager
            .set_budget(&BudgetConfig {
                daily_usd: Some(5.0),
                monthly_usd: None,
            })
            .unwrap();

        let report = manager.check_budget(now).unwrap();
        assert!((report.daily_spent_usd - 6.0).abs() < 1e-6);
        assert_eq!(report.alerts.len(), 1);
        assert!(report.alerts[0].contains("Daily budget exceeded"));
        assert_eq!(report.total_tokens_today, 40_000_000);

        // Below limit -> no alert
        manager
            .set_budget(&BudgetConfig {
                daily_usd: Some(10.0),
                monthly_usd: None,
            })
            .unwrap();
        let report = manager.check_budget(now).unwrap();
        assert!(report.alerts.is_empty());
    }

    #[test]
    fn test_check_budget_counts_only_today() {
        let (manager, temp) = create_test_manager();

        let old = Utc::now() - chrono::Duration::days(2);
        let stale_path = temp.path().join("data").join("ses_stale.yaml");
        std::fs::create_dir_all(stale_path.parent().unwrap()).unwrap();
        let mut stale = manager.create_session("Stale", "codex").unwrap();
        stale.started_at = old;
        stale.model = Some("gpt-4o-mini".to_string());
        stale.usage = Some(SessionUsage {
            total_tokens: 10_000_000,
            input_tokens: 10_000_000,
            output_tokens: 0,
            estimated_cost_usd: 1.5,
            calls: 0,
        });
        manager.save_session(&stale).unwrap();

        manager
            .set_budget(&BudgetConfig {
                daily_usd: Some(0.10),
                monthly_usd: None,
            })
            .unwrap();

        let report = manager.check_budget(Utc::now()).unwrap();
        // Old session not counted today, but counts this month
        assert!(report.daily_spent_usd.abs() < 1e-9);
        assert!((report.monthly_spent_usd - 1.5).abs() < 1e-9);
        assert!(report.alerts.is_empty());
    }

    // ---- Context transfer ----

    #[test]
    fn test_export_context() {
        let (manager, _temp) = create_test_manager();

        let session = manager.create_session("Src", "codex").unwrap();
        manager.add_message(&session.id, "user", "m1").unwrap();
        manager.add_message(&session.id, "assistant", "m2").unwrap();
        manager.add_message(&session.id, "user", "m3").unwrap();

        let context = manager.export_context(&session.id, None).unwrap();
        assert_eq!(context.source_session, session.id);
        assert_eq!(context.messages.len(), 3);
        assert_eq!(context.messages[2].content, "m3");

        // last_n keeps only the trailing messages
        let context = manager.export_context(&session.id, Some(2)).unwrap();
        assert_eq!(context.messages.len(), 2);
        assert_eq!(context.messages[0].content, "m2");

        // JSON round-trips
        let json = manager.export_context_json(&session.id, None).unwrap();
        assert!(json.contains("\"source_session\""));
    }

    #[test]
    fn test_fork_session_carries_context() {
        let (manager, _temp) = create_test_manager();

        let source = manager.create_session("Original", "codex").unwrap();
        manager.set_model(&source.id, "gpt-4o").unwrap();
        manager.add_message(&source.id, "user", "ctx msg").unwrap();
        manager.add_tag(&source.id, "refactor").unwrap();

        let fork = manager
            .fork_session(&source.id, Some("claude-code"), None)
            .unwrap();
        assert_eq!(fork.agent, "claude-code");
        assert_eq!(fork.model.as_deref(), Some("gpt-4o"));
        assert_eq!(fork.messages.len(), 1);
        assert_eq!(fork.messages[0].content, "ctx msg");
        assert!(fork.tags.contains(&"refactor".to_string()));
        assert!(fork.title.starts_with("Fork of"));

        // Default title + same agent
        let fork2 = manager.fork_session(&source.id, None, None).unwrap();
        assert_eq!(fork2.agent, "codex");
    }

    #[test]
    fn rejects_unsafe_session_and_template_ids() {
        let (manager, temp) = create_test_manager();
        let mut session = manager.create_session("Safe", "codex").unwrap();
        session.id = "../escape".to_string();
        assert!(manager.save_session(&session).is_err());
        assert!(manager.get_session("../escape").is_err());
        assert!(!temp.path().join("escape.yaml").exists());

        let template = SessionTemplate {
            id: "../escape".to_string(),
            name: "Unsafe".to_string(),
            description: String::new(),
            agent: None,
            messages: Vec::new(),
            tags: Vec::new(),
            created_at: None,
        };
        assert!(manager.save_template(&template).is_err());
    }

    #[test]
    fn test_get_session_corrupt_file_errors() {
        let temp = TempDir::new().unwrap();
        let manager = SessionManager::new(temp.path().join("sessions"));
        let session = manager.create_session("S1", "codex").unwrap();

        std::fs::write(
            manager
                .sessions_dir()
                .join("data")
                .join(format!("{}.yaml", session.id)),
            "id: \"unterminated",
        )
        .unwrap();
        assert!(manager.get_session(&session.id).is_err());
        // Listing must skip corrupt sessions, not fail.
        assert!(manager.list_sessions().unwrap().is_empty());
    }

    // ---- API-call tracking, aggregation & export ----

    fn session_with_usage(manager: &SessionManager, agent: &str, now: DateTime<Utc>) -> Session {
        let mut session = manager.create_session(agent, agent).unwrap();
        session.started_at = now;
        session.usage.get_or_insert(SessionUsage {
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
            calls: 0,
        });
        manager.save_session(&session).unwrap();
        session
    }

    #[test]
    fn test_record_usage_increments_calls() {
        let (manager, _temp) = create_test_manager();
        let session = manager.create_session("S1", "codex").unwrap();
        let pricing = PricingTable::default();

        manager
            .record_usage(&session.id, 100, 50, &pricing)
            .unwrap();
        manager
            .record_usage(&session.id, 200, 25, &pricing)
            .unwrap();

        let usage = manager.get_session(&session.id).unwrap().usage.unwrap();
        assert_eq!(usage.calls, 2);
        assert_eq!(usage.total_tokens, 375);
        assert_eq!(usage.input_tokens, 300);
        assert_eq!(usage.output_tokens, 75);
        assert!(usage.estimated_cost_usd > 0.0);
    }

    #[test]
    fn test_usage_summary_aggregates() {
        let (manager, _temp) = create_test_manager();
        let pricing = PricingTable::default();
        let s1 = manager.create_session("S1", "codex").unwrap();
        let s2 = manager.create_session("S2", "claude").unwrap();
        manager.record_usage(&s1.id, 100, 50, &pricing).unwrap();
        manager.record_usage(&s2.id, 10, 5, &pricing).unwrap();
        manager.create_session("S3", "nousage").unwrap(); // no usage

        let agg = manager.usage_summary().unwrap();
        assert_eq!(agg.sessions, 2);
        assert_eq!(agg.api_calls, 2);
        assert_eq!(agg.total_tokens, 165);
        assert_eq!(agg.input_tokens, 110);
        assert_eq!(agg.output_tokens, 55);
        assert!(agg.cost_usd > 0.0);
    }

    #[test]
    fn test_usage_trend_buckets_by_day() {
        let (manager, _temp) = create_test_manager();
        let pricing = PricingTable::default();
        let now = Utc::now();

        // Two sessions today, one session three days ago.
        let s1 = session_with_usage(&manager, "codex", now);
        manager.record_usage(&s1.id, 100, 50, &pricing).unwrap();
        let s2 = session_with_usage(&manager, "codex", now);
        manager.record_usage(&s2.id, 10, 5, &pricing).unwrap();
        let old = session_with_usage(&manager, "codex", now - chrono::Duration::days(3));
        manager.record_usage(&old.id, 1, 1, &pricing).unwrap();

        let trend = manager.usage_trend_with_now(7, now).unwrap();
        assert_eq!(trend.len(), 2);
        let today = trend.last().unwrap();
        assert_eq!(today.api_calls, 2);
        assert_eq!(today.tokens, 165);
        let old_day = trend.first().unwrap();
        assert_eq!(old_day.api_calls, 1);

        // Window truncation: with days=1 only today survives.
        let trend = manager.usage_trend_with_now(1, now).unwrap();
        assert_eq!(trend.len(), 1);
        assert_eq!(trend[0].api_calls, 2);
    }

    #[test]
    fn test_export_usage_json_roundtrip() {
        let (manager, _temp) = create_test_manager();
        let pricing = PricingTable::default();
        let session = manager.create_session("S1", "codex").unwrap();
        manager.set_model(&session.id, "gpt-4o").unwrap();
        manager
            .record_usage(&session.id, 100, 50, &pricing)
            .unwrap();

        let json = manager.export_usage_json(30).unwrap();
        let parsed: UsageExport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sessions.len(), 1);
        assert_eq!(parsed.sessions[0].session_id, session.id);
        assert_eq!(parsed.sessions[0].calls, 1);
        assert_eq!(parsed.sessions[0].model.as_deref(), Some("gpt-4o"));
        assert_eq!(parsed.total.api_calls, 1);
        assert_eq!(parsed.daily.len(), 1);
        assert_eq!(parsed.daily[0].api_calls, 1);
        assert_eq!(parsed.total.sessions, 1);
    }
}
