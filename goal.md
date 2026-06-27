# AgentHub Goal — 七大模块架构

> 版本：v0.2  
> 创建日期：2026-06-24  
> 更新日期：2026-06-27  
> 状态：规划草案（Package 模块已实现）

## 愿景

AgentHub 从"AI 编程助手的安装管理器"演进为**全生命周期的 Agent 管理平台**，覆盖安装、配置、运行、记忆和协作的完整链路。

---

## 模块总览

```
┌─────────────────────────────────────────────────┐
│                   management                     │
│          (统一入口、生命周期、权限、监控)          │
├─────────┬──────────┬──────────┬─────────────────┤
│ package │  config  │  prompt  │    session       │
│ 安装管理 │ 配置管理  │ 提示词管理│    会话管理      │
├─────────┴──────────┴──────────┴─────────────────┤
│           skill          │        memory         │
│         技能管理          │       记忆管理         │
└──────────────────────────┴──────────────────────┘
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

**当前状态**：❌ 未实现

**目标能力**：
- 每个 Agent 的配置模板（模型、温度、token 限制等）
- API Key 安全存储（系统密钥链，不明文存储）
- 配置导入/导出（JSON/YAML）
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

**当前状态**：❌ 未实现

**目标能力**：
- 提示词模板库（CRUD + 分类标签）
- 变量插值（`{{language}}`, `{{context}}`）
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

**当前状态**：❌ 未实现（MiMoCode 有内置 session 系统，AgentHub 自身无）

**目标能力**：
- 会话记录（输入、输出、时间戳、使用的 Agent/模型）
- 会话搜索（全文检索、按 Agent/日期/标签过滤）
- 会话回放与导出
- 会话成本追踪（token 用量、API 调用次数）
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

**当前状态**：🟡 基础存在（.mimocode/skills/elixir-dev）

**目标能力**：
- 技能定义格式（SKILL.md frontmatter + 内容）
- 技能安装/卸载/更新
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

**当前状态**：🟡 基础存在（localStorage 缓存 + MiMoCode 内置 memory）

**目标能力**：
- 项目级记忆（架构决策、规则、发现）
- 会话级记忆（当前上下文、工作进度）
- 全局记忆（用户偏好、跨项目知识）
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

## 7. Management（管理中心）

**职责**：统一入口，协调其他六大模块，提供生命周期管理和运维能力。

**当前状态**：❌ 未实现

**目标能力**：
- 仪表盘（Agent 状态总览、健康检查、资源用量）
- 生命周期管理（安装 → 配置 → 使用 → 更新 → 卸载）
- 批量操作（多 Agent 同时配置/更新/卸载）
- 监控与告警（Agent 可用性、API 状态、成本阈值）
- 用户与权限（单用户/多用户、角色、API Key 轮换）
- 审计日志（谁在什么时间对哪个 Agent 做了什么操作）
- 插件系统（第三方扩展入口）

**CLI 扩展**：
```bash
agenthub dashboard        # 打开 Web 仪表盘
agenthub status           # 全局状态概览
agenthub audit --last 7d  # 最近 7 天操作审计
agenthub backup           # 导出所有配置、记忆、技能
agenthub restore          # 从备份恢复
```

---

## 模块依赖关系

```
package ──→ config ──→ session
   │           │          │
   ↓           ↓          ↓
  skill ──→ memory ──→ management
```

- **package** 是基础：先安装才能配置和使用
- **config** 依赖 package：配置对象是已安装的 Agent
- **session** 依赖 config：会话使用配置来连接 Agent
- **skill** 独立于 package：技能不依赖特定 Agent
- **memory** 被所有模块使用：每个模块都产生可检索的知识
- **management** 是顶层：聚合所有模块提供统一视图

---

## 实施路线

| 阶段 | 模块 | 优先级 | 状态 | 预计时间 |
|------|------|--------|------|----------|
| Phase 1 | package（完善现有） | P0 | ✅ 已完成 | 2026-06-15 至 2026-06-27 |
| Phase 2 | config | P0 | 📋 设计中 | 2-3 周 |
| Phase 3 | memory | P1 | 🟡 基础存在 | 2-3 周 |
| Phase 4 | session | P1 | 📋 设计中 | 3-4 周 |
| Phase 5 | prompt | P2 | 📋 设计中 | 2-3 周 |
| Phase 6 | skill | P2 | 🟡 基础存在 | 2-3 周 |
| Phase 7 | management | P3 | 📋 设计中 | 4-6 周 |

---

## 与 PROJECT_PLAN.md 的关系

- `PROJECT_PLAN.md` 定义 v1.0 的交付范围（以 package 为核心）
- `goal.md` 定义更长远的模块化架构愿景
- v1.0 聚焦 package 模块的完善
- v2.0+ 逐步引入 config → memory → session → prompt → skill → management
