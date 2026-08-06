pub mod agent;
pub mod audit;
pub mod backup;
pub mod catalog;
pub mod command_builder;
pub mod config;
pub mod diagnostic;
pub mod error;
pub mod installer;
pub mod management;
pub mod memory;
pub mod prompt;
pub mod session;
pub mod skill;
pub mod status;

pub use agent::{Agent, AgentKind, InstallerConfig, PackageManager, Platform, SupportStatus};
pub use audit::{AuditEvent, AuditManager, AuditQuery};
pub use backup::{BackupCounts, BackupData, BackupManager, BackupManifest};
pub use catalog::Catalog;
pub use command_builder::{
    CommandBuilder, CommandOutput, CommandRunner, MockCommandRunner, MockResponse,
    RealCommandRunner,
};
pub use config::{AgentConfig, ConfigManager, ConfigValue, Environment};
pub use diagnostic::{CheckStatus, DiagnosticCheck, DiagnosticManager, DiagnosticReport};
pub use error::{AgentHubError, Result};
pub use installer::Installer;
pub use management::{CatalogOverview, ManagementReport, StatusOverview};
pub use memory::{MemoryEntry, MemoryManager, MemoryScope, MemoryStats, MemoryType};
pub use prompt::{PromptManager, PromptTemplate, PromptUsage, PromptVariable, PromptVersion};
pub use session::{
    ModelPricing, PricingTable, Session, SessionManager, SessionStats, SessionStatus,
    SessionTemplate, TemplateMessage,
};
pub use skill::{Skill, SkillManager, SkillManifest};
pub use status::{AgentStatus, StatusDetector};
