# AgentHub Management Specification

> 版本：v0.2  
> 创建日期：2026-06-24  
> 更新日期：2026-06-27  
> 状态：设计文档（Package 模块已实现）

## 概述

本文档定义 AgentHub 六大核心模块（package, config, prompt, session, skill, memory）的完整生命周期管理规范，包括数据模型、CRUD 操作、状态监控、审计日志、导入导出和备份恢复。

### 实施状态

| 模块 | 状态 | 说明 |
|------|------|------|
| Package | ✅ 已实现 | `agenthub-core` crate + CLI + GUI，支持 40 个代理 |
| Config | 📋 设计中 | 详见下方设计文档 |
| Prompt | 📋 设计中 | 详见下方设计文档 |
| Session | 📋 设计中 | 详见下方设计文档 |
| Skill | 🟡 基础存在 | `.mimocode/skills/` 目录已有示例 |
| Memory | 🟡 基础存在 | MiMoCode 内置 memory 系统 |

---

## 统一管理原则

### 1. 命名规范

```
agenthub <module> <action> [target] [options]
```

| Action | 说明 | 示例 |
|--------|------|------|
| list | 列出所有条目 | `agenthub package list` |
| show | 查看详情 | `agenthub config show codex` |
| create | 创建新条目 | `agenthub prompt create "code-review"` |
| edit | 编辑条目 | `agenthub prompt edit code-review-v2` |
| delete | 删除条目 | `agenthub skill delete rust-dev` |
| export | 导出数据 | `agenthub memory export --format json` |
| import | 导入数据 | `agenthub config import config.yaml` |
| backup | 备份模块 | `agenthub session backup --output backup.tar` |
| restore | 恢复数据 | `agenthub prompt restore backup.tar` |
| status | 状态概览 | `agenthub package status` |
| audit | 审计日志 | `agenthub config audit --last 7d` |
| doctor | 健康检查 | `agenthub package doctor` |

### 2. 存储结构

```
~/.agenthub/
├── config/
│   ├── global.yaml          # 全局配置
│   └── agents/
│       ├── codex.yaml       # 单 Agent 配置
│       └── claude-code.yaml
├── prompts/
│   ├── templates/
│   │   ├── code-review.yaml
│   │   └── refactor.yaml
│   └── versions/
│       └── code-review/
│           ├── v1.yaml
│           └── v2.yaml
├── sessions/
│   ├── index.yaml           # 会话索引
│   └── data/
│       ├── ses_001.yaml
│       └── ses_002.yaml
├── skills/
│   ├── installed/
│   │   ├── rust-dev/
│   │   └── elixir-dev/
│   └── available/
│       └── registry.yaml
├── memory/
│   ├── global.md
│   ├── projects/
│   │   └── <project-hash>/
│   │       └── MEMORY.md
│   └── sessions/
│       └── <session-id>/
│           ├── checkpoint.md
│           └── notes.md
├── audit/
│   └── audit.log
└── backups/
    ├── 2026-06-24-full.tar.gz
    └── 2026-06-24-config-only.tar.gz
```

### 3. 审计日志格式

```yaml
# audit/audit.log
- timestamp: 2026-06-24T10:30:00Z
  module: package
  action: install
  target: codex
  user: mark
  result: success
  details:
    version: v0.141.0
    manager: npm
  duration_ms: 12500
```

---

## Module 1: Package（安装管理）

> **状态**: ✅ 已实现（v1.0）
> 
> 核心实现：
> - `agenthub-core` crate: 代理模型、清单加载、安装器接口
> - `agenthub-cli`: CLI 命令行工具
> - `agenthub-ui`: Tauri + Vue 3 桌面应用
> - `agents.json`: 共享代理清单（40 个代理）

### 数据模型

```json
// agents.json (共享清单)
{
  "id": "codex",
  "name": "Codex",
  "kind": "cli",
  "provider": "OpenAI",
  "description": "AI coding assistant powered by GPT-4",
  "homepage": "https://openai.com",
  "installers": {
    "windows": {
      "manager": "npm",
      "package": "@openai/codex"
    },
    "macos": {
      "manager": "npm",
      "package": "@openai/codex"
    },
    "linux": {
      "manager": "npm",
      "package": "@openai/codex"
    }
  },
  "status": "verified",
  "catalog_verified_at": "2026-06-27",
  "installer_verified_at": "2026-06-27"
}
```

### CRUD 操作

| 操作 | CLI 命令 | 说明 |
|------|----------|------|
| list | `agenthub list [--type cli\|desktop]` | 列出已注册的 Agent |
| search | `agenthub search <query> [--type cli\|desktop]` | 搜索 Agent 清单 |
| info | `agenthub info <name>` | 查看 Agent 详情（含安装状态） |
| install | `agenthub install <name> [--dry-run] [--yes]` | 安装 Agent |
| uninstall | `agenthub uninstall <name> [--dry-run] [--yes]` | 卸载 Agent |
| doctor | `agenthub doctor` | 检查环境和依赖 |

### 状态监控

```bash
agenthub list
```

输出示例：
```
Available Agents:
+--------------------+--------------------+------+------------------+-----------+----------+
| ID                 | Name               | Type | Provider         | Status    | Platform |
+--------------------+--------------------+------+------------------+-----------+----------+
| codex              | Codex              | CLI  | OpenAI           | Verified  | Npm      |
| claude-code        | Claude Code        | CLI  | Anthropic        | Verified  | Npm      |
| cursor             | Cursor             | Desktop | Cursor       | Verified  | Winget   |
...

Total: 40 agents (20 CLI, 20 Desktop)
```

### 健康检查

```bash
agenthub doctor
```

输出示例：
```
🏥 AgentHub Environment Check
----------------------------------------

Platform:
  Windows

Package Managers:
  ❌ npm - not found
  ✅ pip - pip 25.2
  ✅ winget - v1.29.170-preview

Agent Catalog:
  📦 20 CLI agents
  🖥️  20 Desktop agents
  📊 40 total agents

Installable Agents:
  38 agents can be installed on this platform

----------------------------------------
✅ Environment check complete!
```

检查项：
- 包管理器可用性（npm, pip, winget, brew）
- 清单完整性（所有条目有有效的 installer）
- 版本同步（本地版本 vs 远程最新版本）
- 依赖冲突检测

### 导入导出

```bash
# 导出清单
agenthub package export --format yaml --output agents.yaml
agenthub package export --format json --output agents.json

# 导入清单
agenthub package import agents.yaml
agenthub package import agents.yaml --merge  # 合并而非覆盖
```

### 备份恢复

```bash
# 备份清单和安装状态
agenthub package backup --output package-backup.tar.gz

# 恢复
agenthub package restore package-backup.tar.gz
```

---

## Module 2: Config（配置管理）

### 数据模型

```yaml
# ~/.agenthub/config/agents/codex.yaml
agent_id: codex
version: 1
environment: production          # development | staging | production

settings:
  model: gpt-4
  temperature: 0.7
  max_tokens: 4096
  timeout: 30
  auto_save: true

secrets:
  api_key: "${OPENAI_API_KEY}"   # 引用环境变量
  org_id: "${OPENAI_ORG_ID}"

environment_variables:
  OPENAI_API_KEY: "sk-..."
  OPENAI_ORG_ID: "org-..."

custom:
  preferred_language: typescript
  code_style: prettier

metadata:
  created_at: 2026-06-24T10:00:00Z
  updated_at: 2026-06-24T15:30:00Z
  created_by: mark
```

### CRUD 操作

| 操作 | CLI 命令 | 说明 |
|------|----------|------|
| list | `agenthub config list [--agent <name>]` | 列出所有配置 |
| show | `agenthub config show <agent>` | 查看 Agent 配置 |
| set | `agenthub config set <agent> <key> <value>` | 设置配置项 |
| get | `agenthub config get <agent> <key>` | 获取配置值 |
| unset | `agenthub config unset <agent> <key>` | 删除配置项 |
| edit | `agenthub config edit <agent>` | 用编辑器打开配置文件 |
| reset | `agenthub config reset <agent>` | 重置为默认配置 |
| diff | `agenthub config diff <agent>` | 对比当前配置与默认值 |
| validate | `agenthub config validate <agent>` | 校验配置完整性 |

### 密钥管理

```bash
# 存储密钥（写入系统密钥链，不明文存储）
agenthub config set-secret codex api_key "sk-..."

# 查看密钥引用（不显示实际值）
agenthub config show-secrets codex

# 轮换密钥
agenthub config rotate-secret codex api_key

# 删除密钥
agenthub config delete-secret codex api_key
```

### 多环境支持

```bash
# 切换环境
agenthub config env production
agenthub config env development

# 查看当前环境
agenthub config env

# 为特定环境设置配置
agenthub config set codex model gpt-4 --env production
agenthub config set codex model gpt-3.5-turbo --env development
```

### 配置模板

```bash
# 从模板创建配置
agenthub config create codex --template typescript-dev

# 保存当前配置为模板
agenthub config save-template codex --name "my-typescript-setup"

# 列出可用模板
agenthub config templates
```

### 导入导出

```bash
# 导出所有配置
agenthub config export --output config-backup.yaml

# 导出单个 Agent 配置
agenthub config export codex --output codex-config.yaml

# 导入配置
agenthub config import config-backup.yaml
agenthub config import codex-config.yaml --agent codex
```

### 审计日志

```bash
# 查看配置变更历史
agenthub config audit --last 7d
agenthub config audit --agent codex --last 30d
```

输出示例：
```
Config Audit Log (last 7 days)
══════════════════════════════
2026-06-24 15:30  mark  config.set  codex.model = gpt-4
2026-06-24 14:00  mark  config.set-secret  codex.api_key (rotated)
2026-06-23 10:00  mark  config.create  codex (from template)
```

---

## Module 3: Prompt（提示词管理）

### 数据模型

```yaml
# ~/.agenthub/prompts/templates/code-review.yaml
id: code-review
name: 代码审查
description: 对代码进行结构化审查
version: 2
author: mark

template: |
  Review the following {{language}} code for:
  1. Correctness and edge cases
  2. Performance implications
  3. Security vulnerabilities
  4. Code style and readability

  Code:
  ```{{language}}
  {{code}}
  ```

  Context: {{context}}

variables:
  - name: language
    type: string
    required: true
    description: 编程语言
  - name: code
    type: string
    required: true
    description: 待审查的代码
  - name: context
    type: string
    required: false
    default: "General review"
    description: 审查上下文

tags: [review, quality, security]
category: development
created_at: 2026-06-24T10:00:00Z
updated_at: 2026-06-24T12:00:00Z

usage:
  total_uses: 45
  last_used: 2026-06-24T11:30:00Z
  avg_rating: 4.2
```

### CRUD 操作

| 操作 | CLI 命令 | 说明 |
|------|----------|------|
| list | `agenthub prompt list [--category <cat>] [--tag <tag>]` | 列出提示词模板 |
| show | `agenthub prompt show <id>` | 查看模板详情 |
| create | `agenthub prompt create <id> --name <name> --template <file>` | 创建新模板 |
| edit | `agenthub prompt edit <id>` | 编辑模板 |
| delete | `agenthub prompt delete <id>` | 删除模板 |
| clone | `agenthub prompt clone <id> --new-id <new-id>` | 克隆模板 |
| render | `agenthub prompt render <id> --var language=typescript --var code="..."` | 渲染模板 |
| test | `agenthub prompt test <id> --var language=python` | 测试模板（显示渲染结果） |
| rate | `agenthub prompt rate <id> --rating 4` | 为模板评分 |

### 版本管理

```bash
# 查看版本历史
agenthub prompt versions code-review

# 查看特定版本
agenthub prompt show code-review --version 1

# 回滚到旧版本
agenthub prompt rollback code-review --version 1

# 创建新版本（从当前状态）
agenthub prompt bump-version code-review --message "Added security section"
```

### 分类与标签

```bash
# 按分类列出
agenthub prompt list --category development
agenthub prompt list --category writing

# 按标签筛选
agenthub prompt list --tag review
agenthub prompt list --tag security,performance

# 添加标签
agenthub prompt tag code-review performance

# 移除标签
agenthub prompt untag code-review performance
```

### 使用统计

```bash
# 查看使用统计
agenthub prompt stats code-review

# 查看最常用提示词
agenthub prompt top --limit 10

# 查看最近使用的提示词
agenthub prompt recent --limit 5
```

### 导入导出

```bash
# 导出单个模板
agenthub prompt export code-review --output code-review.yaml

# 导出所有模板
agenthub prompt export --all --output prompts-backup.tar.gz

# 导入模板
agenthub prompt import code-review.yaml

# 从社区导入
agenthub prompt install community/code-review
```

---

## Module 4: Session（会话管理）

### 数据模型

```yaml
# ~/.agenthub/sessions/data/ses_001.yaml
id: ses_001
title: "重构认证模块"
agent: claude-code
model: claude-sonnet-4-20250514
project: my-app
branch: feature/auth-refactor

status: completed                # active | paused | completed | failed
started_at: 2026-06-24T10:00:00Z
ended_at: 2026-06-24T11:30:00Z
duration_minutes: 90

messages:
  - role: user
    content: "Refactor the auth module to use JWT tokens"
    timestamp: 2026-06-24T10:00:00Z
  - role: assistant
    content: "I'll help you refactor the auth module..."
    timestamp: 2026-06-24T10:00:05Z
    tokens: 150
  # ...

usage:
  total_tokens: 25000
  input_tokens: 15000
  output_tokens: 10000
  estimated_cost_usd: 0.075

artifacts:
  files_modified:
    - src/auth/jwt.ts
    - src/auth/middleware.ts
    - src/auth/types.ts
  tests_passed: 12
  tests_failed: 0

tags: [refactor, auth, jwt]
rating: 5
notes: "Successfully refactored auth to JWT-based system"

metadata:
  cli_version: 0.1.0
  os: windows
  node_version: 20.10.0
```

### CRUD 操作

| 操作 | CLI 命令 | 说明 |
|------|----------|------|
| list | `agenthub session list [--agent <name>] [--status <status>] [--last <duration>]` | 列出会话 |
| show | `agenthub session show <id>` | 查看会话详情 |
| search | `agenthub session search <query>` | 全文搜索会话内容 |
| delete | `agenthub session delete <id>` | 删除会话 |
| tag | `agenthub session tag <id> <tag>` | 为会话添加标签 |
| untag | `agenthub session untag <id> <tag>` | 移除标签 |
| rate | `agenthub session rate <id> --rating 5` | 为会话评分 |
| note | `agenthub session note <id> --text "..."` | 添加备注 |

### 会话回放

```bash
# 回放会话（交互式查看）
agenthub session replay <id>

# 导出会话为 Markdown
agenthub session export <id> --format markdown --output session.md

# 导出会话为 HTML
agenthub session export <id> --format html --output session.html

# 导出会话为 JSON
agenthub session export <id> --format json --output session.json
```

### 成本追踪

```bash
# 查看会话成本
agenthub session cost <id>

# 查看总成本（按时间范围）
agenthub session cost --last 7d
agenthub session cost --last 30d --group-by agent

# 查看成本趋势
agenthub session cost-trend --last 30d

# 设置成本阈值告警
agenthub session set-budget --daily 5.00 --monthly 50.00
```

输出示例：
```
Session Cost Report (last 7 days)
═════════════════════════════════
Total Sessions: 23
Total Tokens: 450,000
Total Cost: $1.35

By Agent:
  claude-code    15 sessions   320,000 tokens   $0.96
  codex           8 sessions   130,000 tokens   $0.39

Daily Breakdown:
  2026-06-24   5 sessions   $0.25
  2026-06-23   4 sessions   $0.20
  2026-06-22   6 sessions   $0.30
  ...
```

### 会话模板

```bash
# 从当前会话创建模板
agenthub session save-template <id> --name "code-review-workflow"

# 从模板创建新会话
agenthub session create --template code-review-workflow

# 列出会话模板
agenthub session templates
```

### 统计分析

```bash
# 会话统计
agenthub session stats

# 按 Agent 统计
agenthub session stats --group-by agent

# 按项目统计
agenthub session stats --group-by project
```

输出示例：
```
Session Statistics
═════════════════
Total Sessions: 156
Active Sessions: 3
Completed: 148
Failed: 5

Top Agents:
  1. claude-code    89 sessions (57%)
  2. codex          45 sessions (29%)
  3. kimi-code      22 sessions (14%)

Avg Session Duration: 45 minutes
Avg Tokens per Session: 18,000
Total Cost This Month: $12.50
```

### 导入导出

```bash
# 导出所有会话
agenthub session export --all --output sessions-backup.tar.gz

# 导入会话
agenthub session import sessions-backup.tar.gz
```

---

## Module 5: Skill（技能管理）

### 数据模型

```yaml
# ~/.agenthub/skills/installed/rust-dev/SKILL.md
---
name: rust-dev
description: "Rust development workflow with cargo, testing, and linting"
version: 1.2.0
author: agenthub
license: MIT
homepage: https://github.com/agenthub/skills/rust-dev

triggers:
  - "*.rs"
  - "Cargo.toml"
  - "Cargo.lock"

tags: [rust, cargo, testing, clippy]
category: development
min_agenthub_version: 0.1.0

dependencies:
  - name: cargo
    required: true
    check: "cargo --version"
  - name: clippy
    required: false
    check: "cargo clippy --version"

config:
  run_tests_on_save: true
  auto_format: true
  strict_clippy: false
---

# Rust Development Workflow

## Overview
Standard Rust development cycle: build → test → format → lint.

## Commands
...
```

### CRUD 操作

| 操作 | CLI 命令 | 说明 |
|------|----------|------|
| list | `agenthub skill list [--installed] [--category <cat>]` | 列出技能 |
| show | `agenthub skill show <name>` | 查看技能详情 |
| install | `agenthub skill install <name>` | 安装技能 |
| uninstall | `agenthub skill uninstall <name>` | 卸载技能 |
| update | `agenthub skill update <name>` | 更新技能 |
| create | `agenthub skill create <name>` | 创建新技能（交互式） |
| edit | `agenthub skill edit <name>` | 编辑技能 |
| delete | `agenthub skill delete <name>` | 删除技能 |
| enable | `agenthub skill enable <name>` | 启用技能 |
| disable | `agenthub skill disable <name>` | 禁用技能 |

### 技能市场

```bash
# 搜索可用技能
agenthub skill search rust

# 查看技能详情（从市场）
agenthub skill info community/rust-dev

# 安装社区技能
agenthub skill install community/rust-dev

# 发布技能到市场
agenthub skill publish rust-dev

# 更新技能市场索引
agenthub skill update-registry
```

### 技能配置

```bash
# 查看技能配置
agenthub skill config rust-dev

# 设置技能配置
agenthub skill config rust-dev run_tests_on_save false

# 重置技能配置
agenthub skill config rust-dev --reset
```

### 技能依赖

```bash
# 检查技能依赖
agenthub skill check-deps rust-dev

# 安装技能依赖
agenthub skill install-deps rust-dev
```

### 技能组合（工作流）

```yaml
# ~/.agenthub/workflows/full-review.yaml
name: Full Code Review
description: 完整的代码审查工作流

steps:
  - skill: rust-dev
    action: build
    description: "Build the project"

  - skill: rust-dev
    action: test
    description: "Run tests"

  - skill: code-review
    action: review
    description: "Review code changes"
    config:
      focus: [security, performance]

  - skill: elixir-dev
    action: format
    description: "Format code"
```

```bash
# 运行工作流
agenthub workflow run full-review

# 列出工作流
agenthub workflow list

# 创建工作流
agenthub workflow create full-review
```

### 统计

```bash
# 技能使用统计
agenthub skill stats

# 最常用技能
agenthub skill top --limit 10
```

### 导入导出

```bash
# 导出技能
agenthub skill export rust-dev --output rust-dev.tar.gz

# 导入技能
agenthub skill import rust-dev.tar.gz

# 导出所有已安装技能
agenthub skill export --all --output skills-backup.tar.gz
```

---

## Module 6: Memory（记忆管理）

### 数据模型

```yaml
# ~/.agenthub/memory/config.yaml
storage:
  backend: file                 # file | sqlite | hybrid
  path: ~/.agenthub/memory
  
indexing:
  enabled: true
  engine: bm25                  # bm25 | vector | hybrid
  update_on_write: true
  
retention:
  global: forever
  projects: 90d
  sessions: 30d
  auto_archive: true
  archive_after: 7d
  
privacy:
  encrypt_at_rest: false
  exclude_patterns:
    - "*.env"
    - "credentials.*"
    - "*.key"
```

### CRUD 操作

| 操作 | CLI 命令 | 说明 |
|------|----------|------|
| list | `agenthub memory list [--scope <scope>] [--type <type>]` | 列出记忆条目 |
| show | `agenthub memory show <path>` | 查看记忆内容 |
| search | `agenthub memory search <query> [--scope <scope>]` | 搜索记忆 |
| create | `agenthub memory create <scope> --title <title> --content <content>` | 创建记忆 |
| edit | `agenthub memory edit <path>` | 编辑记忆 |
| delete | `agenthub memory delete <path>` | 删除记忆 |
| tag | `agenthub memory tag <path> <tag>` | 添加标签 |
| archive | `agenthub memory archive <path>` | 归档记忆 |
| restore | `agenthub memory restore <path>` | 恢复归档的记忆 |

### 记忆范围

```bash
# 全局记忆
agenthub memory list --scope global
agenthub memory search "rust" --scope global

# 项目记忆
agenthub memory list --scope projects
agenthub memory show projects/my-app/MEMORY.md

# 会话记忆
agenthub memory list --scope sessions
agenthub memory show sessions/ses_001/checkpoint.md
```

### 记忆类型

```bash
# 按类型筛选
agenthub memory list --type pinned       # 固定记忆（不会衰减）
agenthub memory list --type learning     # 学习笔记
agenthub memory list --type decision     # 决策记录
agenthub memory list --type reference    # 参考资料
agenthub memory list --type feedback     # 反馈记录

# 设置记忆类型
agenthub memory set-type projects/my-app/MEMORY.md pinned
```

### 语义搜索

```bash
# 基础搜索
agenthub memory search "JWT authentication"

# 带过滤器的搜索
agenthub memory search "database" --scope projects --type decision

# 搜索并显示上下文
agenthub memory search "error handling" --context 3

# 搜索相似记忆
agenthub memory similar projects/my-app/MEMORY.md
```

### 知识图谱

```bash
# 查看知识图谱（文本模式）
agenthub memory graph

# 查看特定实体的关系
agenthub memory graph --entity "JWT"

# 导出知识图谱
agenthub memory graph --format dot --output graph.dot
agenthub memory graph --format json --output graph.json
```

输出示例：
```
Knowledge Graph
═══════════════
Entities: 45 | Relations: 78

JWT ──uses──> Authentication
JWT ──stores──> Token
Authentication ──requires──> Database
Database ──uses──> PostgreSQL

Clusters:
  - Auth Cluster: JWT, Authentication, Token, Session
  - DB Cluster: Database, PostgreSQL, Migration, Schema
```

### 记忆维护

```bash
# 清理过期记忆
agenthub memory cleanup

# 重建索引
agenthub memory reindex

# 合并重复记忆
agenthub memory dedupe

# 优化存储
agenthub memory optimize

# 查看存储统计
agenthub memory stats
```

输出示例：
```
Memory Statistics
═════════════════
Total Entries: 234
Storage Size: 2.5 MB

By Scope:
  global:     12 entries (5%)
  projects:  156 entries (67%)
  sessions:   66 entries (28%)

By Type:
  pinned:      8 entries
  learning:   45 entries
  decision:   23 entries
  reference: 158 entries

Index Status: healthy
Last Optimized: 2026-06-24T10:00:00Z
```

### 导入导出

```bash
# 导出所有记忆
agenthub memory export --output memory-backup.tar.gz

# 导出特定范围
agenthub memory export --scope projects --output projects-memory.tar.gz

# 导入记忆
agenthub memory import memory-backup.tar.gz

# 导入并合并
agenthub memory import memory-backup.tar.gz --merge
```

---

## 跨模块操作

### 全局状态

```bash
agenthub status
```

输出示例：
```
AgentHub Status
═══════════════
Version: 0.1.0
Platform: windows
Uptime: 2h 30m

Modules:
  ✅ package    25 agents registered, 8 installed
  ✅ config     8 configs, 3 environments
  ✅ prompt     12 templates, 45 total uses
  ✅ session    156 sessions, 3 active
  ✅ skill      5 installed, 2 enabled
  ✅ memory     234 entries, 2.5 MB

Health:
  ✅ All systems operational
  ⚠ 2 deprecated agents detected
```

### 全局备份

```bash
# 备份所有模块
agenthub backup --output full-backup.tar.gz

# 备份特定模块
agenthub backup --modules config,prompt,memory --output partial-backup.tar.gz

# 恢复所有模块
agenthub restore full-backup.tar.gz

# 恢复特定模块
agenthub restore partial-backup.tar.gz --modules config,prompt
```

### 全局审计

```bash
# 查看所有模块的审计日志
agenthub audit --last 7d

# 按模块筛选
agenthub audit --module config --last 30d

# 按操作类型筛选
agenthub audit --action delete --last 7d

# 导出审计日志
agenthub audit --export --format json --output audit.json
```

### 全局健康检查

```bash
agenthub doctor
```

检查项：
- 所有模块的存储完整性
- 索引健康状态
- 依赖项可用性
- 配置一致性
- 存储空间使用情况

---

## 实施优先级

| 阶段 | 模块 | 功能 | 预计时间 |
|------|------|------|----------|
| P0 | package | CRUD + install/uninstall + status | 已有原型 |
| P0 | config | CRUD + secrets + environments | 2 周 |
| P1 | memory | CRUD + search + retention | 2 周 |
| P1 | session | CRUD + search + cost tracking | 3 周 |
| P2 | prompt | CRUD + versions + render | 2 周 |
| P2 | skill | CRUD + marketplace + workflows | 3 周 |
| P3 | 全局 | backup/restore + audit + doctor | 2 周 |

---

## 附录：CLI 命令速查表

```bash
# Package
agenthub package list|show|add|edit|remove|install|uninstall|update|search|status|doctor|export|import|backup|restore

# Config
agenthub config list|show|set|get|unset|edit|reset|diff|validate|env|templates|export|import|audit|set-secret|show-secrets|rotate-secret|delete-secret

# Prompt
agenthub prompt list|show|create|edit|delete|clone|render|test|rate|versions|rollback|tag|untag|stats|top|recent|export|import|install

# Session
agenthub session list|show|search|delete|tag|untag|rate|note|replay|export|import|cost|cost-trend|set-budget|templates|stats

# Skill
agenthub skill list|show|install|uninstall|update|create|edit|delete|enable|disable|search|info|publish|update-registry|config|check-deps|install-deps|stats|top|export|import

# Memory
agenthub memory list|show|search|create|edit|delete|tag|archive|restore|set-type|graph|cleanup|reindex|dedupe|optimize|stats|export|import

# Global
agenthub status|backup|restore|audit|doctor
```
