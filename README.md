# AgentHub

A comprehensive tool to manage AI coding agents with both CLI and GUI interfaces.

> Current status: prototype. See [PROJECT_PLAN.md](PROJECT_PLAN.md) for the v1.0
> roadmap, known gaps, and release criteria.

## Supported Agents (40 total)

### CLI Agents (20)

Command-line based AI coding assistants.

| # | Agent | Package | Manager | Provider | Install Source | npm Status | Description |
|---|-------|---------|---------|----------|----------------|------------|-------------|
| 1 | codex | @openai/codex | npm | OpenAI | npmjs.com | ✅ v0.137.0 | AI coding assistant powered by GPT-4 |
| 2 | claude-code | @anthropic-ai/claude-code | npm | Anthropic | npmjs.com | ✅ v2.1.168 | AI pair programmer with Claude |
| 3 | kimi-code | @moonshot/kimi-code | npm | Moonshot | npmjs.com | ❌ 未找到 | AI coding assistant with long context |
| 4 | aider | aider-chat | pip | Paul Gauthier | pypi.org | - | AI pair programming in terminal |
| 5 | github-copilot-cli | @githubnext/github-copilot-cli | npm | GitHub | npmjs.com | ✅ v0.1.36 | AI-powered command line assistant |
| 6 | continue-cli | continue | npm | Continue | npmjs.com | ✅ v0.1.0 | Open source AI code assistant |
| 7 | cody-cli | @sourcegraph/cody | npm | Sourcegraph | npmjs.com | ✅ v5.5.26 | AI with codebase context |
| 8 | tabnine-cli | @tabnine/cli | npm | Tabnine | npmjs.com | ❌ 未找到 | AI code completion and chat |
| 9 | amazon-q | @aws/amazon-q-developer-cli | npm | AWS | npmjs.com | ❌ 未找到 | AI coding assistant |
| 10 | gemini-cli | @google/gemini-cli | npm | Google | npmjs.com | ✅ v0.45.2 | AI powered by Gemini |
| 11 | qwen-coder-cli | @qwen/coder-cli | npm | Alibaba | npmjs.com | ❌ 未找到 | AI coding assistant |
| 12 | deepseek-coder | @deepseek/coder | npm | DeepSeek | npmjs.com | ❌ 未找到 | AI with deep reasoning |
| 13 | codeium-cli | @codeium/cli | npm | Codeium | npmjs.com | ❌ 未找到 | Free AI code completion |
| 14 | supermaven-cli | @supermaven/cli | npm | Supermaven | npmjs.com | ❌ 未找到 | Fastest AI code completion |
| 15 | mentat | mentat | pip | Mentat | pypi.org | - | Understands your codebase |
| 16 | gpt-engineer | gpt-engineer | pip | GPT Engineer | pypi.org | - | Specify what to build |
| 17 | sweep | sweepai | pip | Sweep AI | pypi.org | - | AI junior developer |
| 18 | devon | devon-agent | pip | Devon | pypi.org | - | Open source AI engineer |
| 19 | open-interpreter | open-interpreter | pip | Open Interpreter | pypi.org | - | Local code interpreter |
| 20 | antigravity-cli | @google/antigravity-cli | npm | Google | npmjs.com | ❌ 未找到 | Google Antigravity CLI |

### Desktop Agents (20)

Independent desktop applications and AI coding platforms.

| # | Agent | Provider | Type | Install Source | Download URL | Description |
|---|-------|----------|------|----------------|--------------|-------------|
| 1 | cursor | Cursor | IDE | Official | cursor.com | AI-first code editor |
| 2 | windsurf | Codeium | IDE | Official | codeium.com | AI-powered IDE |
| 3 | trae | ByteDance | IDE | Official | trae.ai | AI coding companion |
| 4 | trae-solo | ByteDance | IDE | Official | trae.ai | Standalone AI coding agent |
| 5 | replit-ai | Replit | Platform | Web | replit.com | AI coding platform |
| 6 | codex-desktop | OpenAI | Agent | msstore | openai.com | Codex desktop app |
| 7 | claude-desktop | Anthropic | Agent | Official | claude.ai | Claude desktop app |
| 8 | kimi-desktop | Moonshot | Agent | Official | kimi.ai | Kimi desktop app |
| 9 | workbuddy | Tencent | Agent | Official | workbuddy.ai | AI work assistant |
| 10 | codebuddy | Tencent | Agent | Official | codebuddy.com | AI coding assistant |
| 11 | mavis | Tencent | Agent | Official | marvis.qq.com | 马维斯 AI assistant |
| 12 | qoder | Qoder | Agent | Official | qoder.com | AI coding agent |
| 13 | qoder-work | Qoder | Agent | Official | qoder.com | Qoder work edition |
| 14 | minimax-agent | MiniMax | Agent | Official | minimax.chat | MiniMax AI assistant |
| 15 | zcode | ZCode | Agent | Official | zcode.ai | AI coding assistant |
| 16 | gork-build | xAI | Agent | Official | x.ai | Grok AI coding agent |
| 17 | antigravity | Google | Agent | Official | antigravity.google | Google Antigravity AI assistant |
| 18 | antigravity-ide | Google | IDE | Official | antigravity.google | Google Antigravity IDE |
| 19 | reasonix | Reasonix | Agent | Official | reasonix.ai | AI reasoning and coding agent |
| 20 | opencode | OpenCode | Agent | Official | opencode.ai | Open source AI coding assistant |

## Quick Install

### CLI Agents

```bash
# npm packages (all platforms)
npm install -g @openai/codex
npm install -g @anthropic-ai/claude-code
npm install -g @moonshot/kimi-code
npm install -g @google/gemini-cli
npm install -g @google/antigravity-cli

# pip packages (all platforms)
pip install aider-chat
pip install mentat
```

### Desktop Agents

#### Windows (winget) - 推荐

```powershell
winget install Anysphere.Cursor
winget install Codeium.Windsurf
winget install ByteDance.Trae
winget install ByteDance.TraeSolo
winget install Google.AntigravityIDE
winget install OpenAI.Codex
winget install Anthropic.Claude
winget install MoonshotAI.Kimi
winget install Tencent.WorkBuddy
winget install Tencent.CodeBuddy
winget install Alibaba.Qoder
winget install Alibaba.QoderWork
winget install MiniMax.MiniMaxAgent
winget install ZhipuAI.ZCode
winget install Replit.Replit
winget install Google.Antigravity
winget install Google.AntigravityCLI
winget install ESEngine.Reasonix
winget install SST.OpenCodeDesktop
winget install 9N8CJ4W95TBZ  # Codex (Beta) from msstore
```

#### macOS (brew) - 推荐

```bash
brew install --cask cursor
brew install --cask windsurf
brew install --cask trae
brew install --cask antigravity-ide
brew install --cask codex
brew install --cask claude
brew install --cask kimi
brew install --cask workbuddy
brew install --cask codebuddy
brew install --cask qoder
brew install --cask minimax
brew install --cask zcode
brew install --cask replit
brew install --cask antigravity
brew install --cask antigravity-cli
brew install --cask reasonix
brew install --cask opencode
```

#### Linux / 备用方案

从官网下载：cursor.com, codeium.com, trae.ai, reasonix.ai, opencode.ai 等

## CLI Usage

```bash
# List all agents
agenthub-cli list

# List by type
agenthub-cli list --type cli
agenthub-cli list --type desktop

# Search
agenthub-cli search "keyword"

# Install/Uninstall (single or batch)
agenthub-cli install <name>
agenthub-cli install <name1> <name2> ...  # Batch install
agenthub-cli uninstall <name>
agenthub-cli uninstall <name1> <name2> ...  # Batch uninstall

# Info
agenthub-cli info <name>
```

### Installation Progress

When installing or uninstalling agents, you'll see a progress display:

```
🔧 Installing codex...
   Package: @openai/codex
   Manager: npm

  Step 1/3: Preparing installation...
  [==========--------------------] 1/3 Prepared

  Step 2/3: Downloading and installing...
  [==============================] 3/3 Completed
✅ codex installed successfully!
```

## GUI Usage

```bash
cd agenthub-ui
npm install
npm run tauri dev
```

### GUI Features

- **Desktop-Optimized Layout**: Designed for large screens with maximum information density
- **Batch Operations**: Select multiple agents and install/uninstall them at once
- **Search**: Find agents by name or description with debounced search
- **Filter**: Filter agents by type (CLI/Desktop)
- **Sort**: Sort by name, type, or status with ascending/descending order
- **View Modes**: Switch between Grid view and Table view
- **Statistics Dashboard**: Real-time stats showing total, installed, and available agents
- **Progress Display**: Real-time progress bar during installation/uninstallation
- **Modern UI**: Gradient backgrounds, smooth animations, and responsive design
- **Performance Optimized**: Caching and optimized re-renders

## Providers

| Provider | CLI | Desktop |
|----------|-----|---------|
| OpenAI | codex | codex-desktop |
| Anthropic | claude-code | claude-desktop |
| Google | gemini-cli, antigravity-cli | antigravity, antigravity-ide |
| AWS | amazon-q | - |
| GitHub | github-copilot-cli | - |
| ByteDance | - | trae, trae-solo |
| Moonshot | kimi-code | kimi-desktop |
| Alibaba | qwen-coder-cli | qoder, qoder-work |
| DeepSeek | deepseek-coder | - |
| Tencent | - | workbuddy, codebuddy, mavis |
| MiniMax | - | minimax-agent |
| xAI | - | gork-build |
| Sourcegraph | cody-cli | - |
| Cursor | - | cursor |
| Codeium | codeium-cli | windsurf |
| Replit | - | replit-ai |
| Reasonix | - | reasonix |
| OpenCode | - | opencode |

## License

MIT
