use crate::agent::{Agent, AgentKind, Platform};
use crate::error::{AgentHubError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetadata {
    pub version: String,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogFile {
    #[serde(flatten)]
    pub metadata: CatalogMetadata,
    pub agents: Vec<Agent>,
}

pub struct Catalog {
    agents: Vec<Agent>,
    metadata: CatalogMetadata,
}

impl Catalog {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AgentHubError::CatalogLoadError(format!("Failed to read catalog file: {}", e))
        })?;

        let catalog_file: CatalogFile = serde_json::from_str(&content).map_err(|e| {
            AgentHubError::CatalogLoadError(format!("Failed to parse catalog: {}", e))
        })?;

        Ok(Self {
            agents: catalog_file.agents,
            metadata: catalog_file.metadata,
        })
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let catalog_file: CatalogFile = serde_json::from_str(json).map_err(|e| {
            AgentHubError::CatalogLoadError(format!("Failed to parse catalog: {}", e))
        })?;

        Ok(Self {
            agents: catalog_file.agents,
            metadata: catalog_file.metadata,
        })
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn metadata(&self) -> &CatalogMetadata {
        &self.metadata
    }

    pub fn find_by_id(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<&Agent> {
        let name_lower = name.to_lowercase();
        self.agents
            .iter()
            .find(|a| a.name.to_lowercase() == name_lower || a.id.to_lowercase() == name_lower)
    }

    pub fn filter_by_kind(&self, kind: AgentKind) -> Vec<&Agent> {
        self.agents.iter().filter(|a| a.kind == kind).collect()
    }

    pub fn filter_by_platform(&self, platform: Platform) -> Vec<&Agent> {
        self.agents
            .iter()
            .filter(|a| a.installers.contains_key(&platform))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Agent> {
        let query_lower = query.to_lowercase();
        self.agents
            .iter()
            .filter(|a| {
                a.name.to_lowercase().contains(&query_lower)
                    || a.description.to_lowercase().contains(&query_lower)
                    || a.provider.to_lowercase().contains(&query_lower)
                    || a.id.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn count_by_kind(&self) -> (usize, usize) {
        let cli_count = self
            .agents
            .iter()
            .filter(|a| a.kind == AgentKind::CLI)
            .count();
        let desktop_count = self
            .agents
            .iter()
            .filter(|a| a.kind == AgentKind::Desktop)
            .count();
        (cli_count, desktop_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentKind, InstallerConfig, PackageManager, Platform, SupportStatus};
    use chrono::NaiveDate;
    use std::collections::HashMap;

    fn create_test_catalog_json() -> String {
        r#"{
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
                        "windows": {
                            "manager": "npm",
                            "package": "@test/cli"
                        }
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
                        "windows": {
                            "manager": "winget",
                            "package": "Test.Desktop"
                        }
                    },
                    "status": "community",
                    "catalog_verified_at": "2026-06-27",
                    "installer_verified_at": null
                }
            ]
        }"#
        .to_string()
    }

    #[test]
    fn test_catalog_from_json() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        assert_eq!(catalog.agents().len(), 2);
        assert_eq!(catalog.metadata().version, "1.0.0");
    }

    #[test]
    fn test_find_by_id() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        assert!(catalog.find_by_id("test-cli").is_some());
        assert!(catalog.find_by_id("test-desktop").is_some());
        assert!(catalog.find_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_find_by_name() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        assert!(catalog.find_by_name("Test CLI").is_some());
        assert!(catalog.find_by_name("test cli").is_some()); // Case insensitive
        assert!(catalog.find_by_name("test-cli").is_some()); // Find by ID
        assert!(catalog.find_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_filter_by_kind() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        let cli_agents = catalog.filter_by_kind(AgentKind::CLI);
        let desktop_agents = catalog.filter_by_kind(AgentKind::Desktop);

        assert_eq!(cli_agents.len(), 1);
        assert_eq!(desktop_agents.len(), 1);
        assert_eq!(cli_agents[0].id, "test-cli");
        assert_eq!(desktop_agents[0].id, "test-desktop");
    }

    #[test]
    fn test_filter_by_platform() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        let windows_agents = catalog.filter_by_platform(Platform::Windows);
        let macos_agents = catalog.filter_by_platform(Platform::MacOS);

        assert_eq!(windows_agents.len(), 2);
        assert_eq!(macos_agents.len(), 0);
    }

    #[test]
    fn test_search() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        let results = catalog.search("test");
        assert_eq!(results.len(), 2);

        let results = catalog.search("CLI");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-cli");

        let results = catalog.search("desktop");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-desktop");

        let results = catalog.search("nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_count_by_kind() {
        let json = create_test_catalog_json();
        let catalog = Catalog::from_json(&json).unwrap();

        let (cli_count, desktop_count) = catalog.count_by_kind();
        assert_eq!(cli_count, 1);
        assert_eq!(desktop_count, 1);
    }
}
