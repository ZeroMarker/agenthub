use agenthub_core::{
    Agent, AuditManager, AuditQuery, BackupManager, Catalog, ConfigManager, ConfigValue,
    DiagnosticManager, ImportSummary, Installer, MemoryManager, MemoryScope, Monitor,
    OverviewReport, Platform, PromptManager, RealCommandRunner, SessionManager, SkillManager,
};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Clap CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "agenthub", about = "AI coding agent management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all agents
    List {
        /// Filter by type: cli or desktop
        #[arg(long)]
        kind: Option<String>,
    },
    /// Search agents by query
    Search {
        /// Search query
        query: String,
        /// Filter by type: cli or desktop
        #[arg(long)]
        kind: Option<String>,
    },
    /// Show agent details
    Info {
        /// Agent name or ID
        name: String,
    },
    /// Install an agent
    Install {
        /// Agent name or ID
        name: String,
        /// Preview installation without executing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Uninstall an agent
    Uninstall {
        /// Agent name or ID
        name: String,
        /// Preview uninstallation without executing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Run environment diagnostics
    Doctor,
    /// Show a global overview of the workspace (agents, configs, sessions, ...)
    Status {
        /// Also print a daily trend for the last N days
        #[arg(long)]
        trend: Option<usize>,
    },
    /// Query the audit log
    Audit {
        /// Filter by action substring, e.g. install, config.set
        #[arg(long)]
        action: Option<String>,
        /// Filter by target substring, e.g. an agent id
        #[arg(long)]
        target: Option<String>,
        /// Only show events from the last N days
        #[arg(long)]
        last_days: Option<i64>,
        /// Maximum number of events to show (most recent first)
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Back up all configs, prompts, sessions, memories and audit events
    Backup {
        /// Output file path; defaults to agenthub-backup-<timestamp>.json
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Restore from a backup file
    Restore {
        /// Path to the backup file
        file: PathBuf,
    },
    /// Manage reusable config templates
    #[command(name = "config-template", subcommand)]
    ConfigTemplate(ConfigTemplateCmd),
    /// Export/import prompt templates
    #[command(subcommand)]
    Prompt(PromptArgs),
    /// Export/import memories
    #[command(subcommand)]
    Memory(MemoryArgs),
    /// Session budget & context handoff
    #[command(subcommand)]
    Session(SessionArgs),
    /// Skill version compatibility checks
    #[command(subcommand)]
    Skill(SkillArgs),
    /// Run the monitoring pass (diagnostics + budget + compatibility)
    Monitor,
}

#[derive(Subcommand)]
enum ConfigTemplateCmd {
    /// List config template ids
    List,
    /// Show a template's contents
    Show { id: String },
    /// Create a template
    Create {
        id: String,
        name: String,
        description: String,
        /// Setting as key=value (repeatable)
        #[arg(long = "set")]
        sets: Vec<String>,
        /// Environment variable name to reserve (repeatable)
        #[arg(long = "env")]
        envs: Vec<String>,
        /// Secret key name to reserve (repeatable)
        #[arg(long = "secret")]
        secrets: Vec<String>,
    },
    /// Delete a template
    Delete { id: String },
    /// Apply a template to an agent config (creating it if needed)
    Apply { agent: String, template: String },
}

#[derive(Subcommand)]
enum PromptArgs {
    /// Export a prompt template (with version history) as JSON
    Export {
        id: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Export all prompt templates as JSON
    ExportAll {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Import prompt templates from a JSON export file
    Import {
        file: PathBuf,
        /// Overwrite existing templates (default: skip them)
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum MemoryArgs {
    /// Export memories as JSON
    Export {
        /// Restrict to a scope: global | project | session
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Import memories from a JSON export file
    Import {
        file: PathBuf,
        /// Skip entries whose path already exists
        #[arg(long)]
        merge: bool,
    },
}

#[derive(Subcommand)]
enum SessionArgs {
    /// Show or set cost budget limits
    Budget {
        #[command(subcommand)]
        cmd: BudgetCmd,
    },
    /// Fork a session, carrying its context into a new session (optionally for
    /// another agent)
    Fork {
        id: String,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        title: Option<String>,
    },
}

#[derive(Subcommand)]
enum BudgetCmd {
    /// Show current spending and budget limits
    Show,
    /// Set daily/monthly budget limits (USD)
    Set {
        #[arg(long)]
        daily: Option<f64>,
        #[arg(long)]
        monthly: Option<f64>,
    },
}

#[derive(Subcommand)]
enum SkillArgs {
    /// Check version compatibility against the running AgentHub
    CheckCompat {
        /// Skill name, or "*" for all
        #[arg(default_value = "*")]
        name: String,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn get_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Linux
    }
}

pub fn load_catalog() -> Result<Catalog, String> {
    let paths = [
        PathBuf::from("agents.json"),
        PathBuf::from("../agents.json"),
    ];

    for path in &paths {
        if path.exists() {
            return Catalog::from_file(path).map_err(|e| e.to_string());
        }
    }

    Err("Could not find agents.json".to_string())
}

/// Default data directory, matching the Tauri app layout.
pub fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agenthub")
}

pub fn default_backup_path() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    PathBuf::from(format!("agenthub-backup-{}.json", stamp))
}

// ---------------------------------------------------------------------------
// Command implementations (return strings for testability)
// ---------------------------------------------------------------------------

pub fn cmd_list(kind: Option<String>, catalog: &Catalog) -> String {
    let agents = match kind.as_deref() {
        Some("cli") => catalog.filter_by_kind(agenthub_core::AgentKind::CLI),
        Some("desktop") => catalog.filter_by_kind(agenthub_core::AgentKind::Desktop),
        _ => catalog.agents().iter().collect(),
    };

    if agents.is_empty() {
        return "No agents found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<20} {:<25} {:<8} {:<20} {:<10}\n",
        "ID", "Name", "Type", "Provider", "Status"
    ));
    out.push_str(&format!("{}\n", "-".repeat(85)));

    for agent in &agents {
        out.push_str(&format!(
            "{:<20} {:<25} {:<8} {:<20} {:<10}\n",
            agent.id,
            agent.name,
            format!("{:?}", agent.kind),
            agent.provider,
            format!("{:?}", agent.status)
        ));
    }

    let (cli, desktop) = catalog.count_by_kind();
    out.push_str(&format!(
        "\nTotal: {} agents ({} CLI, {} Desktop)",
        agents.len(),
        cli,
        desktop
    ));
    out
}

pub fn cmd_search(query: &str, kind: Option<String>, catalog: &Catalog) -> String {
    let results = match kind.as_deref() {
        Some("cli") => catalog
            .search(query)
            .into_iter()
            .filter(|a| a.kind == agenthub_core::AgentKind::CLI)
            .collect::<Vec<_>>(),
        Some("desktop") => catalog
            .search(query)
            .into_iter()
            .filter(|a| a.kind == agenthub_core::AgentKind::Desktop)
            .collect::<Vec<_>>(),
        _ => catalog.search(query),
    };

    if results.is_empty() {
        return format!("No agents found matching '{}'", query);
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<20} {:<25} {:<8} {:<20}\n",
        "ID", "Name", "Type", "Provider"
    ));
    out.push_str(&format!("{}\n", "-".repeat(75)));

    for agent in &results {
        out.push_str(&format!(
            "{:<20} {:<25} {:<8} {:<20}\n",
            agent.id,
            agent.name,
            format!("{:?}", agent.kind),
            agent.provider
        ));
    }
    out.push_str(&format!("\nFound {} agent(s)", results.len()));
    out
}

pub fn cmd_info(name: &str, catalog: &Catalog) -> String {
    let agent = match catalog.find_by_name(name) {
        Some(a) => a,
        None => return format!("Agent '{}' not found", name),
    };

    let mut out = String::new();
    out.push_str(&format!("Name:        {}\n", agent.name));
    out.push_str(&format!("ID:          {}\n", agent.id));
    out.push_str(&format!("Type:        {:?}\n", agent.kind));
    out.push_str(&format!("Provider:    {}\n", agent.provider));
    out.push_str(&format!("Description: {}\n", agent.description));
    out.push_str(&format!("Homepage:    {}\n", agent.homepage));
    out.push_str(&format!("Status:      {:?}\n", agent.status));

    if let Some(date) = agent.catalog_verified_at {
        out.push_str(&format!("Catalog Verified: {}\n", date));
    }
    if let Some(date) = agent.installer_verified_at {
        out.push_str(&format!("Installer Verified: {}\n", date));
    }

    out.push_str("\nInstallers:\n");
    let platform = get_platform();
    for (p, config) in &agent.installers {
        let current = if *p == platform { " (current)" } else { "" };
        let pkg = config.package.as_deref().unwrap_or("N/A");
        out.push_str(&format!(
            "  {:?}{}: {:?} {}\n",
            p, current, config.manager, pkg
        ));
    }
    out
}

pub fn cmd_install(_name: &str, dry_run: bool, agent: &Agent, platform: Platform) -> String {
    let runner = RealCommandRunner::new(platform);
    let installer = Installer::new(platform, Box::new(runner));

    let preview = match installer.get_command_preview(agent, false) {
        Some(cmd) => format!(
            "Will execute: {}\nDescription:  {}",
            cmd.command, cmd.description
        ),
        None => return format!("No installer available for {} on this platform", agent.name),
    };

    let result = match installer.execute_install(agent, dry_run, None) {
        Ok(r) => r,
        Err(e) => return format!("Installation failed: {}", e),
    };

    if dry_run {
        format!("{}\n\nDry run — command was not executed", preview)
    } else if result.success {
        format!(
            "{}\n\n✅ {} installed successfully ({})",
            preview, agent.name, result.duration_ms
        )
    } else {
        let mut out = format!("{}\n\n❌ {} installation failed", preview, agent.name);
        if !result.stderr.is_empty() {
            out.push_str(&format!("\n{}", result.stderr));
        }
        out
    }
}

pub fn cmd_uninstall(_name: &str, dry_run: bool, agent: &Agent, platform: Platform) -> String {
    let runner = RealCommandRunner::new(platform);
    let installer = Installer::new(platform, Box::new(runner));

    let preview = match installer.get_command_preview(agent, true) {
        Some(cmd) => format!(
            "Will execute: {}\nDescription:  {}",
            cmd.command, cmd.description
        ),
        None => {
            return format!(
                "No uninstaller available for {} on this platform",
                agent.name
            )
        }
    };

    let result = match installer.execute_uninstall(agent, dry_run, None) {
        Ok(r) => r,
        Err(e) => return format!("Uninstallation failed: {}", e),
    };

    if dry_run {
        format!("{}\n\nDry run — command was not executed", preview)
    } else if result.success {
        format!(
            "{}\n\n✅ {} uninstalled successfully ({})",
            preview, agent.name, result.duration_ms
        )
    } else {
        let mut out = format!("{}\n\n❌ {} uninstallation failed", preview, agent.name);
        if !result.stderr.is_empty() {
            out.push_str(&format!("\n{}", result.stderr));
        }
        out
    }
}

pub fn cmd_doctor() -> String {
    let mut manager = DiagnosticManager::new();
    let report = manager.run_all_checks();

    format!(
        "{}\n\nSummary: {} passed, {} warnings, {} failed",
        DiagnosticManager::format_report(&report),
        report.summary.passed,
        report.summary.warnings,
        report.summary.failed
    )
}

pub fn cmd_status(base_dir: &Path, catalog: &Catalog, trend_days: Option<usize>) -> String {
    let report = OverviewReport::new(base_dir.to_path_buf(), get_platform());
    let overview = match report.overview(catalog) {
        Ok(o) => o,
        Err(e) => return format!("Error: {}", e),
    };

    let c = &overview.catalog;
    let mut out = String::new();
    out.push_str("AgentHub 状态概览\n");
    out.push_str("==================\n");
    out.push_str(&format!("平台:       {}\n", overview.platform));
    out.push_str(&format!("版本:       {}\n", overview.agenthub_version));
    out.push_str(&format!(
        "生成时间:   {}\n",
        overview.generated_at.to_rfc3339()
    ));
    out.push_str(&format!(
        "目录:       {} agents ({} CLI, {} Desktop) — verified {}, community {}, manual {}, deprecated {}\n",
        c.total, c.cli, c.desktop, c.verified, c.community, c.manual, c.deprecated
    ));
    out.push_str(&format!("已安装:     {}\n", overview.installed_agents));
    out.push_str(&format!("配置:       {}\n", overview.configs));
    out.push_str(&format!("提示词:     {}\n", overview.prompts));
    let s = &overview.sessions;
    out.push_str(&format!(
        "会话:       {} (active {}, completed {}, failed {}) — tokens {}, ${:.4}\n",
        s.total, s.active, s.completed, s.failed, s.total_tokens, s.total_cost
    ));
    let m = &overview.memories;
    out.push_str(&format!(
        "记忆:       {} (global {}, project {}, session {}, decayed {})\n",
        m.total, m.global, m.project, m.session, m.decayed
    ));
    out.push_str(&format!(
        "技能:       {} (enabled {})\n",
        overview.skills_total, overview.skills_enabled
    ));
    out.push_str(&format!("审计事件:   {}\n", overview.audit_events));

    if let Some(days) = trend_days {
        out.push_str(&format!("\n趋势 (最近 {} 天):\n", days));
        out.push_str(&format!("{}\n", "-".repeat(80)));
        out.push_str(&format!(
            "{:<12} {:>8} {:>10} {:>12} {:>10} {:>8}\n",
            "日期", "会话", "完成", "Tokens", "成本($)", "审计"
        ));
        match report.trend(days) {
            Ok(points) => {
                for p in &points {
                    out.push_str(&format!(
                        "{:<12} {:>8} {:>10} {:>12} {:>10.4} {:>8}\n",
                        p.date,
                        p.sessions_started,
                        p.sessions_completed,
                        p.tokens,
                        p.cost_usd,
                        p.audit_events
                    ));
                }
            }
            Err(e) => out.push_str(&format!("Error building trend: {}\n", e)),
        }
    }

    out
}

pub fn cmd_audit(
    base_dir: &Path,
    action: Option<&str>,
    target: Option<&str>,
    last_days: Option<i64>,
    limit: usize,
) -> String {
    let manager = AuditManager::new(base_dir.join("audit"));
    let since = last_days.map(|d| Utc::now() - Duration::days(d));
    let query = AuditQuery {
        action: action.map(|s| s.to_string()),
        target: target.map(|s| s.to_string()),
        since,
        limit: Some(limit),
        ..Default::default()
    };

    let events = match manager.query(&query) {
        Ok(events) => events,
        Err(e) => return format!("Error: {}", e),
    };

    if events.is_empty() {
        return "No audit events found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<24} {:<8} {:<18} {:<32} {}\n",
        "Timestamp", "Success", "Action", "Target", "Details"
    ));
    out.push_str(&format!("{}\n", "-".repeat(120)));
    for event in &events {
        let success = if event.success { "ok" } else { "FAIL" };
        let details = event.details.as_deref().unwrap_or("");
        out.push_str(&format!(
            "{:<24} {:<8} {:<18} {:<32} {}\n",
            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
            success,
            event.action,
            event.target,
            details
        ));
    }
    out.push_str(&format!("\n{} event(s)", events.len()));
    out
}

pub fn cmd_backup(base_dir: &Path, output: Option<&Path>) -> String {
    let path = output
        .map(PathBuf::from)
        .unwrap_or_else(default_backup_path);
    let manager = BackupManager::new(base_dir.to_path_buf());

    match manager.create_backup(&path) {
        Ok(manifest) => {
            let c = &manifest.counts;
            format!(
                "✅ Backup written to {}\n\n  configs {} · prompts {} (+{} versions) · sessions {} · templates {} · memories {} · audit {}\n\nCreated {}",
                path.display(),
                c.configs,
                c.prompts,
                c.prompt_versions,
                c.sessions,
                c.session_templates,
                c.memories,
                c.audit_events,
                manifest.created_at.to_rfc3339()
            )
        }
        Err(e) => format!("❌ Backup failed: {}", e),
    }
}

pub fn cmd_restore(base_dir: &Path, file: &Path) -> String {
    let manager = BackupManager::new(base_dir.to_path_buf());

    match manager.restore_backup(file) {
        Ok(manifest) => {
            let c = &manifest.counts;
            format!(
                "✅ Restored from {}\n\n  configs {} · prompts {} · sessions {} · templates {} · memories {} · audit {}\n\nBackup created {}",
                file.display(),
                c.configs,
                c.prompts,
                c.sessions,
                c.session_templates,
                c.memories,
                c.audit_events,
                manifest.created_at.to_rfc3339()
            )
        }
        Err(e) => format!("❌ Restore failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Wave 2 commands: config templates / prompt + memory import-export / session
// budget + fork / skill compatibility / monitor
// ---------------------------------------------------------------------------

fn parse_key_value(s: &str) -> Option<(String, String)> {
    let (key, value) = s.split_once('=')?;
    Some((key.trim().to_string(), value.trim().to_string()))
}

pub fn cmd_config_template_list(base_dir: &Path) -> String {
    let manager = ConfigManager::new(base_dir.to_path_buf());
    let ids = match manager.list_templates() {
        Ok(ids) => ids,
        Err(e) => return format!("Error: {}", e),
    };
    if ids.is_empty() {
        return "No config templates.".to_string();
    }
    let mut out = String::from("Config templates:\n");
    for id in ids {
        out.push_str(&format!("  {}\n", id));
    }
    out
}

pub fn cmd_config_template_show(base_dir: &Path, id: &str) -> String {
    let manager = ConfigManager::new(base_dir.to_path_buf());
    match manager.get_template(id) {
        Ok(template) => {
            serde_json::to_string_pretty(&template).unwrap_or_else(|e| format!("Error: {}", e))
        }
        Err(e) => format!("Error: {}", e),
    }
}

pub fn cmd_config_template_create(
    base_dir: &Path,
    id: &str,
    name: &str,
    description: &str,
    sets: &[String],
    envs: &[String],
    secrets: &[String],
) -> String {
    let manager = ConfigManager::new(base_dir.to_path_buf());
    let mut settings = std::collections::HashMap::new();
    let mut invalid = Vec::new();
    for set in sets {
        match parse_key_value(set) {
            Some((key, value)) => {
                settings.insert(key, ConfigValue::from(value));
            }
            None => invalid.push(set.clone()),
        }
    }
    if !invalid.is_empty() {
        return format!(
            "❌ Invalid --set values (expected key=value): {}",
            invalid.join(", ")
        );
    }

    let mut env_vars = std::collections::HashMap::new();
    for env in envs {
        env_vars.insert(env.clone(), String::new());
    }

    match manager.create_template(
        id,
        name,
        description,
        settings,
        env_vars,
        secrets.to_vec(),
        std::collections::HashMap::new(),
    ) {
        Ok(_) => format!("✅ Template '{}' created", id),
        Err(e) => format!("❌ Failed: {}", e),
    }
}

pub fn cmd_config_template_delete(base_dir: &Path, id: &str) -> String {
    let manager = ConfigManager::new(base_dir.to_path_buf());
    match manager.delete_template(id) {
        Ok(true) => format!("✅ Template '{}' deleted", id),
        Ok(false) => format!("Template '{}' not found", id),
        Err(e) => format!("❌ Failed: {}", e),
    }
}

pub fn cmd_config_template_apply(base_dir: &Path, agent: &str, template: &str) -> String {
    let manager = ConfigManager::new(base_dir.to_path_buf());
    match manager.apply_template(agent, template) {
        Ok(config) => format!(
            "✅ Applied template '{}' to '{}' ({} settings, {} env vars, {} secret keys, v{})",
            template,
            agent,
            config.settings.len(),
            config.environment_variables.len(),
            config.secrets.len(),
            config.version
        ),
        Err(e) => format!("❌ Failed: {}", e),
    }
}

fn write_or_print(output: Option<&Path>, content: &str, label: &str) -> String {
    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            match std::fs::write(path, content) {
                Ok(_) => format!("✅ {} written to {}", label, path.display()),
                Err(e) => format!("❌ Failed to write {}: {}", label, e),
            }
        }
        None => content.to_string(),
    }
}

fn format_import_summary(summary: &ImportSummary, what: &str) -> String {
    format!(
        "✅ Imported {} {}(s), skipped {} existing",
        summary.imported, what, summary.skipped
    )
}

pub fn cmd_prompt_export(base_dir: &Path, id: &str, output: Option<&Path>) -> String {
    let manager = PromptManager::new(base_dir.join("prompts"));
    match manager.export_prompts_json(Some(&[id.to_string()])) {
        Ok(json) => write_or_print(output, &json, "Prompt export"),
        Err(e) => format!("❌ Export failed: {}", e),
    }
}

pub fn cmd_prompt_export_all(base_dir: &Path, output: Option<&Path>) -> String {
    let manager = PromptManager::new(base_dir.join("prompts"));
    match manager.export_prompts_json(None) {
        Ok(json) => write_or_print(output, &json, "Prompt export"),
        Err(e) => format!("❌ Export failed: {}", e),
    }
}

pub fn cmd_prompt_import(base_dir: &Path, file: &Path, force: bool) -> String {
    let manager = PromptManager::new(base_dir.join("prompts"));
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => return format!("❌ Failed to read {}: {}", file.display(), e),
    };
    match manager.import_prompts(&content, force) {
        Ok(summary) => format_import_summary(&summary, "prompt"),
        Err(e) => format!("❌ Import failed: {}", e),
    }
}

pub fn cmd_memory_export(base_dir: &Path, scope: Option<&str>, output: Option<&Path>) -> String {
    let manager = MemoryManager::new(base_dir.join("memory"));
    let scope_enum = match scope {
        Some("global") => Some(MemoryScope::Global),
        Some("project") => Some(MemoryScope::Project),
        Some("session") => Some(MemoryScope::Session),
        Some(other) => {
            return format!(
                "❌ Invalid scope '{}' (expected global|project|session)",
                other
            )
        }
        None => None,
    };
    match manager.export_memories_json(scope_enum) {
        Ok(json) => write_or_print(output, &json, "Memory export"),
        Err(e) => format!("❌ Export failed: {}", e),
    }
}

pub fn cmd_memory_import(base_dir: &Path, file: &Path, merge: bool) -> String {
    let manager = MemoryManager::new(base_dir.join("memory"));
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => return format!("❌ Failed to read {}: {}", file.display(), e),
    };
    match manager.import_memories(&content, merge) {
        Ok(summary) => format_import_summary(&summary, "memory"),
        Err(e) => format!("❌ Import failed: {}", e),
    }
}

pub fn cmd_session_budget_show(base_dir: &Path) -> String {
    let manager = SessionManager::new(base_dir.join("sessions"));
    let report = match manager.check_budget(Utc::now()) {
        Ok(r) => r,
        Err(e) => return format!("Error: {}", e),
    };
    let mut out = format!(
        "今日:      ${:.4} (limit {})\n",
        report.daily_spent_usd,
        opt_usd(report.daily_limit_usd)
    );
    out.push_str(&format!(
        "本月:      ${:.4} (limit {})\n",
        report.monthly_spent_usd,
        opt_usd(report.monthly_limit_usd)
    ));
    out.push_str(&format!("今日tokens: {}\n", report.total_tokens_today));
    if report.alerts.is_empty() {
        out.push_str("✅ 无预算告警\n");
    } else {
        for alert in &report.alerts {
            out.push_str(&format!("⚠️ {}\n", alert));
        }
    }
    out
}

fn opt_usd(limit: Option<f64>) -> String {
    match limit {
        Some(v) => format!("${:.2}", v),
        None => "unlimited".to_string(),
    }
}

pub fn cmd_session_budget_set(base_dir: &Path, daily: Option<f64>, monthly: Option<f64>) -> String {
    let manager = SessionManager::new(base_dir.join("sessions"));
    let mut budget = match manager.get_budget() {
        Ok(b) => b,
        Err(e) => return format!("Error: {}", e),
    };
    if daily.is_some() {
        budget.daily_usd = daily;
    }
    if monthly.is_some() {
        budget.monthly_usd = monthly;
    }
    match manager.set_budget(&budget) {
        Ok(_) => format!(
            "✅ Budget set — daily {}, monthly {}",
            opt_usd(budget.daily_usd),
            opt_usd(budget.monthly_usd)
        ),
        Err(e) => format!("❌ Failed: {}", e),
    }
}

pub fn cmd_session_fork(
    base_dir: &Path,
    id: &str,
    agent: Option<&str>,
    title: Option<&str>,
) -> String {
    let manager = SessionManager::new(base_dir.join("sessions"));
    match manager.fork_session(id, agent, title) {
        Ok(session) => format!(
            "✅ Forked '{}' → {} (agent {}, {} messages)",
            id,
            session.id,
            session.agent,
            session.messages.len()
        ),
        Err(e) => format!("❌ Failed: {}", e),
    }
}

pub fn cmd_skill_check_compat(base_dir: &Path, name: &str) -> String {
    let manager = SkillManager::new(base_dir.join("skills"));
    let results = if name == "*" {
        manager.check_all_compatibility()
    } else {
        match manager.check_compatibility(name) {
            Ok(compat) => Ok(vec![compat]),
            Err(e) => return format!("Error: {}", e),
        }
    };

    match results {
        Ok(results) => {
            if results.is_empty() {
                return "No skills with version constraints.".to_string();
            }
            let mut out = String::new();
            for compat in &results {
                let mark = if compat.compatible { "✅" } else { "❌" };
                out.push_str(&format!(
                    "{} {} v{} — {}\n",
                    mark, compat.skill, compat.skill_version, compat.message
                ));
            }
            out
        }
        Err(e) => format!("Error: {}", e),
    }
}

pub fn cmd_monitor(base_dir: &Path, catalog: &Catalog) -> String {
    let monitor = Monitor::new(base_dir.to_path_buf(), get_platform());
    match monitor.run(catalog) {
        Ok(report) => {
            let status = if report.healthy {
                "✅ HEALTHY"
            } else {
                "⚠️ ISSUES FOUND"
            };
            let mut out = format!("AgentHub Monitor — {}\n", status);
            out.push_str(&format!("{}\n", "=".repeat(50)));
            out.push_str(&format!("版本:       {}\n", report.agenthub_version));
            out.push_str(&format!(
                "已安装:     {} / 目录 {}\n",
                report.installed_agents,
                catalog.agents().len()
            ));
            out.push_str(&format!(
                "诊断:       {} passed, {} warnings, {} failed\n",
                report.diagnostics_passed, report.diagnostics_warnings, report.diagnostics_failed
            ));
            out.push_str(&format!(
                "预算:       今日 ${:.4} / 本月 ${:.4}\n",
                report.budget.daily_spent_usd, report.budget.monthly_spent_usd
            ));
            if !report.missing_agents.is_empty() {
                out.push_str(&format!(
                    "未安装(verified): {}\n",
                    report.missing_agents.join(", ")
                ));
            }
            if !report.incompatible_skills.is_empty() {
                out.push_str(&format!(
                    "不兼容技能: {}\n",
                    report.incompatible_skills.join(", ")
                ));
            }
            for warning in &report.warnings {
                out.push_str(&format!("⚠️ {}\n", warning));
            }
            out
        }
        Err(e) => format!("❌ Monitor failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();
    let platform = get_platform();

    let result = match cli.command {
        Commands::List { kind } => match load_catalog() {
            Ok(catalog) => cmd_list(kind, &catalog),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Search { query, kind } => match load_catalog() {
            Ok(catalog) => cmd_search(&query, kind, &catalog),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Info { name } => match load_catalog() {
            Ok(catalog) => cmd_info(&name, &catalog),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Install { name, dry_run, yes } => {
            let catalog = match load_catalog() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let agent = match catalog.find_by_name(&name) {
                Some(a) => a.clone(),
                None => {
                    eprintln!("Agent '{}' not found", name);
                    std::process::exit(1);
                }
            };
            if !yes && !dry_run {
                print!("\nProceed with installation? [y/N]: ");
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                if input.trim().to_lowercase() != "y" {
                    println!("Installation cancelled.");
                    return;
                }
            }
            cmd_install(&name, dry_run, &agent, platform)
        }
        Commands::Uninstall { name, dry_run, yes } => {
            let catalog = match load_catalog() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let agent = match catalog.find_by_name(&name) {
                Some(a) => a.clone(),
                None => {
                    eprintln!("Agent '{}' not found", name);
                    std::process::exit(1);
                }
            };
            if !yes && !dry_run {
                print!("\nProceed with uninstallation? [y/N]: ");
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                if input.trim().to_lowercase() != "y" {
                    println!("Uninstallation cancelled.");
                    return;
                }
            }
            cmd_uninstall(&name, dry_run, &agent, platform)
        }
        Commands::Doctor => cmd_doctor(),
        Commands::Status { trend } => match load_catalog() {
            Ok(catalog) => cmd_status(&data_dir(), &catalog, trend),
            Err(e) => format!("Error: {}", e),
        },
        Commands::Audit {
            action,
            target,
            last_days,
            limit,
        } => cmd_audit(
            &data_dir(),
            action.as_deref(),
            target.as_deref(),
            last_days,
            limit,
        ),
        Commands::Backup { output } => cmd_backup(&data_dir(), output.as_deref()),
        Commands::Restore { file } => cmd_restore(&data_dir(), &file),
        Commands::ConfigTemplate(cmd) => match cmd {
            ConfigTemplateCmd::List => cmd_config_template_list(&data_dir()),
            ConfigTemplateCmd::Show { id } => cmd_config_template_show(&data_dir(), &id),
            ConfigTemplateCmd::Create {
                id,
                name,
                description,
                sets,
                envs,
                secrets,
            } => cmd_config_template_create(
                &data_dir(),
                &id,
                &name,
                &description,
                &sets,
                &envs,
                &secrets,
            ),
            ConfigTemplateCmd::Delete { id } => cmd_config_template_delete(&data_dir(), &id),
            ConfigTemplateCmd::Apply { agent, template } => {
                cmd_config_template_apply(&data_dir(), &agent, &template)
            }
        },
        Commands::Prompt(cmd) => match cmd {
            PromptArgs::Export { id, output } => {
                cmd_prompt_export(&data_dir(), &id, output.as_deref())
            }
            PromptArgs::ExportAll { output } => {
                cmd_prompt_export_all(&data_dir(), output.as_deref())
            }
            PromptArgs::Import { file, force } => cmd_prompt_import(&data_dir(), &file, force),
        },
        Commands::Memory(cmd) => match cmd {
            MemoryArgs::Export { scope, output } => {
                cmd_memory_export(&data_dir(), scope.as_deref(), output.as_deref())
            }
            MemoryArgs::Import { file, merge } => cmd_memory_import(&data_dir(), &file, merge),
        },
        Commands::Session(cmd) => match cmd {
            SessionArgs::Budget { cmd } => match cmd {
                BudgetCmd::Show => cmd_session_budget_show(&data_dir()),
                BudgetCmd::Set { daily, monthly } => {
                    cmd_session_budget_set(&data_dir(), daily, monthly)
                }
            },
            SessionArgs::Fork { id, agent, title } => {
                cmd_session_fork(&data_dir(), &id, agent.as_deref(), title.as_deref())
            }
        },
        Commands::Skill(cmd) => match cmd {
            SkillArgs::CheckCompat { name } => cmd_skill_check_compat(&data_dir(), &name),
        },
        Commands::Monitor => match load_catalog() {
            Ok(catalog) => cmd_monitor(&data_dir(), &catalog),
            Err(e) => format!("Error: {}", e),
        },
    };

    println!("{}", result);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use agenthub_core::Catalog;
    use tempfile::TempDir;

    fn test_catalog() -> Catalog {
        Catalog::from_json(TEST_AGENTS_JSON).unwrap()
    }

    #[test]
    fn test_get_platform_returns_platform() {
        let platform = get_platform();
        // Should always return a valid platform variant
        match platform {
            Platform::Windows | Platform::MacOS | Platform::Linux => {}
        }
    }

    #[test]
    fn test_cmd_list_all_agents() {
        let catalog = test_catalog();
        let output = cmd_list(None, &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Test Desktop"));
        assert!(output.contains("Total:"));
    }

    #[test]
    fn test_cmd_list_cli_agents() {
        let catalog = test_catalog();
        let output = cmd_list(Some("cli".to_string()), &catalog);
        assert!(output.contains("Test CLI"));
        assert!(!output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_list_desktop_agents() {
        let catalog = test_catalog();
        let output = cmd_list(Some("desktop".to_string()), &catalog);
        assert!(output.contains("Test Desktop"));
        assert!(!output.contains("Test CLI"));
    }

    #[test]
    fn test_cmd_list_empty_kind() {
        let catalog = test_catalog();
        let output = cmd_list(Some("".to_string()), &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_search_found() {
        let catalog = test_catalog();
        let output = cmd_search("CLI", None, &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Found"));
    }

    #[test]
    fn test_cmd_search_not_found() {
        let catalog = test_catalog();
        let output = cmd_search("nonexistent", None, &catalog);
        assert!(output.contains("No agents found"));
    }

    #[test]
    fn test_cmd_search_case_insensitive() {
        let catalog = test_catalog();
        let output = cmd_search("test cli", None, &catalog);
        assert!(output.contains("Test CLI"));
    }

    #[test]
    fn test_cmd_search_by_provider() {
        let catalog = test_catalog();
        let output = cmd_search("Test Provider", None, &catalog);
        assert!(output.contains("Test CLI"));
        assert!(output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_search_filter_cli() {
        let catalog = test_catalog();
        let output = cmd_search("test", Some("cli".to_string()), &catalog);
        assert!(output.contains("Test CLI"));
        assert!(!output.contains("Test Desktop"));
    }

    #[test]
    fn test_cmd_info_found() {
        let catalog = test_catalog();
        let output = cmd_info("Test CLI", &catalog);
        assert!(output.contains("Name:        Test CLI"));
        assert!(output.contains("ID:          test-cli"));
        assert!(output.contains("test-cli.com"));
    }

    #[test]
    fn test_cmd_info_not_found() {
        let catalog = test_catalog();
        let output = cmd_info("nonexistent", &catalog);
        assert!(output.contains("not found"));
    }

    #[test]
    fn test_cmd_info_by_id() {
        let catalog = test_catalog();
        let output = cmd_info("test-cli", &catalog);
        assert!(output.contains("test-cli"));
    }

    #[test]
    fn test_cmd_doctor_returns_report() {
        let output = cmd_doctor();
        assert!(output.contains("Summary:"));
        assert!(output.contains("passed"));
    }

    #[test]
    fn test_load_catalog_from_project_root() {
        // Running from project root, agents.json should be found
        let result = load_catalog();
        assert!(
            result.is_ok(),
            "expected catalog to load: {:?}",
            result.err()
        );
    }

    // ---- Management commands (status / audit / backup / restore) ----

    const MANUAL_AGENTS_JSON: &str = r#"{
        "version": "1.0.0",
        "last_updated": "2026-06-27",
        "agents": [
            {
                "id": "test-cli",
                "name": "Test CLI",
                "kind": "cli",
                "provider": "Test Provider",
                "description": "A test CLI agent",
                "homepage": "https://test-cli.com",
                "installers": {
                    "windows": { "manager": "manual", "package": null },
                    "macos": { "manager": "manual", "package": null },
                    "linux": { "manager": "manual", "package": null }
                },
                "status": "verified",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": "2026-06-27"
            }
        ]
    }"#;

    #[test]
    fn test_cmd_status() {
        let temp = TempDir::new().unwrap();
        let catalog = Catalog::from_json(MANUAL_AGENTS_JSON).unwrap();

        // Seed some workspace data
        let base = temp.path();
        agenthub_core::ConfigManager::new(base.to_path_buf())
            .create_config("test-cli")
            .unwrap();
        agenthub_core::PromptManager::new(base.join("prompts"))
            .create_prompt("p1", "P1", "d", "t")
            .unwrap();
        let sm = agenthub_core::SessionManager::new(base.join("sessions"));
        let session = sm.create_session("S1", "codex").unwrap();
        sm.add_message(&session.id, "user", "hi").unwrap();
        agenthub_core::MemoryManager::new(base.join("memory"))
            .create_entry(
                agenthub_core::MemoryScope::Global,
                None,
                "Note",
                "c",
                agenthub_core::MemoryType::Free,
            )
            .unwrap();
        agenthub_core::AuditManager::new(base.join("audit"))
            .record("cli", "install", "test-cli", None, true)
            .unwrap();

        let output = cmd_status(base, &catalog, None);
        assert!(output.contains("AgentHub 状态概览"));
        assert!(output.contains("目录:"));
        assert!(output.contains("1 agents (1 CLI, 0 Desktop)"));
        assert!(output.contains("配置:"));
        assert!(output.contains("提示词:"));
        assert!(output.contains("会话:"));
        assert!(output.contains("记忆:"));
        assert!(output.contains("审计事件:"));
    }

    #[test]
    fn test_cmd_audit_empty() {
        let temp = TempDir::new().unwrap();
        let output = cmd_audit(temp.path(), None, None, None, 50);
        assert!(output.contains("No audit events found"));
    }

    #[test]
    fn test_cmd_audit_with_events() {
        let temp = TempDir::new().unwrap();
        let manager = agenthub_core::AuditManager::new(temp.path().join("audit"));
        manager
            .record("cli", "install", "claude-code", None, true)
            .unwrap();
        manager
            .record("cli", "config.set", "codex", None, false)
            .unwrap();

        let output = cmd_audit(temp.path(), None, None, None, 50);
        assert!(output.contains("install"));
        assert!(output.contains("config.set"));
        assert!(output.contains("2 event(s)"));

        // Filter by action
        let output = cmd_audit(temp.path(), Some("install"), None, None, 50);
        assert!(output.contains("1 event(s)"));

        // Filter by target
        let output = cmd_audit(temp.path(), None, Some("codex"), None, 50);
        assert!(output.contains("1 event(s)"));
        assert!(output.contains("config.set"));
    }

    #[test]
    fn test_cmd_backup_and_restore() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("data");

        // Seed data
        agenthub_core::ConfigManager::new(base.clone())
            .create_config("test-cli")
            .unwrap();
        agenthub_core::SessionManager::new(base.join("sessions"))
            .create_session("S1", "codex")
            .unwrap();
        agenthub_core::AuditManager::new(base.join("audit"))
            .record("cli", "install", "test-cli", None, true)
            .unwrap();

        let out = temp.path().join("backup.json");
        let output = cmd_backup(&base, Some(&out));
        assert!(output.contains("✅ Backup written"));
        assert!(output.contains("configs 1"));
        assert!(out.exists());

        // Restore into a fresh directory
        let target = temp.path().join("restored");
        let output = cmd_restore(&target, &out);
        assert!(output.contains("✅ Restored"));

        let configs = agenthub_core::ConfigManager::new(target.clone())
            .list_configs()
            .unwrap();
        assert_eq!(configs, vec!["test-cli".to_string()]);
        let sessions = agenthub_core::SessionManager::new(target.join("sessions"))
            .list_sessions()
            .unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_cmd_backup_failure_message() {
        let temp = TempDir::new().unwrap();
        let output = cmd_backup(temp.path(), Some(&temp.path().join("x.json")));
        // Empty workspace still produces a valid empty backup
        assert!(output.contains("✅ Backup written"));
        assert!(output.contains("configs 0"));
    }

    // ---- Wave 2 commands ----

    #[test]
    fn test_cmd_config_template_lifecycle() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        let output = cmd_config_template_list(base);
        assert!(output.contains("No config templates"));

        let output = cmd_config_template_create(
            base,
            "llm-default",
            "LLM Default",
            "Standard settings",
            &["model=gpt-4o".to_string(), "temperature=0.7".to_string()],
            &["OPENAI_API_KEY".to_string()],
            &["api_key".to_string()],
        );
        assert!(output.contains("created"));

        let output = cmd_config_template_list(base);
        assert!(output.contains("llm-default"));

        let output = cmd_config_template_show(base, "llm-default");
        assert!(output.contains("gpt-4o"));
        // Secret key names are reserved (shown as keys); values are never stored
        assert!(output.contains("secret_keys"));
        assert!(output.contains("api_key"));

        let output = cmd_config_template_apply(base, "codex", "llm-default");
        assert!(output.contains("Applied template 'llm-default' to 'codex'"));
        assert!(output.contains("1 secret keys"));

        let output = cmd_config_template_delete(base, "llm-default");
        assert!(output.contains("deleted"));
        assert!(cmd_config_template_list(base).contains("No config templates"));
    }

    #[test]
    fn test_cmd_config_template_invalid_set() {
        let temp = TempDir::new().unwrap();
        let output = cmd_config_template_create(
            temp.path(),
            "bad",
            "Bad",
            "",
            &["no-equals-sign".to_string()],
            &[],
            &[],
        );
        assert!(output.contains("Invalid --set"));
    }

    #[test]
    fn test_cmd_prompt_export_import() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();
        let prompts = agenthub_core::PromptManager::new(base.join("prompts"));
        prompts
            .create_prompt("review", "Review", "d", "review {{code}}")
            .unwrap();
        prompts
            .update_prompt("review", None, None, Some("review v2 {{code}}"))
            .unwrap();

        let export_path = temp.path().join("prompts.json");
        let output = cmd_prompt_export_all(base, Some(&export_path));
        assert!(output.contains("written"));
        assert!(export_path.exists());

        // Import into a fresh dir
        let target = temp.path().join("target");
        let output = cmd_prompt_import(&target, &export_path, false);
        assert!(output.contains("Imported 1 prompt(s)"));

        let imported = agenthub_core::PromptManager::new(target.join("prompts"));
        let p = imported.get_prompt("review").unwrap();
        assert_eq!(p.template, "review v2 {{code}}");
        assert_eq!(imported.list_versions("review").unwrap().len(), 1);
    }

    #[test]
    fn test_cmd_memory_export_import() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();
        let memories = agenthub_core::MemoryManager::new(base.join("memory"));
        memories
            .create_entry(
                agenthub_core::MemoryScope::Global,
                None,
                "Note",
                "content",
                agenthub_core::MemoryType::Learning,
            )
            .unwrap();

        let export_path = temp.path().join("memories.json");
        let output = cmd_memory_export(base, None, Some(&export_path));
        assert!(output.contains("written"));

        // Invalid scope rejected
        let output = cmd_memory_export(base, Some("nope"), None);
        assert!(output.contains("Invalid scope"));

        let target = temp.path().join("target");
        let output = cmd_memory_import(&target, &export_path, false);
        assert!(output.contains("Imported 1 memory(s)"));
        assert_eq!(
            agenthub_core::MemoryManager::new(target.join("memory"))
                .list_entries(None)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_cmd_session_budget_and_fork() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();
        let sessions = agenthub_core::SessionManager::new(base.join("sessions"));
        let session = sessions.create_session("Src", "codex").unwrap();
        sessions.add_message(&session.id, "user", "ctx").unwrap();

        // Budget set + show
        let output = cmd_session_budget_set(base, Some(10.0), Some(100.0));
        assert!(output.contains("Budget set"));
        assert!(output.contains("daily $10.00"));

        let output = cmd_session_budget_show(base);
        assert!(output.contains("今日:"));
        assert!(output.contains("limit $10.00"));

        // Fork carries context, possibly to another agent
        let output = cmd_session_fork(base, &session.id, Some("claude-code"), Some("Handoff"));
        assert!(output.contains("Forked"));
        assert!(output.contains("agent claude-code"));
        assert!(output.contains("1 messages"));

        let sessions = agenthub_core::SessionManager::new(base.join("sessions"));
        assert_eq!(sessions.list_sessions().unwrap().len(), 2);
    }

    #[test]
    fn test_cmd_skill_check_compat() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();
        let dir = base.join("skills").join("installed").join("old-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            "---\nname: old-skill\ndescription: \"x\"\nversion: 1.0.0\nmin_agenthub_version: 99.0.0\n---\n\n# x\n",
        )
        .unwrap();

        let output = cmd_skill_check_compat(base, "*");
        assert!(output.contains("old-skill"));
        assert!(output.contains("❌"));
        assert!(output.contains("upgrade"));

        // Unconstrained skills produce empty result
        let output = cmd_skill_check_compat(base, "does-not-exist");
        assert!(output.contains("Error:"));
    }

    #[test]
    fn test_cmd_status_with_trend() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();
        let catalog = Catalog::from_json(MANUAL_AGENTS_JSON).unwrap();

        let sm = agenthub_core::SessionManager::new(base.join("sessions"));
        let session = sm.create_session("Today", "codex").unwrap();
        sm.add_message(&session.id, "user", "hi").unwrap();

        let output = cmd_status(base, &catalog, Some(7));
        assert!(output.contains("趋势 (最近 7 天):"));
        assert!(output.contains("日期"));
        assert!(output.contains("2026")); // today's date line
    }

    #[test]
    fn test_cmd_monitor_runs() {
        let temp = TempDir::new().unwrap();
        let catalog = Catalog::from_json(MANUAL_AGENTS_JSON).unwrap();
        let output = cmd_monitor(temp.path(), &catalog);
        assert!(output.contains("AgentHub Monitor"));
        assert!(output.contains("诊断:"));
        assert!(output.contains("预算:"));
    }

    const TEST_AGENTS_JSON: &str = r#"{
        "version": "1.0.0",
        "last_updated": "2026-06-27",
        "agents": [
            {
                "id": "test-cli",
                "name": "Test CLI",
                "kind": "cli",
                "provider": "Test Provider",
                "description": "A test CLI agent",
                "homepage": "https://test-cli.com",
                "installers": {
                    "windows": { "manager": "npm", "package": "@test/cli" }
                },
                "status": "verified",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": "2026-06-27"
            },
            {
                "id": "test-desktop",
                "name": "Test Desktop",
                "kind": "desktop",
                "provider": "Test Provider",
                "description": "A test desktop agent",
                "homepage": "https://test-desktop.com",
                "installers": {
                    "windows": { "manager": "winget", "package": "Test.Desktop" }
                },
                "status": "community",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": null
            }
        ]
    }"#;
}
