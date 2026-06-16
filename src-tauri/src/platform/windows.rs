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
/// 一次性授权：经 UAC 提升运行 icacls，把 Modify 权限授予当前用户；之后用户
/// 即可直接写 hosts，无需再提权（对标 macOS 的 chmod +a）。
pub async fn grant_hosts_access() -> Result<(), String> {
    let hosts = hosts_path().to_string_lossy().to_string();
    // Start-Process -Verb RunAs 触发 UAC；-Wait 阻塞；$p.ExitCode 透传结果。
    // 授权对象用「当前(未提权)用户的 SID」而非裸用户名：SID 在本地 / Microsoft /
    // 域 / AzureAD 账户下都能被 icacls 唯一解析，避免裸用户名在域/AzureAD 机器上
    // 解析失败或错配同名本地账户。SID 必须在未提权的外层进程取——提权进程的身份是
    // 管理员而非目标用户。
    let ps = format!(
        "$ErrorActionPreference='Stop'; try {{ $sid = ([System.Security.Principal.WindowsIdentity]::GetCurrent()).User.Value; $p = Start-Process icacls -ArgumentList '\"{hosts}\"','/grant',\"*${{sid}}:(M)\" -Verb RunAs -WindowStyle Hidden -Wait -PassThru; if ($null -eq $p.ExitCode) {{ exit 1 }} else {{ exit $p.ExitCode }} }} catch {{ Write-Error 'UAC_CANCELLED'; exit 1 }}",
        hosts = hosts
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
/// 真正的 PowerShell 配置文件路径：直接问 PowerShell 自己取
/// `$PROFILE.CurrentUserCurrentHost`，因此跨版本正确（Windows PowerShell 5.1 →
/// Documents\WindowsPowerShell\…；PowerShell 7 → Documents\PowerShell\…），且自动
/// 跟随 OneDrive Known-Folder-Move 对「文档」的重定向——避免编辑了一个 `. $PROFILE`
/// 永远加载不到的文件。优先 pwsh(7)，否则用必定存在的 Windows PowerShell 5.1；
/// 两者都失败时回退到 5.1 默认路径。
fn ps_profile_path() -> PathBuf {
    for exe in ["pwsh", "powershell"] {
        let out = Command::new(exe)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$PROFILE.CurrentUserCurrentHost",
            ])
            .output();
        if let Ok(out) = out {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return PathBuf::from(p);
                }
            }
        }
    }
    home().join("Documents\\WindowsPowerShell\\Microsoft.PowerShell_profile.ps1")
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
    // cmd.exe 启动时把 AutoRun 值当命令行执行，并按空格切词。路径裸写时若含空格
    // （如用户名带空格、OneDrive 重定向路径）会被拆成「命令 + 参数」而执行失败，
    // 每开一个 cmd 窗口都报错且配置永不生效。用 call "<path>" 包引号即可正确执行。
    let val = format!("call \"{}\"", path.to_string_lossy());
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
