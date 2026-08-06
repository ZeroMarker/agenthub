# 低配置 Linux 机器上的构建与测试（GitHub Actions）

> 适用场景：RAM < 2 GB、存储 < 40 GB 的 Linux 机器（如小型 VPS、树莓派、受限自托管
> runner）。本文说明如何在此类机器上用 GitHub Actions 完成本项目的构建与测试，以及
> 如何本地复现同样的约束环境。

---

## 1. 为什么需要这份指南

GitHub 托管的 `ubuntu-latest` runner 有 2 vCPU / 7 GB RAM / 14 GB SSD，通常够用；
但**自托管 runner**（self-hosted）跑在你自己控制的机器上，可能是只有 1-2 GB 内存、
40 GB 磁盘的低配设备。本项目包含：

| 组件 | 类型 | 构建资源敏感点 |
|------|------|----------------|
| `agenthub-core` | 纯 Rust 库（约 9k 行） | 内存友好，编译峰值低 |
| `agenthub-cli` | Rust 二进制 | 内存友好 |
| `agenthub-ui/src-tauri` | Tauri 桌面应用 | **链接 webkit2gtk 时内存峰值高** |
| `agenthub-ui` | Vue 3 + Vite 前端 | Node 构建，需控制堆内存 |

实测参考：本项目 `target/` 目录（debug 全量构建）约 **9 GB**；`cargo test --workspace`
在 2 核机器上约 3-8 分钟。在低配机器上，**失控点主要是并行度与 target 目录大小**，
两者都可控。

---

## 2. 内存 < 2 GB 的构建策略

### 2.1 限制并行度（最重要）

cargo 默认并行数 = CPU 核数，每个并行编译单元峰值约 0.5-1 GB。1 GB 内存机器直接跑
会 OOM，必须限制：

```bash
# 全局环境变量方式（推荐在 CI 中使用）
export CARGO_BUILD_JOBS=2

# 或命令行方式
cargo build -j 2
cargo test -j 2

# 测试内部的并行也是同理
cargo nextest run -j 2   # 如使用 nextest
```

### 2.2 降低单次编译内存峰值

```bash
# dev（默认 profile）：关闭 debug info，显著降低内存与磁盘占用
export CARGO_PROFILE_DEV_DEBUG=0
# 或（旧版本写法）
export CARGO_PROFILE_DEV_DEBUGINFO=0

# release：同样关闭 debug info，并放宽 codegen-units 以降低单单元内存
export CARGO_PROFILE_RELEASE_DEBUG=0
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_OPT_LEVEL=2   # 或 1，进一步省内存

# 关闭增量编译（减少磁盘占用，避免增量目录累积）
export CARGO_INCREMENTAL=0
```

> 说明：`codegen-units` 越大，每个单元越小、内存峰值越低（代价是生成的代码稍慢）。
> 本项目对性能不敏感，低配机器上这些取舍完全值得。

### 2.3 跳过桌面端构建（推荐）

Tauri 桌面应用需要 `libwebkit2gtk` 并做最终链接，内存与磁盘开销最大。**低配机器上
做 CI 验证时可以只测核心库与 CLI**：

```bash
cargo test -p agenthub-core -p agenthub-cli
```

桌面端在托管 runner 上由 `release.yml` 负责，不必占用低配 runner 资源。

### 2.4 前端构建内存控制

```bash
export NODE_OPTIONS="--max-old-space-size=512"
npm ci --no-audit --no-fund
npm run build        # vue-tsc + vite build
```

---

## 3. 存储 < 40 GB 的磁盘策略

### 3.1 控制 target 目录

`target/` 是磁盘大头（本项目全量 debug 约 9 GB）。做法：

```bash
# 只保留需要的产物，随时可重建
cargo clean -p agenthub-ui     # 清理桌面端（最大的一块）
cargo clean                    # 全量清理（释放大部分空间）

# 构建前检查磁盘
df -h / && du -sh target 2>/dev/null
```

### 3.2 控制 cargo registry 缓存

```bash
# registry 与增量编译缓存也会累积
du -sh ~/.cargo/registry ~/.cargo/git 2>/dev/null
rm -rf ~/.cargo/registry/cache    # 源码缓存可删（解压后的 src 保留即可）
# 或使用 cargo-sweep 清理过期增量文件
# cargo install cargo-sweep && cargo sweep --time 7d
```

### 3.3 GitHub Actions 缓存策略

`swatinem/rust-cache@v2` 默认会缓存整个 target 目录，低磁盘场景下反而有害。
按需收紧：

```yaml
- uses: swatinem/rust-cache@v2
  with:
    workspaces: './agenthub-ui/src-tauri -> target'
    save-if: ${{ github.ref == 'refs/heads/main' }}   # 只在 main 上写缓存
    cache-on-failure: false
```

- `save-if`：PR 上只读缓存、不写缓存，避免每个 PR 都复制一份大缓存。
- 若磁盘仍紧张，可完全关闭缓存（`cache-provider: "none"` 或去掉该步骤），
  代价是每次全量编译（本项目全量编译约 3-8 分钟，可接受）。

### 3.4 分步清理（workflow 内）

```yaml
- name: Free disk space
  run: |
    sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc
    rm -rf ~/.cargo/registry/cache
    df -h /
```

---

## 4. 低资源 CI workflow

仓库提供 `.github/workflows/ci-low-resource.yml`，可在自托管低配 runner 上运行：

```yaml
# .github/workflows/ci-low-resource.yml
name: CI (low-resource)

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

jobs:
  core:
    runs-on: [self-hosted, linux, low-resource]   # 按需修改 labels
    env:
      CARGO_BUILD_JOBS: 2
      CARGO_PROFILE_DEV_DEBUG: 0
      CARGO_INCREMENTAL: 0
      CARGO_TERM_COLOR: always
    steps:
      - uses: actions/checkout@v4

      - name: Disk check
        run: df -h / && free -m

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache (read-only on PRs)
        uses: swatinem/rust-cache@v2
        with:
          save-if: ${{ github.ref == 'refs/heads/main' }}

      - name: Install core build deps (no webkit)
        run: |
          sudo apt-get update
          sudo apt-get install -y patchelf

      - name: Test core + cli
        run: cargo test -p agenthub-core -p agenthub-cli

      - name: Clippy (core + cli)
        run: cargo clippy -p agenthub-core -p agenthub-cli -- -D warnings

      - name: Format check
        run: cargo fmt --all -- --check

  frontend:
    runs-on: [self-hosted, linux, low-resource]
    env:
      NODE_OPTIONS: --max-old-space-size=512
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 'lts/*'
      - run: npm ci --no-audit --no-fund
        working-directory: agenthub-ui
      - run: npm run build
        working-directory: agenthub-ui
```

要点：

- **不构建 Tauri 桌面端**：避免 webkit 依赖与高内存链接，核心验证已足够。
- **`CARGO_BUILD_JOBS=2`**：内存安全，2 核机器不损失太多时间。
- **`save-if` 控制缓存写入**：防止磁盘被缓存占满。
- **`workflow_dispatch`**：可在低峰期手动触发。

---

## 5. 本地复现（与 CI 相同约束）

在任意 Linux 机器上模拟低配环境：

```bash
# 模拟 2 GB 内存限制（容器方式最接近）
docker run --rm -it --memory=2g --cpus=2 \
  -v $PWD:/src -w /src rust:1-bookworm bash

# 容器内：
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_INCREMENTAL=0
cargo test -p agenthub-core -p agenthub-cli

# 前端（宿主机或 Node 容器）：
export NODE_OPTIONS="--max-old-space-size=512"
cd agenthub-ui && npm ci --no-audit --no-fund && npm run build
```

---

## 6. 常见问题

| 现象 | 原因 | 解决 |
|------|------|------|
| `SIGKILL` / OOM，构建中途退出 | 并行度太高，内存耗尽 | `CARGO_BUILD_JOBS=2`（或 1） |
| `No space left on device` | target / 缓存占满磁盘 | `cargo clean -p`、清 `~/.cargo/registry/cache`、调缓存策略 |
| `link ... failed: signal: 9` | 链接阶段内存峰值超限 | 跳过 Tauri 桌面构建；或临时加 swap（`sudo fallocate -l 2G /swapfile`） |
| `libwebkit2gtk-4.1-dev` 找不到 | 发行版较旧 | 升级系统、或只测 core/cli（见 §2.3） |
| PR 频繁触发导致磁盘写满 | rust-cache 每个 PR 都写缓存 | `save-if` 只允许 main 写缓存 |

---

## 7. 参考

- [GitHub 托管 runner 规格](https://docs.github.com/actions/using-github-hosted-runners/using-github-hosted-runners/about-github-hosted-runners)
- [自托管 runner 入门](https://docs.github.com/actions/hosting-your-own-runners/managing-self-hosted-runners/about-self-hosted-runners)
- [swatinem/rust-cache](https://github.com/Swatinem/rust-cache)
- 本仓库 CI：`.github/workflows/ci.yml`（托管 runner）、`.github/workflows/release.yml`（发布）
