# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Tauri desktop application with Vue 3 frontend
- Shared core library (`agenthub-core`) for agent management
- Agent catalog with 40 agents (20 CLI, 20 Desktop)
- Platform-specific installer configurations (npm, pip, winget, brew)
- Agent search, filtering, and sorting
- Batch install/uninstall operations
- Progress tracking for operations
- Agent detail view with platform installer information
- Status detection and version parsing
- Unit tests for core functionality (13 tests)

### Changed
- Unified data source with `agents.json` as single source of truth
- Platform-aware installer configuration (Windows, macOS, Linux)