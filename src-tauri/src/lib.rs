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
// A single `System` handle lives in Tauri state so CPU deltas are computed
// across polls (sysinfo needs two samples). Each poll refreshes only CPU + RAM
// — no disk/network/process scans — keeping the footprint minimal. The frontend
// polls only while the monitor page is mounted and visible. `sysinfo` is
// cross-platform; note Windows has no load average (returns 0).

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
            read_system_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
