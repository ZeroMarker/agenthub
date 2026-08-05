use agenthub_core::{AgentKind, Catalog, PackageManager, Platform};

#[test]
fn test_real_catalog_loads_from_project_root() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json from project root");

    let agents = catalog.agents();
    assert_eq!(agents.len(), 25, "Expected 25 agents in catalog");

    let (cli, desktop) = catalog.count_by_kind();
    assert_eq!(cli, 7, "Expected 7 CLI agents");
    assert_eq!(desktop, 18, "Expected 18 Desktop agents");
}

#[test]
fn test_all_agents_have_required_fields() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    for agent in catalog.agents() {
        assert!(!agent.id.is_empty(), "Agent has empty id");
        assert!(!agent.name.is_empty(), "Agent '{}' has empty name", agent.id);
        assert!(!agent.description.is_empty(), "Agent '{}' has empty description", agent.id);
        assert!(!agent.homepage.is_empty(), "Agent '{}' has empty homepage", agent.id);
        assert!(!agent.provider.is_empty(), "Agent '{}' has empty provider", agent.id);
        assert!(!agent.installers.is_empty(), "Agent '{}' has no installers", agent.id);
    }
}

#[test]
fn test_every_agent_has_at_least_one_platform_installer() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    for agent in catalog.agents() {
        let has_windows = agent.installers.contains_key(&Platform::Windows);
        let has_macos = agent.installers.contains_key(&Platform::MacOS);

        assert!(
            has_windows || has_macos,
            "Agent '{}' has no Windows or macOS installer",
            agent.id
        );
    }
}

#[test]
fn test_no_agent_has_null_package_on_non_manual_installers() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    for agent in catalog.agents() {
        for (platform, config) in &agent.installers {
            if config.manager != PackageManager::Manual {
                assert!(
                    config.package.is_some(),
                    "Agent '{}' has {:?} installer on {:?} with no package",
                    agent.id,
                    config.manager,
                    platform
                );
            }
        }
    }
}

#[test]
fn test_cli_agents_have_at_least_one_non_manual_installer() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    let cli_agents = catalog.filter_by_kind(AgentKind::CLI);
    for agent in cli_agents {
        let has_auto_installer = agent.installers.values().any(|c| c.manager != PackageManager::Manual);
        assert!(
            has_auto_installer,
            "CLI agent '{}' has only manual installers",
            agent.id
        );
    }
}

#[test]
fn test_catalog_search_includes_all_fields() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    // Search by name
    let by_name = catalog.search("Cursor");
    assert!(!by_name.is_empty(), "Should find Cursor by name");

    // Search by description
    let by_desc = catalog.search("AI");
    assert!(!by_desc.is_empty(), "Should find agents with 'AI' in description");

    // Search by provider
    let by_provider = catalog.search("Google");
    assert!(!by_provider.is_empty(), "Should find Google agents");

    // Search by id
    let by_id = catalog.search("codex");
    assert!(!by_id.is_empty(), "Should find Codex by id");
}

#[test]
fn test_agent_platform_support_is_consistent() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    for agent in catalog.agents() {
        for platform in agent.installers.keys() {
            let installer = agent.get_installer(*platform);
            assert!(
                installer.is_some(),
                "Agent '{}' has inconsistent platform data for {:?}",
                agent.id,
                platform
            );
        }
    }
}

#[test]
fn test_npm_cli_agents_have_install_commands() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    let npm_agents: Vec<_> = catalog
        .agents()
        .iter()
        .filter(|a| {
            a.installers
                .values()
                .any(|c| c.manager == PackageManager::Npm)
        })
        .collect();

    assert!(!npm_agents.is_empty(), "Should have npm-based agents");

    for agent in &npm_agents {
        let cmd = agent.get_install_command(Platform::Windows);
        assert!(
            cmd.is_some(),
            "Agent '{}' should generate an npm install command",
            agent.id
        );
        if let Some(cmd) = cmd {
            assert!(
                cmd.starts_with("npm"),
                "npm command should start with 'npm': {}",
                cmd
            );
        }
    }
}

#[test]
fn test_search_finds_all_agents_by_name() {
    let catalog = Catalog::from_file(std::path::Path::new("../agents.json"))
        .expect("Failed to load agents.json");

    // Every agent should be findable by its own name
    for agent in catalog.agents() {
        let results = catalog.search(&agent.name);
        assert!(
            results.iter().any(|r| r.id == agent.id),
            "Agent '{}' not found by searching for '{}'",
            agent.id,
            agent.name
        );
    }
}
