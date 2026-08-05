use crate::agent::{Agent, Platform};
use crate::command_builder::{CommandOutput, CommandRunner, RealCommandRunner};
use crate::error::{AgentHubError, Result};
use std::time::Duration;

/// Result of an install or uninstall operation.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub agent_id: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

/// Preview of a command that would be executed.
#[derive(Debug, Clone)]
pub struct CommandPreview {
    pub command: String,
    pub description: String,
    pub platform: Platform,
}

/// Package installation and uninstallation engine.
///
/// Delegates command generation and execution to a `CommandRunner`,
/// enabling mocking in tests and consistent platform-aware behavior.
pub struct Installer {
    platform: Platform,
    runner: Box<dyn CommandRunner>,
}

impl Installer {
    pub fn new(platform: Platform, runner: Box<dyn CommandRunner>) -> Self {
        Self { platform, runner }
    }

    /// Create a default installer using the real command runner.
    pub fn new_default(platform: Platform) -> Self {
        Self {
            platform,
            runner: Box::new(RealCommandRunner::new(platform)),
        }
    }

    /// Get a preview of the install command for an agent.
    pub fn get_command_preview(&self, agent: &Agent, uninstall: bool) -> Option<CommandPreview> {
        let installer = agent.get_installer(self.platform)?;
        let package = installer.package.as_ref()?;

        let action = if uninstall { "uninstall" } else { "install" };
        let command = if uninstall {
            self.runner.uninstall_command(&installer.manager, package)
        } else {
            self.runner.install_command(&installer.manager, package)
        }?;

        Some(CommandPreview {
            command,
            description: format!(
                "{} {} via {:?}",
                action, agent.name, installer.manager
            ),
            platform: self.platform,
        })
    }

    /// Build the full command string for installation.
    fn build_install_command(&self, agent: &Agent) -> Result<String> {
        let installer = agent.get_installer(self.platform).ok_or_else(|| {
            AgentHubError::InstallerError(format!("No installer for {}", agent.name))
        })?;
        let package = installer.package.as_ref().ok_or_else(|| {
            AgentHubError::InstallerError(format!(
                "No package defined for {} on {:?}",
                agent.name, self.platform
            ))
        })?;
        self.runner
            .install_command(&installer.manager, package)
            .ok_or_else(|| {
                AgentHubError::InstallerError(format!(
                    "{} ({:?}) is not installable via automated tools",
                    agent.name, installer.manager
                ))
            })
    }

    /// Build the full command string for uninstallation.
    fn build_uninstall_command(&self, agent: &Agent) -> Result<String> {
        let installer = agent.get_installer(self.platform).ok_or_else(|| {
            AgentHubError::InstallerError(format!("No installer for {}", agent.name))
        })?;
        let package = installer.package.as_ref().ok_or_else(|| {
            AgentHubError::InstallerError(format!(
                "No package defined for {} on {:?}",
                agent.name, self.platform
            ))
        })?;
        self.runner
            .uninstall_command(&installer.manager, package)
            .ok_or_else(|| {
                AgentHubError::InstallerError(format!(
                    "{} ({:?}) is not uninstallable via automated tools",
                    agent.name, installer.manager
                ))
            })
    }

    /// Execute agent installation.
    ///
    /// * `dry_run` — if true, returns a successful result without executing anything
    /// * `timeout` — optional maximum duration; `None` means no timeout
    pub fn execute_install(
        &self,
        agent: &Agent,
        dry_run: bool,
        timeout: Option<Duration>,
    ) -> Result<InstallResult> {
        let command = self.build_install_command(agent)?;

        if dry_run {
            return Ok(InstallResult {
                success: true,
                message: format!("Dry run: {}", command),
                agent_id: agent.id.clone(),
                command,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            });
        }

        let output = self.runner.run_command(&command, timeout)?;
        let message = Self::format_message(
            &output,
            agent.name.as_str(),
            "install",
        );

        Ok(InstallResult {
            success: output.success,
            message,
            agent_id: agent.id.clone(),
            command,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms: output.duration_ms,
            timed_out: output.timed_out,
        })
    }

    /// Execute agent uninstallation.
    ///
    /// * `dry_run` — if true, returns a successful result without executing anything
    /// * `timeout` — optional maximum duration; `None` means no timeout
    pub fn execute_uninstall(
        &self,
        agent: &Agent,
        dry_run: bool,
        timeout: Option<Duration>,
    ) -> Result<InstallResult> {
        let command = self.build_uninstall_command(agent)?;

        if dry_run {
            return Ok(InstallResult {
                success: true,
                message: format!("Dry run: {}", command),
                agent_id: agent.id.clone(),
                command,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            });
        }

        let output = self.runner.run_command(&command, timeout)?;
        let message = Self::format_message(
            &output,
            agent.name.as_str(),
            "uninstall",
        );

        Ok(InstallResult {
            success: output.success,
            message,
            agent_id: agent.id.clone(),
            command,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms: output.duration_ms,
            timed_out: output.timed_out,
        })
    }

    fn format_message(output: &CommandOutput, agent_name: &str, action: &str) -> String {
        if output.timed_out {
            format!("{} {} timed out after {}ms", agent_name, action, output.duration_ms)
        } else if output.success {
            format!("{} {}d successfully", agent_name, action)
        } else {
            let detail = if output.stderr.is_empty() {
                &output.stdout
            } else {
                &output.stderr
            };
            if detail.is_empty() {
                format!("Failed to {} {}", action, agent_name)
            } else {
                format!("Failed to {} {}: {}", action, agent_name, detail)
            }
        }
    }

    /// Install multiple agents sequentially.
    pub fn batch_install(
        &self,
        agents: &[Agent],
        dry_run: bool,
        timeout: Option<Duration>,
    ) -> Vec<InstallResult> {
        agents
            .iter()
            .map(|agent| {
                self.execute_install(agent, dry_run, timeout)
                    .unwrap_or_else(|e| InstallResult {
                        success: false,
                        message: e.to_string(),
                        agent_id: agent.id.clone(),
                        command: String::new(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        timed_out: false,
                    })
            })
            .collect()
    }

    /// Uninstall multiple agents sequentially.
    pub fn batch_uninstall(
        &self,
        agents: &[Agent],
        dry_run: bool,
        timeout: Option<Duration>,
    ) -> Vec<InstallResult> {
        agents
            .iter()
            .map(|agent| {
                self.execute_uninstall(agent, dry_run, timeout)
                    .unwrap_or_else(|e| InstallResult {
                        success: false,
                        message: e.to_string(),
                        agent_id: agent.id.clone(),
                        command: String::new(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        timed_out: false,
                    })
            })
            .collect()
    }
}

/// Helper to build a batch summary from individual results.
pub fn summarize_batch(results: &[InstallResult]) -> (usize, usize) {
    let success = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();
    (success, failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, AgentKind, InstallerConfig, PackageManager, Platform, SupportStatus};
    use crate::command_builder::MockCommandRunner;
    use std::collections::HashMap;

    fn create_test_agent() -> Agent {
        let mut installers = HashMap::new();
        installers.insert(
            Platform::Windows,
            InstallerConfig {
                manager: PackageManager::Npm,
                package: Some("@test/package".to_string()),
            },
        );
        installers.insert(
            Platform::MacOS,
            InstallerConfig {
                manager: PackageManager::BrewCask,
                package: Some("test-package".to_string()),
            },
        );
        installers.insert(
            Platform::Linux,
            InstallerConfig {
                manager: PackageManager::Manual,
                package: None,
            },
        );

        Agent {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            kind: AgentKind::CLI,
            provider: "Test Provider".to_string(),
            description: "A test agent".to_string(),
            homepage: "https://test.com".to_string(),
            installers,
            status: SupportStatus::Verified,
            catalog_verified_at: None,
            installer_verified_at: None,
        }
    }

    #[test]
    fn test_install_success() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_install(&agent, false, None)
            .unwrap();

        assert!(result.success, "Expected success, got: {}", result.message);
        assert_eq!(result.command, "npm install -g @test/package");
        assert!(!result.timed_out);
    }

    #[test]
    fn test_install_failure() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::failure();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_install(&agent, false, None)
            .unwrap();

        assert!(!result.success);
        assert!(result.message.contains("Failed"));
    }

    #[test]
    fn test_install_dry_run() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_install(&agent, true, None)
            .unwrap();

        assert!(result.success);
        assert!(result.message.contains("Dry run"));
        assert_eq!(result.duration_ms, 0);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_install_timeout() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::timeout();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_install(&agent, false, Some(Duration::from_millis(1)))
            .unwrap();

        assert!(!result.success);
        assert!(result.timed_out);
        assert!(result.message.contains("timed out"));
    }

    #[test]
    fn test_install_no_installer() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Linux, Box::new(runner));

        let result = installer.execute_install(&agent, false, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_uninstall_success() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_uninstall(&agent, false, None)
            .unwrap();

        assert!(result.success, "Expected success, got: {}", result.message);
        assert_eq!(result.command, "npm uninstall -g @test/package");
    }

    #[test]
    fn test_uninstall_dry_run() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_uninstall(&agent, true, None)
            .unwrap();

        assert!(result.success);
        assert!(result.message.contains("Dry run"));
    }

    #[test]
    fn test_uninstall_failure() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::failure();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let result = installer
            .execute_uninstall(&agent, false, None)
            .unwrap();

        assert!(!result.success);
    }

    #[test]
    fn test_get_command_preview_install() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let preview = installer.get_command_preview(&agent, false).unwrap();
        assert_eq!(preview.command, "npm install -g @test/package");
        assert!(preview.description.contains("install"));
    }

    #[test]
    fn test_get_command_preview_uninstall() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let preview = installer.get_command_preview(&agent, true).unwrap();
        assert_eq!(preview.command, "npm uninstall -g @test/package");
        assert!(preview.description.contains("uninstall"));
    }

    #[test]
    fn test_get_command_preview_manual() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Linux, Box::new(runner));

        let preview = installer.get_command_preview(&agent, false);
        assert!(preview.is_none());
    }

    #[test]
    fn test_batch_install() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let results = installer.batch_install(&[agent], false, None);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }

    #[test]
    fn test_batch_uninstall() {
        let agent = create_test_agent();
        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let results = installer.batch_uninstall(&[agent], false, None);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }

    #[test]
    fn test_batch_mixed_results() {
        let mut agents = Vec::new();
        for i in 0..3 {
            let mut installers = HashMap::new();
            installers.insert(
                Platform::Windows,
                InstallerConfig {
                    manager: PackageManager::Npm,
                    package: Some(format!("@test/package-{}", i)),
                },
            );
            agents.push(Agent {
                id: format!("test-agent-{}", i),
                name: format!("Test Agent {}", i),
                kind: AgentKind::CLI,
                provider: "Test".to_string(),
                description: format!("Test agent {}", i),
                homepage: "https://test.com".to_string(),
                installers,
                status: SupportStatus::Verified,
                catalog_verified_at: None,
                installer_verified_at: None,
            });
        }

        let runner = MockCommandRunner::success();
        let installer = Installer::new(Platform::Windows, Box::new(runner));

        let results = installer.batch_install(&agents, false, None);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_summarize_batch() {
        let results = vec![
            InstallResult {
                success: true,
                message: "ok".to_string(),
                agent_id: "a".to_string(),
                command: String::new(),
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            },
            InstallResult {
                success: false,
                message: "fail".to_string(),
                agent_id: "b".to_string(),
                command: String::new(),
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            },
            InstallResult {
                success: true,
                message: "ok".to_string(),
                agent_id: "c".to_string(),
                command: String::new(),
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            },
        ];

        let (success, failed) = summarize_batch(&results);
        assert_eq!(success, 2);
        assert_eq!(failed, 1);
    }
}
