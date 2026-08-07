use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template: String,
    #[serde(default)]
    pub variables: Vec<PromptVariable>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    /// Number of times the prompt has been rendered.
    #[serde(default)]
    pub usage_count: u64,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Usage statistics for a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUsage {
    pub id: String,
    pub name: String,
    pub usage_count: u64,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Bundle for prompt export/import (current templates + version history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptExportBundle {
    pub prompts: Vec<PromptTemplate>,
    #[serde(default)]
    pub versions: HashMap<String, Vec<PromptTemplate>>,
}

/// Result of an import operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportSummary {
    pub imported: usize,
    pub skipped: usize,
}

/// A single historical snapshot of a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    pub version: u32,
    pub name: String,
    pub description: String,
    pub template: String,
    pub updated_at: Option<DateTime<Utc>>,
}

/// One recorded outcome of using a prompt in a session (effectiveness tracking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOutcome {
    pub session_id: String,
    /// Session rating (1-5), when rated.
    #[serde(default)]
    pub rating: Option<u32>,
    /// Whether the session completed successfully.
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub tokens: u32,
    #[serde(default)]
    pub cost_usd: f64,
    pub recorded_at: DateTime<Utc>,
}

/// Aggregated effectiveness statistics for one prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEffects {
    pub prompt_id: String,
    /// Number of recorded session outcomes.
    pub uses: usize,
    /// Average session rating (1-5), None when unrated.
    pub avg_rating: Option<f64>,
    /// Fraction of sessions marked successful, None when unknown.
    pub success_rate: Option<f64>,
    pub total_tokens: u32,
    pub total_cost_usd: f64,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OutcomesFile {
    #[serde(default)]
    outcomes: Vec<PromptOutcome>,
}

pub struct PromptManager {
    prompts_dir: PathBuf,
}

/// Result of extracting a prompt template from a session message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptExtraction {
    pub prompt: PromptTemplate,
    pub source_session_id: String,
    pub source_message_index: usize,
    pub source_role: String,
}

impl PromptManager {
    pub fn new(prompts_dir: PathBuf) -> Self {
        Self { prompts_dir }
    }

    pub fn prompts_dir(&self) -> &Path {
        &self.prompts_dir
    }

    fn templates_dir(&self) -> PathBuf {
        self.prompts_dir.join("templates")
    }

    fn versions_dir(&self, id: &str) -> PathBuf {
        self.templates_dir().join("versions").join(id)
    }

    fn version_path(&self, id: &str, version: u32) -> PathBuf {
        self.versions_dir(id).join(format!("v{}.yaml", version))
    }

    /// Snapshot the current version of a prompt before it is modified.
    fn snapshot_current(&self, prompt: &PromptTemplate) -> Result<()> {
        let dir = self.versions_dir(&prompt.id);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create versions dir: {}", e))
        })?;

        let path = self.version_path(&prompt.id, prompt.version);
        let content = serde_yaml::to_string(prompt).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to serialize version: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to write version: {}", e)))?;

        Ok(())
    }

    /// List the historical snapshots of a prompt, oldest first.
    pub fn list_versions(&self, id: &str) -> Result<Vec<PromptTemplate>> {
        let dir = self.versions_dir(id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to read versions dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| AgentHubError::PromptError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                if let Ok(version) = self.load_prompt_from_file(&path) {
                    versions.push(version);
                }
            }
        }

        versions.sort_by_key(|v| v.version);
        Ok(versions)
    }

    /// Load a specific historical snapshot of a prompt.
    pub fn get_version(&self, id: &str, version: u32) -> Result<PromptTemplate> {
        let path = self.version_path(id, version);
        if !path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Prompt version {} not found for {}",
                version, id
            )));
        }

        self.load_prompt_from_file(&path)
    }

    /// Restore a prompt to a previous version. The current state is snapshotted
    /// first so no history is lost, and the version number keeps increasing.
    pub fn rollback(&self, id: &str, version: u32) -> Result<PromptTemplate> {
        let current = self.get_prompt(id)?;
        let historical = self.get_version(id, version)?;

        self.snapshot_current(&current)?;

        let mut restored = historical;
        restored.id = id.to_string();
        restored.version = current.version + 1;
        restored.updated_at = Some(Utc::now());

        self.save_prompt(&restored)?;
        Ok(restored)
    }

    /// Import historical snapshots (used by backup restore).
    pub fn import_versions(&self, id: &str, versions: &[PromptTemplate]) -> Result<()> {
        for version in versions {
            let dir = self.versions_dir(id);
            std::fs::create_dir_all(&dir).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to create versions dir: {}", e))
            })?;

            let path = self.version_path(id, version.version);
            let content = serde_yaml::to_string(version).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to serialize version: {}", e))
            })?;

            std::fs::write(&path, content).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to write version: {}", e))
            })?;
        }
        Ok(())
    }

    /// List prompts sorted by usage count (most used first).
    pub fn list_usage(&self) -> Result<Vec<PromptUsage>> {
        let prompts = self.list_prompts()?;
        let mut usage: Vec<PromptUsage> = prompts
            .iter()
            .map(|p| PromptUsage {
                id: p.id.clone(),
                name: p.name.clone(),
                usage_count: p.usage_count,
                last_used_at: p.last_used_at,
            })
            .collect();
        usage.sort_by_key(|b| std::cmp::Reverse(b.usage_count));
        Ok(usage)
    }

    // -----------------------------------------------------------------------
    // Export / import
    // -----------------------------------------------------------------------

    /// Build an export bundle for the given prompt ids (or all prompts),
    /// including each prompt's version history.
    pub fn export_prompts(&self, ids: Option<&[String]>) -> Result<PromptExportBundle> {
        let all = self.list_prompts()?;
        let selected: Vec<PromptTemplate> = match ids {
            Some(ids) => all.into_iter().filter(|p| ids.contains(&p.id)).collect(),
            None => all,
        };

        let mut versions: HashMap<String, Vec<PromptTemplate>> = HashMap::new();
        for prompt in &selected {
            let history = self.list_versions(&prompt.id)?;
            if !history.is_empty() {
                versions.insert(prompt.id.clone(), history);
            }
        }

        Ok(PromptExportBundle {
            prompts: selected,
            versions,
        })
    }

    /// Serialize an export bundle as pretty JSON.
    pub fn export_prompts_json(&self, ids: Option<&[String]>) -> Result<String> {
        let bundle = self.export_prompts(ids)?;
        serde_json::to_string_pretty(&bundle)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to serialize prompts: {}", e)))
    }

    /// Import prompts from a JSON bundle. Existing prompts are skipped unless
    /// `force` is set (then they are overwritten along with their version
    /// history). Returns the number imported / skipped.
    pub fn import_prompts(&self, json: &str, force: bool) -> Result<ImportSummary> {
        let bundle: PromptExportBundle = serde_json::from_str(json).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to parse prompt export: {}", e))
        })?;

        let mut summary = ImportSummary::default();
        for prompt in &bundle.prompts {
            if self.get_prompt(&prompt.id).is_ok() && !force {
                summary.skipped += 1;
                continue;
            }
            self.save_prompt(prompt)?;
            if let Some(versions) = bundle.versions.get(&prompt.id) {
                self.import_versions(&prompt.id, versions)?;
            }
            summary.imported += 1;
        }

        Ok(summary)
    }

    fn prompt_path(&self, id: &str) -> PathBuf {
        self.templates_dir().join(format!("{}.yaml", id))
    }

    pub fn list_prompts(&self) -> Result<Vec<PromptTemplate>> {
        let templates_dir = self.templates_dir();
        if !templates_dir.exists() {
            return Ok(Vec::new());
        }

        let mut prompts = Vec::new();
        for entry in std::fs::read_dir(&templates_dir)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to read prompts dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::PromptError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                match self.load_prompt_from_file(&path) {
                    Ok(prompt) => prompts.push(prompt),
                    Err(e) => {
                        eprintln!("Warning: Failed to load prompt at {:?}: {}", path, e);
                    }
                }
            }
        }

        prompts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(prompts)
    }

    fn load_prompt_from_file(&self, path: &Path) -> Result<PromptTemplate> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to read prompt: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to parse prompt: {}", e)))
    }

    pub fn get_prompt(&self, id: &str) -> Result<PromptTemplate> {
        let path = self.prompt_path(id);
        if !path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Prompt not found: {}",
                id
            )));
        }

        self.load_prompt_from_file(&path)
    }

    pub fn create_prompt(
        &self,
        id: &str,
        name: &str,
        description: &str,
        template: &str,
    ) -> Result<PromptTemplate> {
        let path = self.prompt_path(id);
        if path.exists() {
            return Err(AgentHubError::PromptError(format!(
                "Prompt already exists: {}",
                id
            )));
        }

        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create prompts dir: {}", e))
        })?;

        let now = Utc::now();
        let prompt = PromptTemplate {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            template: template.to_string(),
            variables: Vec::new(),
            tags: Vec::new(),
            category: None,
            version: 1,
            author: None,
            created_at: Some(now),
            updated_at: Some(now),
            usage_count: 0,
            last_used_at: None,
        };

        self.save_prompt(&prompt)?;
        Ok(prompt)
    }

    /// Create a prompt template from a session message. Variable-like tokens in
    /// the message (URLs, file paths, versions, numbers, quoted text, exotic
    /// identifiers) are replaced with `{{placeholder}}` variables so the same
    /// template can be reused with different values.
    pub fn extract_from_message(
        &self,
        id: &str,
        name: &str,
        description: &str,
        source_session_id: &str,
        source_message_index: usize,
        source_role: &str,
        message: &str,
    ) -> Result<PromptExtraction> {
        let (template, variables) = templateize_message(message);
        let prompt = self.create_prompt(id, name, description, &template)?;
        let mut prompt = prompt;
        prompt.variables = variables;
        prompt.tags = vec!["extracted".to_string()];
        prompt.category = Some("session-extracted".to_string());
        prompt.author = Some(source_session_id.to_string());
        self.save_prompt(&prompt)?;

        Ok(PromptExtraction {
            prompt,
            source_session_id: source_session_id.to_string(),
            source_message_index,
            source_role: source_role.to_string(),
        })
    }

    /// Extract a prompt from one message of a stored session.
    /// `message_index` selects which message (default: the last one).
    pub fn extract_from_session(
        &self,
        session_manager: &crate::session::SessionManager,
        session_id: &str,
        message_index: Option<usize>,
        new_id: &str,
        name: &str,
        description: &str,
    ) -> Result<PromptExtraction> {
        let session = session_manager.get_session(session_id)?;
        if session.messages.is_empty() {
            return Err(AgentHubError::PromptError(format!(
                "Session {} has no messages to extract from",
                session_id
            )));
        }
        let idx = message_index.unwrap_or(session.messages.len() - 1);
        if idx >= session.messages.len() {
            return Err(AgentHubError::PromptError(format!(
                "Message index {} out of range for session {} ({} messages)",
                idx,
                session_id,
                session.messages.len()
            )));
        }
        let message = &session.messages[idx];
        self.extract_from_message(
            new_id,
            name,
            description,
            session_id,
            idx,
            &message.role,
            &message.content,
        )
    }

    pub fn save_prompt(&self, prompt: &PromptTemplate) -> Result<()> {
        std::fs::create_dir_all(self.templates_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create prompts dir: {}", e))
        })?;

        let path = self.prompt_path(&prompt.id);
        let content = serde_yaml::to_string(prompt).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to serialize prompt: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to write prompt: {}", e)))?;

        Ok(())
    }

    pub fn update_prompt(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        template: Option<&str>,
    ) -> Result<PromptTemplate> {
        let mut prompt = self.get_prompt(id)?;

        // Snapshot the current state before modifying it.
        self.snapshot_current(&prompt)?;

        if let Some(n) = name {
            prompt.name = n.to_string();
        }
        if let Some(d) = description {
            prompt.description = d.to_string();
        }
        if let Some(t) = template {
            prompt.template = t.to_string();
        }

        prompt.version += 1;
        prompt.updated_at = Some(Utc::now());
        self.save_prompt(&prompt)?;
        Ok(prompt)
    }

    pub fn delete_prompt(&self, id: &str) -> Result<bool> {
        let path = self.prompt_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::PromptError(format!("Failed to delete prompt: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn render_prompt(&self, id: &str, vars: &HashMap<String, String>) -> Result<String> {
        let prompt = self.get_prompt(id)?;
        let rendered = Self::render(&prompt, vars);
        self.record_usage(&prompt)?;
        Ok(rendered)
    }

    /// Render with validation: required variables must be supplied (falling back
    /// to their declared defaults), otherwise an error is returned.
    pub fn render_prompt_checked(
        &self,
        id: &str,
        vars: &HashMap<String, String>,
    ) -> Result<String> {
        let prompt = self.get_prompt(id)?;

        let mut resolved = vars.clone();
        for variable in &prompt.variables {
            if !resolved.contains_key(&variable.name) {
                if let Some(default) = &variable.default {
                    resolved.insert(variable.name.clone(), default.clone());
                } else if variable.required {
                    return Err(AgentHubError::PromptError(format!(
                        "Missing required variable: {}",
                        variable.name
                    )));
                }
            }
        }

        let rendered = Self::render(&prompt, &resolved);
        self.record_usage(&prompt)?;
        Ok(rendered)
    }

    fn render(prompt: &PromptTemplate, vars: &HashMap<String, String>) -> String {
        let mut rendered = prompt.template.clone();
        for (key, value) in vars {
            rendered = rendered.replace(&format!("{{{{{}}}}}", key), value);
        }
        rendered
    }

    /// Increment the usage counters for a prompt after a render.
    fn record_usage(&self, prompt: &PromptTemplate) -> Result<()> {
        let mut updated = prompt.clone();
        updated.usage_count += 1;
        updated.last_used_at = Some(Utc::now());
        self.save_prompt(&updated)
    }

    pub fn add_tag(&self, id: &str, tag: &str) -> Result<()> {
        let mut prompt = self.get_prompt(id)?;
        if !prompt.tags.contains(&tag.to_string()) {
            prompt.tags.push(tag.to_string());
            prompt.updated_at = Some(Utc::now());
            self.save_prompt(&prompt)?;
        }
        Ok(())
    }

    pub fn remove_tag(&self, id: &str, tag: &str) -> Result<()> {
        let mut prompt = self.get_prompt(id)?;
        prompt.tags.retain(|t| t != tag);
        prompt.updated_at = Some(Utc::now());
        self.save_prompt(&prompt)?;
        Ok(())
    }

    // ---- effectiveness tracking (session outcomes) ------------------------

    fn effects_dir(&self) -> PathBuf {
        self.prompts_dir.join("effects")
    }

    fn effects_path(&self, id: &str) -> PathBuf {
        self.effects_dir().join(format!("{}.yaml", id))
    }

    fn load_outcomes(&self, id: &str) -> Result<OutcomesFile> {
        let path = self.effects_path(id);
        if !path.exists() {
            return Ok(OutcomesFile::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to read outcomes: {}", e)))?;
        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to parse outcomes: {}", e)))
    }

    fn save_outcomes(&self, id: &str, file: &OutcomesFile) -> Result<()> {
        std::fs::create_dir_all(self.effects_dir()).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to create effects dir: {}", e))
        })?;
        let content = serde_yaml::to_string(file).map_err(|e| {
            AgentHubError::PromptError(format!("Failed to serialize outcomes: {}", e))
        })?;
        std::fs::write(self.effects_path(id), content)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to write outcomes: {}", e)))
    }

    /// Record a session outcome against a prompt (append-only).
    pub fn record_outcome(
        &self,
        id: &str,
        session_id: &str,
        rating: Option<u32>,
        success: Option<bool>,
        tokens: u32,
        cost_usd: f64,
    ) -> Result<PromptOutcome> {
        self.get_prompt(id)?;
        let mut file = self.load_outcomes(id)?;
        let outcome = PromptOutcome {
            session_id: session_id.to_string(),
            rating,
            success,
            tokens,
            cost_usd,
            recorded_at: Utc::now(),
        };
        file.outcomes.push(outcome.clone());
        self.save_outcomes(id, &file)?;
        Ok(outcome)
    }

    /// Record an outcome derived from a session (rating, tokens, cost).
    pub fn record_outcome_from_session(
        &self,
        id: &str,
        session: &crate::session::Session,
    ) -> Result<PromptOutcome> {
        let (tokens, cost) = match &session.usage {
            Some(u) => (u.total_tokens, u.estimated_cost_usd),
            None => (0, 0.0),
        };
        self.record_outcome(
            id,
            &session.id,
            session.rating,
            Some(session.status == crate::session::SessionStatus::Completed),
            tokens,
            cost,
        )
    }

    /// Aggregate effectiveness statistics for one prompt.
    pub fn get_effects(&self, id: &str) -> Result<PromptEffects> {
        self.get_prompt(id)?;
        Ok(self.aggregate_effects(id, &self.load_outcomes(id)?))
    }

    /// Aggregate effectiveness for every prompt that has recorded outcomes.
    pub fn list_effects(&self) -> Result<Vec<PromptEffects>> {
        let dir = self.effects_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut effects = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to read effects dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::PromptError(format!("Failed to read entry: {}", e)))?;
            if entry
                .path()
                .extension()
                .is_some_and(|e| e == "yaml" || e == "yml")
            {
                if let Some(stem) = entry.path().file_stem() {
                    let id = stem.to_string_lossy().to_string();
                    if let Ok(file) = self.load_outcomes(&id) {
                        effects.push(self.aggregate_effects(&id, &file));
                    }
                }
            }
        }
        effects.sort_by_key(|e| std::cmp::Reverse(e.uses));
        Ok(effects)
    }

    /// Delete all recorded outcomes for a prompt.
    pub fn clear_effects(&self, id: &str) -> Result<bool> {
        let path = self.effects_path(id);
        if !path.exists() {
            return Ok(false);
        }
        std::fs::remove_file(&path)
            .map_err(|e| AgentHubError::PromptError(format!("Failed to clear effects: {}", e)))?;
        Ok(true)
    }

    fn aggregate_effects(&self, id: &str, file: &OutcomesFile) -> PromptEffects {
        let n = file.outcomes.len();
        let rated: Vec<u32> = file.outcomes.iter().filter_map(|o| o.rating).collect();
        let known_success: Vec<bool> = file.outcomes.iter().filter_map(|o| o.success).collect();
        PromptEffects {
            prompt_id: id.to_string(),
            uses: n,
            avg_rating: if rated.is_empty() {
                None
            } else {
                Some(rated.iter().map(|r| *r as f64).sum::<f64>() / rated.len() as f64)
            },
            success_rate: if known_success.is_empty() {
                None
            } else {
                Some(
                    known_success.iter().filter(|s| **s).count() as f64
                        / known_success.len() as f64,
                )
            },
            total_tokens: file.outcomes.iter().map(|o| o.tokens).sum(),
            total_cost_usd: file.outcomes.iter().map(|o| o.cost_usd).sum(),
            last_used: file.outcomes.iter().map(|o| o.recorded_at).max(),
        }
    }
}

// ---------------------------------------------------------------------------
// Message templateization: replace variable-like tokens with {{placeholders}}.
// ---------------------------------------------------------------------------

fn templateize_message(message: &str) -> (String, Vec<PromptVariable>) {
    let mut template = message.to_string();
    let mut var_names: Vec<String> = Vec::new();

    let mut add_var = |name: &str| {
        if !var_names.iter().any(|n| n == name) {
            var_names.push(name.to_string());
        }
    };

    // URLs first (they also contain '/').
    template = replace_urls(&template);
    if template.contains("{{url}}") {
        add_var("url");
    }

    // File paths: whitespace-delimited tokens containing '/' or '\'.
    template = replace_paths(&template);
    if template.contains("{{path}}") {
        add_var("path");
    }

    // Semantic versions like 2.1.0 / v2.1 / 1.2.3-beta.
    template = replace_versions(&template);
    if template.contains("{{version}}") {
        add_var("version");
    }

    // Quoted text.
    template = replace_quoted(&template);
    if template.contains("{{quoted_text}}") {
        add_var("quoted_text");
    }

    // Bare numbers.
    template = replace_numbers(&template);
    if template.contains("{{number}}") {
        add_var("number");
    }

    // Exotic identifiers (digits / underscores / hyphens / mixed case).
    template = replace_identifiers(&template);
    if template.contains("{{identifier}}") {
        add_var("identifier");
    }

    let variables: Vec<PromptVariable> = var_names
        .into_iter()
        .map(|name| PromptVariable {
            name: name.clone(),
            var_type: "string".to_string(),
            required: false,
            description: Some(match name.as_str() {
                "url" => "URL to use".to_string(),
                "path" => "File path to use".to_string(),
                "version" => "Version number".to_string(),
                "number" => "Numeric value".to_string(),
                "quoted_text" => "Quoted text value".to_string(),
                _ => "Value".to_string(),
            }),
            default: None,
        })
        .collect();

    (template, variables)
}

/// Replace `http(s)://...` runs (up to whitespace) with `{{url}}`.
fn replace_urls(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(pos) = rest.find("://") {
        let scheme_start = rest[..pos]
            .rfind(|c: char| !c.is_ascii_alphanumeric())
            .map_or(0, |i| i + 1);
        let start = scheme_start.min(pos);
        let after = &rest[pos + 3..];
        let end = after
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(after.len());
        out.push_str(&rest[..start]);
        out.push_str("{{url}}");
        rest = &rest[pos + 3 + end..];
    }
    out.push_str(rest);
    out
}

/// Replace whitespace-delimited path-like tokens with `{{path}}`.
fn replace_paths(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(|c: char| !c.is_whitespace()) {
        let end = rest[start..]
            .find(char::is_whitespace)
            .map(|i| start + i)
            .unwrap_or(rest.len());
        let token = &rest[start..end];
        out.push_str(&rest[..start]);
        if is_path_token(token) {
            out.push_str("{{path}}");
        } else {
            out.push_str(token);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn is_path_token(token: &str) -> bool {
    if token.contains("://") || token.is_empty() || token.len() < 3 {
        return false;
    }
    let has_sep = token.contains('/') || token.contains('\\');
    if !has_sep {
        return false;
    }
    // A trailing slash (e.g. "and/") or punctuation-only segment is not a path.
    let trimmed = token.trim_end_matches([',', '.', ';', ')']);
    if trimmed.is_empty() {
        return false;
    }
    // Windows drive: C:\...
    if token.len() >= 3 && token.as_bytes()[1] == b':' {
        return true;
    }
    // Require at least two segments with a non-trivial last segment.
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or("");
    !last.is_empty() && trimmed.matches(['/', '\\']).count() >= 1
}

/// Replace `v?N.N(.N)?` version runs with `{{version}}`.
fn replace_versions(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        // optional leading 'v'
        let mut j = i;
        if bytes.get(j) == Some(&'v') || bytes.get(j) == Some(&'V') {
            j += 1;
        }
        if !is_digit_at(&bytes, j) {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // N.N or N.N.N
        let mut k = j;
        while is_digit_at(&bytes, k) {
            k += 1;
        }
        if k < bytes.len() && bytes[k] == '.' && is_digit_at(&bytes, k + 1) {
            k += 1;
            while is_digit_at(&bytes, k) {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == '.' && is_digit_at(&bytes, k + 1) {
                k += 1;
                while is_digit_at(&bytes, k) {
                    k += 1;
                }
            }
            // boundary: previous char not alphanumeric (or start)
            let prev_ok = start == 0 || !bytes[start - 1].is_alphanumeric();
            let next_ok = k >= bytes.len() || !bytes[k].is_alphanumeric();
            if prev_ok && next_ok {
                out.push_str("{{version}}");
                i = k;
                continue;
            }
        }
        // Not a version; keep the leading 'v' if present
        for c in &bytes[start..j] {
            out.push(*c);
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn is_digit_at(bytes: &[char], i: usize) -> bool {
    bytes.get(i).is_some_and(|c| c.is_ascii_digit())
}

/// Replace `"..."` quoted runs (content kept as placeholder) with `"{{quoted_text}}"`.
fn replace_quoted(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        out.push_str(&rest[..=open]);
        let after = &rest[open + 1..];
        match after.find('"') {
            Some(close) => {
                out.push_str("{{quoted_text}}");
                out.push('"');
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Replace standalone numbers with `{{number}}`.
fn replace_numbers(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let prev_ok = start == 0 || !bytes[start - 1].is_alphanumeric();
            let next_ok = i >= bytes.len() || !bytes[i].is_alphanumeric();
            if prev_ok && next_ok {
                out.push_str("{{number}}");
                continue;
            }
            for c in &bytes[start..i] {
                out.push(*c);
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// Replace identifier-like tokens (letters/digits/_/- with digits or mixed
/// case, length >= 3, not already inside a placeholder) with `{{identifier}}`.
fn replace_identifiers(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        // Copy existing {{...}} placeholders verbatim.
        if bytes[i] == '{' && bytes.get(i + 1) == Some(&'{') {
            let mut j = i + 2;
            while j < bytes.len() && !(bytes[j] == '}' && bytes.get(j + 1) == Some(&'}')) {
                j += 1;
            }
            let end = (j + 2).min(bytes.len());
            for c in &bytes[i..end] {
                out.push(*c);
            }
            i = end;
            continue;
        }
        if bytes[i].is_ascii_alphabetic() && is_identifier_char(bytes[i]) {
            let start = i;
            let mut has_digit = false;
            let mut has_upper = false;
            let mut has_sep = false;
            while i < bytes.len() && is_identifier_char(bytes[i]) {
                if bytes[i].is_ascii_digit() {
                    has_digit = true;
                }
                if bytes[i].is_uppercase() {
                    has_upper = true;
                }
                if bytes[i] == '_' || bytes[i] == '-' {
                    has_sep = true;
                }
                i += 1;
            }
            let token: String = bytes[start..i].iter().collect();
            // Not an identifier if it's a plain lowercase word with no digit/sep.
            let exotic = has_digit || has_sep || has_upper;
            let prev_ok = start == 0 || !is_identifier_char(bytes[start - 1]);
            let next_ok = i >= bytes.len() || !is_identifier_char(bytes[i]);
            if token.len() >= 3 && exotic && prev_ok && next_ok {
                out.push_str("{{identifier}}");
                continue;
            }
            out.push_str(&token);
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (PromptManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = PromptManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_create_and_get_prompt() {
        let (manager, _temp) = create_test_manager();

        let prompt = manager
            .create_prompt("test", "Test Prompt", "A test prompt", "Hello {{name}}!")
            .unwrap();

        assert_eq!(prompt.id, "test");
        assert_eq!(prompt.name, "Test Prompt");

        let loaded = manager.get_prompt("test").unwrap();
        assert_eq!(loaded.id, "test");
    }

    #[test]
    fn test_list_prompts() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("p1", "Prompt 1", "First", "Template 1")
            .unwrap();
        manager
            .create_prompt("p2", "Prompt 2", "Second", "Template 2")
            .unwrap();

        let prompts = manager.list_prompts().unwrap();
        assert_eq!(prompts.len(), 2);
    }

    #[test]
    fn test_render_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt(
                "greeting",
                "Greeting",
                "A greeting",
                "Hello {{name}}, welcome to {{place}}!",
            )
            .unwrap();

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("place".to_string(), "Wonderland".to_string());

        let rendered = manager.render_prompt("greeting", &vars).unwrap();
        assert_eq!(rendered, "Hello Alice, welcome to Wonderland!");
    }

    #[test]
    fn test_update_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Template")
            .unwrap();

        let updated = manager
            .update_prompt("test", Some("New Name"), None, None)
            .unwrap();
        assert_eq!(updated.name, "New Name");
    }

    #[test]
    fn test_delete_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Template")
            .unwrap();

        let deleted = manager.delete_prompt("test").unwrap();
        assert!(deleted);

        let result = manager.get_prompt("test");
        assert!(result.is_err());
    }

    #[test]
    fn test_tags() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Template")
            .unwrap();

        manager.add_tag("test", "review").unwrap();
        manager.add_tag("test", "code").unwrap();

        let prompt = manager.get_prompt("test").unwrap();
        assert_eq!(prompt.tags.len(), 2);

        manager.remove_tag("test", "review").unwrap();
        let prompt = manager.get_prompt("test").unwrap();
        assert_eq!(prompt.tags.len(), 1);
    }

    // ---- Version control ----

    #[test]
    fn test_update_prompt_creates_versions() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "v1 content")
            .unwrap();

        let updated = manager
            .update_prompt("test", None, None, Some("v2 content"))
            .unwrap();
        assert_eq!(updated.version, 2);
        assert_eq!(updated.template, "v2 content");

        // History contains the original v1
        let versions = manager.list_versions("test").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[0].template, "v1 content");

        let v1 = manager.get_version("test", 1).unwrap();
        assert_eq!(v1.template, "v1 content");
    }

    #[test]
    fn test_rollback_prompt() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "v1 content")
            .unwrap();
        manager
            .update_prompt("test", None, None, Some("v2 content"))
            .unwrap();
        manager
            .update_prompt("test", None, None, Some("v3 content"))
            .unwrap();

        let rolled = manager.rollback("test", 1).unwrap();
        assert_eq!(rolled.template, "v1 content");
        assert_eq!(rolled.version, 4);

        // The v3 state was snapshotted before rollback
        let versions = manager.list_versions("test").unwrap();
        assert_eq!(versions.len(), 3);
        assert!(versions
            .iter()
            .any(|v| v.version == 3 && v.template == "v3 content"));
    }

    #[test]
    fn test_rollback_missing_version() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "v1 content")
            .unwrap();

        assert!(manager.rollback("test", 99).is_err());
    }

    #[test]
    fn test_import_versions() {
        let (manager, _temp) = create_test_manager();

        let snapshots = vec![PromptTemplate {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Desc".to_string(),
            template: "old content".to_string(),
            variables: Vec::new(),
            tags: Vec::new(),
            category: None,
            version: 1,
            author: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            usage_count: 0,
            last_used_at: None,
        }];

        manager.import_versions("test", &snapshots).unwrap();
        let versions = manager.list_versions("test").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].template, "old content");
    }

    // ---- Usage stats ----

    #[test]
    fn test_render_records_usage() {
        let (manager, _temp) = create_test_manager();

        manager
            .create_prompt("test", "Test", "Desc", "Hello {{name}}")
            .unwrap();

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());

        manager.render_prompt("test", &vars).unwrap();
        manager.render_prompt("test", &vars).unwrap();

        let prompt = manager.get_prompt("test").unwrap();
        assert_eq!(prompt.usage_count, 2);
        assert!(prompt.last_used_at.is_some());

        let usage = manager.list_usage().unwrap();
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].usage_count, 2);
    }

    // ---- Variable validation ----

    #[test]
    fn test_render_prompt_checked() {
        let (manager, _temp) = create_test_manager();

        let prompt = manager
            .create_prompt("checked", "Checked", "Desc", "Hi {{name}} in {{city}}")
            .unwrap();
        // Attach variable declarations directly for validation
        let mut p = prompt;
        p.variables = vec![
            PromptVariable {
                name: "name".to_string(),
                var_type: "string".to_string(),
                required: true,
                description: None,
                default: None,
            },
            PromptVariable {
                name: "city".to_string(),
                var_type: "string".to_string(),
                required: false,
                description: None,
                default: Some("Shanghai".to_string()),
            },
        ];
        manager.save_prompt(&p).unwrap();

        // Missing required variable -> error
        let mut vars = HashMap::new();
        assert!(manager.render_prompt_checked("checked", &vars).is_err());

        // Required present, optional uses default
        vars.insert("name".to_string(), "Alice".to_string());
        let rendered = manager.render_prompt_checked("checked", &vars).unwrap();
        assert_eq!(rendered, "Hi Alice in Shanghai");

        // Explicit override wins over default
        vars.insert("city".to_string(), "Beijing".to_string());
        let rendered = manager.render_prompt_checked("checked", &vars).unwrap();
        assert_eq!(rendered, "Hi Alice in Beijing");
    }

    // ---- Export / import ----

    #[test]
    fn test_export_import_prompts_roundtrip() {
        let (manager, _temp) = create_test_manager();

        manager.create_prompt("a", "A", "d", "tpl a").unwrap();
        manager
            .update_prompt("a", None, None, Some("tpl a v2"))
            .unwrap();
        manager.create_prompt("b", "B", "d", "tpl b").unwrap();

        let json = manager.export_prompts_json(None).unwrap();
        assert!(json.contains("tpl a v2"));

        // Restore into a fresh manager
        let (target, _temp2) = create_test_manager();
        let summary = target.import_prompts(&json, false).unwrap();
        assert_eq!(summary.imported, 2);
        assert_eq!(summary.skipped, 0);

        let a = target.get_prompt("a").unwrap();
        assert_eq!(a.template, "tpl a v2");
        assert_eq!(target.list_versions("a").unwrap().len(), 1);
    }

    #[test]
    fn test_import_prompts_skips_and_forces() {
        let (manager, _temp) = create_test_manager();
        manager.create_prompt("a", "A", "d", "original").unwrap();

        // Export, then modify locally
        let json = manager
            .export_prompts_json(Some(&["a".to_string()]))
            .unwrap();
        manager
            .update_prompt("a", None, None, Some("local change"))
            .unwrap();

        // Without force: skipped
        let summary = manager.import_prompts(&json, false).unwrap();
        assert_eq!(summary.imported, 0);
        assert_eq!(summary.skipped, 1);
        assert_eq!(manager.get_prompt("a").unwrap().template, "local change");

        // With force: overwritten, history kept
        let summary = manager.import_prompts(&json, true).unwrap();
        assert_eq!(summary.imported, 1);
        assert_eq!(manager.get_prompt("a").unwrap().template, "original");
        assert!(!manager.list_versions("a").unwrap().is_empty());
    }

    #[test]
    fn test_export_selected_ids() {
        let (manager, _temp) = create_test_manager();
        manager.create_prompt("a", "A", "d", "t").unwrap();
        manager.create_prompt("b", "B", "d", "t").unwrap();

        let bundle = manager.export_prompts(Some(&["a".to_string()])).unwrap();
        assert_eq!(bundle.prompts.len(), 1);
        assert_eq!(bundle.prompts[0].id, "a");
    }

    // -------------------------------------------------------------------
    // Session extraction
    // -------------------------------------------------------------------

    #[test]
    fn test_templateize_message_placeholders() {
        let (template, vars) = templateize_message(
            "Deploy https://example.com/app to /var/www/release and bump to v2.1.0 with the \"blue-green\" strategy",
        );
        assert!(template.contains("{{url}}"));
        assert!(template.contains("{{path}}"));
        assert!(template.contains("{{version}}"));
        assert!(template.contains("{{quoted_text}}"));
        // variable list contains deduplicated names
        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"url"));
        assert!(names.contains(&"path"));
        assert!(names.contains(&"version"));
    }

    #[test]
    fn test_templateize_message_identifier() {
        let (template, vars) =
            templateize_message("Run release_2024_build now and check the claude-sonnet-4 results");
        assert!(template.contains("{{identifier}}"));
        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"identifier"));
    }

    #[test]
    fn test_extract_from_message_creates_prompt() {
        let (manager, _temp) = create_test_manager();
        let extraction = manager
            .extract_from_message(
                "deploy-prompt",
                "Deploy",
                "From session",
                "ses_123",
                3,
                "user",
                "Deploy https://example.com to /srv/app",
            )
            .unwrap();
        assert_eq!(extraction.source_session_id, "ses_123");
        assert_eq!(extraction.source_role, "user");
        assert_eq!(extraction.source_message_index, 3);
        assert!(extraction.prompt.template.contains("{{url}}"));
        assert_eq!(extraction.prompt.tags, vec!["extracted"]);

        // Saved and readable
        let prompt = manager.get_prompt("deploy-prompt").unwrap();
        assert!(!prompt.variables.is_empty());
    }

    #[test]
    fn test_extract_from_session() {
        use crate::session::SessionManager;
        let (manager, _temp) = create_test_manager();
        let temp = tempfile::tempdir().unwrap();
        let session_manager = SessionManager::new(temp.path().join("sessions"));

        let session = session_manager
            .create_session("claude-code", "deploy")
            .unwrap();
        session_manager
            .add_message(
                &session.id,
                "user",
                "Deploy /srv/app with https://example.com",
            )
            .unwrap();

        let extraction = manager
            .extract_from_session(&session_manager, &session.id, None, "from-ses", "From", "d")
            .unwrap();
        assert_eq!(extraction.source_message_index, 0);
        assert!(extraction.prompt.template.contains("{{path}}"));
        assert!(extraction.prompt.template.contains("{{url}}"));

        // Out of range index errors
        assert!(manager
            .extract_from_session(&session_manager, &session.id, Some(5), "x", "X", "d")
            .is_err());
    }

    // ---- effectiveness tracking ----

    #[test]
    fn test_record_and_aggregate_effects() {
        let (manager, _temp) = create_test_manager();
        manager
            .create_prompt("review", "Review", "d", "review {{code}}")
            .unwrap();

        manager
            .record_outcome("review", "ses_1", Some(5), Some(true), 1200, 0.02)
            .unwrap();
        manager
            .record_outcome("review", "ses_2", Some(3), Some(false), 800, 0.01)
            .unwrap();
        manager
            .record_outcome("review", "ses_3", None, Some(true), 400, 0.005)
            .unwrap();

        let effects = manager.get_effects("review").unwrap();
        assert_eq!(effects.uses, 3);
        assert_eq!(effects.avg_rating.unwrap(), 4.0);
        assert_eq!(effects.success_rate.unwrap(), 2.0 / 3.0);
        assert_eq!(effects.total_tokens, 2400);
        assert!((effects.total_cost_usd - 0.035).abs() < 1e-9);
        assert!(effects.last_used.is_some());

        // Unknown prompt rejected
        assert!(manager
            .record_outcome("nope", "s", None, None, 0, 0.0)
            .is_err());
        assert!(manager.get_effects("nope").is_err());

        // Listing includes only prompts with outcomes
        manager
            .create_prompt("other", "Other", "d", "other")
            .unwrap();
        let all = manager.list_effects().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].prompt_id, "review");

        // Clear
        assert!(manager.clear_effects("review").unwrap());
        assert!(!manager.clear_effects("review").unwrap());
        assert_eq!(manager.get_effects("review").unwrap().uses, 0);
    }

    #[test]
    fn test_record_outcome_from_session() {
        use crate::session::{PricingTable, SessionManager};
        let (manager, _temp) = create_test_manager();
        manager
            .create_prompt("code", "Code", "d", "code {{lang}}")
            .unwrap();

        let temp = tempfile::tempdir().unwrap();
        let session_manager = SessionManager::new(temp.path().join("sessions"));
        let session = session_manager.create_session("S", "codex").unwrap();
        session_manager
            .set_model(&session.id, "gpt-4o-mini")
            .unwrap();
        session_manager
            .record_usage(&session.id, 100_000, 50_000, &PricingTable::builtin())
            .unwrap();
        session_manager
            .update_status(&session.id, crate::session::SessionStatus::Completed)
            .unwrap();
        // Rate the session via YAML reload + rating field
        let mut loaded = session_manager.get_session(&session.id).unwrap();
        loaded.rating = Some(4);
        session_manager.save_session(&loaded).unwrap();

        let loaded = session_manager.get_session(&session.id).unwrap();
        let outcome = manager
            .record_outcome_from_session("code", &loaded)
            .unwrap();
        assert_eq!(outcome.rating, Some(4));
        assert_eq!(outcome.success, Some(true));
        assert!(outcome.tokens > 0);
        assert!(outcome.cost_usd > 0.0);
    }
}
