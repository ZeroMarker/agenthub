# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-08-06

### Added
- Tauri desktop application with Vue 3 frontend (Material You design)
- Shared core library (`agenthub-core`) for agent management
- Agent catalog with 25 agents (7 CLI, 18 Desktop)
- Platform-specific installer configurations (npm, pip, winget, brew)
- Agent search, filtering, and sorting
- Batch install/uninstall operations with progress tracking
- Command cancellation, automatic retry, and failure detail reporting
- Agent detail view with platform installer information
- Status detection and version parsing
- CLI with install/uninstall/list/search commands
- Release automation: multi-platform builds + SHA-256 checksums
- Vitest test infrastructure for the frontend
- Unit tests for core functionality (110 tests)

### Changed
- Unified data source with `agents.json` as single source of truth
- Platform-aware installer configuration (Windows, macOS, Linux)
- Comprehensive README rewrite with full project documentation
