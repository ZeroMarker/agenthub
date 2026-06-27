# AgentHub - Supported Agents

> **Note**: This file is auto-generated from `agents.json`. For the authoritative
> source of truth, see [agents.json](agents.json) and [agents.schema.json](agents.schema.json).

## Architecture

AgentHub uses a shared agent catalog (`agents.json`) with platform-specific installer configurations:

```json
{
  "id": "codex",
  "name": "Codex",
  "kind": "cli",
  "provider": "OpenAI",
  "installers": {
    "windows": { "manager": "npm", "package": "@openai/codex" },
    "macos": { "manager": "npm", "package": "@openai/codex" },
    "linux": { "manager": "npm", "package": "@openai/codex" }
  },
  "status": "verified"
}
```

### Support Status

| Status | Description |
|--------|-------------|
| `verified` | Installer verified on declared platforms |
| `community` | Community-maintained, installer not verified |
| `manual` | Manual installation only (no package manager) |
| `deprecated` | No longer maintained or available |

---

## CLI Agents (20)

Command-line based AI coding assistants.

| # | Agent | Provider | Status | Windows | macOS | Linux |
|---|-------|----------|--------|---------|-------|-------|
| 1 | Codex | OpenAI | ✅ verified | npm: @openai/codex | npm: @openai/codex | npm: @openai/codex |
| 2 | Claude Code | Anthropic | ✅ verified | npm: @anthropic-ai/claude-code | npm: @anthropic-ai/claude-code | npm: @anthropic-ai/claude-code |
| 3 | Kimi Code | Moonshot | 🔄 community | npm: @moonshot/kimi-code | npm: @moonshot/kimi-code | npm: @moonshot/kimi-code |
| 4 | Aider | Paul Gauthier | ✅ verified | pip: aider-chat | pip: aider-chat | pip: aider-chat |
| 5 | GitHub Copilot CLI | GitHub | ✅ verified | npm: @githubnext/github-copilot-cli | npm: @githubnext/github-copilot-cli | npm: @githubnext/github-copilot-cli |
| 6 | Continue CLI | Continue | ✅ verified | npm: continue | npm: continue | npm: continue |
| 7 | Cody CLI | Sourcegraph | ✅ verified | npm: @sourcegraph/cody | npm: @sourcegraph/cody | npm: @sourcegraph/cody |
| 8 | Tabnine CLI | Tabnine | 🔄 community | npm: @tabnine/cli | npm: @tabnine/cli | npm: @tabnine/cli |
| 9 | Amazon Q | AWS | 🔄 community | npm: @aws/amazon-q-developer-cli | npm: @aws/amazon-q-developer-cli | npm: @aws/amazon-q-developer-cli |
| 10 | Gemini CLI | Google | ✅ verified | npm: @google/gemini-cli | npm: @google/gemini-cli | npm: @google/gemini-cli |
| 11 | Qwen Coder CLI | Alibaba | 🔄 community | npm: @qwen/coder-cli | npm: @qwen/coder-cli | npm: @qwen/coder-cli |
| 12 | DeepSeek Coder | DeepSeek | 🔄 community | npm: @deepseek/coder | npm: @deepseek/coder | npm: @deepseek/coder |
| 13 | Codeium CLI | Codeium | 🔄 community | npm: @codeium/cli | npm: @codeium/cli | npm: @codeium/cli |
| 14 | Supermaven CLI | Supermaven | 🔄 community | npm: @supermaven/cli | npm: @supermaven/cli | npm: @supermaven/cli |
| 15 | Mentat | Mentat | ✅ verified | pip: mentat | pip: mentat | pip: mentat |
| 16 | GPT Engineer | GPT Engineer | ✅ verified | pip: gpt-engineer | pip: gpt-engineer | pip: gpt-engineer |
| 17 | Sweep | Sweep AI | ✅ verified | pip: sweepai | pip: sweepai | pip: sweepai |
| 18 | Devon | Devon | ✅ verified | pip: devon-agent | pip: devon-agent | pip: devon-agent |
| 19 | Open Interpreter | Open Interpreter | ✅ verified | pip: open-interpreter | pip: open-interpreter | pip: open-interpreter |
| 20 | Antigravity CLI | Google | 🔄 community | npm: @google/antigravity-cli | npm: @google/antigravity-cli | npm: @google/antigravity-cli |

### By Package Manager

| Manager | Count | Verified | Agents |
|---------|-------|----------|--------|
| npm | 15 | 6 | Codex ✅, Claude Code ✅, Kimi Code 🔄, GitHub Copilot CLI ✅, Continue CLI ✅, Cody CLI ✅, Tabnine CLI 🔄, Amazon Q 🔄, Gemini CLI ✅, Qwen Coder CLI 🔄, DeepSeek Coder 🔄, Codeium CLI 🔄, Supermaven CLI 🔄, Antigravity CLI 🔄 |
| pip | 5 | 5 | Aider ✅, Mentat ✅, GPT Engineer ✅, Sweep ✅, Devon ✅, Open Interpreter ✅ |

---

## Desktop Agents (20)

Independent desktop applications and AI coding platforms.

| # | Agent | Provider | Type | Status | Windows | macOS | Linux |
|---|-------|----------|------|--------|---------|-------|-------|
| 1 | Cursor | Cursor | IDE | ✅ verified | winget: Anysphere.Cursor | brew: cursor | manual |
| 2 | Windsurf | Codeium | IDE | ✅ verified | winget: Codeium.Windsurf | brew: windsurf | manual |
| 3 | Trae | ByteDance | IDE | ✅ verified | winget: ByteDance.Trae | brew: trae | manual |
| 4 | Trae Solo | ByteDance | IDE | ✅ verified | winget: ByteDance.TraeSolo | brew: trae-solo | manual |
| 5 | Replit AI | Replit | Platform | ✅ verified | winget: Replit.Replit | brew: replit | manual |
| 6 | Codex Desktop | OpenAI | Agent | ✅ verified | winget: OpenAI.Codex | brew: codex | manual |
| 7 | Claude Desktop | Anthropic | Agent | ✅ verified | winget: Anthropic.Claude | brew: claude | manual |
| 8 | Kimi Desktop | Moonshot | Agent | ✅ verified | winget: MoonshotAI.Kimi | brew: kimi | manual |
| 9 | WorkBuddy | Tencent | Agent | ✅ verified | winget: Tencent.WorkBuddy | brew: workbuddy | manual |
| 10 | CodeBuddy | Tencent | Agent | ✅ verified | winget: Tencent.CodeBuddy | brew: codebuddy | manual |
| 11 | Mavis | Tencent | Agent | 🔄 community | manual | manual | manual |
| 12 | Qoder | Qoder | Agent | ✅ verified | winget: Alibaba.Qoder | brew: qoder | manual |
| 13 | Qoder Work | Qoder | Agent | ✅ verified | winget: Alibaba.QoderWork | brew: qoder-work | manual |
| 14 | MiniMax Agent | MiniMax | Agent | ✅ verified | winget: MiniMax.MiniMaxAgent | brew: minimax | manual |
| 15 | ZCode | ZCode | Agent | ✅ verified | winget: ZhipuAI.ZCode | brew: zcode | manual |
| 16 | Grok Build | xAI | Agent | 🔄 community | manual | manual | manual |
| 17 | Antigravity | Google | Agent | ✅ verified | winget: Google.Antigravity | brew: antigravity | manual |
| 18 | Antigravity IDE | Google | IDE | ✅ verified | winget: Google.AntigravityIDE | brew: antigravity-ide | manual |
| 19 | Reasonix | Reasonix | Agent | ✅ verified | winget: ESEngine.Reasonix | brew: reasonix | manual |
| 20 | OpenCode | OpenCode | Agent | ✅ verified | winget: SST.OpenCodeDesktop | brew: opencode | manual |

### By Type

| Type | Count | Agents |
|------|-------|--------|
| IDE | 4 | Cursor, Windsurf, Trae, Antigravity IDE |
| Agent | 14 | Codex Desktop, Claude Desktop, Kimi Desktop, WorkBuddy, CodeBuddy, Mavis, Qoder, Qoder Work, MiniMax Agent, ZCode, Grok Build, Antigravity, Reasonix, OpenCode |
| Platform | 1 | Replit AI |
| Standalone | 1 | Trae Solo |

---

## Quick Reference

### CLI Install Commands

```bash
# npm packages (all platforms)
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

# pip packages (all platforms)
pip install aider-chat
pip install mentat
pip install gpt-engineer
pip install sweepai
pip install devon-agent
pip install open-interpreter
```

### Desktop Install Commands

#### Windows (winget) - Recommended

```powershell
# IDE
winget install Anysphere.Cursor
winget install Codeium.Windsurf
winget install ByteDance.Trae
winget install ByteDance.TraeSolo
winget install Google.AntigravityIDE

# AI Assistants
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
```

#### macOS (brew) - Recommended

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

#### Linux / Manual Installation

Download from official websites:
- cursor.com - Cursor IDE
- codeium.com - Windsurf IDE
- trae.ai - Trae / Trae Solo
- replit.com - Replit AI
- openai.com - Codex Desktop
- claude.ai - Claude Desktop
- kimi.ai - Kimi Desktop
- workbuddy.ai - WorkBuddy
- codebuddy.com - CodeBuddy
- marvis.qq.com - Mavis
- qoder.com - Qoder
- minimax.chat - MiniMax Agent
- zcode.ai - ZCode
- x.ai - Grok Build
- antigravity.google - Antigravity
- reasonix.ai - Reasonix
- opencode.ai - OpenCode

---

## All Agent Names

```
CLI:     codex, claude-code, kimi-code, aider, github-copilot-cli, continue-cli, 
         cody-cli, tabnine-cli, amazon-q, gemini-cli, qwen-coder-cli, deepseek-coder, 
         codeium-cli, supermaven-cli, mentat, gpt-engineer, sweep, devon, 
         open-interpreter, antigravity-cli

Desktop: cursor, windsurf, trae, trae-solo, replit-ai, codex-desktop, claude-desktop, 
         kimi-desktop, workbuddy, codebuddy, mavis, qoder, qoder-work, minimax-agent, 
         zcode, gork-build, antigravity, antigravity-ide, reasonix, opencode
```

---

## Summary

| Category | Count | Verified | Community |
|----------|-------|----------|-----------|
| CLI Agents | 20 | 11 | 9 |
| Desktop Agents | 20 | 18 | 2 |
| **Total** | **40** | **29** | **11** |

---

## Schema

The agent catalog is defined in `agents.json` and validated against `agents.schema.json`. 

Key fields:
- `id`: Unique identifier (lowercase, hyphenated)
- `name`: Display name
- `kind`: `cli` or `desktop`
- `provider`: Company or individual
- `installers`: Platform-specific installation configurations
- `status`: `verified`, `community`, `manual`, or `deprecated`
- `catalog_verified_at`: Date when catalog info was verified
- `installer_verified_at`: Date when installer was verified

For the complete schema, see [agents.schema.json](agents.schema.json).