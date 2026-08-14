# AgentHub

统一管理多个 AI 编程助手的平台工具，支持命令行（CLI）与桌面图形界面（GUI）。

> **当前版本**：v0.1.0 · 详细路线图见 [PROJECT_PLAN.md](PROJECT_PLAN.md)

---

## 目录

- [功能特性](#功能特性)
- [项目架构](#项目架构)
- [环境要求](#环境要求)
- [快速开始](#快速开始)
- [支持的 Agent](#支持的-agent)
- [CLI 命令](#cli-命令)
- [GUI 界面](#gui-界面)
- [自定义 URL 协议](#自定义-url-协议)
- [Agent Harness 数据](#agent-harness-数据)
- [核心模块](#核心模块)
- [Agent 目录格式](#agent-目录格式)
- [开发指南](#开发指南)
- [测试](#测试)
- [CI/CD](#cicd)
- [项目状态](#项目状态)
- [常见问题](#常见问题)
- [贡献指南](#贡献指南)
- [许可证](#许可证)

---

## 功能特性

- **统一代理目录** — 一份 `agents.json` 作为单一数据源，CLI 与 GUI 共享
- **跨平台安装** — 支持 npm、pip、winget、brew-cask 四类包管理器
- **平台感知** — 每个 Agent 按 Windows / macOS / Linux 声明独立安装配置
- **安装预览** — `--dry-run` 预览命令，`--yes` 跳过确认
- **状态检测** — 自动检测已安装 Agent 及版本号
- **批量操作** — 一键批量安装/卸载，逐项返回结果
- **诊断工具** — `doctor` 命令检查环境依赖和清单完整性
- **JSON Schema 校验** — 保证目录数据一致性
- **桌面应用** — Tauri 2 + Vue 3 轻量桌面客户端
- **配置管理** — 多环境配置、语义校验与默认回退、变更历史与回滚
- **密钥安全** — 值永不进配置文件；OS keyring 后端 + 文件密钥链自动回退、轮换可回滚
- **成本可观测** — 会话 API 调用次数、按日成本趋势与 JSON 导出
- **技能作用域** — 项目级 / 用户级 / 全局级技能，同名项目优先
- **交互式仪表盘** — `status --html` 生成自包含可交互 Web 仪表盘（窗口切换/图表/钻取）
- **SMTP 直发** — email 告警通道可直接经 SMTP 发送，无需外部 MTA

---

## 项目架构

```
agenthub/
├── agenthub-core/            # Rust 共享核心库
│   └── src/
│       ├── agent.rs          # Agent 数据模型
│       ├── catalog.rs        # 目录加载与查询
│       ├── config.rs         # 配置管理
│       ├── diagnostic.rs     # 环境诊断
│       ├── error.rs          # 错误类型定义
│       ├── installer.rs      # 安装/卸载逻辑
│       ├── lib.rs            # 模块导出
│       ├── memory.rs         # 记忆管理
│       ├── prompt.rs         # 提示词模板管理
│       ├── session.rs        # 会话管理
│       ├── skill.rs          # 技能管理
│       └── status.rs         # 状态检测与版本解析
├── agenthub-ui/              # Tauri 桌面应用
│   ├── src/
│   │   ├── components/       # Vue 3 组件
│   │   │   ├── AgentList.vue       # 代理列表（搜索、筛选、排序、批量操作）
│   │   │   ├── ConfigManager.vue   # 配置管理
│   │   │   ├── DiagnosticView.vue  # 诊断视图
│   │   │   ├── MemoryManager.vue   # 记忆管理
│   │   │   ├── PromptManager.vue   # 提示词管理
│   │   │   ├── SessionManager.vue  # 会话管理
│   │   │   └── SkillManager.vue    # 技能管理
│   │   ├── App.vue
│   │   └── main.ts
│   ├── src-tauri/            # Tauri Rust 后端
│   │   ├── src/main.rs
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   └── package.json
├── agents.json               # Agent 目录（单一数据源）
├── agents.schema.json        # 目录 JSON Schema
├── Cargo.toml                # Rust workspace 配置
├── .github/workflows/
│   ├── ci.yml                # CI 流水线
│   └── release.yml           # 发布流水线
├── CHANGELOG.md
├── goal.md                   # 六大业务模块 + 概览 + 横切能力架构愿景
├── management.md             # 模块管理规范（含概览模块与横切能力）
└── PROJECT_PLAN.md           # v1.0 项目计划
```

### 数据流

```text
agents.json ──→ agenthub-core (Rust)
                  ├── catalog        目录解析与查询
                  ├── installer      安装命令生成与执行
                  ├── status         状态检测与版本解析
                  └── diagnostic     环境健康检查
                  │       │
                  ▼       ▼
             CLI 工具   Tauri 后端
                          │
                          ▼
                       Vue 3 前端
```

---

## 环境要求

| 依赖 | 版本 | 用途 |
|------|------|------|
| **Rust** | 1.75+ | 核心库与 Tauri 后端 |
| **Node.js** | LTS | 前端构建 |
| **npm** | 9+ | 前端依赖管理 |

### 平台特定依赖

**Windows**
- WebView2（Windows 10+ 自带）
- Visual Studio Build Tools

**macOS**
- Xcode Command Line Tools

**Linux**
```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

### 包管理器（按需安装）

| 包管理器 | 用途 | 安装 |
|----------|------|------|
| npm | CLI Agent 安装 | 随 Node.js 安装 |
| pip | Python Agent 安装 | 随 Python 安装 |
| winget | Windows 桌面 Agent | Windows 11 自带 |
| brew | macOS 桌面 Agent | [brew.sh](https://brew.sh) |

---

## 快速开始

### 1. 克隆仓库

```bash
git clone https://github.com/your-org/agenthub.git
cd agenthub
```

### 2. 构建并运行桌面应用

```bash
cd agenthub-ui
npm install
npm run tauri dev
```

### 3. 使用 CLI（开发中）

```bash
cargo run -- list          # 列出所有 Agent
cargo run -- search cursor # 搜索 Agent
cargo run -- info codex    # 查看详情
cargo run -- doctor        # 环境诊断
```

### 4. 构建发布版本

```bash
# 桌面应用
cd agenthub-ui
npm run tauri build

# 或直接 cargo 构建
cargo build --release
```

发布产物的代码签名策略与未签名声明见 [docs/signing-policy.md](docs/signing-policy.md)，校验和由 `scripts/generate-checksums.*` 自动生成。

---

## 支持的 Agent

> 完整目录见 [agents.json](agents.json)，Schema 定义见 [agents.schema.json](agents.schema.json)

### CLI Agent（7 个）

| Agent | 提供商 | 包名 | 包管理器 | 状态 |
|---|---|---|---|---|
| Claude Code | Anthropic | `@anthropic-ai/claude-code` | npm | verified |
| Codex | OpenAI | `@openai/codex` | npm | verified |
| Grok CLI | xAI | `xAI.GrokBuild` | winget | community |
| Kimi Code | Moonshot | `@moonshot-ai/kimi-code` | npm | community |
| MiMo Code | Xiaomi | `@mimo-ai/cli` | npm | community |
| Qwen Code | Alibaba | `@qwen-code/qwen-code` | npm | community |
| Reasonix CLI | Reasonix | `ESEngine.ReasonixCLI` | winget | community |

### Desktop Agent（18 个）

| Agent | 提供商 | Windows | macOS | 状态 |
|---|---|---|---|---|
| Antigravity | Google | winget | brew | verified |
| Antigravity IDE | Google | winget | brew | verified |
| Claude Desktop | Anthropic | winget | brew | verified |
| CodeBuddy | Tencent | winget | brew | verified |
| Codex Desktop | OpenAI | winget | brew | verified |
| Cursor | Cursor | winget | brew | verified |
| Kimi Desktop | Moonshot | winget | brew | verified |
| MiniMax Agent | MiniMax | winget | brew | verified |
| OpenCode | OpenCode | winget | brew | verified |
| OpenWork | DifferentAI | winget | — | community |
| Qoder | Qoder | winget | brew | verified |
| Qoder Work | Qoder | winget | brew | verified |
| Reasonix | Reasonix | winget | brew | verified |
| Trae | ByteDance | winget | brew | verified |
| Trae Solo | ByteDance | winget | brew | verified |
| Windsurf | Codeium | winget | brew | verified |
| WorkBuddy | Tencent | winget | brew | verified |
| ZCode | ZCode | winget | brew | verified |

### 支持状态说明

| 状态 | 含义 |
|------|------|
| `verified` | 官方验证过安装流程，信息可靠 |
| `community` | 社区贡献，未经官方验证 |
| `manual` | 无可靠包管理器来源，仅提供官网链接 |
| `deprecated` | 已废弃，不再维护 |
## CLI 命令

```
agenthub <command> [options]
```

| 命令 | 说明 | 示例 |
|------|------|------|
| `list` | 列出所有 Agent | `agenthub list --type cli` |
| `search` | 搜索 Agent | `agenthub search cursor` |
| `info` | 查看 Agent 详情 | `agenthub info codex` |
| `install` | 安装 Agent | `agenthub install codex --dry-run` |
| `uninstall` | 卸载 Agent | `agenthub uninstall codex --yes` |
| `doctor` | 环境诊断 | `agenthub doctor` |
| `status` | 工作区状态概览（`--trend`/`--html` 交互式仪表盘） | `agenthub status --html dashboard.html` |
| `audit` | 查询审计日志 | `agenthub audit --action install --last-days 7` |
| `backup` | 备份全部数据 | `agenthub backup --output ./backup.json` |
| `restore` | 从备份恢复 | `agenthub restore ./backup.json` |
| `monitor` | 健康/预算/兼容性监控（`--json`/`--watch`/`--notify`） | `agenthub monitor --notify` |
| `config-template` | 配置模板管理 | `agenthub config-template apply codex llm-default` |
| `config validate/repair` | 配置语义校验与默认值回退 | `agenthub config repair codex` |
| `config history/rollback` | 配置变更历史与回滚 | `agenthub config rollback codex 3` |
| `config secret` | 密钥存储（文件/OS keyring，轮换/迁移/后端切换） | `agenthub config secret backend --check` |
| `config user` | 用户管理（角色） | `agenthub config user create alice "Alice" --roles viewer` |
| `config perm` | 细粒度权限 | `agenthub config perm grant alice write --module config` |
| `prompt publish` | 发布提示词到社区目录 | `agenthub prompt publish review --publisher alice` |
| `prompt community` | 社区提示词管理 | `agenthub prompt community install review` |
| `prompt export/import` | 提示词导入导出（含版本） | `agenthub prompt export-all --output p.json` |
| `memory export/import` | 记忆导入导出 | `agenthub memory import memories.json --merge` |
| `session budget` | 成本预算/告警 | `agenthub session budget set --daily 5 --monthly 50` |
| `session fork` | 携带上下文创建新会话 | `agenthub session fork <id> --agent claude-code` |
| `session usage/trend/export-usage` | API 调用次数、成本趋势与 JSON 导出 | `agenthub session export-usage usage.json` |
| `skill list/install/uninstall/enable/disable` | 技能管理（`--scope project|user|global` 三级作用域） | `agenthub skill install rust --scope project` |
| `skill check-compat` | 技能版本兼容检查 | `agenthub skill check-compat *` |
| `skill market` | 技能市场（搜索/评分/安装统计） | `agenthub skill market search rust` |
| `plugin` | 插件注册/钩子执行 | `agenthub plugin run on_monitor` |
| `notify` | 告警推送通道（webhook/email（SMTP 直发或 .eml 落盘）/file，含分级/去重） | `agenthub notify add ops email ops@x.com --smtp-host smtp.x.com` |
| `prompt effects` | 提示词效果追踪（评分/成功率/成本） | `agenthub prompt effects` |
| `memory reindex` | 重建向量索引 | `agenthub memory reindex` |

### 通用选项

| 选项 | 说明 |
|------|------|
| `--dry-run` | 预览将执行的命令，不实际修改系统 |
| `--yes` | 跳过交互确认（适用于脚本） |
| `--type cli\|desktop` | 按类型筛选 |

---

## GUI 界面

桌面应用基于 Tauri 2 + Vue 3，提供以下功能：

| 视图 | 功能 |
|------|------|
| **代理列表** | 搜索、筛选（类型/提供商/状态）、排序、网格/表格视图切换 |
| **代理详情** | 安装方式、平台配置、验证日期、官网链接 |
| **批量操作** | 多选安装/卸载，逐项进度与结果报告 |
| **配置管理** | Agent 运行时配置、多环境支持 |
| **诊断视图** | 环境检查、包管理器可用性、清单完整性 |
| **会话管理** | 会话记录、搜索、成本追踪、回放、模板 |
| **记忆管理** | 跨会话知识持久化、BM25 语义检索、记忆衰减 |
| **提示词管理** | 模板版本控制、使用统计、变量校验 |
| **概览视图** | 仪表盘、审计日志、备份/恢复、预算/监控、趋势图（横切能力） |
| **提示词管理** | 模板 CRUD、变量插值、版本控制 |
| **技能管理** | 技能安装/卸载、依赖检查、工作流编排 |

---

## 自定义 URL 协议

AgentHub 计划使用 `agenthub://` 作为桌面应用的自定义 URL 协议，用于从外部应用唤起 AgentHub 并跳转到 Agent 详情、搜索、诊断和设置视图。

当前协议仍是设计文档，尚未在 Tauri 后端注册。详细格式、安全约束、Tauri 配置示例和平台验证步骤见 [docs/url-protocol.md](docs/url-protocol.md)。

---

## Agent Harness 数据

AgentHub 的 agent harness 负责统一管理 Agent 配置、原生配置入口、会话记录、Skills、Memory 和 Prompt 模板。

当前桌面端数据根目录使用系统配置目录下的 `agenthub`，例如 Windows 为 `%APPDATA%\agenthub`，macOS 为 `~/Library/Application Support/agenthub`，Linux 为 `~/.config/agenthub`。详细路径、文件格式、原生 Agent 配置映射和安全约定见 [docs/agent-harness.md](docs/agent-harness.md)。

---

## 核心模块

`agenthub-core` 提供以下 Rust 模块：

| 模块 | 职责 | 状态 |
|------|------|------|
| `agent` | Agent 数据模型、平台枚举、安装器配置 | ✅ |
| `catalog` | 从 `agents.json` 加载目录、搜索、过滤 | ✅ |
| `installer` | 安装/卸载命令生成与执行、超时处理 | ✅ |
| `status` | 已安装状态检测、版本解析（npm/pip/winget/brew） | ✅ |
| `diagnostic` | 环境健康检查、系统信息采集 | ✅ |
| `config` | Agent 运行时配置、多环境、密钥管理（文件/OS keyring）、校验/默认回退、变更历史/回滚 | ✅ |
| `prompt` | 提示词模板 CRUD、变量插值、版本控制 | ✅ |
| `session` | 会话记录、搜索、成本追踪、API 调用统计、趋势导出 | ✅ |
| `skill` | 技能清单解析、依赖检查、配置管理、三级作用域 | ✅ |
| `memory` | 记忆条目管理、作用域与类型分类 | ✅ |
| `overview` | 状态/趋势/审计只读聚合 + 交互式 Web 仪表盘 | ✅ |
| `notify` | 告警推送（webhook/email(SMTP 或 .eml 落盘)/file）、分级/去重 | ✅ |
| `error` | 统一错误类型定义 | ✅ |

---

## Agent 目录格式

每个 Agent 在 `agents.json` 中的结构：

```json
{
  "id": "cursor",
  "name": "Cursor",
  "kind": "desktop",
  "provider": "Cursor",
  "description": "AI-first code editor",
  "homepage": "https://cursor.com",
  "installers": {
    "windows": { "manager": "winget", "package": "Anysphere.Cursor" },
    "macos":   { "manager": "brew-cask", "package": "cursor" },
    "linux":   { "manager": "manual", "package": null }
  },
  "status": "verified",
  "catalog_verified_at": "2026-06-27",
  "installer_verified_at": "2026-06-27"
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 唯一标识符，仅允许小写字母、数字和连字符 |
| `name` | string | ✅ | 显示名称 |
| `kind` | enum | ✅ | `cli` 或 `desktop` |
| `provider` | string | ✅ | 提供商名称 |
| `description` | string | ✅ | 简短描述 |
| `homepage` | string | ✅ | 官网 URL |
| `installers` | object | ✅ | 按平台声明的安装配置 |
| `status` | enum | ✅ | `verified` / `community` / `manual` / `deprecated` |
| `catalog_verified_at` | date | — | 目录信息验证日期 |
| `installer_verified_at` | date | — | 安装流程验证日期 |

### 包管理器类型

| Manager | 说明 | 典型用途 |
|---------|------|----------|
| `npm` | Node.js 包管理器 | CLI Agent |
| `pip` | Python 包管理器 | Python Agent |
| `winget` | Windows 包管理器 | Windows 桌面 Agent |
| `brew-cask` | macOS Homebrew Cask | macOS 桌面 Agent |
| `manual` | 无可靠安装源 | 仅提供官网链接 |

---

## 开发指南

### 环境准备

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Node.js LTS
# 见 https://nodejs.org

# 克隆并进入项目
git clone https://github.com/your-org/agenthub.git
cd agenthub
```

### 开发模式

```bash
# 启动桌面应用（热重载）
cd agenthub-ui
npm install
npm run tauri dev

# 运行 CLI
cargo run -- list
```

### 代码风格

```bash
# Rust 格式化
cargo fmt --all

# Rust lint
cargo clippy --workspace --all-targets -- -D warnings

# 前端类型检查
cd agenthub-ui && npm run build
```

### 添加新 Agent

1. 在 [agents.json](agents.json) 中添加条目
2. 确保符合 [agents.schema.json](agents.schema.json) 的 Schema
3. 验证安装命令在目标平台可用
4. 更新 `catalog_verified_at` 和 `installer_verified_at`

---

## 测试

```bash
# 运行所有 Rust 测试
cargo test --workspace

# 运行特定模块测试
cargo test -p agenthub-core

# 运行带输出的测试
cargo test -- --nocapture

# 运行前端测试
cd agenthub-ui && npm test
```

### 测试层级

| 层级 | 内容 | 工具 |
|------|------|------|
| 单元测试 | 目录解析、筛选、命令生成、版本解析 | `cargo test` |
| 契约测试 | 安装器参数、输出解析、错误映射 | Mock command runner |
| 前端测试 | 搜索、筛选、选择、进度展示 | Vitest |
| 平台冒烟 | 构建、启动、dry-run | GitHub Actions |

---

## CI/CD

### CI 流水线（`.github/workflows/ci.yml`）

触发条件：push / PR 到 `main`

- Rust 测试 (`cargo test --workspace`)
- 格式检查 (`cargo fmt --check`)
- Clippy lint (`cargo clippy --workspace --all-targets -- -D warnings`)
- 前端构建 (`npm run build`)
- 前端测试 (`npm test`)

### 低资源 CI（`.github/workflows/ci-low-resource.yml`）

触发条件：默认 `workflow_dispatch` 手动触发；注册好自托管 runner 后，可自行将 `on`
改为 push / PR 自动触发（未注册 runner 时自动触发会导致 job 一直排队）。

面向 **RAM < 2 GB、存储 < 40 GB** 的低配 Linux 自托管 runner：

- 只测核心库与 CLI（`cargo test -p agenthub-core -p agenthub-cli`），跳过 Tauri 桌面构建
- 限制并行度（`CARGO_BUILD_JOBS=2`）、关闭 debug info 与增量编译
- 前端限制 Node 堆内存（`NODE_OPTIONS=--max-old-space-size=512`）
- rust-cache 仅在 main 分支写缓存，防止磁盘被缓存占满

使用前请将 `runs-on` 的 labels 改为你自己的 runner labels。完整策略与本地复现方法见
[`docs/low-resource-ci.md`](docs/low-resource-ci.md)。

### 发布流水线（`.github/workflows/release.yml`）

触发条件：推送 `v*` tag，或 `workflow_dispatch` 手动触发（输入 `version`，如 `1.4.0` → 自动创建 `v1.4.0` tag + draft release）

- `prepare` 任务先解析 tag 并**幂等预建 draft release**（`gh release create --draft`，已存在则复用）
- 自动构建 Windows / macOS (ARM + x86) / Linux 安装包，上传到预建的 draft release（`releaseId` 直传，避开 tauri-action 自建 release）
- 生成 GitHub Release draft，全部资产上传后手动发布（`gh release edit --draft=false`）
- 历史问题：tauri-action 在 push-tag 触发下自建 release 曾报 `403 Resource not accessible by integration`（仓库 Actions 默认权限为 read 所致，已改为 write；且预建 draft + `releaseId` 使发布走上传专用路径，不再依赖其 createRelease）
- 支持的目标平台：
  - `x86_64-pc-windows-msvc`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `x86_64-unknown-linux-gnu`

---

## 项目状态

### 已发布

- ✅ **v1.0.0 → v1.4.0** — 9 平台产物 + SHA-256 校验和（2026-08-06 至 2026-08-10）
- ✅ **M0–M4 全部完成** — 目录基线、核心重构、Beta 体验（M3 动效/取消重试/失败详情）、发布准备（CI/CD/安装包/签名策略）

### 已完成（安全与工程治理 P0/P1）

- ✅ **安全加固** — 全模块 ID/路径穿越防护、危险 ID 回归测试、备份恢复复用统一校验
- ✅ **Config 校验与历史** — 语义校验 + 默认值回退 + 变更快照/回滚（含密钥脱敏）
- ✅ **负向测试** — 损坏文件/导入恢复/并发写入（写锁 + 原子落盘）
- ✅ **覆盖率门禁** — `scripts/check-coverage.sh`：每核心文件 ≥80%（整体 87%），CI 独立 job

### 已完成（产品能力 P2 部分）

- ✅ **SMTP 直发** — email 告警通道可直接经 SMTP 发送（零依赖原生客户端）
- ✅ **Session 成本统计** — API 调用次数、按日趋势、JSON 导出
- ✅ **技能三级作用域** — 项目/用户/全局，同名项目优先
- ✅ **OS keyring 后端** — `AGENTHUB_SECRET_BACKEND=auto|file|keyring`，自动回退文件密钥链
- ✅ **交互式 Web 仪表盘** — `status --html`：窗口切换/图表/钻取，无服务器

### 计划中

- 📋 **在线技能市场 / 插件市场** — 需远端服务与信任模型设计
- 📋 **Prompt 社区远程推送** — 需远程同步通道协议
- 📋 **Beta 用户测试** — 3–5 名真实用户完成安装/查询/升级/卸载

### 长期愿景（六大业务模块 + Overview 概览 + 横切能力）

详见 [goal.md](goal.md)：

```
┌─────────────────────────────────────────────┐
│          Overview（概览，只读聚合）           │
├─────────┬──────────┬──────────┬─────────────┤
│ Package │  Config  │  Prompt  │   Session   │
│ 安装管理 │ 配置管理  │ 提示词管理│   会话管理   │
├─────────┴──────────┴──────────┴─────────────┤
│         Skill         │       Memory        │
│       技能管理         │      记忆管理        │
└───────────────────────┴─────────────────────┘

横切能力（非模块）：审计日志 · 备份/恢复 · 监控与告警
```

---

## 常见问题

### Agent 安装失败

运行 `agenthub doctor` 检查包管理器是否可用。确保对应的 npm/pip/winget/brew 已安装且在 PATH 中。

### 状态检测不准确

状态检测依赖包管理器的列表命令输出。如果包管理器版本过旧或输出格式变化，可能导致误判。运行 `cargo test -p agenthub-core` 验证解析逻辑。

### 某些 Agent 显示 `manual`

这表示该 Agent 没有可靠的包管理器来源，AgentHub 不会猜测安装命令，仅提供官网链接。

### 构建 Tauri 应用失败

确保已安装平台特定依赖：
- **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`
- **Windows**: WebView2 (Windows 10+ 自带)
- **macOS**: Xcode Command Line Tools

---

## 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/your-feature`)
3. 提交更改 (`git commit -m 'Add your feature'`)
4. 推送分支 (`git push origin feature/your-feature`)
5. 创建 Pull Request

### 提交规范

- 遵循 Rust 和 Vue 社区代码风格
- 新功能需包含测试
- 确保 `cargo test`、`cargo fmt`、`cargo clippy` 全部通过
- 更新相关文档

### 添加新 Agent

欢迎通过 PR 添加新 Agent！请确保：
- Agent 仍在活跃维护
- 安装命令经过验证
- 符合 `agents.schema.json` 格式

---

## 许可证

[MIT](LICENSE) · Copyright (c) 2026 Mark Chen
