# AgentHub Goal — 六大业务模块 + Overview 概览 + 横切能力架构

> 版本：v0.5
> 创建日期：2026-06-24
> 更新日期：2026-08-06
> 状态：规划草案（Package 已实现，Config / Prompt / Session / Skill / Memory 已具备基础实现；Overview 概览与审计/备份横切能力首波完成）
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

**当前状态**：✅ 基础实现

**已实现能力**：
- ✅ 每个 Agent 独立 YAML 配置（`agents/<agent-id>.yaml`）
- ✅ `development` / `staging` / `production` 环境枚举
- ✅ settings、secrets、environment_variables、custom 分区
- ✅ 配置创建、读取、保存、删除、导入和导出
- ✅ 桌面端读取和保存部分 Agent 原生配置文件

**目标能力**：
- 每个 Agent 的配置模板（模型、温度、token 限制等）
- API Key 安全存储（系统密钥链，不明文存储）
- 配置校验与默认值回退
- 多环境支持（dev / staging / prod）
- 配置变更历史与回滚

**设计原则**：
- 配置文件与代理清单分离
- 敏感信息不出现在日志或 UI 明文区域
- 支持 per-agent 和 global 两级配置

---

## 3. Prompt（提示词管理）

**职责**：创建、组织、版本化和共享提示词模板。

**当前状态**：✅ 基础实现

**已实现能力**：
- ✅ 提示词模板 YAML 存储（`prompts/templates/<id>.yaml`）
- ✅ 模板创建、读取、更新、删除
- ✅ `{{variable}}` 变量插值渲染
- ✅ 标签和分类字段
- ✅ 桌面端列表、创建、渲染和删除入口

**目标能力**：
- 版本控制（每次修改生成新版本）
- 从 Agent 会话中提取和保存提示词
- 提示词效果追踪（关联会话结果）
- 导入/导出、社区共享

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

**当前状态**：✅ 基础实现

**已实现能力**：
- ✅ 会话 YAML 存储（`sessions/data/<session-id>.yaml`）
- ✅ 会话创建、列表、详情和删除
- ✅ 消息追加、标签、评分、备注
- ✅ 状态流转（active / paused / completed / failed）
- ✅ token 和成本字段
- ✅ 标题、Agent、消息内容和备注搜索

**目标能力**：
- 会话回放与导出
- 更完整的会话成本追踪（API 调用次数、模型价格表）
- 跨 Agent 会话上下文传递
- 会话模板（常见工作流的预设会话结构）

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

**当前状态**：✅ 基础实现

**已实现能力**：
- ✅ `SKILL.md` frontmatter 清单解析
- ✅ 本地技能创建、安装、卸载、启用、禁用
- ✅ `.enabled` 文件表示启用状态
- ✅ 依赖命令检查
- ✅ 桌面端额外扫描 `~/.codex/skills`

**目标能力**：
- 技能市场（发现、评分、安装统计）
- 技能组合（workflow 编排多个技能）
- 技能版本管理与兼容性检查
- 项目级 vs 用户级 vs 全局级技能

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

**当前状态**：✅ 基础实现

**已实现能力**：
- ✅ Markdown + YAML frontmatter 存储
- ✅ global / project / session 三类作用域
- ✅ pinned / learning / decision / reference / feedback / free 类型
- ✅ 记忆创建、列表、删除、标签和搜索
- ✅ 桌面端列表、创建、搜索和删除入口

**目标能力**：
- 语义检索（BM25 + 向量搜索）
- 记忆衰减（旧记忆自动降权或归档）
- 记忆导入/导出/同步
- 知识图谱（实体关系可视化）

**记忆层次**：
```
global/     → 用户偏好、跨项目知识
projects/   → 架构决策、项目规则、持久发现
sessions/   → 检查点、任务进度、临时笔记
```

---

## 7. Overview（概览）

**职责**：只读聚合所有模块的状态、成本、审计与统计，提供统一的仪表盘视图。不拥有业务逻辑，不修改任何模块数据。

**当前状态**：✅ 首波完成（2026-08-06）

**已实现能力**：
- ✅ 状态概览 `overview.rs`：`StatusOverview` 聚合目录/已安装/配置/提示词/会话/记忆/技能/审计
- ✅ CLI `agenthub status`；Tauri `get_status_overview`；GUI 概览视图（仪表盘卡片 + 可过滤审计表 + 备份/恢复入口）

**目标能力**：
- `agenthub dashboard`：打开 Web 仪表盘（浏览器独立视图）
- 时间维度趋势（成本 / 会话数 / 审计量随时间变化）

---

## 8. 横切能力（不属于单一模块）

> 早期规划中的 "management" 模块已移除，能力归并到所属模块：统一入口/生命周期/批量操作/健康检查 → **package**，权限与密钥轮换 → **config**，成本阈值告警 → **session**，插件系统 → **skill**。剩余能力是跨模块复用的基础能力（工具而非模块，不承载业务逻辑）。

- **审计日志**：谁在什么时间对哪个 Agent 做了什么操作（`agenthub audit`，JSONL append-only）
- **备份/恢复**：导出/导入所有配置、记忆、技能、会话与审计（`agenthub backup` / `agenthub restore`）
- **监控与告警**：Agent 可用性、API 状态、成本阈值（只读监控，规划中）

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
| Phase 2 | config | P0 | ✅ 基础实现 | 持续增强 |
| Phase 3 | memory | P1 | ✅ 基础实现 | 持续增强 |
| Phase 4 | session | P1 | ✅ 基础实现 | 持续增强 |
| Phase 5 | prompt | P2 | ✅ 基础实现 | 持续增强 |
| Phase 6 | skill | P2 | ✅ 基础实现 | 持续增强 |
| Phase 7 | overview（概览，只读聚合） + 横切能力（审计、备份/恢复） | P3 | ✅ 首波完成（2026-08-06） | 持续增强 |

---

## 与 PROJECT_PLAN.md 的关系

- `PROJECT_PLAN.md` 定义 v1.0 的交付范围（以 package 为核心）
- `goal.md` 定义更长远的模块化架构愿景
- v1.0 聚焦 package 模块的完善
- v2.0+ 重点增强 config、memory、session、prompt、skill，并补齐备份恢复、审计与跨 Agent 协作能力（横切能力，非独立模块）
