# 代码签名策略

> 版本：v1.0
> 状态：规划中（截至 v1.4.0，各平台产物尚未签名；当前依赖 SHA-256 校验和）
> 相关：`SECURITY.md`、`.github/workflows/release.yml`

本文档定义 AgentHub 发布产物的代码签名策略，覆盖 Windows、macOS 和 Linux 三个平台。

## 1. 总体原则

- **用户信任优先**：签名用于帮助用户验证产物来源与完整性，降低供应链攻击风险。
- **未签名必声明**：任何未签名的产物必须在发布说明中明确标注，不允许静默发布。
- **校验和兜底**：无论是否签名，所有产物均附带 SHA-256 校验和（见 `scripts/generate-checksums.*`）。
- **CI 自动化**：签名流程集成到 release 流水线，减少人工步骤。

## 2. 平台签名方案

### Windows（Authenticode）

| 项目 | 要求 |
|---|---|
| 证书类型 | 代码签名证书（OV 或 EV），建议 EV 以获得更高信任等级 |
| 签名工具 | `signtool.exe`（Windows SDK）或 CI 中的 `azure-sign-tool` |
| 时间戳 | 必须加时间戳服务器（如 `http://timestamp.digicert.com`），保证证书过期后签名仍有效 |
| 覆盖产物 | `.exe`、`.msi`、`.dll` |
| 建议 | 在 GitHub Actions 的 `windows-latest` 上使用 `signtool sign /fd SHA256 /tr <ts> /td SHA256` |

### macOS（Developer ID + 公证）

| 项目 | 要求 |
|---|---|
| 证书类型 | Developer ID Application（`Developer ID Installer` 用于 .pkg） |
| 公证 | 所有产物必须提交 Apple 公证（`notarytool`），否则 Gatekeeper 会拦截 |
| Hardened Runtime | 必须启用 `--options runtime` |
| 覆盖产物 | `.app`、`.dmg`、`.pkg` |
| 建议 | 使用 GitHub Actions 的 `macos-latest`，密钥通过 secrets 注入；公证凭据使用 App Store Connect API Key |

### Linux

| 项目 | 要求 |
|---|---|
| 现状 | Linux 无统一的强制签名机制；发行版仓库是主要信任链 |
| 推荐做法 | 发布 `.deb`/`.rpm` 时使用发行版打包签名（如 `debsign`、`rpmsign`），AppImage 可选用 GPG 签名 |
| 最低要求 | 附带 `SHA256SUMS-<target>`；引入 GPG 签名后同时发布公钥，供用户验证 |

## 3. 未签名产物声明

当以下任一情况成立时，产物视为**未签名**，必须在发布说明（Release Notes）显著位置声明：

- 证书或密钥尚未就绪；
- CI 签名步骤被跳过或失败；
- 该平台无签名方案（如 Linux 普通二进制）。

声明模板：

> ⚠️ **未签名产物**：本版本 [平台] 产物未进行代码签名。请务必使用发布页提供的 SHA-256 校验和核验文件完整性后再运行。

## 4. 发布日签名检查清单

- [ ] 所有证书/密钥已注入 CI secrets（`WINDOWS_CERT_BASE64`、`MACOS_CERT_P12`、`APPLE_API_KEY`、`GPG_KEY` 等）
- [ ] Windows：`signtool` 验证通过（`signtool verify /pa /v`）
- [ ] macOS：公证完成且 stapler 成功（`xcrun stapler validate`）
- [x] 所有产物已生成 `SHA256SUMS-<target>` 并随 Release 上传
- [ ] 未签名平台已按模板声明
- [ ] 下载并核验任一产物的校验和与签名

## 5. 验证指南（用户侧）

```bash
# 校验和
sha256sum -c SHA256SUMS-<target>

# Windows 签名验证
signtool verify /pa /v <file>.exe

# macOS 公证状态
spctl --assess --type execute <file>.app
```

---

如需申请签名证书或配置 CI secrets，请参考 `.github/workflows/release.yml` 和项目维护者文档。
