use crate::agent::{Agent, PackageManager, Platform};
use crate::error::{AgentHubError, Result};
use std::process::Command;

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

#[derive(Debug, Clone)]
pub struct CommandPreview {
    pub command: String,
    pub description: String,
    pub platform: Platform,
}

pub struct Installer {
    platform: Platform,
}

impl Installer {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn get_install_command(&self, agent: &Agent) -> Option<CommandPreview> {
        let installer = agent.get_installer(self.platform)?;
        let package = installer.package.as_ref()?;

        let (command, description) = match installer.manager {
            PackageManager::Npm => (
                format!("npm install -g {}", package),
                format!("Install {} via npm", agent.name),
            ),
            PackageManager::Pip => (
                format!("pip install {}", package),
                format!("Install {} via pip", agent.name),
            ),
            PackageManager::Winget => (
                format!("winget install {}", package),
                format!("Install {} via winget", agent.name),
            ),
            PackageManager::BrewCask => (
                format!("brew install --cask {}", package),
                format!("Install {} via Homebrew", agent.name),
            ),
            PackageManager::Manual => return None,
        };

        Some(CommandPreview {
            command,
            description,
            platform: self.platform,
        })
    }

    pub fn get_uninstall_command(&self, agent: &Agent) -> Option<CommandPreview> {
        let installer = agent.get_installer(self.platform)?;
        let package = installer.package.as_ref()?;

        let (command, description) = match installer.manager {
            PackageManager::Npm => (
                format!("npm uninstall -g {}", package),
                format!("Uninstall {} via npm", agent.name),
            ),
            PackageManager::Pip => (
                format!("pip uninstall -y {}", package),
                format!("Uninstall {} via pip", agent.name),
            ),
            PackageManager::Winget => (
                format!("winget uninstall {}", package),
                format!("Uninstall {} via winget", agent.name),
            ),
            PackageManager::BrewCask => (
                format!("brew uninstall --cask {}", package),
                format!("Uninstall {} via Homebrew", agent.name),
            ),
            PackageManager::Manual => return None,
        };

        Some(CommandPreview {
            command,
            description,
            platform: self.platform,
        })
    }

    pub fn execute_install(&self, agent: &Agent, dry_run: bool) -> Result<InstallResult> {
        let preview = self.get_install_command(agent).ok_or_else(|| {
            AgentHubError::InstallerError(format!("No installer available for {}", agent.name))
        })?;

        if dry_run {
            return Ok(InstallResult {
                success: true,
                message: format!("Dry run: {}", preview.command),
                agent_id: agent.id.clone(),
                command: preview.command,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            });
        }

        let start = std::time::Instant::now();
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &preview.command]).output()
        } else {
            let parts: Vec<&str> = preview.command.split_whitespace().collect();
            if parts.is_empty() {
                return Err(AgentHubError::InstallerError("Empty command".to_string()));
            }
            Command::new(parts[0]).args(&parts[1..]).output()
        }
        .map_err(|e| AgentHubError::InstallerError(format!("Failed to execute command: {}", e)))?;

        let duration = start.elapsed();
        let success = output.status.success();
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let message = if success {
            format!("{} installed successfully", agent.name)
        } else {
            format!(
                "Failed to install {}: {}",
                agent.name,
                if stderr.is_empty() { &stdout } else { &stderr }
            )
        };

        Ok(InstallResult {
            success,
            message,
            agent_id: agent.id.clone(),
            command: preview.command,
            exit_code,
            stdout,
            stderr,
            duration_ms: duration.as_millis() as u64,
            timed_out: false,
        })
    }

    pub fn execute_uninstall(&self, agent: &Agent, dry_run: bool) -> Result<InstallResult> {
        let preview = self.get_uninstall_command(agent).ok_or_else(|| {
            AgentHubError::InstallerError(format!("No uninstaller available for {}", agent.name))
        })?;

        if dry_run {
            return Ok(InstallResult {
                success: true,
                message: format!("Dry run: {}", preview.command),
                agent_id: agent.id.clone(),
                command: preview.command,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                timed_out: false,
            });
        }

        let start = std::time::Instant::now();
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &preview.command]).output()
        } else {
            let parts: Vec<&str> = preview.command.split_whitespace().collect();
            if parts.is_empty() {
                return Err(AgentHubError::InstallerError("Empty command".to_string()));
            }
            Command::new(parts[0]).args(&parts[1..]).output()
        }
        .map_err(|e| AgentHubError::InstallerError(format!("Failed to execute command: {}", e)))?;

        let duration = start.elapsed();
        let success = output.status.success();
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let message = if success {
            format!("{} uninstalled successfully", agent.name)
        } else {
            format!(
                "Failed to uninstall {}: {}",
                agent.name,
                if stderr.is_empty() { &stdout } else { &stderr }
            )
        };

        Ok(InstallResult {
            success,
            message,
            agent_id: agent.id.clone(),
            command: preview.command,
            exit_code,
            stdout,
            stderr,
            duration_ms: duration.as_millis() as u64,
            timed_out: false,
        })
    }
}
