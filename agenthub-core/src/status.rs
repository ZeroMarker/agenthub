use crate::agent::{Agent, PackageManager, Platform};
use crate::error::{AgentHubError, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub agent_id: String,
    pub installed: bool,
    pub version: Option<String>,
    pub detection_method: String,
}

pub struct StatusDetector {
    platform: Platform,
}

impl StatusDetector {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn check_agent(&self, agent: &Agent) -> Result<AgentStatus> {
        let installer = agent.get_installer(self.platform).ok_or_else(|| {
            AgentHubError::InstallerError(format!(
                "No installer configuration for {} on {:?}",
                agent.name, self.platform
            ))
        })?;

        match installer.manager {
            PackageManager::Npm => self.check_npm_package(agent, installer.package.as_deref()),
            PackageManager::Pip => self.check_pip_package(agent, installer.package.as_deref()),
            PackageManager::Winget => {
                self.check_winget_package(agent, installer.package.as_deref())
            }
            PackageManager::BrewCask => {
                self.check_brew_package(agent, installer.package.as_deref())
            }
            PackageManager::Manual => Ok(AgentStatus {
                agent_id: agent.id.clone(),
                installed: false,
                version: None,
                detection_method: "manual".to_string(),
            }),
        }
    }

    fn check_npm_package(&self, agent: &Agent, package: Option<&str>) -> Result<AgentStatus> {
        let package = package.ok_or_else(|| {
            AgentHubError::InstallerError(format!("No package name for {}", agent.name))
        })?;

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "npm", "list", "-g", "--depth=0", package])
                .output()
        } else {
            Command::new("npm")
                .args(["list", "-g", "--depth=0", package])
                .output()
        }
        .map_err(|e| AgentHubError::InstallerError(format!("Failed to run npm: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let installed = stdout.contains(package);

        let version = if installed {
            self.parse_npm_version(&stdout, package)
        } else {
            None
        };

        Ok(AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "npm".to_string(),
        })
    }

    fn check_pip_package(&self, agent: &Agent, package: Option<&str>) -> Result<AgentStatus> {
        let package = package.ok_or_else(|| {
            AgentHubError::InstallerError(format!("No package name for {}", agent.name))
        })?;

        let output = Command::new("pip")
            .args(["show", package])
            .output()
            .map_err(|e| AgentHubError::InstallerError(format!("Failed to run pip: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let installed = output.status.success() && stdout.contains("Name:");

        let version = if installed {
            self.parse_pip_version(&stdout)
        } else {
            None
        };

        Ok(AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "pip".to_string(),
        })
    }

    fn check_winget_package(&self, agent: &Agent, package: Option<&str>) -> Result<AgentStatus> {
        let package = package.ok_or_else(|| {
            AgentHubError::InstallerError(format!("No package name for {}", agent.name))
        })?;

        if !cfg!(target_os = "windows") {
            return Ok(AgentStatus {
                agent_id: agent.id.clone(),
                installed: false,
                version: None,
                detection_method: "winget".to_string(),
            });
        }

        let output = Command::new("cmd")
            .args(["/C", "winget", "list", "--id", package])
            .output()
            .map_err(|e| AgentHubError::InstallerError(format!("Failed to run winget: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let installed = stdout.contains(package);

        let version = if installed {
            self.parse_winget_version(&stdout, package)
        } else {
            None
        };

        Ok(AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "winget".to_string(),
        })
    }

    fn check_brew_package(&self, agent: &Agent, package: Option<&str>) -> Result<AgentStatus> {
        let package = package.ok_or_else(|| {
            AgentHubError::InstallerError(format!("No package name for {}", agent.name))
        })?;

        if !cfg!(target_os = "macos") {
            return Ok(AgentStatus {
                agent_id: agent.id.clone(),
                installed: false,
                version: None,
                detection_method: "brew".to_string(),
            });
        }

        let output = Command::new("brew")
            .args(["list", "--cask", package])
            .output()
            .map_err(|e| AgentHubError::InstallerError(format!("Failed to run brew: {}", e)))?;

        let installed = output.status.success();

        let version = if installed {
            self.parse_brew_version(package)
        } else {
            None
        };

        Ok(AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "brew".to_string(),
        })
    }

    fn parse_npm_version(&self, output: &str, package: &str) -> Option<String> {
        for line in output.lines() {
            if line.contains(package) {
                // Format: "package@version" or "package@version -> ..."
                // Find the package name and then extract version after @
                if let Some(package_pos) = line.find(package) {
                    let after_package = &line[package_pos + package.len()..];
                    if let Some(version_part) = after_package.strip_prefix('@') {
                        // Extract version until space, arrow, or end
                        let version = version_part
                            .split([' ', '-', '\0'])
                            .next()
                            .unwrap_or("");
                        if !version.is_empty() {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn parse_pip_version(&self, output: &str) -> Option<String> {
        for line in output.lines() {
            if line.starts_with("Version:") {
                return Some(line.trim_start_matches("Version:").trim().to_string());
            }
        }
        None
    }

    fn parse_winget_version(&self, output: &str, package: &str) -> Option<String> {
        for line in output.lines() {
            if line.contains(package) {
                // Try to find version in the line
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    if part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                        return Some(part.to_string());
                    }
                }
            }
        }
        None
    }

    fn parse_brew_version(&self, package: &str) -> Option<String> {
        let output = Command::new("brew")
            .args(["info", "--json=v2", "--cask", package])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Simple JSON parsing for version
        if let Some(version_start) = stdout.find("\"version\":") {
            let version_part = &stdout[version_start + 10..];
            if let Some(quote_start) = version_part.find('"') {
                let version_rest = &version_part[quote_start + 1..];
                if let Some(quote_end) = version_rest.find('"') {
                    return Some(version_rest[..quote_end].to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npm_version() {
        let detector = StatusDetector::new(Platform::Windows);
        let output = "C:\\Users\\test\\AppData\\Roaming\\npm\\@openai\\codex@0.137.0 -> .\\.\\node_modules\\@openai\\codex\\bin\\codex";
        assert_eq!(
            detector.parse_npm_version(output, "@openai\\codex"),
            Some("0.137.0".to_string())
        );
    }

    #[test]
    fn test_parse_pip_version() {
        let detector = StatusDetector::new(Platform::Windows);
        let output =
            "Name: aider-chat\nVersion: 0.25.0\nSummary: AI pair programming in your terminal";
        assert_eq!(
            detector.parse_pip_version(output),
            Some("0.25.0".to_string())
        );
    }
}
