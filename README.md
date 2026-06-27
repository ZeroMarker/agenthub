# AgentHub

A comprehensive tool to manage AI coding agents with both CLI and GUI interfaces.

> Current status: v1.0 development. See [PROJECT_PLAN.md](PROJECT_PLAN.md) for the
> roadmap, known gaps, and release criteria.

## Architecture

Standard Tauri 2 project structure with shared core library:

```
agenthub/
├── agenthub-core/        # Shared Rust library
│   └── src/
│       ├── agent.rs      # Agent model
│       ├── catalog.rs    # Catalog loading/querying
│       ├── config.rs     # Configuration management
│       ├── skill.rs      # Skill management
│       ├── diagnostic.rs # Diagnostic tools
│       ├── installer.rs  # Install/uninstall logic
│       └── status.rs     # Status detection
├── agenthub-ui/          # Tauri application
│   ├── src/              # Vue 3 frontend
│   ├── src-tauri/        # Rust backend
│   │   ├── src/main.rs
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   └── package.json
└── agents.json           # Agent catalog (single source of truth)
```

### Key Features

- **Single source of truth**: Agent data is maintained once in `agents.json`
- **Platform-aware installers**: Each agent has platform-specific installation configurations
- **Shared core library**: CLI and GUI use the same `agenthub-core` crate
- **Schema validation**: JSON schema ensures catalog consistency
- **Comprehensive testing**: Unit tests for core functionality

## Supported Agents (25 total)

### CLI Agents (7)

Command-line based AI coding assistants.

| # | Agent | Provider | Package | Manager |
|---|-------|----------|---------|---------|
| 1 | codex | OpenAI | @openai/codex | npm |
| 2 | claude-code | Anthropic | @anthropic-ai/claude-code | npm |
| 3 | kimi-code | Moonshot | @moonshot-ai/kimi-code | npm |
| 4 | qwen-code | Alibaba | @qwen-code/qwen-code | npm |
| 5 | reasonix-cli | Reasonix | ESEngine.ReasonixCLI | winget |
| 6 | mimo-code | Xiaomi | @mimo-ai/cli | npm |
| 7 | grok-cli | xAI | xAI.GrokBuild | winget |

### Desktop Agents (18)

Independent desktop applications and AI coding platforms.

| # | Agent | Provider | Windows | macOS |
|---|-------|----------|---------|-------|
| 1 | cursor | Cursor | winget | brew |
| 2 | windsurf | Codeium | winget | brew |
| 3 | trae | ByteDance | winget | brew |
| 4 | trae-solo | ByteDance | winget | brew |
| 5 | codex-desktop | OpenAI | winget | brew |
| 6 | claude-desktop | Anthropic | winget | brew |
| 7 | kimi-desktop | Moonshot | winget | brew |
| 8 | workbuddy | Tencent | winget | brew |
| 9 | codebuddy | Tencent | winget | brew |
| 10 | qoder | Qoder | winget | brew |
| 11 | qoder-work | Qoder | winget | brew |
| 12 | minimax-agent | MiniMax | winget | brew |
| 13 | zcode | ZCode | winget | brew |
| 14 | antigravity | Google | winget | brew |
| 15 | antigravity-ide | Google | winget | brew |
| 16 | reasonix | Reasonix | winget | brew |
| 17 | opencode | OpenCode | winget | brew |
| 18 | openwork | DifferentAI | winget | - |

## Quick Install

### CLI Agents

```bash
# npm packages
npm install -g @openai/codex
npm install -g @anthropic-ai/claude-code
npm install -g @moonshot-ai/kimi-code
npm install -g @qwen-code/qwen-code
npm install -g @mimo-ai/cli
```

### Desktop Agents (Windows)

```powershell
winget install Anysphere.Cursor
winget install Codeium.Windsurf
winget install ByteDance.Trae
winget install ByteDance.TraeSolo
winget install OpenAI.Codex
winget install Anthropic.Claude
winget install MoonshotAI.Kimi
winget install Tencent.WorkBuddy
winget install Tencent.CodeBuddy
winget install Alibaba.Qoder
winget install Alibaba.QoderWork
winget install MiniMax.MiniMaxAgent
winget install ZhipuAI.ZCode
winget install Google.Antigravity
winget install Google.AntigravityIDE
winget install ESEngine.Reasonix
winget install SST.OpenCodeDesktop
winget install DifferentAI.OpenWork
```

### Desktop Agents (macOS)

```bash
brew install --cask cursor
brew install --cask windsurf
brew install --cask trae
brew install --cask trae-solo
brew install --cask codex
brew install --cask claude
brew install --cask kimi
brew install --cask workbuddy
brew install --cask codebuddy
brew install --cask qoder
brew install --cask minimax
brew install --cask zcode
brew install --cask antigravity
brew install --cask antigravity-ide
brew install --cask reasonix
brew install --cask opencode
```

## Usage

### Development

```bash
# Install frontend dependencies
cd agenthub-ui
npm install

# Run in development mode
npm run tauri dev
```

### Build

```bash
# Build the application
cargo build --release

# Or use npm script
cd agenthub-ui
npm run tauri build
```

## Project Status

### Completed (v1.0 Development)

- ✅ **M0: Baseline Confirmation**: Shared agent catalog with 25 agents
- ✅ **M1: Core Refactoring**: `agenthub-core` crate with catalog, config, skill, diagnostic modules
- ✅ **M2: Reliability & Security**: 32 unit tests passing, status detection, error handling

### In Progress

- 🔄 **M3: Beta Experience**: GUI improvements, user testing

### Planned

- 📋 **M4: Release Preparation**: CI/CD, installation packages, documentation

## Providers

| Provider | CLI | Desktop |
|----------|-----|---------|
| OpenAI | codex | codex-desktop |
| Anthropic | claude-code | claude-desktop |
| Google | - | antigravity, antigravity-ide |
| ByteDance | - | trae, trae-solo |
| Moonshot | kimi-code | kimi-desktop |
| Alibaba | qwen-code | qoder, qoder-work |
| Tencent | - | workbuddy, codebuddy |
| MiniMax | - | minimax-agent |
| xAI | grok-cli | - |
| Xiaomi | mimo-code | - |
| Cursor | - | cursor |
| Codeium | - | windsurf |
| Reasonix | reasonix-cli | reasonix |
| OpenCode | - | opencode |
| DifferentAI | - | openwork |

## License

MIT
