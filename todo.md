# TODO

> 规划已修订（2026-08-06）：**移除独立的 management 模块**。原功能归并到所属模块（package/config/session/skill）；**保留 overview（概览）为独立只读模块**；审计日志、备份/恢复作为横切能力（非模块），详见 goal.md v0.5。

## 长期规划第二波（2026-08-06）— ✅ 完成（数据可移植性 + 可观测性）

### ✅ Config 配置模板
- [x] `ConfigTemplate`：settings/env 变量/secret key 保留名/custom 四类字段，模板 CRUD（list/get/create/delete/save）
- [x] `save_config_as_template`（secret 值永不落盘，仅保留 key 名）+ `apply_template`（合并到 agent 配置、版本号递增）
- [x] CLI `agenthub config-template list|show|create|delete|apply`；Tauri `list/create/apply/delete_config_template`

### ✅ Session 成本阈值告警 + 上下文传递
- [x] `BudgetConfig`（daily/monthly USD 上限，`sessions/budget.yaml`）+ `check_budget`（按会话开始日期聚合今日/本月花费，超限产生告警）
- [x] `SessionContext` 导出（`export_context` / JSON）+ `fork_session`（携带消息/模型/标签/项目到新会话，可换 agent）
- [x] CLI `session budget show|set` / `session fork`；Tauri `get/set_session_budget` / `fork_session`

### ✅ Prompt 导入/导出（含版本历史）
- [x] `PromptExportBundle`（当前模板 + 版本快照）JSON 导出；`import_prompts`（force 覆盖 / 默认跳过已有）
- [x] CLI `prompt export|export-all|import`；Tauri `export/import_prompts_json`

### ✅ Memory 导入/导出
- [x] `export_memories_json`（可按 scope 过滤）；`import_memories`（merge 跳过已有路径 / 覆盖）
- [x] CLI `memory export|import`；Tauri `export/import_memories_json`

### ✅ Skill 版本兼容性检查
- [x] `check_compatibility`（`min_agenthub_version` vs 运行版本，semver 三元组比较）+ `check_all_compatibility`
- [x] CLI `skill check-compat [name|*]`；Tauri `check_skill_compatibility`

### ✅ Overview 时间维度趋势
- [x] `TrendPoint`（按 UTC 日期分桶：会话开始/完成、tokens、成本、记忆创建、审计事件）+ `trend(days)`/`trend_with_now`
- [x] CLI `status --trend <days>`；Tauri `get_trend`；GUI 概览视图趋势柱状图（成本/会话）

### ✅ 横切能力：监控与告警（第一版）
- [x] `monitor.rs`：`MonitorReport` 聚合诊断结果、verified 未安装 Agent、预算告警、不兼容技能 → 健康状态
- [x] CLI `agenthub monitor`；Tauri `run_monitor`；GUI 概览视图监控面板

### 📊 验证结果
- Rust：173 测试全过（131 core + 9 集成 + 28 cli + 5 tauri），clippy 0 警告，fmt 干净
- 前端：11 测试全过，vue-tsc + vite build 通过

### 本波未覆盖（留待后续）
- Config：API Key 密钥链存储（需 keyring/系统依赖，待评估）、API Key 轮换
- Memory：向量检索、知识图谱
- Skill：技能市场（需网络）、工作流编排、插件系统
- Overview：Web 仪表盘（浏览器独立视图）
- 横切：监控定时化/告警推送（当前为手动 CLI/UI 触发）

## 长期规划第一波（2026-08-06）— ✅ 完成

### ✅ Overview 概览模块（此前完全未实现）
- [x] **状态概览** `overview.rs`：`StatusOverview` 聚合目录/已安装/配置/提示词/会话/记忆/技能/审计；`overview_with_status` 便于测试
- [x] **CLI**：`agenthub status`
- [x] **Tauri + UI**：`get_status_overview` 命令；新增概览视图（仪表盘卡片 + 可过滤审计表 + 备份/恢复入口）

### ✅ 横切能力：审计日志 / 备份恢复（此前完全未实现）
- [x] **审计日志** `audit.rs`：append-only JSONL 日志、`AuditQuery` 过滤（action/target/actor/since/until/limit）、action_counts、clear、import_events
- [x] **备份/恢复** `backup.rs`：全工作区快照（configs、prompts+版本历史、sessions+模板、memories、audit）为单个 JSON；restore 校验 format_version 并回写全部数据
- [x] **CLI**：`agenthub audit` / `backup` / `restore`；Tauri `list_audit`/`clear_audit`/`create_backup`/`restore_backup`；install/uninstall 自动记录审计事件

### ✅ Session 成本追踪 + 回放 + 模板
- [x] `PricingTable` 内置 17 个常见模型价格（USD/1M tokens）+ 未知模型回退价
- [x] `record_usage`（按 input/output tokens 累积成本）、`set_model`、`add_message_with_tokens`
- [x] `replay_session`：Markdown 会话回放导出
- [x] `SessionTemplate`：模板 CRUD + `create_session_from_template` 预置消息创建会话

### ✅ Prompt 版本控制 + 使用统计
- [x] `update_prompt` 每次修改前自动快照历史版本，`list_versions`/`get_version`/`rollback`（回滚前也快照，版本号单调递增）
- [x] `render_prompt` 自动记录 `usage_count`/`last_used_at`，`list_usage` 排行
- [x] `render_prompt_checked`：必填变量校验 + 默认值回退

### ✅ Memory BM25 语义检索 + 记忆衰减
- [x] 纯 Rust BM25 检索（title×3 / tags×2 / content×1 加权），`search_entries_bm25(query, top_k)`
- [x] 记忆衰减：`importance`（0-10）、`last_accessed_at`、`touch`/`revive`/`set_importance`，`apply_decay(older_than_days)` 自动归档低重要性陈旧条目，已衰减条目默认排除出搜索，`MemoryStats.decayed` 计数

### 📊 验证结果
- Rust：147 测试全过（113 core + 9 集成 + 20 cli + 5 tauri），clippy 0 警告，fmt 干净
- 前端：11 测试全过，vue-tsc + vite build 通过

### 本波未覆盖（留待后续，已按所属模块归类）
- Config：API Key 系统密钥链存储（需引入 keyring/系统依赖，待评估）、配置模板
- Skill：技能市场（需网络）、工作流编排、版本兼容性检查、插件系统（归并自原 management）
- Memory：向量检索、知识图谱
- Session：成本阈值告警（成本监控延伸）
- Overview：Web 仪表盘、时间维度趋势
- 横切：监控与告警（Agent 可用性、API 状态、成本阈值）

## 当前优化波次（2026-08-06）— ✅ 全部完成

### ✅ 交付与推送
- [x] 全部优化工作已提交 `23f2215`（feat: cancellation/retry/failure-details, vitest test infra, release automation）并推送 `origin/main`
- [x] 工作区干净，无残留变更
- [x] 早期三路后台委派（B/D/E）已停用：未落盘冲突改动，其结果被直接执行所覆盖，不产生二次合并

### ✅ 基线已建立
- [x] M3 全部工作已提交为 checkpoint（`ba1c73b`）并推送 `origin/main`
- [x] 仓库卫生：`.mimocode`/`.reasonix`/`.commandcode` 工具本地状态移出追踪并加入 `.gitignore`

### ✅ 本次优化内容
- [x] **前端测试设施**（B）：Vitest 4.1 + Vue Test Utils + happy-dom 配置完成，AgentList 11 项组件测试（搜索/筛选/批量选择/排序/状态/取消/重试/失败详情）全绿，`vue-tsc` 构建无回归
- [x] **Rust 代码卫生**（D）：clippy 0 警告；精确扫描确认生产代码 0 处 unwrap/expect（按文件粗算的 90 处几乎全部在测试模块内），仅 2 处启动期 expect 合理保留
- [x] **发布准备**（E）：`scripts/generate-checksums.ps1/.sh`（实测通过）、`docs/signing-policy.md`、`scripts/generate-support-matrix.py`（修正 README 与 agents.json 的 13 处漂移）、release.yml 校验和附加到 GitHub Release
- [x] **M3 体验补全**（C）：取消机制（`cancel_operation` 命令 + `operation-cancelled` 事件 + 进程树 kill）、卡片/表格/批量进度面板的取消/重试按钮、可展开失败详情（命令/退出码/stderr/stdout）
- [x] **可访问性**：全局 `:focus-visible` 焦点环、`prefers-reduced-motion` 支持（ModalDialog 原有 Escape/焦点陷阱/焦点恢复保留）
- [x] 修复 `run_command` 超时路径 bug：超时/取消时真正 kill 进程树（原实现注释承认无法 kill，子进程会残留）

### 📊 验证结果
- Rust：110 测试全过（81 core + 9 集成 + 15 tauri + 5 cli），clippy 0 警告，fmt 干净
- 前端：11 测试全过，vue-tsc + vite build 通过
- 变更：19 文件改动 + 4 新增（scripts/ ×3、signing-policy.md、vitest.config.ts、__tests__/）

## M3: Beta 体验

### ✅ Material You (Material 3) Expressive Design Refactor — Complete

All UI components have been migrated to the Material 3 design token system:

**Foundation (`style.css`):**
- Full M3 Light/Dark color palette (`--md-sys-color-primary`, `--md-sys-color-surface`, etc.)
- Elevation tokens 0-5 with M3 box-shadows
- Shape tokens (xs/sm/md/lg/xl/full)
- M3 Typography scale (display/headline/title/body/label)
- Motion tokens (emphasized easing + duration)
- Utility classes: `.m3-btn-filled`, `.m3-btn-tonal`, `.m3-btn-outlined`, `.m3-chip`, `.m3-surface`

**Navigation Rail (`App.vue`):**
- Fixed left rail, 80px collapsed, 240px on hover
- M3 surface bg + outline-variant border
- Active: `secondary-container` bg + `on-secondary-container` text
- Responsive collapse to 60px / 48px

**Shared Components:**
- `PageHeader` — headline-medium typography, accent bar (4px primary pill)
- `NotificationBar` — secondary-container surface for success, error-container for errors
- `LoadingSpinner` — primary spinner on surface-variant track
- `EmptyState` — muted icon + on-surface-variant text
- `ModalDialog` — elevation-3, shape-xl, smooth M3 scaling
- `StatusBadge` — tonal pill (positive/caution/negative/neutral)

**AgentList & Agent Sub-Components:**
- M3 segmented tabs (surface-variant bg, active tab lifts via elevation-1)
- Stat chips: secondary-container tonal pills
- Agent cards: elevated M3 cards (elevation-1 → elevation-3 on hover), shape-md
- Agent table: M3 data table with surface-variant sticky header
- Toolbar: M3 chips for sort, M3 tonal/outlined buttons
- Batch bar: tonal select-all with tonal/outlined action buttons
- Detail modal: M3 list items, tonal badges, outlined footer buttons
- Progress indicators: primary color fill + surface-variant track

**Manager Views:**
- Consistent M3 surface + elevation-1 panel layout
- M3 tonal/outlined button styling throughout
- All hardcoded colors replaced with CSS custom properties

## M4: 发布准备

### CI/CD
- [x] GitHub Actions: Windows/macOS/Linux 构建矩阵（`release.yml` ✅）
- [x] GitHub Actions: cargo test + clippy + fmt 检查（`ci.yml` ✅）
- [x] GitHub Actions: 前端 npm run build 检查（`ci.yml` ✅）

### 发布产物
- [x] CLI 二进制 + SHA-256 校验和（`scripts/generate-checksums.ps1/.sh`，release.yml 已集成并附加到 Release）
- [x] 桌面安装包（Tauri build 配置完成，release workflow 已就绪）
- [x] 代码签名策略文档（`docs/signing-policy.md`）

### 文档
- [x] CONTRIBUTING.md
- [x] 安全策略（SECURITY.md）
- [x] 问题模板（issue templates）
- [x] 更新 README 支持矩阵自动化生成（`scripts/generate-support-matrix.py`）

### 发布
- [x] 发布 v1.0.0（2026-08-06，GitHub Release + 9 平台产物 + SHA-256 校验和，https://github.com/ZeroMarker/agenthub/releases/tag/v1.0.0）
- [x] 安装/升级/卸载回归测试（CLI: npm install→upgrade 0.145.0→0.146.1→uninstall ✅；.deb: dpkg install→reinstall→remove ✅；AppImage: 启动冒烟 ✅；校验和 sha256sum -c 全部通过 ✅）

## 长期规划（goal.md：六大业务模块 + Overview 概览 + 横切能力）

> Management 已从规划中移除：统一入口/生命周期/批量操作/健康检查归 package，权限与密钥归 config，成本监控归 session，插件归 skill；overview（概览）保留为独立只读模块；审计/备份/监控为横切能力（非模块）。

### Package 安装管理
- [x] 生命周期管理（安装→配置→使用→更新→卸载）✅（v1.0 已有）
- [x] 批量操作（多 Agent 同时配置/更新/卸载）✅（v1.0 已有）
- [x] 健康检查（`agenthub doctor`）✅（v1.0 已有）

### Config 配置管理
- [x] 多环境配置（development / staging / production）✅（基础实现）
- [x] 配置模板（模型、温度、token 限制）✅（2026-08-06 第二波）
- [ ] API Key 密钥链存储（需评估 keyring/系统依赖）
- [ ] API Key 轮换、用户与权限（归并自原 management）

### Session 会话管理
- [x] 成本追踪（模型价格表 + record_usage）✅（2026-08-06 首波）
- [x] 会话回放、会话模板 ✅（2026-08-06 首波）
- [x] 成本阈值告警（daily/monthly 预算）✅（2026-08-06 第二波）
- [x] 跨 Agent 会话上下文传递（fork）✅（2026-08-06 第二波）

### Prompt 提示词管理
- [x] 版本控制、变量校验、使用统计 ✅（2026-08-06 首波）
- [x] 导入/导出（含版本历史）✅（2026-08-06 第二波）
- [ ] 社区共享、从 Agent 会话中提取提示词

### Skill 技能管理
- [x] 版本管理与兼容性检查 ✅（2026-08-06 第二波）
- [ ] 技能市场（发现、评分、安装统计）
- [ ] 工作流编排（多技能组合）
- [ ] 插件系统（第三方扩展入口，归并自原 management）

### Memory 记忆管理
- [x] BM25 语义检索 ✅（2026-08-06 首波）
- [x] 记忆衰减（importance + 归档）✅（2026-08-06 首波）
- [x] 记忆导入/导出/同步 ✅（2026-08-06 第二波）
- [ ] 向量检索、知识图谱

### Overview 概览模块（只读聚合，不承载业务逻辑）
- [x] 状态概览 `agenthub status` / GUI 仪表盘 ✅（2026-08-06 首波）
- [x] 时间维度趋势（成本 / 会话数 / 审计量）✅（2026-08-06 第二波）
- [ ] Web 仪表盘（浏览器独立视图）

### 横切能力（非模块，工具而非业务模块）
- [x] 审计日志 ✅（2026-08-06 首波，install/uninstall 已接入）
- [x] 备份/恢复 ✅（2026-08-06 首波）
- [x] 监控与告警（第一版：诊断/未安装/预算/兼容性）✅（2026-08-06 第二波）
- [ ] 监控定时化/告警推送
