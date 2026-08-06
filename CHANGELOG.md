# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-08-06 (Wave 2: portability + observability)

### Added
- **Config templates**: reusable `ConfigTemplate` (settings / env vars / secret key reservations / custom), CRUD, save-from-config (secret values never persisted) and apply-to-agent
- **Session budget & alerts**: daily/monthly USD limits (`sessions/budget.yaml`), `check_budget` aggregates today/this-month spend and emits threshold alerts
- **Session context handoff**: `export_context` (last-N messages as portable JSON) and `fork_session` (carry messages/model/tags/project into a new session, optionally for another agent)
- **Prompt import/export**: JSON bundle with version history, `import_prompts` with force-overwrite/skip semantics
- **Memory import/export**: JSON export (optionally scope-filtered), import with merge/skip semantics
- **Skill compatibility check**: `min_agenthub_version` vs running version (semver triples), per-skill and bulk
- **Overview trend**: per-day buckets (sessions started/completed, tokens, cost, memories created, audit events) for the last N days
- **Monitor (cross-cutting, v1)**: `MonitorReport` aggregates diagnostics, missing verified agents, budget alerts and incompatible skills into a healthy/unhealthy status
- **CLI**: `config-template`, `prompt export|export-all|import`, `memory export|import`, `session budget|fork`, `skill check-compat`, `monitor`, `status --trend`
- **GUI**: Overview view gains a budget card (with editable limits), a monitor panel and a CSS-bar trend chart

### Changed
- `SessionManager::set_budget` now creates the sessions directory automatically

## [Unreleased] - 2026-08-06 (Wave 1)

### Added
- **Overview 模块（概览，只读聚合）**: workspace status overview (`OverviewReport` in `overview.rs`), `agenthub status`, GUI dashboard view
- **横切能力（非模块）**: append-only audit log (`audit/events.jsonl`), whole-workspace backup/restore (configs, prompts + versions, sessions + templates, memories, audit)
- **CLI**: `agenthub status` (overview), `agenthub audit [--action --target --last-days --limit]`, `agenthub backup [--output]`, `agenthub restore <file>`
- **GUI**: Overview view with dashboard stat cards, filterable audit log table, and backup/restore controls
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
