use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use sysinfo::System;

const HOSTS_PATH: &str = "/etc/hosts";

fn current_user() -> String {
    std::env::var("USER").unwrap_or_default()
}

/// Whether the current user can write the hosts file without elevation.
#[tauri::command]
fn hosts_writable() -> bool {
    fs::OpenOptions::new().write(true).open(HOSTS_PATH).is_ok()
}

/// Read the system hosts file (world-readable, no elevation needed).
#[tauri::command]
fn read_hosts() -> Result<String, String> {
    fs::read_to_string(HOSTS_PATH).map_err(|e| e.to_string())
}

/// One-time permission grant: add an ACL entry giving the current user write
/// access to /etc/hosts (ownership stays root:wheel). This shows a single
/// native admin prompt; afterwards `write_hosts` writes directly with no
/// further prompts. Async + spawn_blocking so the modal dialog never blocks
/// the UI thread (which previously triggered macOS "not responding").
#[tauri::command]
async fn grant_hosts_access() -> Result<(), String> {
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
        // osascript reports a cancelled auth dialog as error -128.
        if err.contains("-128") {
            Err("已取消".into())
        } else {
            Err(err)
        }
    }
}

/// Write the hosts file directly. Requires the one-time grant above; if the
/// file is not writable, returns NO_PERMISSION so the UI can prompt for it.
/// A backup of the previous content is kept in the user's home directory.
#[tauri::command]
fn write_hosts(content: String) -> Result<(), String> {
    if !hosts_writable() {
        return Err("NO_PERMISSION".into());
    }

    if let (Ok(current), Ok(home)) = (fs::read_to_string(HOSTS_PATH), std::env::var("HOME")) {
        let _ = fs::write(format!("{home}/.hostguard.hosts.bak"), current);
    }

    fs::write(HOSTS_PATH, content).map_err(|e| e.to_string())
}

// ───────────────────────── Shell / zsh config ─────────────────────────

const SHELL_FILES: [&str; 4] = ["zshrc", "zprofile", "zshenv", "zlogin"];

#[derive(serde::Serialize)]
struct ShellConfig {
    name: String,
    exists: bool,
}

fn shell_path(name: &str) -> Option<PathBuf> {
    if !SHELL_FILES.contains(&name) {
        return None;
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(format!(".{name}")))
}

/// List the known zsh config files and whether each currently exists.
#[tauri::command]
fn list_shell_configs() -> Vec<ShellConfig> {
    SHELL_FILES
        .iter()
        .map(|name| ShellConfig {
            name: (*name).to_string(),
            exists: shell_path(name).map(|p| p.exists()).unwrap_or(false),
        })
        .collect()
}

/// Read a zsh config file; returns "" if it does not exist yet.
#[tauri::command]
fn read_shell_config(name: String) -> Result<String, String> {
    let path = shell_path(&name).ok_or("无效的配置文件")?;
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Write a zsh config file (user-owned home file, no elevation needed). The
/// previous content is backed up to ~/.hostguard.<name>.bak.
#[tauri::command]
fn write_shell_config(name: String, content: String) -> Result<(), String> {
    let path = shell_path(&name).ok_or("无效的配置文件")?;
    if let (Ok(current), Ok(home)) = (fs::read_to_string(&path), std::env::var("HOME")) {
        let _ = fs::write(format!("{home}/.hostguard.{name}.bak"), current);
    }
    fs::write(&path, content).map_err(|e| e.to_string())
}

// ───────────────────────── DNS cache flush ─────────────────────────
//
// Fully flushing DNS on modern macOS needs root: `killall -HUP mDNSResponder`
// signals a root-owned process. Rather than prompt on every flush, we install
// a one-time, tightly-scoped sudoers drop-in (single admin prompt) that lets
// the current user run ONLY these two exact commands without a password.
// Afterwards `flush_dns_cache` runs them via `sudo -n` with no prompt, and the
// grant persists across app restarts.

const DNS_SUDOERS_PATH: &str = "/etc/sudoers.d/hostguard-dns";
const DSCACHEUTIL: &str = "/usr/bin/dscacheutil";
const KILLALL: &str = "/usr/bin/killall";

fn dns_sudoers_line(user: &str) -> String {
    format!("{user} ALL=(root) NOPASSWD: {DSCACHEUTIL} -flushcache, {KILLALL} -HUP mDNSResponder\n")
}

/// Whether the one-time DNS-flush authorization is already in place — i.e. the
/// current user may run both flush commands via `sudo` with no password. Probed
/// with `sudo -n -l <cmd>`, which lists the permission without executing it.
#[tauri::command]
async fn dns_flush_granted() -> bool {
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

/// One-time grant: install the sudoers drop-in. The file content is written
/// here (fully controlled), then validated with `visudo -cf` and installed as
/// root:wheel 0440 in a single admin prompt. Validating first means a malformed
/// rule can never reach /etc/sudoers.d and break sudo system-wide.
#[tauri::command]
async fn grant_dns_flush_access() -> Result<(), String> {
    let user = current_user();
    if user.is_empty() {
        return Err("无法获取当前用户".into());
    }

    let tmp = std::env::temp_dir().join("hostguard-dns");
    fs::write(&tmp, dns_sudoers_line(&user)).map_err(|e| e.to_string())?;
    let tmp_str = tmp.to_string_lossy().to_string();

    // Single-quote the paths for the shell; literal single quotes inside an
    // AppleScript string need no escaping. Temp/dest paths contain no quotes.
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

    let _ = fs::remove_file(&tmp); // best-effort cleanup

    let output = result.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // osascript reports a cancelled auth dialog as error -128.
        if err.contains("-128") {
            Err("已取消".into())
        } else {
            Err(err)
        }
    }
}

/// Flush the DNS cache without prompting. Requires the one-time grant above; if
/// the passwordless rule is missing, returns NO_PERMISSION so the UI can grant.
#[tauri::command]
async fn flush_dns_cache() -> Result<(), String> {
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
            // `sudo -n` without a passwordless rule fails asking for a password.
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

// ───────────────────────── System telemetry ─────────────────────────
//
// A single `System` handle lives in Tauri state so CPU deltas are computed
// across polls (sysinfo needs two samples). Each poll refreshes only CPU + RAM
// — no disk/network/process scans — keeping the footprint minimal. The frontend
// polls only while the monitor page is mounted and visible.

#[derive(serde::Serialize)]
struct SystemStats {
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

/// Sample current CPU + memory usage. Cheap: refreshes only CPU usage and RAM
/// on the persistent handle; load average / uptime / host name are syscalls.
#[tauri::command]
fn read_system_stats(state: tauri::State<'_, Mutex<System>>) -> Result<SystemStats, String> {
    let mut sys = state.lock().map_err(|e| e.to_string())?;
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let load = System::load_average();
    Ok(SystemStats {
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

/// Build the shared `System` handle, priming the CPU list + first sample so the
/// first frontend poll already has a valid delta to diff against.
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
