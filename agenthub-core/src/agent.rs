use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    CLI,
    Desktop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PackageManager {
    Npm,
    Pip,
    Winget,
    BrewCask,
    Manual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SupportStatus {
    Verified,
    Community,
    Manual,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub manager: PackageManager,
    pub package: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub kind: AgentKind,
    pub provider: String,
    pub description: String,
    pub homepage: String,
    pub installers: HashMap<Platform, InstallerConfig>,
    pub status: SupportStatus,
    pub catalog_verified_at: Option<NaiveDate>,
    pub installer_verified_at: Option<NaiveDate>,
}

impl Agent {
    pub fn get_installer(&self, platform: Platform) -> Option<&InstallerConfig> {
        self.installers.get(&platform)
    }

    pub fn is_installable(&self, platform: Platform) -> bool {
        if let Some(installer) = self.get_installer(platform) {
            installer.manager != PackageManager::Manual
        } else {
            false
        }
    }

    pub fn get_install_command(&self, platform: Platform) -> Option<String> {
        let installer = self.get_installer(platform)?;
        let package = installer.package.as_ref()?;

        match installer.manager {
            PackageManager::Npm => Some(format!("npm install -g {}", package)),
            PackageManager::Pip => Some(format!("pip install {}", package)),
            PackageManager::Winget => Some(format!("winget install {}", package)),
            PackageManager::BrewCask => Some(format!("brew install --cask {}", package)),
            PackageManager::Manual => None,
        }
    }

    pub fn get_uninstall_command(&self, platform: Platform) -> Option<String> {
        let installer = self.get_installer(platform)?;
        let package = installer.package.as_ref()?;

        match installer.manager {
            PackageManager::Npm => Some(format!("npm uninstall -g {}", package)),
            PackageManager::Pip => Some(format!("pip uninstall -y {}", package)),
            PackageManager::Winget => Some(format!("winget uninstall {}", package)),
            PackageManager::BrewCask => Some(format!("brew uninstall --cask {}", package)),
            PackageManager::Manual => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            catalog_verified_at: Some(NaiveDate::from_ymd_opt(2026, 6, 27).unwrap()),
            installer_verified_at: Some(NaiveDate::from_ymd_opt(2026, 6, 27).unwrap()),
        }
    }

    #[test]
    fn test_get_installer() {
        let agent = create_test_agent();

        assert!(agent.get_installer(Platform::Windows).is_some());
        assert!(agent.get_installer(Platform::MacOS).is_some());
        assert!(agent.get_installer(Platform::Linux).is_some());
    }

    #[test]
    fn test_is_installable() {
        let agent = create_test_agent();

        assert!(agent.is_installable(Platform::Windows));
        assert!(agent.is_installable(Platform::MacOS));
        assert!(!agent.is_installable(Platform::Linux)); // Manual is not installable
    }

    #[test]
    fn test_get_install_command() {
        let agent = create_test_agent();

        assert_eq!(
            agent.get_install_command(Platform::Windows),
            Some("npm install -g @test/package".to_string())
        );
        assert_eq!(
            agent.get_install_command(Platform::MacOS),
            Some("brew install --cask test-package".to_string())
        );
        assert_eq!(agent.get_install_command(Platform::Linux), None);
    }

    #[test]
    fn test_get_uninstall_command() {
        let agent = create_test_agent();

        assert_eq!(
            agent.get_uninstall_command(Platform::Windows),
            Some("npm uninstall -g @test/package".to_string())
        );
        assert_eq!(
            agent.get_uninstall_command(Platform::MacOS),
            Some("brew uninstall --cask test-package".to_string())
        );
        assert_eq!(agent.get_uninstall_command(Platform::Linux), None);
    }
}
