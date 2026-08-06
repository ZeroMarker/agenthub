# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-08-06

### Added
- **Management module**: append-only audit log (`audit/events.jsonl`), whole-workspace backup/restore (configs, prompts + versions, sessions + templates, memories, audit), and workspace status overview (`ManagementReport`)
- **CLI**: `agenthub status`, `agenthub audit [--action --target --last-days --limit]`, `agenthub backup [--output]`, `agenthub restore <file>`
- **GUI**: Management view with dashboard stat cards, filterable audit log table, and backup/restore controls
- **Session cost tracking**: built-in model pricing table (17 common models) with fallback, `record_usage` accumulation, `replay_session` markdown export, and reusable session templates
- **Prompt version control**: automatic snapshot on every update, `list_versions` / `get_version` / `rollback`, plus usage counters (`usage_count` / `last_used_at`) and required-variable validation with defaults
- **Memory semantic search**: pure-Rust BM25 scoring (title 3x / tags 2x / content 1x weighting), plus importance, touch, revive and age-based decay (low-importance stale entries auto-archived and excluded from search)
- **Audit integration**: install/uninstall operations in the Tauri backend record audit events automatically

### Changed
- `SessionManager::add_message` now delegates to `add_message_with_tokens`; usage can be recorded per message
- `PromptManager::update_prompt` bumps `version` and snapshots history; `render_prompt` records usage
- `MemoryManager::search_entries` excludes decayed entries; `MemoryStats` reports the decayed count

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
