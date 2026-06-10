# HostGuard

一款 macOS 桌面工具：管理 **hosts**、**Shell / 系统配置**，并查看**系统用量**。
CleanMyMac X 风格「Sentinel」深色玻璃拟态界面（异形分层窗口）。

> Tauri 2 · Vue 3.5 · TypeScript · Vite · Pinia · Vue Router · Naive UI · UnoCSS · CodeMirror 6 · oxlint / oxfmt

## 功能

- **Hosts 管理**：读取 / 编辑 `/etc/hosts`，启用停用、搜索、一次性授权后直接保存（自动备份）。
- **Shell 配置**：`~/.zshrc` / `.zprofile` / `.zshenv` / `.zlogin` 多文件语法高亮编辑器，保存前备份。
- 概览仪表盘、深 / 浅色主题、系统配置与系统用量（开发中）。

详细路线图见 [`TASKS.md`](./TASKS.md)。

## 开发

```bash
pnpm install
pnpm tauri dev      # 启动桌面应用（热更新）
```

常用脚本：`pnpm build`（前端构建 + 类型检查）、`pnpm lint`（oxlint）、`pnpm format`（oxfmt）。

## 打包为 macOS 应用

```bash
pnpm tauri build    # 或 pnpm app:build
```

产物位于：

- 应用：`src-tauri/target/release/bundle/macos/HostGuard.app`
- 安装包：`src-tauri/target/release/bundle/dmg/HostGuard_<版本>_<架构>.dmg`
  （Apple Silicon 为 `aarch64`，Intel 为 `x64`）

## 安装

**方式一（推荐，DMG）**：双击打开 `.dmg`，把 `HostGuard` 拖到 `Applications` 文件夹。

**方式二**：直接把 `HostGuard.app` 拷贝到「应用程序」。

### 关于 Gatekeeper（未签名应用）

当前未做 Apple Developer 签名 / 公证。**本机自行构建**的应用可直接打开。若是从别处**下载**的 `.dmg`，macOS 会拦截，按需任选其一：

- 右键 App →「打开」→ 在弹窗中再次「打开」；或
- 终端执行：`xattr -cr /Applications/HostGuard.app`

如需面向他人分发，请配置 Apple Developer ID 签名 + 公证（在 `src-tauri/tauri.conf.json` 的 `bundle.macOS` 中设置 `signingIdentity`，并配合 `tauri build` 的公证环境变量）。

## 权限说明

修改 `/etc/hosts` 需要管理员权限：应用会**一次性**弹出系统授权（为当前用户添加写入 ACL），之后保存无需重复输入密码。`~` 下的 Shell 配置为用户文件，无需授权。
