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
/// 一次性 ACL 授权：chmod +a 给当前用户写权限（单次管理员弹窗）。async +
/// spawn_blocking，避免 osascript 模态对话框阻塞 UI 线程。
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
            let path = shell_path(name)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
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
