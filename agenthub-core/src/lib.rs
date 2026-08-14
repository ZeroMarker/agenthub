pub mod agent;
pub mod audit;
pub mod backup;
pub mod catalog;
pub mod command_builder;
pub mod community;
pub mod config;
pub mod diagnostic;
pub mod error;
pub mod graph;
pub mod installer;
pub mod marketplace;
pub mod memory;
pub mod monitor;
pub mod notify;
pub mod overview;
pub mod plugin;
pub mod prompt;
pub mod secrets;
pub mod session;
pub mod skill;
pub mod status;
mod storage;
pub mod users;
pub mod workflow;

pub use agent::{Agent, AgentKind, InstallerConfig, PackageManager, Platform, SupportStatus};
pub use audit::{AuditEvent, AuditManager, AuditQuery};
pub use backup::{BackupCounts, BackupData, BackupManager, BackupManifest};
pub use catalog::Catalog;
pub use command_builder::{
    CommandBuilder, CommandOutput, CommandRunner, MockCommandRunner, MockResponse,
    RealCommandRunner,
};
pub use community::{CommunityManager, CommunityPrompt};
pub use config::{
    normalize_settings, validate_config, validate_settings, AgentConfig, ConfigIssue,
    ConfigManager, ConfigTemplate, ConfigValue, Environment, IssueSeverity,
};
pub use diagnostic::{CheckStatus, DiagnosticCheck, DiagnosticManager, DiagnosticReport};
pub use error::{AgentHubError, Result};
pub use graph::{
    EntityKind, GraphEdge, GraphNode, GraphSummary, KnowledgeGraph, KnowledgeGraphBuilder,
};
pub use installer::Installer;
pub use marketplace::{MarketplaceManager, MarketplaceSkill, MarketplaceStats, SkillRating};
pub use memory::{
    cosine_similarity, embed_text, MemoryEntry, MemoryManager, MemoryMatch, MemoryScope,
    MemoryStats, MemoryType, VectorIndex, VectorIndexEntry, VectorIndexSummary, EMBEDDING_DIM,
};
pub use monitor::{AlertSeverity, Monitor, MonitorReport};
pub use notify::{
    ChannelConfig, ChannelResult, NotificationPayload, Notifier, NotifyChannel, SmtpConfig,
};
pub use overview::{CatalogOverview, OverviewReport, StatusOverview, TrendPoint};
pub use plugin::{
    Plugin, PluginHook, PluginManager, PluginManifest, PluginRunResult, HOOK_BACKUP, HOOK_INSTALL,
    HOOK_MONITOR, HOOK_SESSION_END, HOOK_UNINSTALL,
};
pub use prompt::{
    ImportSummary, PromptEffects, PromptExportBundle, PromptExtraction, PromptManager,
    PromptOutcome, PromptTemplate, PromptUsage, PromptVariable, PromptVersion,
};
pub use secrets::{
    PreviousSecret, RotationResult as SecretRotationResult, SecretEntry, SecretInfo, SecretStore,
};
pub use session::{
    BudgetConfig, BudgetReport, ContextMessage, ModelPricing, PricingTable, Session,
    SessionContext, SessionManager, SessionStats, SessionStatus, SessionTemplate, SessionUsage,
    SessionUsageAggregate, SessionUsageRow, TemplateMessage, UsageExport, UsageTrendPoint,
};
pub use skill::{Skill, SkillCompatibility, SkillManager, SkillManifest};
pub use status::{AgentStatus, StatusDetector};
pub use users::{Permission, User, UserManager, ROLE_ADMIN, ROLE_OPERATOR, ROLE_VIEWER};
pub use workflow::{
    Workflow, WorkflowManager, WorkflowRunReport, WorkflowStep, WorkflowStepResult,
};
