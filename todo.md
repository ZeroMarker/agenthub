# TODO

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
- [ ] 发布 v1.0.0
- [ ] 安装/升级/卸载回归测试

## 长期规划（goal.md 七大模块）

- [ ] **Config** 模块：API Key 密钥链存储、多环境配置、配置模板
- [ ] **Memory** 模块：语义搜索（BM25 + 向量）、知识图谱、记忆衰减
- [ ] **Session** 模块：成本追踪、会话回放、会话模板
- [ ] **Prompt** 模块：版本控制、变量插值、使用统计
- [ ] **Skill** 模块：技能市场、工作流编排、依赖检查
- [ ] **Management** 模块：仪表盘、审计日志、备份恢复、插件系统
