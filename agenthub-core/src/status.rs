use crate::agent::{Agent, PackageManager, Platform};
use crate::error::Result;
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
        let results = self.check_agents(std::slice::from_ref(agent));
        Ok(results.into_iter().next().unwrap_or(AgentStatus {
            agent_id: agent.id.clone(),
            installed: false,
            version: None,
            detection_method: "error".to_string(),
        }))
    }

    /// Check multiple agents at once, using cached package lists for efficiency
    pub fn check_agents(&self, agents: &[Agent]) -> Vec<AgentStatus> {
        // Pre-fetch package lists once
        let npm_list = self.get_npm_list();
        let pip_list = self.get_pip_list();
        let winget_list = self.get_winget_list();
        let brew_list = self.get_brew_list();

        agents
            .iter()
            .map(|agent| {
                let installer = agent.get_installer(self.platform);
                match installer {
                    Some(installer) => match installer.manager {
                        PackageManager::Npm => self.check_npm_from_cache(
                            agent,
                            installer.package.as_deref(),
                            &npm_list,
                        ),
                        PackageManager::Pip => self.check_pip_from_cache(
                            agent,
                            installer.package.as_deref(),
                            &pip_list,
                        ),
                        PackageManager::Winget => self.check_winget_from_cache(
                            agent,
                            installer.package.as_deref(),
                            &winget_list,
                        ),
                        PackageManager::BrewCask => self.check_brew_from_cache(
                            agent,
                            installer.package.as_deref(),
                            &brew_list,
                        ),
                        PackageManager::Manual => AgentStatus {
                            agent_id: agent.id.clone(),
                            installed: false,
                            version: None,
                            detection_method: "manual".to_string(),
                        },
                    },
                    None => AgentStatus {
                        agent_id: agent.id.clone(),
                        installed: false,
                        version: None,
                        detection_method: "none".to_string(),
                    },
                }
            })
            .collect()
    }

    fn get_npm_list(&self) -> String {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "npm", "list", "-g", "--depth=0"])
                .output()
        } else {
            Command::new("npm")
                .args(["list", "-g", "--depth=0"])
                .output()
        };

        match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(_) => String::new(),
        }
    }

    fn get_pip_list(&self) -> String {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "pip", "list", "--format=columns"])
                .output()
        } else {
            Command::new("pip")
                .args(["list", "--format=columns"])
                .output()
        };

        match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(_) => String::new(),
        }
    }

    fn get_winget_list(&self) -> String {
        if !cfg!(target_os = "windows") {
            return String::new();
        }

        let output = Command::new("cmd").args(["/C", "winget", "list"]).output();

        match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(_) => String::new(),
        }
    }

    fn get_brew_list(&self) -> String {
        if !cfg!(target_os = "macos") {
            return String::new();
        }

        let output = Command::new("brew").args(["list", "--cask"]).output();

        match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(_) => String::new(),
        }
    }

    fn check_npm_from_cache(
        &self,
        agent: &Agent,
        package: Option<&str>,
        npm_list: &str,
    ) -> AgentStatus {
        let package = match package {
            Some(p) => p,
            None => {
                return AgentStatus {
                    agent_id: agent.id.clone(),
                    installed: false,
                    version: None,
                    detection_method: "npm".to_string(),
                };
            }
        };

        let installed = npm_list.contains(package);
        let version = if installed {
            self.parse_npm_version(npm_list, package)
        } else {
            None
        };

        AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "npm".to_string(),
        }
    }

    fn check_pip_from_cache(
        &self,
        agent: &Agent,
        package: Option<&str>,
        pip_list: &str,
    ) -> AgentStatus {
        let package = match package {
            Some(p) => p,
            None => {
                return AgentStatus {
                    agent_id: agent.id.clone(),
                    installed: false,
                    version: None,
                    detection_method: "pip".to_string(),
                };
            }
        };

        // Parse from cached pip list output instead of running pip show per agent
        let installed = pip_list.contains(package);
        let version = if installed {
            self.parse_pip_version_from_list(pip_list, package)
        } else {
            None
        };

        AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "pip".to_string(),
        }
    }

    /// Parse pip version directly from `pip list --format=columns` cached output.
    ///
    /// Expected format:
    /// ```text
    /// Package    Version
    /// ---------- -------
    /// aider-chat 0.25.0
    /// ```
    fn parse_pip_version_from_list(&self, output: &str, package: &str) -> Option<String> {
        // Skip first two lines (header + separator)
        let lines: Vec<&str> = output.lines().skip(2).collect();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Split on 2+ spaces to handle variable-width columns
            let parts: Vec<&str> = trimmed
                .split(|c: char| c.is_whitespace())
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() >= 2 && parts[0] == package {
                return Some(parts[1].to_string());
            }
        }
        None
    }

    fn check_winget_from_cache(
        &self,
        agent: &Agent,
        package: Option<&str>,
        winget_list: &str,
    ) -> AgentStatus {
        let package = match package {
            Some(p) => p,
            None => {
                return AgentStatus {
                    agent_id: agent.id.clone(),
                    installed: false,
                    version: None,
                    detection_method: "winget".to_string(),
                };
            }
        };

        // Search by package ID in the cached winget list
        let installed = winget_list.contains(package);
        let version = if installed {
            self.parse_winget_version(winget_list, package)
        } else {
            None
        };

        AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "winget".to_string(),
        }
    }

    fn check_brew_from_cache(
        &self,
        agent: &Agent,
        package: Option<&str>,
        brew_list: &str,
    ) -> AgentStatus {
        let package = match package {
            Some(p) => p,
            None => {
                return AgentStatus {
                    agent_id: agent.id.clone(),
                    installed: false,
                    version: None,
                    detection_method: "brew".to_string(),
                };
            }
        };

        let installed = brew_list.contains(package);
        let version = if installed {
            self.parse_brew_version(package)
        } else {
            None
        };

        AgentStatus {
            agent_id: agent.id.clone(),
            installed,
            version,
            detection_method: "brew".to_string(),
        }
    }

    fn parse_npm_version(&self, output: &str, package: &str) -> Option<String> {
        for line in output.lines() {
            if line.contains(package) {
                if let Some(package_pos) = line.find(package) {
                    let after_package = &line[package_pos + package.len()..];
                    if let Some(version_part) = after_package.strip_prefix('@') {
                        let version = version_part.split([' ', '-', '\0']).next().unwrap_or("");
                        if !version.is_empty() {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn parse_winget_version(&self, output: &str, package: &str) -> Option<String> {
        for line in output.lines() {
            if line.contains(package) {
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
        let output = "Package    Version\n---------- -------\naider-chat 0.25.0\npip        25.2.1\nsetuptools 78.1.0";
        assert_eq!(
            detector.parse_pip_version_from_list(output, "aider-chat"),
            Some("0.25.0".to_string())
        );
        assert_eq!(
            detector.parse_pip_version_from_list(output, "nonexistent"),
            None
        );
    }
}
