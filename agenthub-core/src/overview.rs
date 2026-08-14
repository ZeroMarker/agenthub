use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::agent::{AgentKind, Platform, SupportStatus};
use crate::audit::AuditManager;
use crate::catalog::Catalog;
use crate::config::ConfigManager;
use crate::error::Result;
use crate::memory::{MemoryManager, MemoryStats};
use crate::prompt::PromptManager;
use crate::session::{SessionManager, SessionStats};
use crate::skill::SkillManager;
use crate::status::{AgentStatus, StatusDetector};

/// Catalog-level counts for the status overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogOverview {
    pub total: usize,
    pub cli: usize,
    pub desktop: usize,
    pub verified: usize,
    pub community: usize,
    pub manual: usize,
    pub deprecated: usize,
}

/// A point-in-time snapshot of the whole AgentHub workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusOverview {
    pub generated_at: DateTime<Utc>,
    pub platform: String,
    pub agenthub_version: String,
    pub catalog: CatalogOverview,
    /// Number of catalog agents detected as installed on this machine.
    pub installed_agents: usize,
    pub configs: usize,
    pub prompts: usize,
    pub sessions: SessionStats,
    pub memories: MemoryStats,
    pub skills_total: usize,
    pub skills_enabled: usize,
    pub audit_events: usize,
}

/// One day bucket in a [`TrendReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    /// UTC date `YYYY-MM-DD`.
    pub date: String,
    pub sessions_started: usize,
    pub sessions_completed: usize,
    pub tokens: u64,
    pub cost_usd: f64,
    pub memories_created: usize,
    pub audit_events: usize,
}

/// Aggregates data across every manager to produce a unified overview
/// (dashboard / `agenthub status`).
pub struct OverviewReport {
    base_dir: PathBuf,
    platform: Platform,
}

impl OverviewReport {
    pub fn new(base_dir: PathBuf, platform: Platform) -> Self {
        Self { base_dir, platform }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Build an overview, detecting installed agents with [`StatusDetector`].
    pub fn overview(&self, catalog: &Catalog) -> Result<StatusOverview> {
        let statuses = StatusDetector::new(self.platform).check_agents(catalog.agents());
        self.overview_with_status(catalog, &statuses)
    }

    /// Build an overview from pre-computed agent statuses (no subprocess calls;
    /// used by tests and by callers that already have status data).
    pub fn overview_with_status(
        &self,
        catalog: &Catalog,
        statuses: &[AgentStatus],
    ) -> Result<StatusOverview> {
        let config_manager = ConfigManager::new(self.base_dir.clone());
        let prompt_manager = PromptManager::new(self.base_dir.join("prompts"));
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let memory_manager = MemoryManager::new(self.base_dir.join("memory"));
        let skill_manager = SkillManager::new(self.base_dir.join("skills"));
        let audit_manager = AuditManager::new(self.base_dir.join("audit"));

        let agents = catalog.agents();
        let catalog_overview = CatalogOverview {
            total: agents.len(),
            cli: agents.iter().filter(|a| a.kind == AgentKind::CLI).count(),
            desktop: agents
                .iter()
                .filter(|a| a.kind == AgentKind::Desktop)
                .count(),
            verified: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Verified)
                .count(),
            community: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Community)
                .count(),
            manual: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Manual)
                .count(),
            deprecated: agents
                .iter()
                .filter(|a| a.status == SupportStatus::Deprecated)
                .count(),
        };

        let skills = skill_manager.list_skills()?;

        Ok(StatusOverview {
            generated_at: Utc::now(),
            platform: format!("{:?}", self.platform),
            agenthub_version: env!("CARGO_PKG_VERSION").to_string(),
            catalog: catalog_overview,
            installed_agents: statuses.iter().filter(|s| s.installed).count(),
            configs: config_manager.list_configs()?.len(),
            prompts: prompt_manager.list_prompts()?.len(),
            sessions: session_manager.get_stats()?,
            memories: memory_manager.get_stats()?,
            skills_total: skills.len(),
            skills_enabled: skills.iter().filter(|s| s.enabled).count(),
            audit_events: audit_manager.count()?,
        })
    }

    /// Daily trend for the last `days` days (oldest first, includes today),
    /// bucketed by UTC date.
    pub fn trend(&self, days: usize) -> Result<Vec<TrendPoint>> {
        self.trend_with_now(days, Utc::now())
    }

    pub fn trend_with_now(&self, days: usize, now: DateTime<Utc>) -> Result<Vec<TrendPoint>> {
        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let memory_manager = MemoryManager::new(self.base_dir.join("memory"));
        let audit_manager = AuditManager::new(self.base_dir.join("audit"));

        let today = now.date_naive();
        let mut points: Vec<TrendPoint> = (0..days)
            .map(|i| {
                let date = today - chrono::Duration::days((days - 1 - i) as i64);
                TrendPoint {
                    date: date.to_string(),
                    sessions_started: 0,
                    sessions_completed: 0,
                    tokens: 0,
                    cost_usd: 0.0,
                    memories_created: 0,
                    audit_events: 0,
                }
            })
            .collect();
        let index: std::collections::HashMap<String, usize> = points
            .iter()
            .enumerate()
            .map(|(i, p)| (p.date.clone(), i))
            .collect();

        for session in session_manager.list_sessions()? {
            if let Some(&i) = index.get(&session.started_at.date_naive().to_string()) {
                points[i].sessions_started += 1;
                if session.status == crate::session::SessionStatus::Completed {
                    points[i].sessions_completed += 1;
                }
                if let Some(usage) = &session.usage {
                    points[i].tokens += usage.total_tokens as u64;
                    points[i].cost_usd += usage.estimated_cost_usd;
                }
            }
        }

        for memory in memory_manager.list_entries(None)? {
            if let Some(&i) = index.get(&memory.created_at.date_naive().to_string()) {
                points[i].memories_created += 1;
            }
        }

        for event in audit_manager.load_all()? {
            if let Some(&i) = index.get(&event.timestamp.date_naive().to_string()) {
                points[i].audit_events += 1;
            }
        }

        Ok(points)
    }

    /// Render a self-contained HTML dashboard (inline CSS/JS + embedded JSON)
    /// that can be opened in any browser — the "Web dashboard" slice of the
    /// Overview module, with no server needed.
    /// Render a self-contained interactive dashboard (inline CSS/JS + embedded
    /// JSON) that opens in any browser with no server needed: a 7/30/90-day
    /// trend range switcher, SVG cost/token charts, sortable trend table and
    /// click-to-drilldown panels for catalog agents, skills, per-agent session
    /// usage and recent audit events.
    pub fn render_dashboard_html(&self, catalog: &Catalog, trend_days: usize) -> Result<String> {
        let statuses = StatusDetector::new(self.platform).check_agents(catalog.agents());
        let overview = self.overview_with_status(catalog, &statuses)?;

        // Trend windows: always include 7/30/90 plus the requested default.
        let mut days_set = vec![7usize, 30, 90];
        if !days_set.contains(&trend_days) {
            days_set.push(trend_days);
        }
        days_set.sort_unstable();
        days_set.dedup();
        let mut trends = Vec::new();
        for days in &days_set {
            trends.push(TrendReport {
                days: *days,
                points: self.trend(*days)?,
            });
        }

        // Drilldown data.
        let agents: Vec<DashboardAgentRow> = catalog
            .agents()
            .iter()
            .map(|agent| {
                let status = statuses
                    .iter()
                    .find(|s| s.agent_id == agent.id)
                    .map(|s| (s.installed, s.detection_method.clone()))
                    .unwrap_or((false, "unknown".to_string()));
                DashboardAgentRow {
                    id: agent.id.clone(),
                    name: agent.name.clone(),
                    kind: format!("{:?}", agent.kind).to_lowercase(),
                    installed: status.0,
                    status: status.1,
                }
            })
            .collect();

        let skills: Vec<DashboardSkillRow> = SkillManager::new(self.base_dir.join("skills"))
            .list_skills()
            .unwrap_or_default()
            .iter()
            .map(|s| DashboardSkillRow {
                name: s.manifest.name.clone(),
                version: s.manifest.version.clone(),
                enabled: s.enabled,
            })
            .collect();

        let session_manager = SessionManager::new(self.base_dir.join("sessions"));
        let mut usage: HashMap<String, AgentUsageRow> = HashMap::new();
        for session in session_manager.list_sessions().unwrap_or_default() {
            let row = usage.entry(session.agent.clone()).or_insert(AgentUsageRow {
                agent: session.agent.clone(),
                sessions: 0,
                tokens: 0,
                cost_usd: 0.0,
            });
            row.sessions += 1;
            if let Some(u) = &session.usage {
                row.tokens += u.total_tokens as u64;
                row.cost_usd += u.estimated_cost_usd;
            }
        }
        let mut sessions_by_agent: Vec<AgentUsageRow> = usage.into_values().collect();
        sessions_by_agent.sort_by(|a, b| {
            b.cost_usd
                .partial_cmp(&a.cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let audit_manager = AuditManager::new(self.base_dir.join("audit"));
        let recent_audit: Vec<AuditRow> = audit_manager
            .load_all()
            .unwrap_or_default()
            .iter()
            .rev()
            .take(50)
            .map(|e| AuditRow {
                timestamp: e.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                action: e.action.clone(),
                target: e.target.clone(),
                actor: e.actor.clone(),
            })
            .collect();

        let cards = format!(
            r#"<div class="card clickable" data-drill="agents"><div class="card-value">{total}</div><div class="card-label">Catalog agents</div></div>
<div class="card clickable" data-drill="agents"><div class="card-value">{installed}</div><div class="card-label">Installed</div></div>
<div class="card"><div class="card-value">{configs}</div><div class="card-label">Configs</div></div>
<div class="card"><div class="card-value">{prompts}</div><div class="card-label">Prompts</div></div>
<div class="card clickable" data-drill="sessions"><div class="card-value">{sessions}</div><div class="card-label">Sessions</div></div>
<div class="card"><div class="card-value">{memories}</div><div class="card-label">Memories</div></div>
<div class="card clickable" data-drill="skills"><div class="card-value">{skills}</div><div class="card-label">Skills ({enabled} enabled)</div></div>
<div class="card clickable" data-drill="audit"><div class="card-value">{audit}</div><div class="card-label">Audit events</div></div>"#,
            total = overview.catalog.total,
            installed = overview.installed_agents,
            configs = overview.configs,
            prompts = overview.prompts,
            sessions = overview.sessions.total,
            memories = overview.memories.total,
            skills = overview.skills_total,
            enabled = overview.skills_enabled,
            audit = overview.audit_events,
        );

        // Trend table rows for every window; the JS range switcher toggles
        // rows by their data-days attribute.
        let mut rows = String::new();
        for report in &trends {
            let max_cost = report
                .points
                .iter()
                .map(|p| p.cost_usd)
                .fold(0.0f64, f64::max)
                .max(0.01);
            let max_tokens = report
                .points
                .iter()
                .map(|p| p.tokens)
                .max()
                .unwrap_or(0)
                .max(1);
            for point in &report.points {
                let cost_pct = (point.cost_usd / max_cost * 100.0).max(0.5);
                let tokens_pct = (point.tokens as f64 / max_tokens as f64 * 100.0).max(0.5);
                rows.push_str(&format!(
                    r#"<tr class="trend-row" data-days="{days}">
<td class="date">{date}</td><td>{started}</td><td>{completed}</td>
<td><div class="meter"><div class="meter-fill meter-cost" style="width:{cost_pct:.0}%"></div></div>${cost:.3}</td>
<td><div class="meter"><div class="meter-fill meter-tokens" style="width:{tokens_pct:.0}%"></div></div>{tokens}</td>
<td>{memories}</td><td>{audit}</td></tr>"#,
                    days = report.days,
                    date = point.date,
                    started = point.sessions_started,
                    completed = point.sessions_completed,
                    cost_pct = cost_pct,
                    cost = point.cost_usd,
                    tokens_pct = tokens_pct,
                    tokens = point.tokens,
                    memories = point.memories_created,
                    audit = point.audit_events,
                ));
            }
        }

        // SVG bar charts for cost and tokens, one series per window.
        let mut charts = String::new();
        for metric in ["cost", "tokens"] {
            for report in &trends {
                let label = if metric == "cost" {
                    "Cost (USD)"
                } else {
                    "Tokens"
                };
                let max = if metric == "cost" {
                    report
                        .points
                        .iter()
                        .map(|p| p.cost_usd)
                        .fold(0.0f64, f64::max)
                        .max(0.01)
                } else {
                    report
                        .points
                        .iter()
                        .map(|p| p.tokens as f64)
                        .fold(0.0f64, f64::max)
                        .max(1.0)
                };
                let bars: Vec<String> = report
                    .points
                    .iter()
                    .map(|p| {
                        let v = if metric == "cost" { p.cost_usd } else { p.tokens as f64 };
                        let h = (v / max * 100.0).max(1.0);
                        let tip = if metric == "cost" {
                            format!("{:.3}", p.cost_usd)
                        } else {
                            format!("{}", p.tokens)
                        };
                        format!(
                            r#"<div class="bar" style="height:{h:.1}%" title="{} · {}"><span class="bar-val">{}</span></div>"#,
                            p.date, tip, tip
                        )
                    })
                    .collect();
                charts.push_str(&format!(
                    r#"<div class="chart" data-chart="{metric}" data-days="{days}"><div class="chart-title">{label} · last {days}d</div><div class="bars">{bars}</div></div>"#,
                    metric = metric,
                    days = report.days,
                    label = label,
                    bars = bars.join(""),
                ));
            }
        }

        // Drilldown tables.
        let agent_rows: String = agents
            .iter()
            .map(|a| {
                format!(
                    r#"<tr><td>{name}</td><td><code>{id}</code></td><td>{kind}</td><td>{status}</td><td class="{cls}">{installed}</td></tr>"#,
                    name = a.name,
                    id = a.id,
                    kind = a.kind,
                    status = a.status,
                    installed = if a.installed { "✓ installed" } else { "—" },
                    cls = if a.installed { "ok" } else { "miss" },
                )
            })
            .collect();
        let skill_rows: String = skills
            .iter()
            .map(|s| {
                format!(
                    r#"<tr><td>{name}</td><td>{version}</td><td class="{cls}">{enabled}</td></tr>"#,
                    name = s.name,
                    version = s.version,
                    enabled = if s.enabled { "enabled" } else { "disabled" },
                    cls = if s.enabled { "ok" } else { "miss" },
                )
            })
            .collect();
        let session_rows: String = sessions_by_agent
            .iter()
            .map(|r| {
                format!(
                    r#"<tr><td>{agent}</td><td>{sessions}</td><td>{tokens}</td><td>${cost:.3}</td></tr>"#,
                    agent = r.agent,
                    sessions = r.sessions,
                    tokens = r.tokens,
                    cost = r.cost_usd,
                )
            })
            .collect();
        let audit_rows: String = recent_audit
            .iter()
            .map(|r| {
                format!(
                    r#"<tr><td class="date">{ts}</td><td><code>{action}</code></td><td>{target}</td><td>{actor}</td></tr>"#,
                    ts = r.timestamp,
                    action = r.action,
                    target = r.target,
                    actor = r.actor,
                )
            })
            .collect();

        let data = serde_json::to_string_pretty(&DashboardData {
            overview: overview.clone(),
            trends: trends.clone(),
            agents: agents.clone(),
            skills: skills.clone(),
            sessions_by_agent: sessions_by_agent.clone(),
            recent_audit: recent_audit.clone(),
        })
        .map_err(|e| {
            crate::error::AgentHubError::ManagementError(format!(
                "Failed to serialize dashboard data: {}",
                e
            ))
        })?;

        let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AgentHub Dashboard</title>
<style>
:root { --primary:#6750a4; --surface:#fef7ff; --surface2:#f3edf7; --on-surface:#1d1b20; --outline:#79747e; }
* { box-sizing:border-box; margin:0; padding:0; }
body { font-family:system-ui,-apple-system,'Segoe UI',Roboto,sans-serif; background:var(--surface); color:var(--on-surface); padding:32px; max-width:1200px; margin:0 auto; }
h1 { font-size:1.6rem; margin-bottom:4px; }
.subtitle { color:var(--outline); font-size:.85rem; margin-bottom:20px; }
.grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:16px; margin-bottom:20px; }
.card { background:var(--surface2); border-radius:16px; padding:20px; text-align:center; transition:transform .15s ease, box-shadow .15s ease; }
.clickable { cursor:pointer; }
.clickable:hover { transform:translateY(-2px); box-shadow:0 4px 12px rgba(0,0,0,.12); }
.card-value { font-size:1.8rem; font-weight:700; color:var(--primary); }
.card-label { font-size:.8rem; color:var(--outline); margin-top:4px; }
.toolbar { display:flex; gap:8px; align-items:center; margin-bottom:16px; flex-wrap:wrap; }
.toolbar .range-btn { border:1px solid var(--outline); background:transparent; color:var(--on-surface); border-radius:99px; padding:6px 14px; cursor:pointer; font-size:.85rem; }
.toolbar .range-btn.active { background:var(--primary); color:#fff; border-color:var(--primary); }
h2 { font-size:1.1rem; margin-bottom:12px; }
.charts { display:grid; grid-template-columns:repeat(auto-fit,minmax(320px,1fr)); gap:16px; margin-bottom:24px; }
.chart { background:var(--surface2); border-radius:16px; padding:16px; }
.chart-title { font-size:.85rem; color:var(--outline); margin-bottom:10px; }
.bars { display:flex; align-items:flex-end; gap:3px; height:120px; }
.bar { flex:1; min-width:3px; background:var(--primary); border-radius:2px 2px 0 0; position:relative; }
.bar:hover { background:#7d6bb3; }
.bar-val { display:none; position:absolute; top:-18px; left:50%; transform:translateX(-50%); font-size:.65rem; color:var(--on-surface); background:var(--surface2); border-radius:6px; padding:2px 6px; white-space:nowrap; }
.bar:hover .bar-val { display:block; }
table { width:100%; border-collapse:collapse; background:var(--surface2); border-radius:16px; overflow:hidden; }
th,td { padding:10px 14px; text-align:left; font-size:.85rem; border-bottom:1px solid #e6e0e9; }
th { background:#e8def8; color:var(--primary); user-select:none; }
th.sortable { cursor:pointer; }
th.sortable:hover { text-decoration:underline; }
td.date { font-variant-numeric:tabular-nums; white-space:nowrap; }
.meter { width:120px; height:8px; background:#e6e0e9; border-radius:99px; display:inline-block; vertical-align:middle; margin-right:8px; }
.meter-fill { height:100%; border-radius:99px; }
.meter-cost { background:#b3261e; }
.meter-tokens { background:#6750a4; }
.ok { color:#1a7f37; }
.miss { color:#b3261e; }
.drill { display:none; margin-bottom:24px; }
.drill.open { display:block; }
.drill-head { display:flex; align-items:center; gap:12px; margin-bottom:10px; }
.drill-head h2 { margin:0; }
.close-btn { border:none; background:var(--surface2); color:var(--on-surface); border-radius:99px; padding:4px 12px; cursor:pointer; font-size:.8rem; }
.empty { color:var(--outline); font-style:italic; padding:12px; }
@media (prefers-color-scheme: dark) { :root { --primary:#d0bcff; --surface:#141218; --surface2:#211f26; --on-surface:#e6e0e9; --outline:#938f99; } th { background:#4a4458; } .meter { background:#49454f; } .toolbar .range-btn.active { color:#141218; } }
</style>
</head>
<body>
<h1>AgentHub Dashboard</h1>
<div class="subtitle">__PLATFORM__ · v__VERSION__ · generated __GENERATED__ UTC · interactive — refresh by re-running <code>agenthub status --html</code></div>

<div class="grid">__CARDS__</div>

<div class="toolbar">
  <span style="font-size:.85rem;color:var(--outline)">Trend window:</span>
  <button class="range-btn" data-range="7">7 days</button>
  <button class="range-btn" data-range="30">30 days</button>
  <button class="range-btn" data-range="90">90 days</button>
  <span style="margin-left:auto;font-size:.8rem;color:var(--outline)">Click a card to drill down</span>
</div>

<div class="charts">__CHARTS__</div>

<h2>Daily trend</h2>
<table>
<thead><tr>
<th class="sortable" data-sort="date">Date</th>
<th class="sortable" data-sort="started">Started</th>
<th class="sortable" data-sort="completed">Completed</th>
<th class="sortable" data-sort="cost">Cost</th>
<th class="sortable" data-sort="tokens">Tokens</th>
<th class="sortable" data-sort="memories">Memories</th>
<th class="sortable" data-sort="audit">Audit</th>
</tr></thead>
<tbody id="trend-body">__ROWS__</tbody>
</table>

<div class="drill" id="drill-agents">
  <div class="drill-head"><h2>Catalog agents</h2><button class="close-btn">close</button></div>
  <table><thead><tr><th>Name</th><th>ID</th><th>Kind</th><th>Detection</th><th>Status</th></tr></thead><tbody>__AGENT_ROWS__</tbody></table>
</div>
<div class="drill" id="drill-sessions">
  <div class="drill-head"><h2>Sessions by agent</h2><button class="close-btn">close</button></div>
  <table><thead><tr><th>Agent</th><th>Sessions</th><th>Tokens</th><th>Cost</th></tr></thead><tbody>__SESSION_ROWS__</tbody></table>
</div>
<div class="drill" id="drill-skills">
  <div class="drill-head"><h2>Skills</h2><button class="close-btn">close</button></div>
  <table><thead><tr><th>Name</th><th>Version</th><th>State</th></tr></thead><tbody>__SKILL_ROWS__</tbody></table>
</div>
<div class="drill" id="drill-audit">
  <div class="drill-head"><h2>Recent audit events</h2><button class="close-btn">close</button></div>
  <table><thead><tr><th>Time</th><th>Action</th><th>Target</th><th>Actor</th></tr></thead><tbody>__AUDIT_ROWS__</tbody></table>
</div>

<script>
// Raw data embedded for tooling and future drilldowns.
window.__AGENTHUB_DASHBOARD__ = __DATA__;

(function () {
  var activeRange = __DEFAULT_DAYS__;
  var rangeBtns = document.querySelectorAll('.range-btn');
  var trendRows = document.querySelectorAll('#trend-body .trend-row');
  var charts = document.querySelectorAll('.chart');
  var sortState = { key: null, desc: false };

  function setRange(days) {
    activeRange = days;
    rangeBtns.forEach(function (b) { b.classList.toggle('active', Number(b.dataset.range) === days); });
    trendRows.forEach(function (r) { r.style.display = Number(r.dataset.days) <= days ? '' : 'none'; });
    charts.forEach(function (c) { c.style.display = Number(c.dataset.days) <= days ? '' : 'none'; });
    sortState.key = null;
  }
  rangeBtns.forEach(function (b) { b.addEventListener('click', function () { setRange(Number(b.dataset.range)); }); });

  // Card drilldowns.
  document.querySelectorAll('.card[data-drill]').forEach(function (card) {
    card.addEventListener('click', function () {
      var panel = document.getElementById('drill-' + card.dataset.drill);
      if (panel) panel.classList.add('open');
    });
  });
  document.querySelectorAll('.drill .close-btn').forEach(function (btn) {
    btn.addEventListener('click', function () { btn.closest('.drill').classList.remove('open'); });
  });

  // Sortable trend table (sorts only the visible rows).
  function textOf(cell) { return cell.textContent.trim().replace(/[$]/g, ''); }
  document.querySelectorAll('th.sortable').forEach(function (th) {
    th.addEventListener('click', function () {
      var key = th.dataset.sort;
      if (sortState.key === key) { sortState.desc = !sortState.desc; } else { sortState.key = key; sortState.desc = false; }
      var body = document.getElementById('trend-body');
      var visible = Array.prototype.filter.call(trendRows, function (r) { return r.style.display !== 'none'; });
      var idx = Array.prototype.indexOf.call(body.children, visible[0]);
      visible.sort(function (a, b) {
        var av = textOf(a.children[idx]);
        var bv = textOf(b.children[idx]);
        var an = parseFloat(av), bn = parseFloat(bv);
        var cmp = (isNaN(an) || isNaN(bn)) ? av.localeCompare(bv) : an - bn;
        return sortState.desc ? -cmp : cmp;
      });
      visible.forEach(function (r) { body.appendChild(r); });
    });
  });

  setRange(activeRange);
})();
</script>
</body>
</html>"#;

        Ok(html
            .replace("__CARDS__", &cards)
            .replace("__ROWS__", &rows)
            .replace("__CHARTS__", &charts)
            .replace("__AGENT_ROWS__", &agent_rows)
            .replace("__SESSION_ROWS__", &session_rows)
            .replace("__SKILL_ROWS__", &skill_rows)
            .replace("__AUDIT_ROWS__", &audit_rows)
            .replace("__DATA__", &data)
            .replace("__PLATFORM__", &overview.platform)
            .replace("__VERSION__", &overview.agenthub_version)
            .replace(
                "__GENERATED__",
                &overview
                    .generated_at
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            )
            .replace("__DEFAULT_DAYS__", &trend_days.to_string()))
    }
}

/// Serialized payload embedded in the dashboard HTML.
#[derive(Debug, Serialize)]
struct DashboardData {
    overview: StatusOverview,
    trends: Vec<TrendReport>,
    agents: Vec<DashboardAgentRow>,
    skills: Vec<DashboardSkillRow>,
    sessions_by_agent: Vec<AgentUsageRow>,
    recent_audit: Vec<AuditRow>,
}

#[derive(Debug, Clone, Serialize)]
struct TrendReport {
    days: usize,
    points: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Serialize)]
struct DashboardAgentRow {
    id: String,
    name: String,
    kind: String,
    installed: bool,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
struct DashboardSkillRow {
    name: String,
    version: String,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AgentUsageRow {
    agent: String,
    sessions: usize,
    tokens: u64,
    cost_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
struct AuditRow {
    timestamp: String,
    action: String,
    target: String,
    actor: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use tempfile::TempDir;

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
                    "windows": { "manager": "manual", "package": null }
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
                    "windows": { "manager": "manual", "package": null }
                },
                "status": "community",
                "catalog_verified_at": "2026-06-27",
                "installer_verified_at": null
            }
        ]
    }"#;

    #[test]
    fn test_overview_with_status() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().to_path_buf();

        // Populate some data
        ConfigManager::new(base.clone())
            .create_config("test-cli")
            .unwrap();
        PromptManager::new(base.join("prompts"))
            .create_prompt("p1", "P1", "d", "t")
            .unwrap();
        SessionManager::new(base.join("sessions"))
            .create_session("S1", "codex")
            .unwrap();
        MemoryManager::new(base.join("memory"))
            .create_entry(
                crate::memory::MemoryScope::Global,
                None,
                "Note",
                "c",
                crate::memory::MemoryType::Free,
            )
            .unwrap();
        AuditManager::new(base.join("audit"))
            .record("cli", "install", "test-cli", None, true)
            .unwrap();

        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();
        let report = OverviewReport::new(base, Platform::Windows);

        let statuses = vec![
            AgentStatus {
                agent_id: "test-cli".to_string(),
                installed: true,
                version: Some("1.2.3".to_string()),
                detection_method: "npm".to_string(),
            },
            AgentStatus {
                agent_id: "test-desktop".to_string(),
                installed: false,
                version: None,
                detection_method: "winget".to_string(),
            },
        ];

        let overview = report.overview_with_status(&catalog, &statuses).unwrap();
        assert_eq!(overview.catalog.total, 2);
        assert_eq!(overview.catalog.cli, 1);
        assert_eq!(overview.catalog.desktop, 1);
        assert_eq!(overview.catalog.verified, 1);
        assert_eq!(overview.catalog.community, 1);
        assert_eq!(overview.installed_agents, 1);
        assert_eq!(overview.configs, 1);
        assert_eq!(overview.prompts, 1);
        assert_eq!(overview.sessions.total, 1);
        assert_eq!(overview.memories.total, 1);
        assert_eq!(overview.audit_events, 1);
        assert_eq!(overview.platform, "Windows");
    }

    #[test]
    fn test_overview_empty_workspace() {
        let temp = TempDir::new().unwrap();
        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();
        let report = OverviewReport::new(temp.path().to_path_buf(), Platform::Linux);

        let overview = report.overview_with_status(&catalog, &[]).unwrap();
        assert_eq!(overview.installed_agents, 0);
        assert_eq!(overview.configs, 0);
        assert_eq!(overview.sessions.total, 0);
        assert_eq!(overview.audit_events, 0);
    }

    // ---- Trend ----

    #[test]
    fn test_trend_buckets_by_day() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().to_path_buf();
        let report = OverviewReport::new(base.clone(), Platform::Linux);
        let now = Utc::now();

        // Sessions started today and 2 days ago
        let sm = SessionManager::new(base.join("sessions"));
        let today_session = sm.create_session("Today", "codex").unwrap();
        sm.record_usage(
            &today_session.id,
            1_000_000,
            0,
            &crate::session::PricingTable::builtin(),
        )
        .unwrap();
        sm.update_status(&today_session.id, crate::session::SessionStatus::Completed)
            .unwrap();

        let old = now - chrono::Duration::days(2);
        let mut old_session = sm.create_session("Old", "codex").unwrap();
        old_session.started_at = old;
        old_session.usage = Some(crate::session::SessionUsage {
            total_tokens: 500_000,
            input_tokens: 500_000,
            output_tokens: 0,
            estimated_cost_usd: 1.0,
            calls: 0,
        });
        sm.save_session(&old_session).unwrap();

        // Memory created today
        MemoryManager::new(base.join("memory"))
            .create_entry(
                crate::memory::MemoryScope::Global,
                None,
                "Note",
                "c",
                crate::memory::MemoryType::Free,
            )
            .unwrap();

        // Audit event today
        AuditManager::new(base.join("audit"))
            .record("cli", "install", "x", None, true)
            .unwrap();

        let points = report.trend_with_now(7, now).unwrap();
        assert_eq!(points.len(), 7);

        let today_idx = 6; // last bucket = today
        assert_eq!(points[today_idx].sessions_started, 1);
        assert_eq!(points[today_idx].sessions_completed, 1);
        assert!((points[today_idx].cost_usd - 3.0).abs() < 1e-9); // fallback pricing $3/1M
        assert_eq!(points[today_idx].memories_created, 1);
        assert_eq!(points[today_idx].audit_events, 1);

        // 2 days ago: old session
        let old_idx = 4; // 7 buckets: idx 0..6, old is 2 days before today -> idx 4
        assert_eq!(points[old_idx].sessions_started, 1);
        assert!((points[old_idx].cost_usd - 1.0).abs() < 1e-9);
        assert_eq!(points[old_idx].tokens, 500_000);

        // Other days empty
        assert_eq!(points[0].sessions_started, 0);
    }

    // ---- HTML dashboard ----

    #[test]
    fn test_render_dashboard_html() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().to_path_buf();
        ConfigManager::new(base.clone())
            .create_config("test-cli")
            .unwrap();
        PromptManager::new(base.join("prompts"))
            .create_prompt("p1", "P1", "d", "t")
            .unwrap();
        let catalog = Catalog::from_json(TEST_AGENTS_JSON).unwrap();
        let report = OverviewReport::new(base, Platform::Linux);

        let html = report.render_dashboard_html(&catalog, 7).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("AgentHub Dashboard"));
        assert!(html.contains("Catalog agents"));
        assert!(html.contains("Daily trend"));
        assert!(html.contains("__AGENTHUB_DASHBOARD__"));
        // Interactive dashboard markers.
        assert!(html.contains("data-range=\"30\""), "range switcher");
        assert!(html.contains("data-drill=\"agents\""), "card drilldown");
        assert!(html.contains("id=\"drill-audit\""), "audit drill panel");
        assert!(html.contains("data-chart=\"cost\""), "cost chart");
        assert!(html.contains("class=\"sortable\""), "sortable table");
        assert!(html.contains("\"trends\""), "trend windows payload");
        assert!(html.contains("\"agents\""), "agents payload");
        assert!(html.contains("\"recent_audit\""), "audit payload");
        assert!(
            html.contains("window.__AGENTHUB_DASHBOARD__ = {"),
            "embedded json"
        );
    }
}
