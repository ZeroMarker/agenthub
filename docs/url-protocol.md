# 自定义 URL 协议

本文档定义 AgentHub 桌面应用的自定义 URL 协议设计，用于从浏览器、终端、文档或其他应用中唤起 AgentHub 并跳转到指定功能。

> 状态：设计文档。当前仓库尚未接入 `@tauri-apps/plugin-deep-link` / `tauri-plugin-deep-link`，实现时应以本文档作为行为契约。

## 协议名称

AgentHub 使用固定 scheme：

```text
agenthub://
```

命名规则：

| 项 | 约定 |
|----|------|
| Scheme | `agenthub` |
| 大小写 | URL 解析时大小写不敏感，文档和示例统一使用小写 |
| 编码 | 参数值必须使用 URL encode |
| 兼容性 | 不复用第三方 Agent 的 scheme，避免抢占其他应用入口 |

## URL 格式

通用格式：

```text
agenthub://<resource>/<action>?<query>
```

示例：

```text
agenthub://agent/codex
agenthub://agent/cursor?action=install
agenthub://search?q=claude&type=desktop
agenthub://diagnostic
agenthub://settings
```

## 支持的入口

| URL | 行为 | 备注 |
|-----|------|------|
| `agenthub://` | 打开 AgentHub 主窗口 | 不执行额外操作 |
| `agenthub://agent/<id>` | 打开指定 Agent 详情 | `<id>` 必须匹配 `agents.json` 中的 `id` |
| `agenthub://agent/<id>?action=install` | 打开 Agent 详情并预选安装动作 | 必须展示确认，不允许静默安装 |
| `agenthub://agent/<id>?action=uninstall` | 打开 Agent 详情并预选卸载动作 | 必须展示确认，不允许静默卸载 |
| `agenthub://search?q=<query>` | 打开搜索结果 | `q` 为空时回到代理列表 |
| `agenthub://search?q=<query>&type=cli` | 搜索 CLI Agent | `type` 可为 `cli` 或 `desktop` |
| `agenthub://diagnostic` | 打开诊断视图 | 仅展示诊断页面，不自动修改系统 |
| `agenthub://settings` | 打开配置管理 | 用于外部文档指向设置页 |

未识别的 URL 必须降级为打开主窗口，并显示可恢复的错误提示。

## 安全约束

URL 协议只能用于导航和预填操作，不得直接执行会修改系统的动作。

实现时必须遵守：

1. 安装、卸载、批量操作必须继续要求用户确认。
2. URL 中的 Agent ID 必须从 `agents.json` 精确匹配，不允许拼接为命令参数。
3. `action` 只接受白名单值：`install`、`uninstall`。
4. `type` 只接受白名单值：`cli`、`desktop`。
5. 忽略未知参数，不把未知参数透传给后端命令。
6. 日志中可以记录 URL 路由结果，但不要记录包含令牌或个人信息的 query 参数。

## Tauri 配置

Tauri 2 推荐使用 deep link 插件注册桌面 scheme。静态注册示例：

```json
{
  "plugins": {
    "deep-link": {
      "desktop": {
        "schemes": ["agenthub"]
      }
    }
  }
}
```

Rust 依赖示例：

```toml
[dependencies]
tauri-plugin-deep-link = "2"
tauri-plugin-single-instance = { version = "2", features = ["deep-link"] }
```

桌面端建议同时启用 single instance 插件，避免 Windows 和 Linux 在已有窗口运行时再次启动一个独立实例。

```rust
#[cfg(desktop)]
{
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
        // 已运行实例被 URL 唤起时，deep-link 插件会触发对应事件。
        // 如果未来使用运行时注册 scheme，也需要在这里校验 argv。
        let _ = app;
        let _ = argv;
    }));
}

builder = builder.plugin(tauri_plugin_deep_link::init());
```

## 前端路由约定

前端应把 URL 解析成统一的路由对象，再交给界面状态处理：

```ts
type UrlRoute =
  | { view: "agents" }
  | { view: "agent-detail"; agentId: string; action?: "install" | "uninstall" }
  | { view: "search"; query: string; kind?: "cli" | "desktop" }
  | { view: "diagnostic" }
  | { view: "settings" };
```

解析建议：

1. 使用标准 `URL` API 解析。
2. 先规范化 host、pathname 和 query。
3. 对所有枚举字段做白名单校验。
4. 对非法 URL 返回 `{ view: "agents" }` 并附带错误消息。

## 处理启动和运行中唤起

应用启动时读取当前 deep link：

```ts
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";

const startUrls = await getCurrent();
if (startUrls?.length) {
  handleAgentHubUrl(startUrls[startUrls.length - 1]);
}

await onOpenUrl((urls) => {
  const latest = urls[urls.length - 1];
  if (latest) {
    handleAgentHubUrl(latest);
  }
});
```

同一次唤起包含多个 URL 时，AgentHub 默认处理最后一个 URL，并把其他 URL 记录为调试信息。

## 平台验证

### Windows

```powershell
Start-Process "agenthub://agent/codex"
Start-Process "agenthub://search?q=cursor&type=desktop"
```

### macOS

```bash
open "agenthub://agent/codex"
open "agenthub://diagnostic"
```

### Linux

```bash
xdg-open "agenthub://agent/codex"
xdg-open "agenthub://settings"
```

验证清单：

| 场景 | 预期结果 |
|------|----------|
| 应用未运行时打开 URL | 启动应用并跳转到目标页面 |
| 应用运行中打开 URL | 聚焦已有窗口并跳转 |
| Agent ID 不存在 | 打开主窗口并提示找不到 Agent |
| `action=install` | 打开详情和确认流程，不自动执行安装 |
| 未识别路径 | 打开主窗口并提示 URL 不受支持 |

## 实现任务

落地该能力时建议拆分为以下任务：

1. 添加 `@tauri-apps/plugin-deep-link`、`tauri-plugin-deep-link` 和 `tauri-plugin-single-instance`。
2. 在 `agenthub-ui/src-tauri/tauri.conf.json` 注册 `agenthub` scheme。
3. 在 Tauri 后端初始化 deep link 和 single instance 插件。
4. 在前端新增 URL 解析模块和单元测试。
5. 在应用启动流程中处理 `getCurrent()` 和 `onOpenUrl()`。
6. 为 Windows、macOS、Linux 添加手动验收记录。

## 参考资料

- [Tauri Deep Linking](https://v2.tauri.app/plugin/deep-linking/)
- [@tauri-apps/plugin-deep-link API](https://v2.tauri.app/reference/javascript/deep-link/)
