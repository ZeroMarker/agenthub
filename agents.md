# AgentHub - Supported Agents

## CLI Agents (20)

Command-line based AI coding assistants.

| # | Agent | Package | Manager | Provider | Install Source | npm Status | Description |
|---|-------|---------|---------|----------|----------------|------------|-------------|
| 1 | codex | @openai/codex | npm | OpenAI | npmjs.com | ✅ v0.137.0 | AI coding assistant powered by GPT-4 |
| 2 | claude-code | @anthropic-ai/claude-code | npm | Anthropic | npmjs.com | ✅ v2.1.168 | AI pair programmer with Claude |
| 3 | kimi-code | @moonshot/kimi-code | npm | Moonshot | npmjs.com | ❌ 未找到 | AI coding assistant with long context |
| 5 | aider | aider-chat | pip | Paul Gauthier | pypi.org | - | AI pair programming in terminal |
| 7 | github-copilot-cli | @githubnext/github-copilot-cli | npm | GitHub | npmjs.com | ✅ v0.1.36 | AI-powered command line assistant |
| 8 | continue-cli | continue | npm | Continue | npmjs.com | ✅ v0.1.0 | Open source AI code assistant |
| 9 | cody-cli | @sourcegraph/cody | npm | Sourcegraph | npmjs.com | ✅ v5.5.26 | AI with codebase context |
| 10 | tabnine-cli | @tabnine/cli | npm | Tabnine | npmjs.com | ❌ 未找到 | AI code completion and chat |
| 11 | amazon-q | @aws/amazon-q-developer-cli | npm | AWS | npmjs.com | ❌ 未找到 | AI coding assistant |
| 12 | gemini-cli | @google/gemini-cli | npm | Google | npmjs.com | ✅ v0.45.2 | AI powered by Gemini |
| 13 | qwen-coder-cli | @qwen/coder-cli | npm | Alibaba | npmjs.com | ❌ 未找到 | AI coding assistant |
| 14 | deepseek-coder | @deepseek/coder | npm | DeepSeek | npmjs.com | ❌ 未找到 | AI with deep reasoning |
| 15 | codeium-cli | @codeium/cli | npm | Codeium | npmjs.com | ❌ 未找到 | Free AI code completion |
| 16 | supermaven-cli | @supermaven/cli | npm | Supermaven | npmjs.com | ❌ 未找到 | Fastest AI code completion |
| 17 | mentat | mentat | pip | Mentat | pypi.org | - | Understands your codebase |
| 18 | gpt-engineer | gpt-engineer | pip | GPT Engineer | pypi.org | - | Specify what to build |
| 19 | sweep | sweepai | pip | Sweep AI | pypi.org | - | AI junior developer |
| 20 | devon | devon-agent | pip | Devon | pypi.org | - | Open source AI engineer |
| 21 | open-interpreter | open-interpreter | pip | Open Interpreter | pypi.org | - | Local code interpreter |
| 22 | antigravity-cli | @google/antigravity-cli | npm | Google | npmjs.com | ❌ 未找到 | Google Antigravity CLI |

### By Install Source

| Source | Count | Verified | Agents |
|--------|-------|----------|--------|
| npm | 15 | 6 | codex ✅, claude-code ✅, kimi-code ❌, github-copilot-cli ✅, continue-cli ✅, cody-cli ✅, tabnine-cli ❌, amazon-q ❌, gemini-cli ✅, qwen-coder-cli ❌, deepseek-coder ❌, codeium-cli ❌, supermaven-cli ❌, antigravity-cli ❌ |
| pip | 5 | - | aider, mentat, gpt-engineer, sweep, devon, open-interpreter |

---

## Desktop Agents (20)

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
| 17 | antigravity | Google | Agent | Official | antigravity.google | Google Antigravity AI coding assistant |
| 18 | antigravity-ide | Google | IDE | Official | antigravity.google | Google Antigravity IDE |
| 19 | reasonix | Reasonix | Agent | Official | reasonix.ai | AI reasoning and coding agent |
| 20 | opencode | OpenCode | Agent | Official | opencode.ai | Open source AI coding assistant |

### By Install Source

| Source | Count | Agents |
|--------|-------|--------|
| Official Website | 17 | cursor, windsurf, trae, trae-solo, codex-desktop, claude-desktop, kimi-desktop, workbuddy, codebuddy, mavis, qoder, qoder-work, minimax-agent, zcode, gork-build, antigravity, antigravity-ide |
| Web Platform | 1 | replit-ai |

### By Type

| Type | Count | Agents |
|------|-------|--------|
| IDE | 4 | cursor, windsurf, trae, antigravity-ide |
| Agent | 14 | codex-desktop, claude-desktop, kimi-desktop, workbuddy, codebuddy, mavis, qoder, qoder-work, minimax-agent, zcode, gork-build, antigravity, reasonix, opencode |
| Platform | 1 | replit-ai |
| Standalone | 1 | trae-solo |

---

## Quick Reference

### CLI Install Commands

```bash
# npm packages
npm install -g @openai/codex
npm install -g @anthropic-ai/claude-code
npm install -g @moonshot/kimi-code
npm install -g @githubnext/github-copilot-cli
npm install -g continue
npm install -g @sourcegraph/cody
npm install -g @tabnine/cli
npm install -g @aws/amazon-q-developer-cli
npm install -g @google/gemini-cli
npm install -g @qwen/coder-cli
npm install -g @deepseek/coder
npm install -g @codeium/cli
npm install -g @supermaven/cli
npm install -g @google/antigravity-cli

# pip packages
pip install aider-chat
pip install mentat
pip install gpt-engineer
pip install sweepai
pip install devon-agent
pip install open-interpreter
```

### Desktop Install Commands

#### Windows (winget) - 推荐

```powershell
# IDE
winget install Anysphere.Cursor
winget install Codeium.Windsurf
winget install ByteDance.Trae
winget install ByteDance.TraeSolo
winget install Google.AntigravityIDE

# AI Assistants (winget)
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

# AI Assistants (msstore)
winget install 9N8CJ4W95TBZ  # Codex (Beta)
```

#### macOS (brew) - 推荐

```bash
# IDE
brew install --cask cursor
brew install --cask windsurf
brew install --cask trae
brew install --cask antigravity-ide

# AI Assistants
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

#### Linux / 备用方案 - 官网下载

```
cursor.com        - Cursor IDE
codeium.com       - Windsurf IDE
trae.ai           - Trae / Trae Solo
replit.com        - Replit AI
openai.com        - Codex Desktop
claude.ai         - Claude Desktop
kimi.ai           - Kimi Desktop
workbuddy.ai      - WorkBuddy
codebuddy.com     - CodeBuddy (腾讯)
marvis.qq.com     - 马维斯 (腾讯)
qoder.com         - Qoder
minimax.chat      - MiniMax Agent
zcode.ai          - ZCode
x.ai              - Grok Build
```

### Verified Winget IDs

| Agent | Winget ID | Source | Verified |
|-------|-----------|--------|----------|
| cursor | Anysphere.Cursor | winget | ✅ |
| windsurf | Codeium.Windsurf | winget | ✅ |
| trae | ByteDance.Trae | winget | ✅ |
| trae-solo | ByteDance.TraeSolo | winget | ✅ |
| codex | OpenAI.Codex | winget | ✅ |
| codex-desktop | 9N8CJ4W95TBZ | msstore | ✅ |
| claude | Anthropic.Claude | winget | ✅ |
| kimi | MoonshotAI.Kimi | winget | ✅ |
| workbuddy | Tencent.WorkBuddy | winget | ✅ |
| codebuddy | Tencent.CodeBuddy | winget | ✅ |
| qoder | Alibaba.Qoder | winget | ✅ |
| qoder-work | Alibaba.QoderWork | winget | ✅ |
| minimax | MiniMax.MiniMaxAgent | winget | ✅ |
| zcode | ZhipuAI.ZCode | winget | ✅ |
| replit | Replit.Replit | winget | ✅ |
| antigravity | Google.Antigravity | winget | ✅ |
| antigravity-cli | Google.AntigravityCLI | winget | ✅ |
| antigravity-ide | Google.AntigravityIDE | winget | ✅ |
| reasonix | ESEngine.Reasonix | winget | ✅ |
| opencode | SST.OpenCodeDesktop | winget | ✅ |
| mavis | N/A (官网: marvis.qq.com) | - | ❌ |
| gork-build | N/A (官网: x.ai) | - | ❌ |

### All Agent Names

```
CLI:     codex, claude-code, kimi-code, aider, github-copilot-cli, continue-cli, 
         cody-cli, tabnine-cli, amazon-q, gemini-cli, qwen-coder-cli, deepseek-coder, 
         codeium-cli, supermaven-cli, mentat, gpt-engineer, sweep, devon, 
         open-interpreter, antigravity-cli

Desktop: cursor, windsurf, trae, trae-solo, replit-ai, codex-desktop, claude-desktop, 
         kimi-desktop, workbuddy, codebuddy, mavis, qoder, qoder-work, minimax-agent, 
         zcode, gork-build, antigravity, antigravity-ide, reasonix, opencode
```

### Total Count

| Category | Count |
|----------|-------|
| CLI Agents | 20 |
| Desktop Agents | 20 |
| **Total** | **40** |
