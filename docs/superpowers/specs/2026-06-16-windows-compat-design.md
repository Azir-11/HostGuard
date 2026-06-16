# HostGuard —— Windows 兼容层设计（Spec）

- 日期：2026-06-16
- 状态：待评审
- 目标平台：macOS（已验证基线）、Windows（本次新增，代码就绪 + CI 编译验证）

## 1. 背景与目标

HostGuard 当前后端（`src-tauri/src/lib.rs`）把 macOS 专属实现写死：`/etc/hosts`、`chmod +a` ACL、`osascript`、zsh 配置文件、`dscacheutil`/`killall mDNSResponder` 等。目标是在**后端 API 层**引入一层平台抽象，使同一套 Tauri 命令在 macOS / Windows 上各自走对应实现，并写好 Windows 侧实现。

**非目标（本次不做）：** 前端窗口外观层（无边框 / 透明 / 自定义红绿灯，依赖 `macos-private-api`）在 Windows 上行为不同，属前端窗口层，列为日后单独事项。

## 2. 范围

**In scope**
- 后端按平台抽象：Hosts 读写与提权、DNS 刷新、Shell 配置。
- Windows 实现：hosts 路径与 `icacls` 一次性授权、`ipconfig /flushdns`、PowerShell `$PROFILE` 与 cmd `AutoRun`。
- 系统监控：`sysinfo` 已跨平台，仅处理 Windows 平台差异（无 load average）。
- 最小前端改动：让 Shell 页、Hosts 文案由后端数据驱动，不再硬编码平台字符串。
- CI：GitHub Actions，**macOS + Windows 双 job** 编译验证。

**Out of scope**
- 窗口 chrome / `macos-private-api` 的 Windows 替代。
- 代码签名 / 公证 / Windows 安装包分发。

## 3. 架构（方案 A：`#[cfg]` 平台模块 + 统一函数签名）

编译期按平台选择实现，零运行时开销，每平台代码隔离在各自文件。

### 3.1 目录结构

```
src-tauri/src/
  lib.rs              # Tauri 命令定义（薄封装，转调 platform::*）+ sysinfo（跨平台，保留）
  platform/
    mod.rs            # 共享类型、共享读写逻辑、按 cfg `pub use` 当前平台实现
    macos.rs          # #[cfg(target_os = "macos")] 现有 macOS 逻辑迁入
    windows.rs        # #[cfg(target_os = "windows")] 新 Windows 逻辑
```

### 3.2 共享类型（`platform/mod.rs`）

```rust
#[derive(serde::Serialize)]
pub struct ShellConfig {
    pub name: String,        // 稳定标识，命令参数用（如 "zshrc" / "powershell" / "cmd"）
    pub label: String,       // 展示名（如 ".zshrc" / "PowerShell" / "命令提示符 (cmd)"）
    pub path: String,        // 完整路径（展示用）
    pub exists: bool,
    pub reload_hint: String, // 重载提示命令（如 "source ~/.zshrc" / ". $PROFILE"）
}
```

### 3.3 平台接口（`macos.rs` / `windows.rs` 各自提供同名项）

```rust
// —— Hosts ——
pub fn hosts_path() -> std::path::PathBuf;
pub fn hosts_writable() -> bool;                       // 尝试以写打开
pub async fn grant_hosts_access() -> Result<(), String>;

// —— DNS ——
pub async fn dns_flush_granted() -> bool;
pub async fn grant_dns_flush_access() -> Result<(), String>;
pub async fn flush_dns() -> Result<(), String>;

// —— Shell ——
pub fn shell_configs() -> Vec<ShellConfig>;            // 平台各自的配置清单
pub fn shell_path(name: &str) -> Option<std::path::PathBuf>;
pub fn on_shell_saved(name: &str, path: &std::path::Path) -> Result<(), String>; // 写后钩子（Windows+cmd 设注册表 AutoRun；其余 noop）

// —— 通用 ——
pub fn backup_path(file_name: &str) -> Option<std::path::PathBuf>; // 备份目录（家目录）下的 .hostguard.* 文件
```

共享逻辑（`mod.rs`）：`read_hosts` / `write_hosts`（备份 + 写，路径来自 `platform::hosts_path()`）、`read_shell_config` / `write_shell_config`（路径来自 `platform::shell_path()`，写后调 `platform::on_shell_saved()`）。`lib.rs` 的 `#[tauri::command]` 只做参数转发与错误归一。

## 4. 子系统设计

### 4.1 Hosts

| | macOS（不变） | Windows（新增） |
|---|---|---|
| 路径 | `/etc/hosts` | `%SystemRoot%\System32\drivers\etc\hosts`（经 `SystemRoot` 环境变量解析） |
| 可写检测 | 以写打开 | 以写打开 |
| 一次性授权 | `chmod +a '<user> allow …' /etc/hosts`（osascript 管理员，单次弹窗） | `icacls "<hosts>" /grant "<user>:(M)"`，经 `powershell Start-Process icacls -Verb RunAs -Wait` 触发**一次 UAC**；之后用户可直接写 |
| 写入 | 授权后 `fs::write`，备份 `~/.hostguard.hosts.bak` | 授权后 `fs::write`，备份 `%USERPROFILE%\.hostguard.hosts.bak` |

> 解析器 `src/utils/hosts.ts` 与文件格式跨平台一致，无需改。

### 4.2 DNS 刷新

| | macOS（不变） | Windows（新增） |
|---|---|---|
| 命令 | `dscacheutil -flushcache` + `killall -HUP mDNSResponder` | `ipconfig /flushdns` |
| 提权 | 一次性 sudoers 免密 | **无需提权**（标准用户即可） |
| `dns_flush_granted()` | 探测 sudoers 规则 | 恒 `true` |
| `grant_dns_flush_access()` | 安装 sudoers | 空操作 `Ok(())` |

前端 flush 流程不变（先查 granted → 必要时 grant → flush），Windows 下 granted 恒 true 故直接 flush、零弹窗。

### 4.3 Shell 配置

**macOS（不变）：** `zshrc` / `zprofile` / `zshenv` / `zlogin`，路径 `~/.<name>`，reload `source ~/.<name>`。

**Windows（新增，覆盖 PowerShell 与 cmd 两个 shell）：**

| name | label | path | reload_hint | 写后钩子 |
|---|---|---|---|---|
| `powershell` | `PowerShell` | `%USERPROFILE%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1` | `. $PROFILE` | 无 |
| `cmd` | `命令提示符 (cmd)` | `%USERPROFILE%\.hostguard.cmd_autorun.cmd` | `重启 cmd 生效` | `reg add "HKCU\Software\Microsoft\Command Processor" /v AutoRun /t REG_SZ /d "<path>" /f`（HKCU，无需提权） |

- PowerShell：直接读写 profile.ps1（用户文件，无需提权；目录不存在则创建）。
- cmd：HostGuard 托管一个 AutoRun 批处理文件；保存时写文件并经 `reg.exe` 确保 HKCU `AutoRun` 指向它（每次 cmd 启动自动执行）。读取时文件不存在则返回空串。
- 保存前均备份到 `%USERPROFILE%\.hostguard.<name>.bak`。

### 4.4 系统监控（`sysinfo`）

基本无改动。已知平台差异：**Windows 无 load average**（`System::load_average()` 返回 0/0/0）。前端在 Windows 隐藏「平均负载」卡或显示 “N/A”（由后端在 stats 中带 `os` 标记，前端据此判断）。

## 5. 提权模型小结（两端都"授权一次"）

- **Hosts**：macOS `chmod +a` ACL ≈ Windows `icacls` NTFS ACL —— 都是一次管理员/UAC 授权后直接写，不再弹窗。
- **DNS**：macOS 一次性 sudoers；Windows 完全无需提权。
- **Shell**：两端都是用户文件 / HKCU，无需提权。

## 6. 前端改动（最小必要）

- `src/views/shell/index.vue`：移除硬编码 `["zshrc",…]`、`~/.${name}`、`source …`、`.{name}` 标签；改用后端 `ShellConfig` 的 `label` / `path` / `reload_hint`；默认选中列表首项而非写死 `"zshrc"`。
- `src/views/hosts/index.vue`：把写死的 “/etc/hosts” 文案（保存成功 / 读取失败 / 权限横幅）改为后端返回路径。**具体做法**：新增轻量命令 `hosts_path() -> String`，Hosts 页 `onMounted` 取一次并缓存到 ref，文案统一引用之。
- `src/views/monitor/index.vue`：Windows 下隐藏「平均负载」卡（依据 stats 的 `os` 字段）。
- 文案类（settings/system 占位页的 “macOS …” 字样）择机调整，非阻塞。

## 7. 依赖

Windows 实现**不新增 Rust crate**：全部经 `std::process::Command` 调用系统自带 `icacls` / `ipconfig` / `reg.exe` / `powershell`。`tauri` 的 `macos-private-api` feature 在 Windows 构建中无副作用（保留）。

## 8. CI（GitHub Actions：macOS + Windows）

新增 `.github/workflows/ci.yml`，两个 job（`macos-latest`、`windows-latest`），步骤一致：

1. checkout
2. setup Node + pnpm，`pnpm install`
3. `pnpm build`（vite build + vue-tsc，产出 `dist/` 供 tauri 编译期嵌入）
4. `pnpm lint`、`pnpm format:check`
5. `cargo build --manifest-path src-tauri/Cargo.toml`（Windows job 即验证 `#[cfg(windows)]` 分支编译通过）

> Windows runner 自带 WebView2 与 MSVC 工具链，tauri 可直接编译。

## 9. 验证策略

- **macOS**：`cargo check` + `cargo build` + `pnpm typecheck/lint/format` + 现有功能回归（本机可验证）。
- **Windows**：代码 `#[cfg(windows)]` 写好力求正确；本机（Mac）无法运行 Windows 目标，**靠 CI 的 windows-latest job 保证编译通过**；运行期行为待你在 Windows 机器上实测（hosts 写入、UAC、DNS 刷新、PowerShell/cmd 配置）。

## 10. 风险与缓解

- **Windows hosts 受 AV/Defender 保护**：部分安全软件即便 ACL 允许仍拦截 hosts 写入 → 写失败时返回清晰错误，提示用户检查安全软件。
- **cmd AutoRun 较冷门 / 可能与用户已有 AutoRun 冲突**：HostGuard 仅托管自有批处理并覆盖 HKCU AutoRun；若检测到已有不同 AutoRun，保存前提示（可作为后续增强，首版直接覆盖并在 UI 注明）。
- **本机无法运行期验证 Windows**：以 CI 编译 + 代码审查兜底，运行期由用户在 Windows 实测。
- **load average 在 Windows 为 0**：前端隐藏该卡，避免误导。

## 11. 交付清单

- [ ] `platform/{mod,macos,windows}.rs` 落地，`lib.rs` 改为薄封装
- [ ] `ShellConfig` 扩展字段 + 共享读写逻辑迁入 `mod.rs`
- [ ] Windows 实现：hosts(icacls)、dns(ipconfig)、shell(PowerShell + cmd/reg)
- [ ] 前端 shell 页数据驱动化 + hosts 文案去硬编码 + monitor 负载卡按平台隐藏
- [ ] `.github/workflows/ci.yml`（macOS + Windows）
- [ ] macOS 本机回归验证通过
