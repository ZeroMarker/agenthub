# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added (Wave 5: effectiveness tracking + index persistence + alert grading/dedup)
- **Prompt effectiveness tracking**: `prompt effects` / `record-outcome` link session
  outcomes (rating, success, tokens, cost) to prompts; aggregated stats (avg rating,
  success rate, cost) persisted under `prompts/effects/`; GUI Effects tab
- **Memory vector index persistence**: `memory/vector_index.json` caches weighted
  embeddings with staleness detection; incremental recompute on edit, full rebuild
  via `memory reindex`; deleted entries drop their cache
- **Memory knowledge graph GUI**: interactive entity + relation panel in Memory
  Manager (build graph, browse entities, explore neighbors)
- **Alert grading & dedup (notify)**: `AlertSeverity` (info/warning/critical) derived
  from the monitor report; per-channel `min_severity` filter and `dedup_minutes`
  window (state in `notify_state.json`); `--force` bypass; `send_custom` for
  non-monitor alerts (key rotation notifications)
- **Secret ops audit**: `config secret set|rotate|migrate|delete` now record audit
  events; `config rotate --notify` pushes a rotation alert through the channels
- **CLI**: `prompt effects|record-outcome|clear-effects`, `memory reindex`,
  `notify add --min-severity/--dedup-minutes`, `notify send --force`, `notify clear-state`,
  `monitor --notify-force`, `config rotate --notify`
- **Tauri**: `get/list_prompt_effects`, `record_prompt_outcome`, `clear_prompt_effects`,
  `build_vector_index`; `add_notify_channel` gains severity/dedup options,
  `send_notification` gains force

### Changed
- **UI: full M3 token migration** — ConfigManager, SkillManager, PromptManager, SessionManager,
  MemoryManager and DiagnosticView were still on legacy hardcoded colors (~150 occurrences)
  that broke dark mode; all migrated to M3 tokens (context-aware `color: white` → `on-*`)
- Fixed SessionManager badge/delete colors, ConfigManager cancel button, DiagnosticView status
  colors; AgentList gradient background → token; tab switch no longer leaves a stale search filter
- Added missing `.m3-tabs`/`.m3-tab` and `.agent-stats`/`.stat-chip` styles
- **Users & permissions (Config)**: `UserManager` with `users.yaml` / `permissions.yaml`;
  built-in `admin` role bypasses all checks, `operator`/`viewer` roles, fine-grained
  grants scoped per module and per agent (`read|write|admin`, `*` wildcards, write-implies-read)
- **Prompt community sharing**: `prompts/community/` snapshots with provenance
  (publisher, publish time, source id) — publish/install fully offline; the community
  directory can be synced via git/shared storage
- **Skill marketplace**: local registry under `skills/marketplace/` with search
  (name/description/tags), ratings (1-5 with history), install counters, index refresh
  and package import from a source directory
- **Plugin system (Skill)**: plugins are directories with `plugin.yaml` manifests and
  hook commands for `on_install` / `on_uninstall` / `on_session_end` / `on_monitor` /
  `on_backup`; hooks run with captured output and a bounded timeout
- **Alert notification channels (cross-cutting)**: `notify.yaml`-configured channels —
  webhook (HTTP(S) JSON POST via `ureq`, bounded timeouts), email (RFC-2822 `.eml` spool,
  MTA delivery out of scope) and file append; `monitor --notify` pushes alerts through them
- **Backup extension**: users, permissions, community prompts and notify channels are now
  included in backups (secret values remain excluded)
- **CLI**: `config user list|show|create|delete|role`, `config perm grant|revoke|list|check`,
  `prompt publish`, `prompt community list|show|install|delete`,
  `skill market refresh|search|info|install|rate|stats|add-package`, `plugin list|show|register|unregister|enable|disable|run`,
  `notify list|add|remove|enable|disable|send`, `monitor --notify`
- **Tauri**: `list/create/delete_user`, `add/remove_user_role`, `grant/revoke/list/check_permission`,
  `publish_prompt`, `list/get/install/delete_community_prompt`, `market_*`,
  `list/register/unregister_plugin`, `set_plugin_enabled`, `run_plugin_hook`,
  `list/add/remove/set_enabled_notify_channel`, `send_notification`
- **UI**: new **Extensions** view (Marketplace / Plugins / Notifications / Users tabs),
  Prompt Manager community tab (publish/install/delete community prompts)

### Changed
- **UI: full M3 token migration** — ConfigManager, SkillManager, PromptManager, SessionManager,
  MemoryManager and DiagnosticView were still on legacy hardcoded colors (~150 occurrences)
  that broke dark mode; all migrated to M3 tokens (context-aware `color: white` → `on-*`)
- Fixed SessionManager badge/delete colors, ConfigManager cancel button, DiagnosticView status
  colors; AgentList gradient background → token; tab switch no longer leaves a stale search filter
- Added missing `.m3-tabs`/`.m3-tab` and `.agent-stats`/`.stat-chip` styles

## [1.2.0] - 2026-08-06

### Added
- **Material 3 Expressive design (UI)**: spring overshoot easing, expressive shape scale
  (16/24/32/40px), M3 state-layer colors; spring micro-interactions on buttons/chips/cards,
  tertiary pill active indicator on the nav rail, expressive modal entrance

### Changed
- Release workflow: `tauri-apps/tauri-action` upgraded `@v0` → `@v1` (v1.0.0); current
  inputs are compatible, no breaking-change migration needed

## [1.1.0] - 2026-08-06

### Added (Wave 3: security + intelligence + orchestration)
- **Secret keystore (Config)**: file-backed `SecretStore` (`secrets.yaml`, 0600 permissions) — secret values never live in agent config files or templates; redacted listing, rotation with archived previous values, and migration of legacy inline secrets out of config files
- **API key rotation**: `rotate_secret` archives the old value (grace-period rollback) while activating the new one
- **Memory vector search**: local embeddings (FNV-1a feature-hashed char n-grams → 256-dim, no network/model weights) with `search_entries_vector` and `hybrid_search` (BM25 + vector, 50/50 normalized blend) returning scored `MemoryMatch` results
- **Memory knowledge graph**: entity extraction (tags / title tokens / quoted phrases) with weighted co-occurrence edges, persisted to `memory/graph.json`; `build_graph` / `load_graph` / `neighbors` / `summary`
- **Skill workflows**: ordered step pipelines (`skill[:opt][;k=v]`) validated against installed skills (existence, enabled, dependency commands, version compatibility); optional steps skip without failing the run
- **Prompt extraction from sessions**: `extract_from_session` turns a session message into a reusable template by replacing URLs, paths, versions, numbers, quoted text and identifiers with `{{placeholder}}` variables
- **HTML dashboard (Overview)**: `render_dashboard_html` produces a self-contained browser view (inline CSS/JS + embedded JSON) — no server needed
- **Monitor JSON + watch**: `MonitorReport::to_json` and `alert_summary` for cron/systemd; `agenthub monitor --json` / `--watch <sec>` loop mode
- **Backup extension**: workflows and the memory graph are now included in backups; secret values remain intentionally excluded
- **CLI**: `config secret set|get|delete|list`, `config rotate`, `config migrate`, `memory search-vector|search-hybrid`, `memory graph build|entities|neighbors|export`, `skill workflow list|show|create|delete|run`, `prompt extract`, `status --html`, `monitor --json/--watch`
- **Tauri**: `get/set/delete/list/rotate/migrate_secret`, `search_memories_vector/hybrid`, `build/get_memory_graph`, `graph_neighbors`, `list/create/delete/run_workflow`, `extract_prompt_from_session`, `get_dashboard_html`
- **Low-resource CI**: `docs/low-resource-ci.md` guide + `.github/workflows/ci-low-resource.yml` for self-hosted runners with RAM < 2 GB / storage < 40 GB (limits parallelism, drops debug info, tests core+cli only, cache write on main only)

### Changed
- Backup format v1 extended with defaulted `workflows`/`memory_graph` fields (old backups still restore)
- README/CONTRIBUTING document the low-resource build/test strategy

### Added (Wave 2: portability + observability)
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

### Added (Wave 1: overview + audit/backup + cost tracking + prompt versioning + BM25)
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
