# GitHub 自动更新 + Tag 发布（0.0.10 → 0.1.0）设计

日期：2026-06-17
状态：已批准，待实现

## 背景与目标

HostGuard 已具备跨平台（macOS + Windows）CI 打包能力，但 `ci.yml` 只在
push/PR 时构建并上传 artifact，**没有**在打 tag 时创建 GitHub Release 的流程，
也**没有**任何应用内更新能力。

本次要交付两件事：

1. 通过打 tag 触发 CI，发布 `0.0.10` 版本，Release 页提供 Windows 与 macOS
   （Apple Silicon / M 芯片）的可下载安装包。
2. 基于 GitHub Releases 实现应用内更新（in-app update），并以真实的
   `0.0.10 → 0.1.0` 升级作为端到端测试，最终发布带更新能力的 `0.1.0`。

## 关键约束（决定了执行顺序）

Tauri 的自动更新器**只能更新「自身已内置更新器」的旧版本**。第一个内置更新器的
版本是「地板版本」——比它更早的版本无法自动升级，用户必须手动重装一次。

因此 `0.0.10` 必须**内置更新器**，否则：
- 用户想要的「从上一个 release 测试更新」无法实现（裸 `0.0.10` 没有检查更新的代码）；
- `0.0.10` 用户会卡在 `0.0.10`，直到手动下载 `0.1.0`。

结论：**先做更新器功能，再发 `0.0.10`（已内置更新器），最后发 `0.1.0`。**

## 执行阶段

### Phase 0 — 基础设施 + 功能（暂不发布）
- 接入更新器插件（Rust + JS）与 process 插件（用于重启）。
- Settings → 关于 增加「检查更新」的手动按钮 UI。
- 生成 Tauri 更新签名密钥对，pubkey 写入 `tauri.conf.json`。
- 新增 `release.yml` 工作流（打 tag 触发）。

### Phase 1 — 发布 0.0.10（part 1 交付物）
- 三处版本号统一改为 `0.0.10`。
- 提交并打 tag `v0.0.10`，push → CI 发布 GitHub Release，含 Windows + macOS(M 芯片) 下载。

### Phase 2 — 发布 0.1.0 + 测试更新（part 2 交付物）
- 版本号 bump 到 `0.1.0`，打 tag `v0.1.0`，push → CI 发布 `0.1.0` Release。
- 测试：安装已发布的 `0.0.10` → 打开 → 点「检查更新」→ 发现 / 校验 / 安装 `0.1.0` → 重启。

## Tauri GitHub 更新器工作原理（教学要点）

- 每个构建内嵌一个**公钥**。每个 release 产物用对应**私钥**签名（Tauri 自带的
  *minisign*，免费，**与 Apple 公证完全无关**）。
- 构建时每个平台产出「更新归档」+ `.sig` 文件，外加一个 `latest.json` 清单，列出
  最新版本号、各平台下载 URL 与签名。
- 运行中的应用拉取
  `https://github.com/Azir-11/HostGuard/releases/latest/download/latest.json`，
  比对自身版本与清单；若有更新：下载归档 → **用内嵌公钥校验签名** → 安装 → 重启。
- macOS 通过 `.app.tar.gz` 更新；Windows 通过 NSIS `-setup.exe` 更新；两者都随同一
  release 一起发布。
- **关键**：release 必须是**已发布且标记为 "latest"**（GitHub 不会对 draft 提供
  `/releases/latest/`），所以工作流要自动 publish。

## 后端改动（`src-tauri`）

- 新增依赖：`tauri-plugin-updater`、`tauri-plugin-process`（重启用）。
- 在 `lib.rs` 的 builder 中注册这两个插件。**不需要新增自定义 `#[tauri::command]`**
  —— JS 插件驱动全部流程。
- `tauri.conf.json`：新增 `plugins.updater`（`endpoints` + `pubkey`）以及
  `bundle.createUpdaterArtifacts: true`。
- `capabilities/default.json`：新增 `updater:default` 与 `process:allow-restart`。

## 前端改动（仅手动触发）

- 新增依赖：`@tauri-apps/plugin-updater`、`@tauri-apps/plugin-process`。
- Settings → 关于：
  - 版本号改为运行时 `getVersion()` 读取（去掉硬编码的 `v0.1.0`）。
  - 增加「检查更新」行 + 按钮：
    - 点击 → `check()`。无更新 → `useMessage` 提示「已是最新版本」。
    - 有更新 → 弹窗显示新版本号 + release notes + 「下载并安装」（含下载进度），
      完成后 `relaunch()`。
    - 非 Tauri 环境（web dev）→ 像现有 `openRepo` 一样安全 no-op。

## CI / 发布工作流（新增 `release.yml`，`ci.yml` 不动）

- 触发：`push: tags: ['v*']`（外加手动 `workflow_dispatch`）；`permissions: contents: write`。
- 矩阵：`macos-latest` 以 `aarch64-apple-darwin`（M 芯片）构建；`windows-latest`（x64）。
- 使用 `tauri-apps/tauri-action` 一步完成：构建、签名、创建并发布 Release，上传
  dmg / nsis / 更新归档 / `.sig` / `latest.json`。
- Release body 说明两个下载入口，以及 macOS「右键 → 打开」首次启动提示（未公证）。

## 用户需自行完成（我无法代劳）：两个 GitHub Secret

我在本地生成密钥对（私钥写入被 git 忽略的文件，**绝不提交**），并给出要粘贴到
**repo → Settings → Secrets → Actions** 的值：
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

在这两个 Secret 存在之前，触发发布的 push 会因签名失败而报错。因此顺序为：
我搭好一切 → 你添加 2 个 Secret → 我 push tag。

## 版本同步

`0.0.10` 必须在 `package.json`、`tauri.conf.json`、`Cargo.toml` 三处写成完全一致
（更新器比较的是 `tauri.conf.json` 的版本）。Settings UI 运行时读取，从此不再漂移。

## 范围之外（YAGNI）

- 启动时自动检查更新
- Apple 公证 / 代码签名
- Intel mac / universal 构建
- 增量（delta）更新
- changelog 自动化
```
