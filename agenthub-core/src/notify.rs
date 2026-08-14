//! Cross-cutting alert notification channels (webhook / email spool / file).
//!
//! Channels are configured in `<config-dir>/notify.yaml` and consumed by the
//! monitor (`MonitorReport`) and any other alert producer. Webhook channels
//! POST a JSON payload to an HTTP(S) endpoint (via `ureq`); email channels
//! write RFC-2822 `.eml` messages to a local outbox spool (delivery to an MTA
//! is out of scope — the spool can be picked up by `msmtp`/`sendmail` or a
//! mail client); file channels append to a log file.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::monitor::{AlertSeverity, MonitorReport};

/// Kind-specific channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ChannelConfig {
    /// POST a JSON payload to an HTTP(S) webhook URL.
    Webhook {
        url: String,
        /// Optional extra headers serialized as "K: V" pairs.
        #[serde(default)]
        headers: Vec<String>,
    },
    /// Write an RFC-2822 message to the local outbox spool.
    Email {
        to: String,
        from: String,
        #[serde(default)]
        subject_prefix: Option<String>,
    },
    /// Append alert lines to a file.
    File { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyChannel {
    pub id: String,
    #[serde(flatten)]
    pub config: ChannelConfig,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Minimum alert severity this channel delivers (info|warning|critical).
    #[serde(default = "default_min_severity")]
    pub min_severity: String,
    /// Skip re-sending an identical alert within this window (minutes).
    #[serde(default = "default_dedup_minutes")]
    pub dedup_minutes: u64,
    pub created_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

fn default_min_severity() -> String {
    "info".to_string()
}

fn default_dedup_minutes() -> u64 {
    15
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct NotifyConfigFile {
    #[serde(default)]
    channels: Vec<NotifyChannel>,
}

/// Outcome of delivering one notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResult {
    pub channel: String,
    pub kind: String,
    pub ok: bool,
    pub message: String,
}

/// Manages notification channels and delivers alert payloads.
pub struct Notifier {
    base_dir: PathBuf,
}

impl Notifier {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    fn config_path(&self) -> PathBuf {
        self.base_dir.join("notify.yaml")
    }

    pub fn outbox_dir(&self) -> PathBuf {
        self.base_dir.join("notifications").join("outbox")
    }

    fn load_config(&self) -> Result<NotifyConfigFile> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(NotifyConfigFile::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to read notify config: {}", e))
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to parse notify config: {}", e))
        })
    }

    fn save_config(&self, config: &NotifyConfigFile) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to create config dir: {}", e))
        })?;
        let content = serde_yaml::to_string(config).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to serialize notify config: {}", e))
        })?;
        std::fs::write(self.config_path(), content).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to write notify config: {}", e))
        })?;
        Ok(())
    }

    pub fn list_channels(&self) -> Result<Vec<NotifyChannel>> {
        let config = self.load_config()?;
        let mut channels = config.channels;
        channels.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(channels)
    }

    /// Add a channel. Validates webhook URLs (must be http/https) and file
    /// paths (relative paths are resolved against the config dir).
    pub fn add_channel(&self, id: &str, config: ChannelConfig) -> Result<NotifyChannel> {
        self.add_channel_with_options(id, config, None, None)
    }

    /// Add a channel with explicit severity/dedup settings.
    pub fn add_channel_with_options(
        &self,
        id: &str,
        config: ChannelConfig,
        min_severity: Option<&str>,
        dedup_minutes: Option<u64>,
    ) -> Result<NotifyChannel> {
        if id.is_empty() {
            return Err(AgentHubError::ManagementError(
                "Channel id must not be empty".to_string(),
            ));
        }
        if let Some(sev) = min_severity {
            sev.parse::<AlertSeverity>().map_err(|e| {
                AgentHubError::ManagementError(format!("Invalid min_severity: {}", e))
            })?;
        }
        if let ChannelConfig::Webhook { url, .. } = &config {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(AgentHubError::ManagementError(format!(
                    "Invalid webhook URL '{}' (expected http:// or https://)",
                    url
                )));
            }
        }
        let mut file = self.load_config()?;
        if file.channels.iter().any(|c| c.id == id) {
            return Err(AgentHubError::ManagementError(format!(
                "Channel already exists: {}",
                id
            )));
        }
        let channel = NotifyChannel {
            id: id.to_string(),
            config,
            enabled: true,
            min_severity: min_severity.unwrap_or("info").to_string(),
            dedup_minutes: dedup_minutes.unwrap_or(default_dedup_minutes()),
            created_at: Utc::now(),
        };
        file.channels.push(channel.clone());
        self.save_config(&file)?;
        Ok(channel)
    }

    pub fn remove_channel(&self, id: &str) -> Result<bool> {
        let mut file = self.load_config()?;
        let before = file.channels.len();
        file.channels.retain(|c| c.id != id);
        if file.channels.len() == before {
            return Ok(false);
        }
        self.save_config(&file)?;
        Ok(true)
    }

    pub fn set_channel_enabled(&self, id: &str, enabled: bool) -> Result<NotifyChannel> {
        let mut file = self.load_config()?;
        let channel = file
            .channels
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| AgentHubError::ManagementError(format!("Channel not found: {}", id)))?;
        channel.enabled = enabled;
        let channel = channel.clone();
        self.save_config(&file)?;
        Ok(channel)
    }

    /// Import a full channel list (used by backup restore).
    pub fn import_channels(&self, channels: &[NotifyChannel]) -> Result<()> {
        let file = NotifyConfigFile {
            channels: channels.to_vec(),
        };
        self.save_config(&file)
    }

    /// Deliver an alert for a monitor report to every enabled channel.
    pub fn send(&self, report: &MonitorReport, force: bool) -> Result<Vec<ChannelResult>> {
        let payload = NotificationPayload::from_report(report);
        let mut results = Vec::new();
        for channel in self.load_config()?.channels.iter().filter(|c| c.enabled) {
            results.push(self.deliver(channel, &payload, force));
        }
        Ok(results)
    }

    /// Send only to a single named channel (enabled or not).
    pub fn send_to(
        &self,
        channel_id: &str,
        report: &MonitorReport,
        force: bool,
    ) -> Result<ChannelResult> {
        let config = self.load_config()?;
        let channel = config
            .channels
            .iter()
            .find(|c| c.id == channel_id)
            .ok_or_else(|| {
                AgentHubError::ManagementError(format!("Channel not found: {}", channel_id))
            })?;
        let payload = NotificationPayload::from_report(report);
        Ok(self.deliver(channel, &payload, force))
    }

    /// Deliver a custom alert (not tied to a monitor report), e.g. key
    /// rotation notifications. Respects severity and dedup like `send`.
    pub fn send_custom(
        &self,
        summary: &str,
        severity: AlertSeverity,
        data: serde_json::Value,
        force: bool,
    ) -> Result<Vec<ChannelResult>> {
        let payload = NotificationPayload::custom(summary, severity, data);
        let mut results = Vec::new();
        for channel in self.load_config()?.channels.iter().filter(|c| c.enabled) {
            results.push(self.deliver(channel, &payload, force));
        }
        Ok(results)
    }

    fn deliver(
        &self,
        channel: &NotifyChannel,
        payload: &NotificationPayload,
        force: bool,
    ) -> ChannelResult {
        // Minimum severity filter.
        let min_severity = channel
            .min_severity
            .parse::<AlertSeverity>()
            .unwrap_or(AlertSeverity::Info);
        let severity = payload
            .severity
            .parse::<AlertSeverity>()
            .unwrap_or(AlertSeverity::Info);
        if severity < min_severity {
            return ChannelResult {
                channel: channel.id.clone(),
                kind: channel_kind(&channel.config),
                ok: true,
                message: format!("skipped (severity {} < min {})", severity, min_severity),
            };
        }

        // Dedup window.
        if !force && channel.dedup_minutes > 0 && self.is_duplicate(channel, payload) {
            return ChannelResult {
                channel: channel.id.clone(),
                kind: channel_kind(&channel.config),
                ok: true,
                message: format!(
                    "skipped (dedup: identical alert within {}m)",
                    channel.dedup_minutes
                ),
            };
        }

        let result = match &channel.config {
            ChannelConfig::Webhook { url, headers } => {
                let payload_json = serde_json::to_value(payload).unwrap_or_default();
                match send_webhook(url, headers, &payload_json) {
                    Ok(message) => ChannelResult {
                        channel: channel.id.clone(),
                        kind: "webhook".to_string(),
                        ok: true,
                        message,
                    },
                    Err(message) => ChannelResult {
                        channel: channel.id.clone(),
                        kind: "webhook".to_string(),
                        ok: false,
                        message,
                    },
                }
            }
            ChannelConfig::Email {
                to,
                from,
                subject_prefix,
            } => {
                match self.write_email_spool(channel, to, from, subject_prefix.as_deref(), payload)
                {
                    Ok(path) => ChannelResult {
                        channel: channel.id.clone(),
                        kind: "email".to_string(),
                        ok: true,
                        message: format!("spooled to {}", path.display()),
                    },
                    Err(e) => ChannelResult {
                        channel: channel.id.clone(),
                        kind: "email".to_string(),
                        ok: false,
                        message: e.to_string(),
                    },
                }
            }
            ChannelConfig::File { path } => {
                let target = self.resolve_path(path);
                let line = format!(
                    "[{}] [{}] {}",
                    Utc::now().to_rfc3339(),
                    payload.severity,
                    payload.summary
                );
                match append_line(&target, &line) {
                    Ok(()) => ChannelResult {
                        channel: channel.id.clone(),
                        kind: "file".to_string(),
                        ok: true,
                        message: format!("appended to {}", target.display()),
                    },
                    Err(e) => ChannelResult {
                        channel: channel.id.clone(),
                        kind: "file".to_string(),
                        ok: false,
                        message: e.to_string(),
                    },
                }
            }
        };

        if result.ok {
            let _ = self.record_sent(channel, payload);
        }
        result
    }

    // ---- dedup state ----

    fn state_path(&self) -> PathBuf {
        self.base_dir.join("notify_state.json")
    }

    fn load_state(&self) -> NotifyState {
        let path = self.state_path();
        if !path.exists() {
            return NotifyState::default();
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save_state(&self, state: &NotifyState) {
        let _ = std::fs::create_dir_all(&self.base_dir);
        if let Ok(content) = serde_json::to_string_pretty(state) {
            let _ = std::fs::write(self.state_path(), content);
        }
    }

    fn signature(channel: &NotifyChannel, payload: &NotificationPayload) -> String {
        format!("{}:{}", channel.id, payload.summary)
    }

    fn is_duplicate(&self, channel: &NotifyChannel, payload: &NotificationPayload) -> bool {
        let state = self.load_state();
        let Some(last) = state.last_sent.get(&Self::signature(channel, payload)) else {
            return false;
        };
        let window = chrono::Duration::minutes(channel.dedup_minutes as i64);
        Utc::now().signed_duration_since(*last) < window
    }

    fn record_sent(&self, channel: &NotifyChannel, payload: &NotificationPayload) -> Result<()> {
        let mut state = self.load_state();
        state
            .last_sent
            .insert(Self::signature(channel, payload), Utc::now());
        self.save_state(&state);
        Ok(())
    }

    /// Clear the dedup state (e.g. after a manual `--force` run).
    pub fn clear_dedup_state(&self) -> Result<()> {
        let path = self.state_path();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::ManagementError(format!("Failed to clear dedup state: {}", e))
            })?;
        }
        Ok(())
    }

    /// Relative paths in file channels resolve against the config dir.
    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.base_dir.join(p)
        }
    }

    fn write_email_spool(
        &self,
        channel: &NotifyChannel,
        to: &str,
        from: &str,
        subject_prefix: Option<&str>,
        payload: &NotificationPayload,
    ) -> Result<PathBuf> {
        let dir = self.outbox_dir();
        std::fs::create_dir_all(&dir).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to create outbox dir: {}", e))
        })?;
        let now = Utc::now();
        let filename = format!(
            "{}-{}.eml",
            now.format("%Y%m%d-%H%M%S"),
            sanitize(&channel.id)
        );
        let path = dir.join(filename);

        let subject = format!(
            "{}AgentHub alert [{}] — {}",
            subject_prefix.unwrap_or_default(),
            payload.severity,
            payload.summary
        );
        let payload_json =
            serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string());
        let body = format!(
            "AgentHub alert notification\n\nSeverity: {}\nSummary: {}\nHealthy: {}\n\nPayload:\n{}\n",
            payload.severity, payload.summary, payload.healthy, payload_json
        );
        let message = format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nContent-Type: text/plain; charset=utf-8\r\nMIME-Version: 1.0\r\n\r\n{}",
            from,
            to,
            subject,
            now.to_rfc3339(),
            body
        );
        std::fs::write(&path, message).map_err(|e| {
            AgentHubError::ManagementError(format!("Failed to write email spool: {}", e))
        })?;
        Ok(path)
    }
}

fn channel_kind(config: &ChannelConfig) -> String {
    match config {
        ChannelConfig::Webhook { .. } => "webhook".to_string(),
        ChannelConfig::Email { .. } => "email".to_string(),
        ChannelConfig::File { .. } => "file".to_string(),
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", line)
}

/// Persisted dedup state: channel+summary signature -> last sent time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct NotifyState {
    #[serde(default)]
    last_sent: std::collections::HashMap<String, DateTime<Utc>>,
}

/// The JSON payload delivered to webhooks / embedded in emails & files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub summary: String,
    pub healthy: bool,
    pub severity: String,
    pub generated_at: DateTime<Utc>,
    /// The originating monitor report, when the alert came from `monitor`.
    #[serde(default)]
    pub report: Option<MonitorReport>,
    /// Arbitrary extra data (e.g. rotation details).
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

impl NotificationPayload {
    pub fn from_report(report: &MonitorReport) -> Self {
        Self {
            summary: report.alert_summary(),
            healthy: report.healthy,
            severity: report.severity().to_string(),
            generated_at: report.generated_at,
            report: Some(report.clone()),
            data: None,
        }
    }

    pub fn custom(summary: &str, severity: AlertSeverity, data: serde_json::Value) -> Self {
        Self {
            summary: summary.to_string(),
            healthy: severity != AlertSeverity::Critical,
            severity: severity.to_string(),
            generated_at: Utc::now(),
            report: None,
            data: Some(data),
        }
    }
}

/// POST a JSON payload to an HTTP(S) webhook with a bounded timeout.
pub fn send_webhook(
    url: &str,
    headers: &[String],
    payload: &serde_json::Value,
) -> std::result::Result<String, String> {
    let mut builder = ureq::post(url)
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .timeout_connect(Some(std::time::Duration::from_secs(5)))
        .build();
    builder = builder.header("Content-Type", "application/json");
    for h in headers {
        if let Some((k, v)) = h.split_once(':') {
            builder = builder.header(k.trim(), v.trim());
        }
    }
    match builder.send_json(payload) {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if (200..300).contains(&status) {
                Ok(format!("HTTP {}", status))
            } else {
                Err(format!("HTTP {}", status))
            }
        }
        Err(ureq::Error::StatusCode(code)) => Err(format!("HTTP {}", code)),
        Err(ureq::Error::Timeout(_)) => Err("request timed out".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::diagnostic::{
        CheckStatus, DiagnosticCheck, DiagnosticReport, DiagnosticSummary, SystemInfo,
    };
    use crate::monitor::Monitor;
    use crate::session::{BudgetConfig, BudgetReport};
    use tempfile::TempDir;

    fn sample_report() -> MonitorReport {
        MonitorReport {
            generated_at: Utc::now(),
            agenthub_version: "1.2.0".to_string(),
            platform: "Linux".to_string(),
            healthy: false,
            warnings: vec!["2 diagnostic check(s) failed".to_string()],
            installed_agents: 1,
            missing_agents: vec!["Codex".to_string()],
            budget: BudgetReport {
                daily_limit_usd: Some(5.0),
                daily_spent_usd: 6.5,
                monthly_limit_usd: None,
                monthly_spent_usd: 6.5,
                total_tokens_today: 100,
                alerts: vec!["Daily budget exceeded".to_string()],
            },
            incompatible_skills: Vec::new(),
            diagnostics_passed: 3,
            diagnostics_warnings: 0,
            diagnostics_failed: 2,
        }
    }

    #[test]
    fn test_channel_crud() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("config");
        let notifier = Notifier::new(base.clone());

        notifier
            .add_channel(
                "ops",
                ChannelConfig::Webhook {
                    url: "https://example.com/hook".to_string(),
                    headers: vec!["X-Token: abc".to_string()],
                },
            )
            .unwrap();
        notifier
            .add_channel(
                "alerts-file",
                ChannelConfig::File {
                    path: "alerts.log".to_string(),
                },
            )
            .unwrap();
        notifier
            .add_channel(
                "team",
                ChannelConfig::Email {
                    to: "team@example.com".to_string(),
                    from: "agenthub@example.com".to_string(),
                    subject_prefix: Some("[AGENTHUB] ".to_string()),
                },
            )
            .unwrap();

        // Invalid webhook URL rejected
        assert!(notifier
            .add_channel(
                "bad",
                ChannelConfig::Webhook {
                    url: "ftp://x".to_string(),
                    headers: vec![]
                }
            )
            .is_err());

        // Duplicate id rejected
        assert!(notifier
            .add_channel(
                "ops",
                ChannelConfig::Webhook {
                    url: "https://x".to_string(),
                    headers: vec![]
                }
            )
            .is_err());

        let channels = notifier.list_channels().unwrap();
        assert_eq!(channels.len(), 3);

        // Enable/disable
        notifier.set_channel_enabled("ops", false).unwrap();
        assert!(
            !notifier
                .list_channels()
                .unwrap()
                .iter()
                .find(|c| c.id == "ops")
                .unwrap()
                .enabled
        );

        assert!(notifier.remove_channel("ops").unwrap());
        assert!(!notifier.remove_channel("ops").unwrap());
    }

    #[test]
    fn test_send_to_file_channel() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("config");
        let notifier = Notifier::new(base.clone());
        notifier
            .add_channel(
                "log",
                ChannelConfig::File {
                    path: "alerts.log".to_string(),
                },
            )
            .unwrap();

        let report = sample_report();
        let results = notifier.send(&report, true).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ok, "{:?}", results[0]);
        assert_eq!(results[0].kind, "file");

        let content = std::fs::read_to_string(base.join("alerts.log")).unwrap();
        assert!(content.contains("WARN"));

        // Disabled channels are skipped
        notifier.set_channel_enabled("log", false).unwrap();
        assert!(notifier.send(&report, true).unwrap().is_empty());
    }

    #[test]
    fn test_send_to_email_spool() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("config");
        let notifier = Notifier::new(base.clone());
        notifier
            .add_channel(
                "team",
                ChannelConfig::Email {
                    to: "team@example.com".to_string(),
                    from: "agenthub@example.com".to_string(),
                    subject_prefix: Some("[AGENTHUB] ".to_string()),
                },
            )
            .unwrap();

        let report = sample_report();
        let results = notifier.send(&report, true).unwrap();
        assert!(results[0].ok, "{:?}", results[0]);

        let dir = notifier.outbox_dir();
        let files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 1);
        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("To: team@example.com"));
        assert!(content.contains("Subject: [AGENTHUB] AgentHub alert"));
        assert!(content.contains("Daily budget exceeded"));
    }

    #[test]
    fn test_webhook_validation() {
        let temp = TempDir::new().unwrap();
        let notifier = Notifier::new(temp.path().to_path_buf());
        assert!(notifier
            .add_channel(
                "ok",
                ChannelConfig::Webhook {
                    url: "https://h.example.com/x".to_string(),
                    headers: vec![]
                }
            )
            .is_ok());
        assert!(notifier
            .add_channel(
                "nope",
                ChannelConfig::Webhook {
                    url: "not-a-url".to_string(),
                    headers: vec![]
                }
            )
            .is_err());
    }

    #[test]
    fn test_payload_serialization() {
        let payload = NotificationPayload::from_report(&sample_report());
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"healthy\":false"));
        assert!(json.contains("\"severity\":\"critical\""));
        assert!(json.contains("Daily budget exceeded"));
    }

    #[test]
    fn test_severity_filtering() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("config");
        let notifier = Notifier::new(base.clone());
        notifier
            .add_channel(
                "warn-only",
                ChannelConfig::File {
                    path: "warn.log".to_string(),
                },
            )
            .unwrap();
        // Bump the channel's minimum severity to warning.
        let mut file: NotifyConfigFile =
            serde_yaml::from_str(&std::fs::read_to_string(base.join("notify.yaml")).unwrap())
                .unwrap();
        file.channels[0].min_severity = "warning".to_string();
        std::fs::write(
            base.join("notify.yaml"),
            serde_yaml::to_string(&file).unwrap(),
        )
        .unwrap();

        // The sample report is critical, so it is delivered.
        let results = notifier.send(&sample_report(), false).unwrap();
        assert!(results[0].ok);
        assert!(!results[0].message.contains("skipped"));

        // An info-severity report is skipped.
        let mut info_report = sample_report();
        info_report.healthy = true;
        info_report.warnings.clear();
        info_report.diagnostics_failed = 0;
        info_report.budget.alerts.clear();
        let results = notifier.send(&info_report, false).unwrap();
        assert!(results[0].ok);
        assert!(results[0].message.contains("skipped (severity"));
    }

    #[test]
    fn test_dedup_window() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("config");
        let notifier = Notifier::new(base.clone());
        notifier
            .add_channel(
                "log",
                ChannelConfig::File {
                    path: "alerts.log".to_string(),
                },
            )
            .unwrap();
        // Shrink dedup window to 1 minute (default is 15).
        let mut file: NotifyConfigFile =
            serde_yaml::from_str(&std::fs::read_to_string(base.join("notify.yaml")).unwrap())
                .unwrap();
        file.channels[0].dedup_minutes = 1;
        std::fs::write(
            base.join("notify.yaml"),
            serde_yaml::to_string(&file).unwrap(),
        )
        .unwrap();

        let report = sample_report();
        // First send delivers.
        let results = notifier.send(&report, false).unwrap();
        assert!(results[0].message.contains("appended"));
        // Second identical send within the window is deduped.
        let results = notifier.send(&report, false).unwrap();
        assert!(results[0].message.contains("dedup"));
        // Forcing bypasses the window.
        let results = notifier.send(&report, true).unwrap();
        assert!(results[0].message.contains("appended"));

        // State file exists.
        assert!(base.join("notify_state.json").exists());
        notifier.clear_dedup_state().unwrap();
        assert!(!base.join("notify_state.json").exists());
    }

    #[test]
    fn test_send_custom_payload() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("config");
        let notifier = Notifier::new(base.clone());
        notifier
            .add_channel(
                "log",
                ChannelConfig::File {
                    path: "events.log".to_string(),
                },
            )
            .unwrap();
        let results = notifier
            .send_custom(
                "API key rotated for agent-a",
                AlertSeverity::Warning,
                serde_json::json!({"agent": "agent-a", "key": "api_key"}),
                true,
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        let content = std::fs::read_to_string(base.join("events.log")).unwrap();
        assert!(content.contains("warning"));
        assert!(content.contains("API key rotated"));
    }

    // Ensure the test-catalog type compiles against the real API surface used
    // by Monitor (keeps notify.rs dependency on monitor honest).
    #[allow(dead_code)]
    fn _compile_check(base: &Path) {
        let _ = Monitor::new(base.to_path_buf(), crate::agent::Platform::Linux);
        let _ = Catalog::from_json("{}");
        let _ = DiagnosticReport {
            timestamp: "t".to_string(),
            platform: "p".to_string(),
            checks: vec![DiagnosticCheck {
                name: "n".to_string(),
                category: "c".to_string(),
                status: CheckStatus::Passed,
                message: "m".to_string(),
                details: None,
                duration_ms: 0,
            }],
            summary: DiagnosticSummary {
                total: 1,
                passed: 1,
                warnings: 0,
                failed: 0,
                skipped: 0,
                duration_ms: 0,
            },
            system_info: SystemInfo {
                os: "o".to_string(),
                arch: "a".to_string(),
                hostname: "h".to_string(),
                rust_version: None,
                node_version: None,
                npm_version: None,
                cargo_version: None,
            },
        };
        let _ = BudgetConfig {
            daily_usd: None,
            monthly_usd: None,
        };
    }

    #[test]
    fn test_list_channels_corrupt_config_errors() {
        let temp = TempDir::new().unwrap();
        let notifier = Notifier::new(temp.path().to_path_buf());
        std::fs::write(temp.path().join("notify.yaml"), "channels: [unterminated").unwrap();
        assert!(notifier.list_channels().is_err());
    }
}
