# TODO

> 规划已修订（2026-08-06）：**移除独立的 management 模块**。原功能归并到所属模块（package/config/session/skill）；**保留 overview（概览）为独立只读模块**；审计日志、备份/恢复作为横切能力（非模块），详见 goal.md v0.5。

## 当前待办（2026-08-13）

> 以本节作为当前执行清单；下方各波次和发布记录仅作历史存档。

### P0：安全与正确性

- [x] 配置、配置模板、提示词和工作流持久化入口拒绝路径穿越与非法 ID（`10c59ef`）
- [x] 将安全 ID 校验扩展到 Session、Community、Marketplace、Skill、Plugin 和 Memory 等其余文件持久化模块
- [x] 为各持久化模块增加危险 ID / 相对路径回归测试（`../`、路径分隔符、控制字符、过长 ID）
- [x] 加固备份恢复和导入流程，所有按 ID/路径恢复的模块复用同一安全校验
- [x] Config：配置校验与默认值回退（已知设置项语义校验 + 缺省/越界值安全默认回退 + `config validate|repair` + 旧配置缺字段宽容解析）
- [x] Config：配置变更历史与回滚（每次变更前快照 `history/<agent>/v<N>.yaml`，版本单调递增，快照脱敏内联密钥；`config history|rollback`）

### P1：CI 与工程治理

- [x] CI 前端任务加入 `npm test`，避免仅构建不运行已有 Vitest 测试
- [x] CI Clippy 扩展为 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 升级 `actions/checkout` 与 `actions/setup-node` 到 v5，消除 Node.js 20 action runtime 弃用警告
- [ ] 建立覆盖率采集与门禁，落实核心模块不低于 80% 的验收目标
- [x] 补充导入/恢复、损坏文件、并发写入等负向测试（19 个新测试：全模块损坏文件优雅报错/降级、导入恢复拒绝恶意与损坏数据、并发写入锁防丢失更新）
- [x] 建立 ADR，记录清单格式、安装器接口和平台策略
- [x] 将 `PROJECT_PLAN.md` 标记为 v1.0 历史计划，并清理已完成但仍未勾选的发布任务

### P2：产品能力增强

- [ ] Config：增加 OS keyring 后端，保留文件密钥存储作为兼容回退
- [ ] Skill：在线技能市场与插件市场
- [ ] Skill：项目级 / 用户级 / 全局级技能作用域
- [ ] Notify：SMTP 直接发送（当前 email 通道仅生成 `.eml`）
- [ ] Session：API 调用次数、成本趋势与导出
- [ ] Prompt：社区远程推送与同步渠道
- [ ] Overview：交互式 Web 仪表盘与指标钻取（当前为静态 HTML 导出）
- [ ] Beta：邀请 3–5 名真实用户完成安装、查询、升级和卸载任务测试

### 最近验证

- [x] 2026-08-14 本机验证：cargo test --workspace 全绿（290 测试：229 core + 47 cli + 9 集成 + 5 tauri），fmt 干净，all-targets clippy 0 警告；tauri cargo check 通过；并发测试已在无锁状态下验证可稳定捕获丢失更新（8 线程×25 写 Barrier）

## 发布记录（2026-08-10）— ✅ v1.4.0 已发布

- [x] v1.4.0（图标重设计 + 发版流程修复）：9 平台产物 + 4 SHA-256 校验和，https://github.com/ZeroMarker/agenthub/releases/tag/v1.4.0
- [x] 新发版流程首次实战验证：push tag → prepare 幂等预建 draft → releaseId 直传 → 4 平台构建上传 → 校验和附加 → `gh release edit --draft=false`，全程零手动干预
- [x] 新流程暴露 2 个 bug 已修复（同轮内）：① prepare 无 checkout，gh 无法推断仓库 → 显式 `-R github.repository`；② `gh release view --json id` 返回 GraphQL node_id（`RE_...`）导致 tauri-action `Number()` → NaN 跳过全部上传 → 改用 `--json databaseId`（数字 REST id）
- [x] 校验和验证：SHA256SUMS 4 文件 9 条目全部 sha256sum -c PASS
- [x] 版本同步 1.3.0→1.4.0：Cargo.toml/Cargo.lock（含 src-tauri `agenthub` 包 1.2.0 漂移一并修正）/package.json/package-lock.json/tauri.conf.json

## 发布记录（2026-08-07）— ✅ v1.3.0 已发布

- [x] v1.3.0（wave 4 + wave 5 + M3 token 全量迁移）：9 平台产物 + 4 SHA-256 校验和，https://github.com/ZeroMarker/agenthub/releases/tag/v1.3.0
- [x] 发版前审查：修复 CHANGELOG Unreleased 结构损坏（重复 `### Changed` 标题 + wave 4 内容错分类）；版本 1.2.0→1.3.0 同步 Cargo.toml/Cargo.lock（3 包）/package.json/package-lock.json/tauri.conf.json
- [x] 验证全走 GitHub Actions（本机构建测试不再执行）：CI（cargo test/fmt/clippy -D warnings + 前端 build）绿；release workflow 4 平台构建成功
- [x] 流程：预建 draft release → push tag v1.3.0 → tauri-action 上传至已存在 draft（绕过 push-tag 403）→ 抽查 Linux 3 产物 sha256sum -c 全部 OK → `gh release edit --draft=false` 正式发布
- [x] macOS `.app.tar.gz` 校验和与上传资产名对齐（v1.1.0/v1.2.0 老 bug 修复验证）：aarch64/x64 各 2 资产文件名与 SHA256SUMS 完全一致

## 发布记录（2026-08-06）— ✅ v1.1.0 / v1.2.0 已发布

- [x] v1.1.0（wave 3）：9 平台产物 + 4 SHA-256 校验和，https://github.com/ZeroMarker/agenthub/releases/tag/v1.1.0
- [x] v1.2.0（M3 Expressive + tauri-action v1）：9 平台产物 + 4 校验和，https://github.com/ZeroMarker/agenthub/releases/tag/v1.2.0
- [x] v1.2.0 实测 tauri-action@v1 正常（预建 draft 流程）；发现并修复 macOS `.app.tar.gz` 校验和与上传资产名不一致的老 bug（v1.1.0 已受影响，已一并修正线上资产）
- [x] 发版中 tauri-action 创建 release 403 排查：probe 证实 GITHUB_TOKEN 有写权限（POST 201）；workflow_dispatch 下 tauri-action 正常；最终以「预创建 draft release → push tag」绕过，4/4 平台上传成功
- [x] tauri-action 升级 `@v0` → `@v1`（v1.0.0，2026-06-29），inputs 全兼容
- [x] UI：Material 3 Expressive 增强（spring 动效、expressive 形状、导航激活指示条）
- [x] 遗留已修复（2026-08-10）：push-tag 场景 tauri-action 403 根因确认——仓库 Actions 默认权限 `default_workflow_permissions: read`（`Resource not accessible by integration`）；已改为 `write` 并加固 release.yml：`prepare` 任务幂等预建 draft release + `releaseId` 直传（tauri-action 只走上传路径）+ `workflow_dispatch` 支持 `version` 输入手动发版

## UI 全面优化（2026-08-06）— ✅ 完成（M3 token 全量迁移 + 缺陷修复）

- [x] 审查发现：6 个 Manager 视图（Config/Skill/Prompt/Session/Memory/Diagnostic）从未迁移到 M3 设计系统——约 150 处硬编码旧色（蓝/绿/红/白底卡片），暗色模式完全破坏、视图间视觉割裂
- [x] 全部硬编码色/阴影/圆角 → M3 token（`color: white` 上下文感知映射为 on-*，按钮暗色下保持对比度）
- [x] 修复 SessionManager：completed 徽标 `#1565c0` → on-primary-container、paused → tertiary-container 系、删除按钮 on-primary → on-error
- [x] 修复 ConfigManager 取消按钮（灰底 on-primary → on-surface）、DiagnosticView 警告/通过/失败状态色 → container 语义色
- [x] 修复 AgentList：页面渐变背景 → background token；切 Tab 后 `debouncedSearchQuery` 搜索过滤残留 bug
- [x] 补全引用但缺失的 `.m3-tabs`/`.m3-tab`（M3 primary tab 底部指示条）与 `.agent-stats`/`.stat-chip`（tonal pill）样式
- [x] 验证：vue-tsc 0 错误、vite build 通过、vitest 11/11 全过
- [x] 提交 `f7df6d9` 已推送 origin/main；未发版（v1.3.0 候选）

## 第五波（2026-08-07）— ✅ 完成（效果追踪 + 索引持久化 + 告警分级/去重 + 审计接入）

### ✅ Prompt 效果追踪（关联会话结果）
- [x] `PromptOutcome`（session_id/rating/success/tokens/cost）+ `PromptEffects`（uses/avg_rating/success_rate/total_cost/last_used），存储 `prompts/effects/<id>.yaml`
- [x] `record_outcome` / `record_outcome_from_session`（从会话自动提取评分/tokens/成本）/ `get_effects` / `list_effects` / `clear_effects`
- [x] CLI `prompt effects [id]` / `prompt record-outcome <id> --session <sid>` / `prompt clear-effects <id>`；Tauri `get/list_prompt_effects`、`record_prompt_outcome`、`clear_prompt_effects`
- [x] GUI Prompt 管理器新增 Effects 页签（记录会话结果 + 效果排行表）

### ✅ Memory 向量索引持久化
- [x] `VectorIndex`（`memory/vector_index.json`）：按 path 缓存加权嵌入（title×3/tags×2/content×1），`indexed_at` 与 `entry.updated_at` 比较判定失效；搜索时增量重算并回写
- [x] `build_vector_index`（全量重建，跳过 decayed）+ `delete_entry` 同步清除缓存
- [x] CLI `memory reindex`；Tauri `build_vector_index`；GUI 新增知识图谱面板（实体列表 + 邻居关系，交互式）

### ✅ 横切：告警分级 + 去重
- [x] `AlertSeverity`（info/warning/critical）：`MonitorReport::severity()` 派生（诊断失败/预算超限/不兼容技能 → critical）
- [x] 通道级 `min_severity` 过滤 + `dedup_minutes` 去重窗口（`notify_state.json` 持久化），`--force` 绕过
- [x] `send_custom`（非监控告警，如密钥轮换通知）；CLI `notify add --min-severity --dedup-minutes`、`notify send --force`、`notify clear-state`；`monitor --notify-force`

### ✅ Config：API Key 轮换通知/审计接入
- [x] `config secret set|rotate|migrate|delete` 自动记录审计事件（`config.secret.*`）；`config rotate --notify` 通过通道推送轮换告警

### 📊 验证结果
- Rust：246 测试全过（188 core + 9 集成 + 44 cli + 5 tauri），clippy 0 警告，fmt 干净
- 前端：11 测试全过，vue-tsc 0 错误，vite build 通过
- 变更：prompt 效果追踪 + memory 向量索引 + notify 分级去重 + 审计接入 + GUI（Prompt Effects 页签、Memory 图谱面板）

### 本波未覆盖（留待后续）
- Config：OS keyring 后端、基于权限的 CLI 鉴权强制执行
- Prompt：社区推送渠道（当前本地目录同步）
- Skill：在线技能市场（需网络）、插件市场
- 横切：SMTP 直发（当前 email 通道落盘 .eml 待 MTA 投递）
- Overview：仪表盘交互化（当前静态导出）

## 第四波（2026-08-07）— ✅ 完成（治理 + 共享 + 扩展 + 告警）

### ✅ Config 用户与权限（归并自原 management）
- [x] `UserManager`（`users.yaml`/`permissions.yaml`）：内置 `admin` 角色全量放行，`operator`/`viewer` 角色；用户 CRUD + 角色增删
- [x] 细粒度权限：`grant/revoke/list/check_permission`，按 module 与 agent 作用域（`read|write|admin`，`*` 通配，write 隐含 read）
- [x] CLI `config user list|show|create|delete|role add|remove` / `config perm grant|revoke|list|check`；Tauri `list/create/delete_user`、`add/remove_user_role`、`grant/revoke/list/check_permission`

### ✅ Prompt 社区共享
- [x] `CommunityManager`：`prompts/community/` 快照带来源追溯（publisher/published_at/source），`publish`（force 覆盖）/`list`/`get`/`delete`/`install`（可换 id）
- [x] CLI `prompt publish` / `prompt community list|show|install|delete`；Tauri `publish_prompt`、`list/get/install/delete_community_prompt`
- [x] GUI Prompt 管理器新增 Community 页签（发布/安装/删除）

### ✅ Skill 技能市场（本地注册表，离线可用）
- [x] `MarketplaceManager`：`skills/marketplace/`（packages + index.json + ratings），搜索（name/description/tags）、评分（1-5 历史）、安装计数、`refresh` 重建索引保留统计、`add_package` 导入
- [x] CLI `skill market refresh|search|info|install|rate|stats|add-package`；Tauri `market_*`

### ✅ Skill 插件系统（归并自原 management）
- [x] `PluginManager`：`skills/plugins/<name>/plugin.yaml` 清单（entry + hooks），`register/unregister/enable/disable/run_hook`，钩子事件 `on_install/on_uninstall/on_session_end/on_monitor/on_backup`，命令执行捕获输出 + 30s 超时
- [x] CLI `plugin list|show|register|unregister|enable|disable|run <event>`；Tauri `list/register/unregister_plugin`、`set_plugin_enabled`、`run_plugin_hook`

### ✅ 横切：告警推送渠道
- [x] `Notifier`（`notify.yaml` 通道配置）：webhook（ureq HTTP(S) JSON POST，带超时）、email（RFC-2822 `.eml` 落盘 outbox，MTA 投递不在 v1 范围）、file（追加日志）
- [x] `monitor --notify` 将监控报告推送至启用通道；CLI `notify list|add|remove|enable|disable|send`；Tauri `list/add/remove/set_enabled_notify_channel`、`send_notification`

### ✅ 备份扩展
- [x] 备份纳入 users/permissions/community_prompts/notify_channels（secrets 值仍有意排除）

### 📊 验证结果
- Rust：236 测试全过（182 core + 9 集成 + 40 cli + 5 tauri），clippy 0 警告，fmt 干净
- 前端：11 测试全过，vue-tsc 0 错误，vite build 通过
- 变更：5 个新 core 模块（users/community/marketplace/plugin/notify）+ backup 扩展 + CLI 11 组命令 + Tauri 24 个命令 + 新 Extensions 视图 + Prompt 社区页签

### 本波未覆盖（留待后续）
- Config：OS keyring 后端、基于权限的 CLI 鉴权强制执行（当前为查询接口）
- Prompt：社区推送渠道（当前本地目录同步）、提示词效果追踪
- Skill：在线技能市场（需网络）、插件市场
- 横切：SMTP 直发（当前 email 通道落盘 .eml 待 MTA 投递）、告警去重/分级

## 长期规划第三波（2026-08-06）— ✅ 完成（安全存储 + 智能检索 + 工作流编排）

### ✅ Config 密钥链存储 + API Key 轮换
- [x] `SecretStore`：文件密钥链（`secrets.yaml`，0600 权限），值永不出现在 agent 配置/模板中；`redact` 脱敏列表；轮换历史归档（`previous` 可回滚）
- [x] 内联密钥迁移（`migrate_secret`：旧配置明文值搬入密钥链并清空配置文件）；`get_secret` 兼容旧内联值回退
- [x] CLI `config secret set|get|delete|list` / `config rotate` / `config migrate`；Tauri `get/set/delete/list/rotate/migrate_secret`
- [x] 说明：OS keyring 已评估（libsecret/Keychain/DPAPI 系统依赖、headless Linux 不可用），当前文件密钥链零依赖，未来可换 keyring 后端

### ✅ Memory 向量检索 + 知识图谱
- [x] 本地向量嵌入（FNV-1a 特征哈希字符 3-gram → 256 维，L2 归一化，无网络/无模型权重）+ `search_entries_vector`（title×3/tags×2/content×1 加权）
- [x] `hybrid_search`：BM25 与向量分数独立归一化后 50/50 融合，返回 `MemoryMatch`（score + method）
- [x] 知识图谱：实体抽取（tags / 标题 token / 内容引号短语）+ 共现关系权重，持久化 `memory/graph.json`；`build_graph`/`load_graph`/`neighbors`/`summary`
- [x] CLI `memory search-vector|search-hybrid` / `memory graph build|entities|neighbors|export`；Tauri `search_memories_vector/hybrid`、`build/get_memory_graph`、`graph_neighbors`

### ✅ Skill 工作流编排
- [x] `Workflow`（id/name/description/steps），步骤含 `args` 与 `optional`；`run_workflow` 逐步骤校验（存在/启用/依赖命令/版本兼容），可选步骤失败标记 skipped 不阻断
- [x] CLI `skill workflow list|show|create|delete|run`（步骤语法 `skill[:opt][;k=v;...]`）；Tauri `list/create/delete/run_workflow`

### ✅ Prompt 从会话提取
- [x] `extract_from_message` / `extract_from_session`：URL/路径/版本号/数字/引号文本/异形标识符 → `{{占位符}}` 变量模板，保存为新 prompt（tag=extracted，category=session-extracted）
- [x] CLI `prompt extract <session> [--message N] [--id] [--name] [--description]`；Tauri `extract_prompt_from_session`

### ✅ Overview Web 仪表盘（浏览器独立视图）
- [x] `render_dashboard_html`：自包含 HTML（内联 CSS/JS + 嵌入 JSON `__AGENTHUB_DASHBOARD__`），无服务器
- [x] CLI `status --html <file>`；Tauri `get_dashboard_html`

### ✅ 横切：监控 JSON + 定时化入口
- [x] `MonitorReport::to_json` + `alert_summary`（供 cron/systemd 消费）；CLI `monitor --json` / `monitor --watch <sec>` 循环

### ✅ 备份扩展
- [x] 备份纳入 workflows + memory_graph（secrets 值有意排除，restore 不重建密钥链）

### 📊 验证结果
- Rust：207 测试全过（159 core + 9 集成 + 34 cli + 5 tauri），clippy 0 警告，fmt 干净
- 前端：11 测试全过，vue-tsc + vite build 通过
- 提交 `ddc2bae` + 文档提交 `a9467f2` + todo/CHANGELOG 提交 `ca5ef5f` 已推送 origin/main

### 本波未覆盖（留待后续）
- Config：OS keyring 后端（已评估，留接口）、API Key 轮换通知/审计接入
- Memory：向量索引持久化（当前每次搜索即时嵌入）、知识图谱可视化前端
- Skill：技能市场（需网络）、插件系统
- Prompt：社区共享、提示词效果追踪（关联会话结果）
- Overview：仪表盘交互化（当前静态导出）
- 横切：告警推送渠道（邮件/webhook）

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
- Config：OS keyring 后端（已评估）、API Key 轮换通知/审计接入
- Memory：向量索引持久化、知识图谱可视化前端
- Skill：技能市场（需网络）、插件系统
- Overview：仪表盘交互化（当前静态导出）
- 横切：告警推送渠道（邮件/webhook）

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
- [x] 低配置（RAM<2G / 存储<40G）Linux 构建测试：`ci-low-resource.yml` + `docs/low-resource-ci.md` ✅（2026-08-06）

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
- [x] API Key 密钥链存储（文件密钥链 SecretStore，0600 权限；OS keyring 已评估暂缓）✅（2026-08-06 第三波）
- [x] API Key 轮换（rotate 归档旧值可回滚）✅（2026-08-06 第三波）
- [x] 用户与权限（✅ 2026-08-07 第四波）

### Session 会话管理
- [x] 成本追踪（模型价格表 + record_usage）✅（2026-08-06 首波）
- [x] 会话回放、会话模板 ✅（2026-08-06 首波）
- [x] 成本阈值告警（daily/monthly 预算）✅（2026-08-06 第二波）
- [x] 跨 Agent 会话上下文传递（fork）✅（2026-08-06 第二波）

### Prompt 提示词管理
- [x] 版本控制、变量校验、使用统计 ✅（2026-08-06 首波）
- [x] 导入/导出（含版本历史）✅（2026-08-06 第二波）
- [x] 从 Agent 会话中提取提示词（URL/路径/版本等 → {{占位符}} 模板）✅（2026-08-06 第三波）
- [x] 社区共享（✅ 2026-08-07 第四波）

### Skill 技能管理
- [x] 版本管理与兼容性检查 ✅（2026-08-06 第二波）
- [x] 工作流编排（多技能组合，含可选步骤/依赖/兼容性校验）✅（2026-08-06 第三波）
- [x] 技能市场（✅ 2026-08-07 第四波）
- [x] 插件系统（✅ 2026-08-07 第四波）

### Memory 记忆管理
- [x] BM25 语义检索 ✅（2026-08-06 首波）
- [x] 记忆衰减（importance + 归档）✅（2026-08-06 首波）
- [x] 记忆导入/导出/同步 ✅（2026-08-06 第二波）
- [x] 向量检索（本地特征哈希嵌入，无网络）+ 混合检索（BM25×向量）✅（2026-08-06 第三波）
- [x] 知识图谱（实体抽取 + 共现关系，persist graph.json）✅（2026-08-06 第三波）

### Overview 概览模块（只读聚合，不承载业务逻辑）
- [x] 状态概览 `agenthub status` / GUI 仪表盘 ✅（2026-08-06 首波）
- [x] 时间维度趋势（成本 / 会话数 / 审计量）✅（2026-08-06 第二波）
- [x] Web 仪表盘（浏览器独立视图，`status --html` 静态导出）✅（2026-08-06 第三波）

### 横切能力（非模块，工具而非业务模块）
- [x] 审计日志 ✅（2026-08-06 首波，install/uninstall 已接入）
- [x] 备份/恢复 ✅（2026-08-06 首波；第三波扩展纳入 workflows + memory_graph）
- [x] 监控与告警（第一版：诊断/未安装/预算/兼容性）✅（2026-08-06 第二波）
- [x] 监控定时化入口（`monitor --json` / `--watch` 供 cron/systemd）✅（2026-08-06 第三波）
- [x] 告警推送渠道（webhook/email-spool/file，✅ 2026-08-07 第四波）
