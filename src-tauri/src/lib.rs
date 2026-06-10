use std::fs;
use std::path::PathBuf;
use std::process::Command;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            hosts_writable,
            read_hosts,
            grant_hosts_access,
            write_hosts,
            list_shell_configs,
            read_shell_config,
            write_shell_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
