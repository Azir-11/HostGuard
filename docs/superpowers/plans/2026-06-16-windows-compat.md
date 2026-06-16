# Windows 兼容层 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 HostGuard 后端按平台抽象为一层 `platform` 模块，迁入现有 macOS 实现并新增 Windows 实现，配最小前端改动与双平台 CI 编译验证。

**Architecture:** `#[cfg]` 编译期选择平台模块（方案 A）。`src-tauri/src/platform/{mod,macos,windows}.rs`：`mod.rs` 放共享类型与共享读写逻辑并按 `cfg` 选择 `macos`/`windows` 为 `sys`；`lib.rs` 的 `#[tauri::command]` 变薄封装转调 `platform::*`。`sysinfo` 监控本就跨平台，留在 `lib.rs`。

**Tech Stack:** Rust / Tauri 2 / sysinfo；Vue 3.5 / UnoCSS；GitHub Actions。Windows 实现零新增 crate，全用系统自带 `icacls`/`ipconfig`/`reg`/`powershell`。

**测试策略说明（重要）：** 本仓库无单元测试框架，质量门是 `cargo build`/`cargo check` + `pnpm typecheck`/`lint`/`format:check` + macOS 手动回归，Windows 侧由 CI `cargo build` 编译验证 + 用户在 Windows 实测。本计划以这些**真实存在的门**作为每步的“验证”，不引入新测试框架（YAGNI，遵循现有工程化约定）。

参考 spec：`docs/superpowers/specs/2026-06-16-windows-compat-design.md`

---

## File Structure

```
src-tauri/src/
  lib.rs              # 改：命令变薄封装；保留 sysinfo（SystemStats 增 os 字段）；新增 hosts_path 命令
  platform/
    mod.rs            # 新：ShellConfig 类型 + 共享 read/write 逻辑 + cfg 选择 sys + 再导出平台项
    macos.rs          # 新：#[cfg(target_os="macos")] 现有 macOS 逻辑迁入
    windows.rs        # 新：#[cfg(target_os="windows")] 新 Windows 逻辑
src/views/
  shell/index.vue     # 改：数据驱动（label/path/reload_hint），去硬编码 zsh/~/./source
  hosts/index.vue     # 改：文案用后端 hosts_path
  monitor/index.vue   # 改：os==="windows" 隐藏「平均负载」卡
.github/workflows/
  ci.yml              # 新：macOS + Windows 双 job 编译验证
TASKS.md              # 改：勾选阶段 6 跨平台项
```

---

## Task 1: 后端平台模块 + macOS 迁移 + lib.rs 薄封装

**Files:**

- Create: `src-tauri/src/platform/mod.rs`
- Create: `src-tauri/src/platform/macos.rs`
- Modify: `src-tauri/src/lib.rs`（整体重写命令层；保留 sysinfo 段并增 `os` 字段；新增 `hosts_path` 命令）

- [ ] **Step 1: 创建 `src-tauri/src/platform/mod.rs`**

```rust
use std::path::PathBuf;

/// 一个 shell 启动配置文件的描述（前端据此渲染，无需知道平台细节）。
#[derive(serde::Serialize)]
pub struct ShellConfig {
    /// 稳定标识，作命令参数（如 "zshrc" / "powershell" / "cmd"）。
    pub name: String,
    /// 展示名（如 ".zshrc" / "PowerShell" / "命令提示符 (cmd)"）。
    pub label: String,
    /// 完整路径（展示用）。
    pub path: String,
    pub exists: bool,
    /// 重载提示命令（如 "source ~/.zshrc" / ". $PROFILE"）。
    pub reload_hint: String,
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use self::macos as sys;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows as sys;

// 直接由命令调用的“平台专属”入口，原样再导出。
pub use sys::{
    dns_flush_granted, flush_dns, grant_dns_flush_access, grant_hosts_access, hosts_writable,
    shell_configs, shell_path,
};

/// 当前平台的 hosts 路径（字符串，供前端文案）。
pub fn hosts_path_string() -> String {
    sys::hosts_path().to_string_lossy().to_string()
}

/// 读取 hosts（全平台可读，无需提权）。
pub fn read_hosts() -> Result<String, String> {
    std::fs::read_to_string(sys::hosts_path()).map_err(|e| e.to_string())
}

/// 写 hosts：无写权限返回 NO_PERMISSION；写前备份到家目录。
pub fn write_hosts(content: String) -> Result<(), String> {
    if !sys::hosts_writable() {
        return Err("NO_PERMISSION".into());
    }
    if let (Ok(current), Some(bak)) = (
        std::fs::read_to_string(sys::hosts_path()),
        sys::backup_path(".hostguard.hosts.bak"),
    ) {
        let _ = std::fs::write(bak, current);
    }
    std::fs::write(sys::hosts_path(), content).map_err(|e| e.to_string())
}

/// 读 shell 配置；不存在返回空串。
pub fn read_shell_config(name: String) -> Result<String, String> {
    let path = sys::shell_path(&name).ok_or("无效的配置文件")?;
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// 写 shell 配置：先建父目录（PowerShell profile 目录可能不存在），写前备份，
/// 写后调用平台钩子（Windows+cmd 设置注册表 AutoRun；其余 noop）。
pub fn write_shell_config(name: String, content: String) -> Result<(), String> {
    let path = sys::shell_path(&name).ok_or("无效的配置文件")?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let (Ok(current), Some(bak)) = (
        std::fs::read_to_string(&path),
        sys::backup_path(&format!(".hostguard.{name}.bak")),
    ) {
        let _ = std::fs::write(bak, current);
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    sys::on_shell_saved(&name, &path)
}
```

- [ ] **Step 2: 创建 `src-tauri/src/platform/macos.rs`**（迁入现有逻辑，签名对齐 mod.rs 调用）

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::platform::ShellConfig;

const HOSTS_PATH: &str = "/etc/hosts";
const SHELL_FILES: [&str; 4] = ["zshrc", "zprofile", "zshenv", "zlogin"];
const DNS_SUDOERS_PATH: &str = "/etc/sudoers.d/hostguard-dns";
const DSCACHEUTIL: &str = "/usr/bin/dscacheutil";
const KILLALL: &str = "/usr/bin/killall";

fn current_user() -> String {
    std::env::var("USER").unwrap_or_default()
}
fn home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// 备份目录（家目录）下的 .hostguard.* 文件路径。
pub fn backup_path(file_name: &str) -> Option<PathBuf> {
    home().map(|h| h.join(file_name))
}

// ───── Hosts ─────
pub fn hosts_path() -> PathBuf {
    PathBuf::from(HOSTS_PATH)
}
pub fn hosts_writable() -> bool {
    std::fs::OpenOptions::new().write(true).open(HOSTS_PATH).is_ok()
}
pub async fn grant_hosts_access() -> Result<(), String> {
    let user = current_user();
    if user.is_empty() {
        return Err("无法获取当前用户".into());
    }
    let script = format!(
        "do shell script \"/bin/chmod +a '{} allow read,write,append,readattr,writeattr,readextattr,writeextattr' /etc/hosts\" with administrator privileges",
        user
    );
    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("osascript").args(["-e", &script]).output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.contains("-128") {
            Err("已取消".into())
        } else {
            Err(err)
        }
    }
}

// ───── DNS ─────
fn dns_sudoers_line(user: &str) -> String {
    format!("{user} ALL=(root) NOPASSWD: {DSCACHEUTIL} -flushcache, {KILLALL} -HUP mDNSResponder\n")
}
pub async fn dns_flush_granted() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        let allowed = |args: &[&str]| {
            Command::new("sudo")
                .arg("-n")
                .arg("-l")
                .args(args)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        allowed(&[DSCACHEUTIL, "-flushcache"]) && allowed(&[KILLALL, "-HUP", "mDNSResponder"])
    })
    .await
    .unwrap_or(false)
}
pub async fn grant_dns_flush_access() -> Result<(), String> {
    let user = current_user();
    if user.is_empty() {
        return Err("无法获取当前用户".into());
    }
    let tmp = std::env::temp_dir().join("hostguard-dns");
    std::fs::write(&tmp, dns_sudoers_line(&user)).map_err(|e| e.to_string())?;
    let tmp_str = tmp.to_string_lossy().to_string();
    let script = format!(
        "do shell script \"/usr/sbin/visudo -cf '{t}' && /usr/bin/install -m 0440 -o root -g wheel '{t}' '{dst}'\" with administrator privileges",
        t = tmp_str,
        dst = DNS_SUDOERS_PATH
    );
    let result = tauri::async_runtime::spawn_blocking(move || {
        Command::new("osascript").args(["-e", &script]).output()
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    let output = result.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.contains("-128") {
            Err("已取消".into())
        } else {
            Err(err)
        }
    }
}
pub async fn flush_dns() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let run = |args: &[&str]| -> Result<(), String> {
            let out = Command::new("sudo")
                .arg("-n")
                .args(args)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                return Ok(());
            }
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if err.contains("password is required") || err.contains("terminal is required") {
                Err("NO_PERMISSION".into())
            } else if err.is_empty() {
                Err("刷新失败".into())
            } else {
                Err(err)
            }
        };
        run(&[DSCACHEUTIL, "-flushcache"])?;
        run(&[KILLALL, "-HUP", "mDNSResponder"])?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ───── Shell ─────
pub fn shell_configs() -> Vec<ShellConfig> {
    SHELL_FILES
        .iter()
        .map(|name| {
            let path = shell_path(name).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let exists = shell_path(name).map(|p| p.exists()).unwrap_or(false);
            ShellConfig {
                name: (*name).to_string(),
                label: format!(".{name}"),
                path,
                exists,
                reload_hint: format!("source ~/.{name}"),
            }
        })
        .collect()
}
pub fn shell_path(name: &str) -> Option<PathBuf> {
    if !SHELL_FILES.contains(&name) {
        return None;
    }
    home().map(|h| h.join(format!(".{name}")))
}
pub fn on_shell_saved(_name: &str, _path: &Path) -> Result<(), String> {
    Ok(())
}
```

- [ ] **Step 3: 重写 `src-tauri/src/lib.rs`**（命令薄封装 + sysinfo 段增 `os`）

```rust
mod platform;

use std::sync::Mutex;

use sysinfo::System;

// ───────────────────────── Hosts ─────────────────────────
#[tauri::command]
fn hosts_writable() -> bool {
    platform::hosts_writable()
}

#[tauri::command]
fn hosts_path() -> String {
    platform::hosts_path_string()
}

#[tauri::command]
fn read_hosts() -> Result<String, String> {
    platform::read_hosts()
}

#[tauri::command]
async fn grant_hosts_access() -> Result<(), String> {
    platform::grant_hosts_access().await
}

#[tauri::command]
fn write_hosts(content: String) -> Result<(), String> {
    platform::write_hosts(content)
}

// ───────────────────────── Shell ─────────────────────────
#[tauri::command]
fn list_shell_configs() -> Vec<platform::ShellConfig> {
    platform::shell_configs()
}

#[tauri::command]
fn read_shell_config(name: String) -> Result<String, String> {
    platform::read_shell_config(name)
}

#[tauri::command]
fn write_shell_config(name: String, content: String) -> Result<(), String> {
    platform::write_shell_config(name, content)
}

// ───────────────────────── DNS cache flush ─────────────────────────
#[tauri::command]
async fn dns_flush_granted() -> bool {
    platform::dns_flush_granted().await
}

#[tauri::command]
async fn grant_dns_flush_access() -> Result<(), String> {
    platform::grant_dns_flush_access().await
}

#[tauri::command]
async fn flush_dns_cache() -> Result<(), String> {
    platform::flush_dns().await
}

// ───────────────────────── System telemetry ─────────────────────────
#[derive(serde::Serialize)]
struct SystemStats {
    os: String,
    cpu_usage: f32,
    per_core: Vec<f32>,
    mem_total: u64,
    mem_used: u64,
    mem_available: u64,
    swap_total: u64,
    swap_used: u64,
    load_one: f64,
    load_five: f64,
    load_fifteen: f64,
    uptime: u64,
    core_count: usize,
    cpu_brand: String,
    host_name: String,
}

#[tauri::command]
fn read_system_stats(state: tauri::State<'_, Mutex<System>>) -> Result<SystemStats, String> {
    let mut sys = state.lock().map_err(|e| e.to_string())?;
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let load = System::load_average();
    Ok(SystemStats {
        os: std::env::consts::OS.to_string(),
        cpu_usage: sys.global_cpu_usage(),
        per_core: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
        mem_total: sys.total_memory(),
        mem_used: sys.used_memory(),
        mem_available: sys.available_memory(),
        swap_total: sys.total_swap(),
        swap_used: sys.used_swap(),
        load_one: load.one,
        load_five: load.five,
        load_fifteen: load.fifteen,
        uptime: System::uptime(),
        core_count: sys.cpus().len(),
        cpu_brand: sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default(),
        host_name: System::host_name().unwrap_or_default(),
    })
}

fn init_system() -> System {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(init_system()))
        .invoke_handler(tauri::generate_handler![
            hosts_writable,
            hosts_path,
            read_hosts,
            grant_hosts_access,
            write_hosts,
            list_shell_configs,
            read_shell_config,
            write_shell_config,
            dns_flush_granted,
            grant_dns_flush_access,
            flush_dns_cache,
            read_system_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: 编译验证（macOS）**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: `Finished`，无 error/warning（尤其无 unused import）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/platform/mod.rs src-tauri/src/platform/macos.rs src-tauri/src/lib.rs
git commit -m "refactor: 后端按平台抽象为 platform 模块（macOS 迁入）"
```

---

## Task 2: Windows 实现（windows.rs）

**Files:**

- Create: `src-tauri/src/platform/windows.rs`

- [ ] **Step 1: 创建 `src-tauri/src/platform/windows.rs`**

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::platform::ShellConfig;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
}

/// 备份目录（%USERPROFILE%）下的 .hostguard.* 文件路径。
pub fn backup_path(file_name: &str) -> Option<PathBuf> {
    let h = home();
    if h.as_os_str().is_empty() {
        None
    } else {
        Some(h.join(file_name))
    }
}

// ───── Hosts ─────
pub fn hosts_path() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(root).join("System32\\drivers\\etc\\hosts")
}
pub fn hosts_writable() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open(hosts_path())
        .is_ok()
}
/// 一次性授权：经 UAC 提升运行 icacls，把 Modify 权限授予当前用户；
/// 之后用户即可直接写 hosts，无需再提权（对标 macOS 的 chmod +a）。
pub async fn grant_hosts_access() -> Result<(), String> {
    let user = std::env::var("USERNAME").unwrap_or_default();
    if user.is_empty() {
        return Err("无法获取当前用户".into());
    }
    let hosts = hosts_path().to_string_lossy().to_string();
    // Start-Process -Verb RunAs 触发 UAC；-Wait 阻塞；用 $p.ExitCode 透传结果。
    let ps = format!(
        "$ErrorActionPreference='Stop'; try {{ $p = Start-Process icacls -ArgumentList '\"{hosts}\"','/grant','\"{user}:(M)\"' -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $p.ExitCode }} catch {{ Write-Error 'UAC_CANCELLED'; exit 1 }}",
        hosts = hosts,
        user = user
    );
    let out = tauri::async_runtime::spawn_blocking(move || {
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if err.contains("UAC_CANCELLED") || err.to_lowercase().contains("cancel") {
            Err("已取消".into())
        } else if err.is_empty() {
            Err("授权失败（可能已取消 UAC）".into())
        } else {
            Err(err)
        }
    }
}

// ───── DNS ─────（ipconfig /flushdns 无需提权）
pub async fn dns_flush_granted() -> bool {
    true
}
pub async fn grant_dns_flush_access() -> Result<(), String> {
    Ok(())
}
pub async fn flush_dns() -> Result<(), String> {
    let out = tauri::async_runtime::spawn_blocking(|| {
        Command::new("ipconfig").arg("/flushdns").output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { "刷新失败".into() } else { err })
    }
}

// ───── Shell ─────（PowerShell $PROFILE + cmd AutoRun）
fn ps_profile_path() -> PathBuf {
    home().join("Documents\\PowerShell\\Microsoft.PowerShell_profile.ps1")
}
fn cmd_autorun_path() -> PathBuf {
    home().join(".hostguard.cmd_autorun.cmd")
}
pub fn shell_configs() -> Vec<ShellConfig> {
    let ps = ps_profile_path();
    let cmd = cmd_autorun_path();
    vec![
        ShellConfig {
            name: "powershell".to_string(),
            label: "PowerShell".to_string(),
            path: ps.to_string_lossy().to_string(),
            exists: ps.exists(),
            reload_hint: ". $PROFILE".to_string(),
        },
        ShellConfig {
            name: "cmd".to_string(),
            label: "命令提示符 (cmd)".to_string(),
            path: cmd.to_string_lossy().to_string(),
            exists: cmd.exists(),
            reload_hint: "重启 cmd 生效".to_string(),
        },
    ]
}
pub fn shell_path(name: &str) -> Option<PathBuf> {
    match name {
        "powershell" => Some(ps_profile_path()),
        "cmd" => Some(cmd_autorun_path()),
        _ => None,
    }
}
/// 写后钩子：cmd 配置保存后，把 HKCU 的 AutoRun 指向该批处理（无需提权）。
pub fn on_shell_saved(name: &str, path: &Path) -> Result<(), String> {
    if name != "cmd" {
        return Ok(());
    }
    let val = path.to_string_lossy().to_string();
    let out = Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Command Processor",
            "/v",
            "AutoRun",
            "/t",
            "REG_SZ",
            "/d",
            &val,
            "/f",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            "设置 cmd AutoRun 失败".into()
        } else {
            err
        })
    }
}
```

- [ ] **Step 2: macOS 构建不受影响（windows.rs 被 cfg 排除）**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: `Finished`（macOS 下 `windows.rs` 因 `#[cfg(target_os="windows")]` 不参与编译）。

- [ ] **Step 3: 尽力做本地 Windows 交叉类型检查（非阻塞）**

Run:

```bash
rustup target add x86_64-pc-windows-msvc
cargo check --target x86_64-pc-windows-msvc --manifest-path src-tauri/Cargo.toml
```

Expected: 理想为 `Finished`（`cargo check` 不链接，多数情况可在 macOS 上类型检查 Windows 目标）。若因宿主缺少 MSVC 相关 build 依赖而失败，**跳过本步**，以 Task 4 的 CI 为权威验证。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/platform/windows.rs
git commit -m "feat: platform 模块新增 Windows 实现（icacls/ipconfig/PowerShell+cmd）"
```

---

## Task 3: 前端去硬编码（数据驱动）

**Files:**

- Modify: `src/views/shell/index.vue`（整体重写 script + 模板相关处）
- Modify: `src/views/hosts/index.vue`（文案改用后端 hosts_path）
- Modify: `src/views/monitor/index.vue`（os==="windows" 隐藏负载卡）

- [ ] **Step 1: 重写 `src/views/shell/index.vue`**

```vue
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useDialog, useMessage } from "naive-ui";
import CodeEditor from "@/components/CodeEditor.vue";
import { useAppStore } from "@/store/app";
import { useFabStore } from "@/store/fab";

interface ShellConfig {
  name: string;
  label: string;
  path: string;
  exists: boolean;
  reload_hint: string;
}

const message = useMessage();
const dialog = useDialog();
const appStore = useAppStore();
const fab = useFabStore();

const files = ref<ShellConfig[]>([]);
const active = ref("");
const content = ref("");
const original = ref("");

const dirty = computed(() => content.value !== original.value);
const activeConfig = computed(() => files.value.find((f) => f.name === active.value));
const activePath = computed(() => activeConfig.value?.path ?? "");
const reloadHint = computed(() => activeConfig.value?.reload_hint ?? "");

async function refreshList() {
  try {
    files.value = await invoke<ShellConfig[]>("list_shell_configs");
  } catch {
    /* not in Tauri */
  }
}

async function loadFile(name: string) {
  try {
    const text = await invoke<string>("read_shell_config", { name });
    content.value = text;
    original.value = text;
    active.value = name;
  } catch (e) {
    message.error(`读取失败：${String(e)}`);
  }
}

function switchFile(name: string) {
  if (name === active.value) return;
  if (dirty.value) {
    dialog.warning({
      title: "未保存的更改",
      content: `「${activeConfig.value?.label ?? active.value}」有未保存的更改，切换将丢弃，确定继续？`,
      positiveText: "丢弃并切换",
      negativeText: "取消",
      onPositiveClick: () => loadFile(name),
    });
  } else {
    loadFile(name);
  }
}

async function save(): Promise<boolean> {
  try {
    await invoke("write_shell_config", { name: active.value, content: content.value });
    original.value = content.value;
    await refreshList();
    message.success(`已保存 ${activePath.value}`);
    return true;
  } catch (e) {
    message.error(`保存失败：${String(e)}`);
    return false;
  }
}

async function copySource() {
  try {
    await navigator.clipboard.writeText(reloadHint.value);
    message.success("已复制命令");
  } catch {
    message.info(reloadHint.value);
  }
}

onMounted(async () => {
  await refreshList();
  const first = files.value[0]?.name;
  if (first) await loadFile(first);
  fab.set({ label: "保存", icon: "i-ph-floppy-disk-duotone", run: save });
});
onUnmounted(() => fab.clear());
</script>

<template>
  <div class="h-full flex flex-col gap-14px">
    <!-- file tabs + toolbar -->
    <div class="flex items-center gap-12px">
      <div class="flex gap-4px p-4px rounded-md border border-hairline bg-[var(--c-bg-0)]">
        <button
          v-for="f in files"
          :key="f.name"
          class="inline-flex items-center gap-6px px-12px py-7px rounded-sm border-none cursor-pointer text-13px font-mono transition-colors"
          :class="
            active === f.name
              ? 'bg-grad-accent !text-[#04130b] font-600'
              : 'bg-transparent text-fg-2 hover:text-fg'
          "
          @click="switchFile(f.name)"
        >
          {{ f.label }}
          <span v-if="!f.exists" class="w-5px h-5px rounded-full bg-fg-3" title="尚未创建" />
        </button>
      </div>
      <div class="flex-1" />
      <span v-if="dirty" class="inline-flex items-center gap-6px text-12px text-amber">
        <span class="w-7px h-7px rounded-full bg-amber" /> 未保存
      </span>
      <NButton quaternary @click="loadFile(active)">
        <template #icon><span class="i-ph-arrow-clockwise-bold" /></template>
        重载
      </NButton>
    </div>

    <!-- reload hint -->
    <div class="flex items-center gap-10px px-14px py-9px card text-12px text-fg-2">
      <span class="i-ph-info-duotone text-16px text-accent-2 shrink-0" />
      <span class="flex-1">
        保存后需在终端执行
        <code class="font-mono text-fg">{{ reloadHint }}</code>
        或重启终端生效。
      </span>
      <button
        class="inline-flex items-center gap-5px border-none bg-transparent cursor-pointer text-12px text-accent-2 hover:text-accent"
        @click="copySource"
      >
        <span class="i-ph-copy-bold" /> 复制
      </button>
    </div>

    <!-- editor -->
    <div class="flex-1 min-h-0 card overflow-hidden">
      <CodeEditor v-model="content" :dark="appStore.isDark" />
    </div>
  </div>
</template>
```

- [ ] **Step 2: `src/views/hosts/index.vue` 文案改用后端 hosts_path**

在 `<script setup>` 中，`const writable = ref(false);` 附近新增：

```ts
const hostsPath = ref("/etc/hosts");
```

在 `onMounted` 里，把：

```ts
onMounted(async () => {
  await Promise.all([load(), checkPerm()]);
  syncFab();
});
```

改为（追加取 hosts 路径）：

```ts
onMounted(async () => {
  try {
    hostsPath.value = await invoke<string>("hosts_path");
  } catch {
    /* keep default */
  }
  await Promise.all([load(), checkPerm()]);
  syncFab();
});
```

把 `save()` 内成功提示：

```ts
message.success("已保存到 /etc/hosts");
```

改为：

```ts
message.success(`已保存到 ${hostsPath.value}`);
```

模板里权限横幅文案：

```html
修改 /etc/hosts 需一次性管理员授权，授权后即可直接保存，无需重复输入密码。
```

改为：

```html
修改 {{ hostsPath }} 需一次性管理员授权，授权后即可直接保存，无需重复输入密码。
```

模板里错误文案：

```html
<p class="m-0 max-w-420px text-fg-2 leading-[1.6]">读取 /etc/hosts 失败：{{ error }}</p>
```

改为：

```html
<p class="m-0 max-w-420px text-fg-2 leading-[1.6]">读取 {{ hostsPath }} 失败：{{ error }}</p>
```

- [ ] **Step 3: `src/views/monitor/index.vue` 在 Windows 隐藏负载卡**

在 `interface SystemStats` 顶部新增字段：

```ts
interface SystemStats {
  os: string;
  cpu_usage: number;
  // …其余不变
}
```

把「平均负载」`<article>` 的根标签加上条件渲染（Windows 无 load average）：找到

```html
<!-- load + uptime -->
<article class="card p-20px flex flex-col gap-14px"></article>
```

改为：

```html
<!-- load + uptime -->
<article v-if="stats.os !== 'windows'" class="card p-20px flex flex-col gap-14px"></article>
```

> 注：该卡同时承载「运行时长 / 主机名」。Windows 下负载无意义而隐藏整卡可接受；如需保留运行时长可作后续增强，本次从简。

- [ ] **Step 4: 前端门禁校验**

Run:

```bash
pnpm typecheck && pnpm lint && pnpm format:check
```

Expected: 全部 exit 0。若 `format:check` 报错，执行 `pnpm format` 后重跑。

- [ ] **Step 5: 提交**

```bash
git add src/views/shell/index.vue src/views/hosts/index.vue src/views/monitor/index.vue
git commit -m "feat: 前端去平台硬编码（shell 数据驱动 / hosts 路径 / Windows 隐藏负载卡）"
```

---

## Task 4: CI（GitHub Actions：macOS + Windows）

**Files:**

- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: 创建 `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    name: build (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 10

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Install JS deps
        run: pnpm install --frozen-lockfile

      - name: Frontend build (vite + vue-tsc)
        run: pnpm build

      - name: Frontend lint & format
        run: pnpm lint && pnpm format:check

      - name: Rust build
        run: cargo build --manifest-path src-tauri/Cargo.toml
```

> `pnpm build` 先产出 `dist/`，供 `tauri::generate_context!` 在 `cargo build` 时嵌入。`windows-latest` runner 自带 WebView2 + MSVC，无需额外系统依赖。

- [ ] **Step 2: 提交并推送，确认 CI 通过**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: 新增 macOS + Windows 双平台编译验证"
git push
```

Expected: GitHub Actions 两个 job 均绿。**Windows job 通过 = Windows 侧 `#[cfg(windows)]` 代码编译验证通过**。若 Windows job 报编译错误，按报错修 `windows.rs`，重复直至通过。

---

## Task 5: macOS 回归 + 文档收尾

**Files:**

- Modify: `TASKS.md`

- [ ] **Step 1: macOS 全链路回归**

Run: `pnpm tauri dev`（或 `pnpm app:build` 产出后自行安装验证）
手动核对：

- Hosts：读取、增删改、授予权限、保存、刷新 DNS 缓存 均正常。
- Shell：四个 zsh 文件切换 / 编辑 / 保存 / 重载提示正常（label 显示 `.zshrc` 等）。
- 系统用量：CPU/内存/负载实时更新（macOS 显示负载卡）。

- [ ] **Step 2: 更新 `TASKS.md` 阶段 6**

把：

```markdown
- [ ] Rust 命令统一封装 + 前端 `invoke` 类型安全封装
- [ ] 提权 / 权限统一处理（Hosts、系统设置）
```

改为：

```markdown
- [x] Rust 命令按平台抽象为 `platform` 模块（`#[cfg]` macOS/Windows），命令层薄封装
- [x] 提权 / 权限统一处理：macOS（chmod +a ACL / sudoers）与 Windows（icacls 一次性 / ipconfig 免提权）对齐“授权一次”
- [ ] Windows 运行期实测（hosts 写入 / UAC / DNS / PowerShell+cmd 配置）
- [x] CI：macOS + Windows 双平台编译验证（GitHub Actions）
```

- [ ] **Step 3: 提交**

```bash
git add TASKS.md
git commit -m "docs: TASKS 勾选跨平台抽象与 CI"
```

---

## Self-Review

**Spec coverage：**

- 架构 cfg 平台模块 → Task 1 ✓
- Hosts/DNS/Shell Windows 映射 → Task 2 ✓
- 系统监控 os 字段 + Windows 隐藏负载 → Task 1（os 字段）/ Task 3 Step 3 ✓
- 前端去硬编码（shell/hosts）→ Task 3 ✓
- 不新增 crate → Task 2 全部 std::process::Command ✓
- CI 双平台 → Task 4 ✓
- 窗口层范围外 → 未含任务（正确）✓

**类型一致性：** `ShellConfig{name,label,path,exists,reload_hint}` 在 mod.rs 定义、macos.rs/windows.rs 构造、前端 interface 三处字段一致 ✓。命令名与前端 `invoke(...)` 字符串一致（含新增 `hosts_path`）✓。平台再导出项 `dns_flush_granted/flush_dns/grant_dns_flush_access/grant_hosts_access/hosts_writable/shell_configs/shell_path` 在 macos.rs 与 windows.rs 均提供，签名（async/sync）一致 ✓。

**占位符扫描：** 无 TBD/TODO 式占位；每步含完整代码或确切命令 ✓。
