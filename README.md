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
├── goal.md                   # 七模块架构愿景
├── management.md             # 模块管理规范
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

---

## 支持的 Agent

> 完整目录见 [agents.json](agents.json)，Schema 定义见 [agents.schema.json](agents.schema.json)

### CLI Agent（7 个）

| Agent | 提供商 | 包名 | 包管理器 | 状态 |
|-------|--------|------|----------|------|
| Codex | OpenAI | `@openai/codex` | npm | verified |
| Claude Code | Anthropic | `@anthropic-ai/claude-code` | npm | verified |
| Kimi Code | Moonshot | `@moonshot-ai/kimi-code` | npm | community |
| Qwen Code | Alibaba | `@qwen-code/qwen-code` | npm | community |
| Reasonix CLI | Reasonix | `ESEngine.ReasonixCLI` | winget | verified |
| MiMo Code | Xiaomi | `@mimo-ai/cli` | npm | verified |
| Grok CLI | xAI | `xAI.GrokBuild` | winget | verified |

### Desktop Agent（18 个）

| Agent | 提供商 | Windows | macOS | 状态 |
|-------|--------|---------|-------|------|
| Cursor | Cursor | winget | brew | verified |
| Windsurf | Codeium | winget | brew | verified |
| Trae | ByteDance | winget | brew | verified |
| Trae Solo | ByteDance | winget | brew | verified |
| Codex Desktop | OpenAI | winget | brew | verified |
| Claude Desktop | Anthropic | winget | brew | verified |
| Kimi Desktop | Moonshot | winget | brew | community |
| WorkBuddy | Tencent | winget | brew | community |
| CodeBuddy | Tencent | winget | brew | community |
| Qoder | Alibaba | winget | brew | community |
| Qoder Work | Alibaba | winget | brew | community |
| MiniMax Agent | MiniMax | winget | brew | community |
| ZCode | ZhipuAI | winget | brew | community |
| Antigravity | Google | winget | brew | verified |
| Antigravity IDE | Google | winget | brew | verified |
| Reasonix | Reasonix | winget | brew | verified |
| OpenCode | OpenCode | winget | brew | verified |
| OpenWork | DifferentAI | winget | — | community |

### 支持状态说明

| 状态 | 含义 |
|------|------|
| `verified` | 官方验证过安装流程，信息可靠 |
| `community` | 社区贡献，未经官方验证 |
| `manual` | 无可靠包管理器来源，仅提供官网链接 |
| `deprecated` | 已废弃，不再维护 |

---

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
| **会话管理** | 会话记录、搜索、成本追踪 |
| **记忆管理** | 跨会话知识持久化、语义检索 |
| **提示词管理** | 模板 CRUD、变量插值、版本控制 |
| **技能管理** | 技能安装/卸载、依赖检查、工作流编排 |

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
| `config` | Agent 运行时配置、多环境、密钥管理 | ✅ |
| `prompt` | 提示词模板 CRUD、变量插值、版本控制 | ✅ |
| `session` | 会话记录、搜索、成本追踪 | ✅ |
| `skill` | 技能清单解析、依赖检查、配置管理 | ✅ |
| `memory` | 记忆条目管理、作用域与类型分类 | ✅ |
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
cargo clippy --workspace -- -D warnings

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
- Clippy lint (`cargo clippy -D warnings`)
- 前端构建 (`npm run build`)

### 发布流水线（`.github/workflows/release.yml`）

触发条件：推送 `v*` tag

- 自动构建 Windows / macOS (ARM + x86) / Linux 安装包
- 生成 GitHub Release draft
- 支持的目标平台：
  - `x86_64-pc-windows-msvc`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `x86_64-unknown-linux-gnu`

---

## 项目状态

### 已完成

- ✅ **M0：基线确认** — 25 个 Agent 目录，平台安装映射
- ✅ **M1：核心重构** — `agenthub-core` 共享库，统一数据源
- ✅ **M2：可靠性与安全** — 32+ 单元测试，状态检测，错误分类

### 进行中

- 🔄 **M3：Beta 体验** — GUI 异步任务、逐项进度、详情页

### 计划中

- 📋 **M4：发布准备** — CI/CD 完善、安装包、文档、v1.0.0

### 长期愿景（七模块架构）

详见 [goal.md](goal.md)：

```
┌─────────────────────────────────────────────┐
│              Management (管理中心)            │
├─────────┬──────────┬──────────┬─────────────┤
│ Package │  Config  │  Prompt  │   Session   │
│ 安装管理 │ 配置管理  │ 提示词管理│   会话管理   │
├─────────┴──────────┴──────────┴─────────────┤
│         Skill         │       Memory        │
│       技能管理         │      记忆管理        │
└───────────────────────┴─────────────────────┘
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
