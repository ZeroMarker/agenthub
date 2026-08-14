# AgentHub Goal — 六大业务模块 + Overview 概览 + 横切能力架构

> 版本：v0.5
> 创建日期：2026-06-24
> 更新日期：2026-08-07
> 状态：规划草案（Package 已实现；Config/Prompt/Session/Skill/Memory 具备基础实现并完成首波+第二波+第三波增强；Overview 概览与审计/备份/监控横切能力已落地；第四波完成用户权限、提示词社区共享、技能市场、插件系统与告警推送渠道；第五波完成提示词效果追踪、向量索引持久化、告警分级/去重与密钥轮换审计接入）
> 变更：v0.4 移除独立 management 模块，能力归并到所属模块；v0.5 保留 overview（概览）为独立只读模块，审计/备份/监控作为横切能力（非模块）

## 愿景

AgentHub 从"AI 编程助手的安装管理器"演进为**全生命周期的 Agent 管理平台**，覆盖安装、配置、运行、记忆和协作的完整链路。

---

## 模块总览

```
┌───────────────────────────────────────────────┐
│              overview（概览）                   │
│       只读聚合：状态 · 成本 · 审计 → 仪表盘      │
├─────────┬──────────┬──────────┬───────────────┤
│ package │  config  │  prompt  │    session     │
│ 安装管理 │ 配置管理  │ 提示词管理│    会话管理    │
├─────────┴──────────┴──────────┴───────────────┤
│           skill          │       memory        │
│         技能管理          │      记忆管理        │
└──────────────────────────┴────────────────────┘

横切能力（不构成独立模块，供所有模块复用）：
审计日志 · 备份/恢复 · 监控与告警
```

---

## 1. Package（安装管理）

**职责**：Agent 的发现、安装、卸载、版本管理和状态检测。

**当前状态**：✅ 已实现（v1.0）

**已实现能力**：
- ✅ 统一代理清单（agents.json）作为单一事实来源
- ✅ npm / pip / winget / brew-cask 四类安装器适配
- ✅ 平台级安装配置（Windows / macOS / Linux）
- ✅ 安装前预览（--dry-run）、交互确认、--yes 非交互模式
- ✅ 批量安装/卸载，逐项结果报告
- ✅ 状态检测和版本解析
- ✅ 超时处理和错误分类
- ✅ JSON schema 验证

**关键指标**：
- verified 安装器成功率 ≥ 95%
- 安装失败 100% 返回可诊断信息

---

## 2. Config（配置管理）

**职责**：管理 Agent 的运行时配置，包括模型选择、API Key、环境变量和服务端点。

**当前状态**：✅ 基础实现（配置模板/密钥链（文件+OS keyring）/轮换/校验/历史/用户权限，2026-08-14 完成剩余项）

**已实现能力**：
- ✅ 每个 Agent 独立 YAML 配置（`agents/<agent-id>.yaml`）
- ✅ `development` / `staging` / `production` 环境枚举
- ✅ settings、secrets、environment_variables、custom 分区
- ✅ 配置创建、读取、保存、删除、导入和导出
- ✅ 桌面端读取和保存部分 Agent 原生配置文件
- ✅ 配置模板（`ConfigTemplate` CRUD、从现有配置另存、应用到 Agent，secret 值永不落盘）
- ✅ 配置校验与默认值回退（已知设置项语义校验 + 缺省/越界安全默认 + `config validate|repair` + 旧配置缺字段宽容解析）
- ✅ 配置变更历史与回滚（每次变更前快照 `history/<agent>/v<N>.yaml`、版本单调递增、快照脱敏内联密钥；`config history|rollback`）

**目标能力**：
- 用户与权限（✅ 2026-08-07 第四波：`UserManager` 角色 + 按 module/agent 的细粒度权限）
- API Key 安全存储（OS keyring + 文件回退，✅ 2026-08-14：`AGENTHUB_SECRET_BACKEND=auto|file|keyring`，auto 探测不可用自动回退文件密钥链；值永不落盘）
- API Key 轮换 ✅（2026-08-06 第三波）
- 配置校验与默认值回退 ✅（2026-08-14）
- 配置变更历史与回滚 ✅（2026-08-14）

**设计原则**：
- 配置文件与代理清单分离
- 敏感信息不出现在日志或 UI 明文区域
- 支持 per-agent 和 global 两级配置

---

## 3. Prompt（提示词管理）

**职责**：创建、组织、版本化和共享提示词模板。

**当前状态**：✅ 基础实现（版本控制/使用统计/导入导出第二波完成）

**已实现能力**：
- ✅ 提示词模板 YAML 存储（`prompts/templates/<id>.yaml`）
- ✅ 模板创建、读取、更新、删除
- ✅ `{{variable}}` 变量插值渲染 + 必填变量校验（默认值回退）
- ✅ 标签和分类字段
- ✅ 桌面端列表、创建、渲染和删除入口
- ✅ 版本控制（每次修改快照历史版本、`rollback` 回滚）
- ✅ 使用统计（`usage_count` / `last_used_at`、排行）
- ✅ 导入/导出（JSON 含版本历史，force 覆盖）

**目标能力**：
- 从 Agent 会话中提取和保存提示词 ✅（2026-08-06 第三波）
- 提示词效果追踪（✅ 2026-08-07 第五波：`PromptEffects` 聚合会话评分/成功率/成本）
- 社区共享 ✅（2026-08-07 第四波：`prompts/community/` 快照 + 发布/安装）

**数据模型**：
```yaml
prompt:
  id: "code-review-v2"
  name: "代码审查"
  template: "Review the following {{language}} code..."
  variables: [language, context]
  tags: [review, quality]
  version: 2
  created_at: 2026-06-24
```

---

## 4. Session（会话管理）

**职责**：记录、检索和管理与 Agent 的交互会话。

**当前状态**：✅ 基础实现（成本预算/上下文传递/调用统计，2026-08-14 完成剩余项）

**已实现能力**：
- ✅ 会话 YAML 存储（`sessions/data/<session-id>.yaml`）
- ✅ 会话创建、列表、详情和删除
- ✅ 消息追加、标签、评分、备注
- ✅ 状态流转（active / paused / completed / failed）
- ✅ token 和成本字段
- ✅ 标题、Agent、消息内容和备注搜索
- ✅ 成本追踪（内置模型价格表 + `record_usage` 累积成本）
- ✅ 会话回放（Markdown 导出）与会话模板
- ✅ 成本预算告警（daily/monthly USD 上限 + `check_budget`）
- ✅ 跨 Agent 会话上下文传递（`fork_session` 携带消息/模型/标签）
- ✅ API 调用次数（`SessionUsage.calls` 每次 `record_usage` 递增）
- ✅ 成本趋势与导出（`session usage|trend|export-usage`：跨会话聚合、按日趋势、JSON 导出）

**目标能力**：
- 更完整的会话成本追踪（API 调用次数）✅（2026-08-14）
- 会话成本趋势与导出 ✅（2026-08-14）

**数据模型**：
```yaml
session:
  id: "ses_abc123"
  agent: "claude-code"
  model: "claude-sonnet-4-20250514"
  started_at: 2026-06-24T10:00:00Z
  messages: [...]
  tokens_used: 15000
  cost_usd: 0.045
  tags: [refactor, auth-module]
```

---

## 5. Skill（技能管理）

**职责**：定义、安装和管理可复用的技能包，扩展 Agent 的能力边界。

**当前状态**：✅ 基础实现（版本兼容检查/作用域，2026-08-14 完成）

**已实现能力**：
- ✅ `SKILL.md` frontmatter 清单解析
- ✅ 本地技能创建、安装、卸载、启用、禁用
- ✅ `.enabled` 文件表示启用状态
- ✅ 依赖命令检查
- ✅ 桌面端额外扫描 `~/.codex/skills`
- ✅ 版本兼容性检查（`min_agenthub_version` vs 运行版本，单个/批量）
- ✅ 项目级 / 用户级 / 全局级三级作用域（`SkillScope`：project > user > global 解析优先级，同名项目覆盖用户/全局；`skill list|install|uninstall --scope`，根目录可用 `AGENTHUB_PROJECT_SKILLS_DIR`/`AGENTHUB_GLOBAL_SKILLS_DIR` 注入）

**目标能力**：
- 技能市场（✅ 2026-08-07 第四波：本地注册表，搜索/评分/安装统计）
- 技能组合（workflow 编排多个技能）✅（2026-08-06 第三波）
- 插件系统（✅ 2026-08-07 第四波：`skills/plugins/` + 生命周期钩子）
- 项目级 vs 用户级 vs 全局级技能 ✅（2026-08-14）

**技能定义格式**：
```markdown
---
name: rust-dev
description: "Rust development workflow"
version: 1.0.0
author: agenthub
tags: [rust, cargo, testing]
triggers:
  - "*.rs"
  - "Cargo.toml"
---

# Rust Development Workflow
...
```

---

## 6. Memory（记忆管理）

**职责**：跨会话的知识持久化、检索和组织。

**当前状态**：✅ 基础实现（BM25/衰减/导入导出第二波完成）

**已实现能力**：
- ✅ Markdown + YAML frontmatter 存储
- ✅ global / project / session 三类作用域
- ✅ pinned / learning / decision / reference / feedback / free 类型
- ✅ 记忆创建、列表、删除、标签和搜索
- ✅ 桌面端列表、创建、搜索和删除入口
- ✅ BM25 语义检索（title×3 / tags×2 / content×1 加权）
- ✅ 记忆衰减（importance + 陈旧低重要度条目自动归档，搜索默认排除）
- ✅ 导入/导出（JSON，可按 scope 过滤，merge 语义）

**目标能力**：
- 向量检索 ✅（2026-08-06 第三波）
- 向量索引持久化（✅ 2026-08-07 第五波：`vector_index.json` 缓存 + 增量失效 + `memory reindex`）
- 知识图谱（实体关系可视化）✅（2026-08-06 第三波构建 + 2026-08-07 第五波 GUI 面板）

**记忆层次**：
```
global/     → 用户偏好、跨项目知识
projects/   → 架构决策、项目规则、持久发现
sessions/   → 检查点、任务进度、临时笔记
```

---

## 7. Overview（概览）

**职责**：只读聚合所有模块的状态、成本、审计与统计，提供统一的仪表盘视图。不拥有业务逻辑，不修改任何模块数据。

**当前状态**：✅ 首波 + 第二波 + 交互式仪表盘完成（2026-08-14）

**已实现能力**：
- ✅ 状态概览 `overview.rs`：`StatusOverview` 聚合目录/已安装/配置/提示词/会话/记忆/技能/审计
- ✅ CLI `agenthub status`；Tauri `get_status_overview`；GUI 概览视图（仪表盘卡片 + 可过滤审计表 + 备份/恢复入口）
- ✅ 时间维度趋势：`trend(days)` 按日分桶（会话/成本/记忆/审计），`status --trend`、`get_trend`、GUI 趋势柱状图
- ✅ 交互式 Web 仪表盘（2026-08-14）：`status --html` 自包含单页，7/30/90 天窗口切换 + SVG 成本/令牌图 + 可排序趋势表 + 卡片钻取（Agent/技能/分 Agent 会话用量/最近审计），数据内嵌 JSON 无服务器

**目标能力**：
- `agenthub dashboard`：打开 Web 仪表盘（浏览器独立视图）✅（`status --html` 已生成交互式仪表盘；浏览器打开生成文件即可）

---

## 8. 横切能力（不属于单一模块）

> 早期规划中的 "management" 模块已移除，能力归并到所属模块：统一入口/生命周期/批量操作/健康检查 → **package**，权限与密钥轮换 → **config**，成本阈值告警 → **session**，插件系统 → **skill**。剩余能力是跨模块复用的基础能力（工具而非模块，不承载业务逻辑）。

- **审计日志**：谁在什么时间对哪个 Agent 做了什么操作（`agenthub audit`，JSONL append-only）✅
- **备份/恢复**：导出/导入所有配置、记忆、技能、会话与审计（`agenthub backup` / `agenthub restore`）✅（第四波纳入 users/permissions/community/notify 通道）
- **监控与告警**（第一版 ✅）：`agenthub monitor` / `run_monitor` 聚合诊断结果、verified 未安装 Agent、预算告警与不兼容技能 → 健康状态；**告警推送渠道**（webhook/email-spool/file，✅ 2026-08-07 第四波）：`notify.yaml` 配置通道，`monitor --notify` 推送；**告警分级/去重**（✅ 2026-08-07 第五波）：`AlertSeverity` + 通道级 `min_severity` 过滤 + `dedup_minutes` 去重窗口；**SMTP 直发**（✅ 2026-08-14）：email 通道配置 `smtp`（host/port/username/password/tls）后经原生 RFC 5321 客户端直接发送，未配置时仍落盘 `.eml` 待 MTA 投递

---

## 模块依赖关系

```
package ──→ config ──→ session
   │           │          │
   ↓           ↓          ↓
  skill ──→ memory

overview（概览）──只读──→ 所有模块（聚合状态/成本/审计）
```

- **package** 是基础：先安装才能配置和使用
- **config** 依赖 package：配置对象是已安装的 Agent
- **session** 依赖 config：会话使用配置来连接 Agent
- **skill** 独立于 package：技能不依赖特定 Agent
- **memory** 被所有模块使用：每个模块都产生可检索的知识
- **overview** 只读依赖所有模块：聚合状态、成本、审计到仪表盘，不承载业务逻辑
- **横切能力**（审计、备份/恢复、监控）不构成独立模块，供所有模块复用

---

## 实施路线

| 阶段 | 模块 | 优先级 | 状态 | 预计时间 |
|------|------|--------|------|----------|
| Phase 1 | package（完善现有） | P0 | ✅ 已完成 | 2026-06-15 至 2026-06-27 |
| Phase 2 | config | P0 | ✅ 基础实现（模板/密钥链（文件+OS keyring）/轮换/校验/历史/用户权限） | 持续增强 |
| Phase 3 | memory | P1 | ✅ 基础实现 | 持续增强 |
| Phase 4 | session | P1 | ✅ 基础实现（成本/预算/调用统计/趋势导出） | 持续增强 |
| Phase 5 | prompt | P2 | ✅ 基础实现 | 持续增强 |
| Phase 6 | skill | P2 | ✅ 基础实现（+ 技能市场/插件系统 2026-08-07 第四波；+ 三级作用域 2026-08-14） | 持续增强 |
| Phase 7 | overview（概览，只读聚合） + 横切能力（审计、备份/恢复、监控、告警推送） | P3 | ✅ 首波+第二波+第四波+交互式仪表盘+SMTP 直发完成（2026-08-14） | 持续增强 |

---

## 与 PROJECT_PLAN.md 的关系

- `PROJECT_PLAN.md` 定义 v1.0 的交付范围（以 package 为核心）
- `goal.md` 定义更长远的模块化架构愿景
- v1.0 聚焦 package 模块的完善
- v2.0+ 重点增强 config、memory、session、prompt、skill，并补齐备份恢复、审计与跨 Agent 协作能力（横切能力，非独立模块）
