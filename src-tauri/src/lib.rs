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
//
// 实时指标（CPU 用量/频率、内存、交换、负载）由 `read_system_stats` 轮询：它只刷新
// CPU 与内存，footprint 极小（sysinfo 需两次采样算 CPU 增量，故 `System` 常驻 Tauri
// state）。静态信息（系统/内核/架构/主机/CPU 型号）走 `read_host_info`，只在挂载时取
// 一次。磁盘与温度变化慢、探测较贵（Windows 温度走 WMI），不混进快轮询，单独按需读取。
// 首页挂载时按需轮询；其它页面不轮询。注意 Windows 无平均负载（返回 0）。

#[derive(serde::Serialize)]
struct SystemStats {
    cpu_usage: f32,
    per_core: Vec<f32>,
    cpu_freq: u64, // MHz（首个核心当前频率）
    mem_total: u64,
    mem_used: u64,
    mem_available: u64,
    swap_total: u64,
    swap_used: u64,
    load_one: f64,
    load_five: f64,
    load_fifteen: f64,
    uptime: u64,
}

#[tauri::command]
fn read_system_stats(state: tauri::State<'_, Mutex<System>>) -> Result<SystemStats, String> {
    let mut sys = state.lock().map_err(|e| e.to_string())?;
    sys.refresh_cpu_all(); // 用量 + 频率
    sys.refresh_memory();

    let load = System::load_average();
    Ok(SystemStats {
        cpu_usage: sys.global_cpu_usage(),
        per_core: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
        cpu_freq: sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
        mem_total: sys.total_memory(),
        mem_used: sys.used_memory(),
        mem_available: sys.available_memory(),
        swap_total: sys.total_swap(),
        swap_used: sys.used_swap(),
        load_one: load.one,
        load_five: load.five,
        load_fifteen: load.fifteen,
        uptime: System::uptime(),
    })
}

/// 静态/缓变的主机信息，前端只在页面挂载时取一次。
#[derive(serde::Serialize)]
struct HostInfo {
    os: String,
    os_version: String,
    kernel_version: String,
    long_os_version: String,
    arch: String,
    host_name: String,
    cpu_brand: String,
    core_count: usize,
    physical_core_count: Option<usize>,
    boot_time: u64,
}

#[tauri::command]
fn read_host_info(state: tauri::State<'_, Mutex<System>>) -> Result<HostInfo, String> {
    let sys = state.lock().map_err(|e| e.to_string())?;
    Ok(HostInfo {
        os: std::env::consts::OS.to_string(),
        os_version: System::os_version().unwrap_or_default(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        long_os_version: System::long_os_version().unwrap_or_default(),
        arch: std::env::consts::ARCH.to_string(),
        host_name: System::host_name().unwrap_or_default(),
        cpu_brand: sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default(),
        core_count: sys.cpus().len(),
        physical_core_count: sys.physical_core_count(),
        boot_time: System::boot_time(),
    })
}

/// 单个磁盘/分区的容量信息。
#[derive(serde::Serialize)]
struct DiskInfo {
    name: String,
    mount: String,
    fs: String,
    total: u64,
    available: u64,
    used: u64,
}

/// 磁盘占用，按需读取（挂载 + 手动刷新）。变化慢，不进快轮询。
#[tauri::command]
fn read_disks() -> Vec<DiskInfo> {
    use sysinfo::Disks;
    Disks::new_with_refreshed_list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_string_lossy().into_owned(),
                fs: d.file_system().to_string_lossy().into_owned(),
                total,
                available,
                used: total.saturating_sub(available),
            }
        })
        .collect()
}

/// 尽力而为的 CPU 温度（℃）。Windows 走 WMI 可能无数据 → None（前端隐藏该卡片）。
/// 探测较贵，前端以较慢节奏单独取，不混进快轮询。
#[tauri::command]
fn read_cpu_temp() -> Option<f32> {
    use sysinfo::Components;
    let comps = Components::new_with_refreshed_list();
    let mut cpu_labeled: Option<f32> = None;
    let mut best: Option<f32> = None;
    for c in comps.iter() {
        let Some(t) = c.temperature() else {
            continue;
        };
        if !t.is_finite() || t <= 0.0 {
            continue;
        }
        let label = c.label().to_lowercase();
        if label.contains("cpu")
            || label.contains("core")
            || label.contains("package")
            || label.contains("tdie")
            || label.contains("tctl")
        {
            cpu_labeled = Some(cpu_labeled.map_or(t, |b| b.max(t)));
        }
        best = Some(best.map_or(t, |b| b.max(t)));
    }
    // 优先 CPU 相关传感器，否则退回最高有效温度。
    cpu_labeled.or(best)
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            read_system_stats,
            read_host_info,
            read_disks,
            read_cpu_temp
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
