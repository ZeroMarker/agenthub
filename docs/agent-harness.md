# Agent Harness 数据与配置说明

本文档说明 AgentHub 项目中 agent harness 相关的数据入口：统一配置文件、各 Agent 原生配置文件、对话记录、Skills 和 Memory。

> 状态：实现说明。以下路径以当前 `agenthub-ui/src-tauri/src/main.rs` 和 `agenthub-core` 模块为准；`management.md` 中的 `~/.agenthub` 是较早的规划路径。

## Harness 边界

AgentHub 的 harness 是包在各类 AI 编程助手外层的统一管理壳层，负责：

- 从 `agents.json` 读取支持的 Agent 目录和安装器配置。
- 为每个 Agent 保存 AgentHub 自己的运行时配置。
- 读取或编辑部分 Agent 的原生配置文件。
- 记录 AgentHub 管理的会话、提示词、技能和记忆。

AgentHub 当前不直接接管第三方 Agent 的全部内部状态。比如 Codex、Claude Code、Cursor 等工具自己的完整对话历史，仍以各自工具的本地实现为准；AgentHub 的 `session` 模块保存的是 AgentHub 管理层记录的对话摘要和消息流。

## 目录总览

桌面端使用系统配置目录下的 `agenthub` 作为数据根目录：

| 平台 | AgentHub 数据根目录 |
|------|---------------------|
| Windows | `%APPDATA%\agenthub` |
| macOS | `~/Library/Application Support/agenthub` |
| Linux | `~/.config/agenthub` |

当前实现的子目录：

```text
<agenthub-config-dir>/
├── agents/                  # AgentHub 统一运行时配置
│   └── <agent-id>.yaml
├── prompts/
│   └── templates/
│       └── <prompt-id>.yaml
├── sessions/
│   └── data/
│       └── <session-id>.yaml
├── skills/
│   └── installed/
│       └── <skill-name>/
│           ├── SKILL.md
│           └── .enabled
└── memory/
    ├── global/
    │   └── <title>.md
    ├── projects/
    │   └── <project-id>/
    │       └── <title>.md
    └── sessions/
        └── <session-id>/
            └── <title>.md
```

## 统一配置文件

AgentHub 的统一配置由 `ConfigManager` 管理，文件路径为：

```text
<agenthub-config-dir>/agents/<agent-id>.yaml
```

示例：

```yaml
agent_id: codex
version: 1
environment: development
settings:
  model: gpt-5
  approval_policy: never
secrets: {}
environment_variables:
  OPENAI_API_KEY: "${OPENAI_API_KEY}"
custom:
  project_profile: rust-vue
metadata:
  created_at: 2026-07-07T10:00:00Z
  updated_at: 2026-07-07T10:30:00Z
  created_by: null
```

字段说明：

| 字段 | 说明 |
|------|------|
| `agent_id` | 对应 `agents.json` 中的 Agent ID |
| `version` | 配置结构版本 |
| `environment` | `development`、`staging` 或 `production` |
| `settings` | AgentHub 运行时设置 |
| `secrets` | 密钥引用或占位。当前是 YAML 字段，后续应接入系统密钥链 |
| `environment_variables` | 启动 Agent 时可注入的环境变量 |
| `custom` | 不进入通用字段的扩展配置 |
| `metadata` | 创建和更新时间 |

## Agent 原生配置文件

桌面端还提供 `get_native_config` 和 `save_native_config`，用于查看或保存部分 Agent 的原生配置文件。映射如下：

| Agent ID | 原生配置路径 |
|----------|--------------|
| `codex` / `codex-desktop` | `~/.codex/config.toml` |
| `claude-code` / `claude-desktop` | `~/.claude/settings.json` |
| `cursor` | `~/.cursor/argv.json` |
| `windsurf` | `~/.windsurf/settings.json` |
| `kimi-code` / `kimi-desktop` | `~/.kimi/config.toml` |
| `qwen-code` | `~/.qwen/settings.json` |
| `mimo-code` | `~/.local/share/mimocode/auth.json` |
| `reasonix` / `reasonix-cli` | `~/.reasonix/config.json` |
| `trae` / `trae-solo` | `~/.trae/argv.json` |
| `antigravity` | `~/.antigravity/argv.json` |
| `antigravity-ide` | `~/.antigravity-ide/argv.json` |
| `qoder` / `qoder-work` | `~/.qoder/argv.json` |
| `minimax-agent` | `~/.minimax-agent/config.json` |
| `zcode` | `~/.zcode/config.json` |
| `workbuddy` | `~/.workbuddy/.mcp.json` |
| `codebuddy` | `~/.codebuddy/config.json` |
| `openwork` | `~/.openwork/config.json` |
| `opencode` | `<system-config-dir>/ai.opencode.desktop/opencode.settings` |
| `grok-cli` | `~/.grok/auth.json` |

读取逻辑会按扩展名解析 JSON 或 TOML，并保留原始文本。保存逻辑会直接覆盖原文件内容；调用方应在 UI 中提供确认和备份提示。

## 对话记录

AgentHub 的会话记录由 `SessionManager` 管理，文件路径为：

```text
<agenthub-config-dir>/sessions/data/<session-id>.yaml
```

示例：

```yaml
id: ses_1780000000000_123456
title: "重构认证模块"
agent: codex
model: gpt-5
project: agenthub
status: active
started_at: 2026-07-07T10:00:00Z
ended_at: null
duration_minutes: null
messages:
  - role: user
    content: "添加配置说明"
    timestamp: 2026-07-07T10:00:00Z
    tokens: null
usage:
  total_tokens: 12000
  input_tokens: 8000
  output_tokens: 4000
  estimated_cost_usd: 0.05
tags:
  - docs
rating: null
notes: null
```

支持的状态为 `active`、`paused`、`completed`、`failed`。会话列表按 `started_at` 倒序排序，搜索会匹配标题、Agent、消息内容和备注。

## Skills

AgentHub 的技能由 `SkillManager` 管理，安装目录为：

```text
<agenthub-config-dir>/skills/installed/<skill-name>/
```

桌面端还会额外扫描 Codex skills 目录：

```text
~/.codex/skills
```

技能必须包含 `SKILL.md`，并使用 YAML frontmatter 描述清单：

```markdown
---
name: rust-dev
description: "Rust development workflow"
version: 0.1.0
author: agenthub
triggers:
  - "*.rs"
tags:
  - rust
category: development
dependencies:
  - name: cargo
    required: true
    check: "cargo --version"
config:
  run_tests_on_save: true
---

# Rust Development Workflow

Build, test, format and lint Rust projects.
```

启用状态由同目录下的空文件 `.enabled` 表示：

| 文件 | 含义 |
|------|------|
| `SKILL.md` | 技能说明和清单 |
| `.enabled` | 存在表示启用，不存在表示禁用 |

依赖检查会执行 `dependencies[].check` 中声明的命令，因此技能来源必须可信。

## Memory

AgentHub 的记忆由 `MemoryManager` 管理，文件路径按 scope 分层：

| Scope | 路径 |
|-------|------|
| `global` | `<agenthub-config-dir>/memory/global/<title>.md` |
| `project` | `<agenthub-config-dir>/memory/projects/<project-id>/<title>.md` |
| `session` | `<agenthub-config-dir>/memory/sessions/<session-id>/<title>.md` |

记忆文件是 Markdown，带 YAML frontmatter：

```markdown
---
path: global/code-style.md
scope: global
scope_id: null
title: Code Style
content: ""
memory_type: decision
tags:
  - docs
created_at: 2026-07-07T10:00:00Z
updated_at: 2026-07-07T10:00:00Z
---

项目文档使用中文说明，代码和路径保持原始英文命名。
```

支持的记忆类型：

| 类型 | 用途 |
|------|------|
| `pinned` | 长期固定信息 |
| `learning` | 学习和发现 |
| `decision` | 架构或产品决策 |
| `reference` | 参考资料 |
| `feedback` | 用户反馈 |
| `free` | 默认自由文本 |

搜索支持文件扫描/BM25、向量和混合检索；向量索引持久化于 `memory/vector_index.json`，知识图谱持久化于 `memory/graph.json`。

## 安全和隐私约定

- 不要把 API Key、令牌、Cookie 或私钥直接写入 `settings`、`memory`、`session` 或 `SKILL.md`。
- 统一配置里的 `secrets` 仅保留兼容字段；生产级密钥应使用 `SecretStore` 的文件密钥链或 OS keyring 后端。
- 原生配置保存会覆盖第三方 Agent 文件，修改前应提示用户确认。
- `sessions` 和 `memory` 可能包含用户输入、项目路径和业务上下文，导出或同步前应做脱敏。
- 第三方 skill 的依赖检查命令会被执行，安装前应确认来源可信。

## 相关源码

| 功能 | 源码 |
|------|------|
| 统一配置 | `agenthub-core/src/config.rs` |
| 原生配置映射 | `agenthub-ui/src-tauri/src/main.rs` |
| 会话记录 | `agenthub-core/src/session.rs` |
| Skills | `agenthub-core/src/skill.rs` |
| Memory | `agenthub-core/src/memory.rs` |
| Prompt 模板 | `agenthub-core/src/prompt.rs` |
| 远程 registry | `agenthub-core/src/remote.rs`、`docs/remote-registry.md` |
