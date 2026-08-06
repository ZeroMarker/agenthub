# Contributing to AgentHub

## Project Structure

```
agenthub-core/    -- Shared Rust library (catalog, installer adapters, status detection)
agenthub-cli/     -- Command-line interface
agenthub-ui/      -- Tauri desktop application (Vue 3 frontend + Tauri backend)
```

## Development Setup

### Prerequisites

- Rust (stable) — install via [rustup](https://rustup.rs/)
- Node.js 18+ (for frontend)
- Tauri system dependencies — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Quick Start

```bash
# Build and run the CLI
cargo build -p agenthub-cli
cargo run -p agenthub-cli -- list

# Run tests
cargo test --workspace

# Format and lint
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings

# Frontend (Tauri desktop)
cd agenthub-ui
npm install
npm run build
```

## Making Changes

1. Create a feature branch from `main`
2. Make your changes
3. Run `cargo test --workspace` to ensure all tests pass
4. Run `cargo fmt --all` to format code
5. Run `cargo clippy --workspace -- -D warnings` for lint
6. Submit a pull request

## Code Standards

- Respect the single source of truth: agent metadata lives in `agents.json` and is consumed through `agenthub-core`.
- All installer logic must go through the installer adapter layer in `agenthub-core` — no hardcoded commands in CLI or UI.
- New agents or changes to agent metadata should be made in `agents.json` with appropriate verification dates.
- Tests are required for new installer adapters, status detection, and catalog parsing.
- Use `CommandBuilder` for any external command execution — raw `Command::new` is discouraged.

## Pull Request Guidelines

- Link related issues in the PR description
- Include before/after screenshots for UI changes
- Update CHANGELOG.md under the Unreleased section
- Ensure CI passes (format, clippy, test, build)

## Low-Resource Machines

On machines with limited resources (RAM < 2 GB, storage < 40 GB), see
[`docs/low-resource-ci.md`](docs/low-resource-ci.md) for build/test strategies and
the dedicated GitHub Actions workflow (`.github/workflows/ci-low-resource.yml`).
Key points:

- Limit parallelism: `export CARGO_BUILD_JOBS=2`
- Skip the Tauri desktop build for quick verification: `cargo test -p agenthub-core -p agenthub-cli`
- Keep the `target/` directory and cargo caches under control (`cargo clean -p`, prune caches)
- Frontend: `export NODE_OPTIONS="--max-old-space-size=512"`

## Adding a New Agent

1. Add the agent entry to `agents.json`
2. Verify the installer package names and platforms
3. Set `catalog_verified_at` to the current date
4. Run `cargo test` to ensure the catalog parses correctly
5. Test the install/uninstall flow on the target platform

## Adding a New Installer Adapter

1. Create a new module in `agenthub-core/src/installer/`
2. Implement the `InstallerAdapter` trait
3. Register the adapter in the installer factory
4. Add unit tests with mock command runner
5. Add integration tests with fixture data

## Quality Checklist

Before submitting a PR, verify:

- [ ] `cargo test --workspace` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cd agenthub-ui && npm run build` succeeds
- [ ] New agents are verified and tested on at least one platform
- [ ] CHANGELOG.md is updated
