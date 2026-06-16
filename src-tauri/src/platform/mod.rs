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
    shell_configs,
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
