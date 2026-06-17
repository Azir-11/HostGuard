import { invoke } from "@tauri-apps/api/core";
import { onMounted, onUnmounted, ref } from "vue";

/** 实时指标（CPU/内存/交换/负载/频率），由后端 `read_system_stats` 轮询。 */
export interface SystemStats {
  cpu_usage: number;
  per_core: number[];
  cpu_freq: number; // MHz
  mem_total: number;
  mem_used: number;
  mem_available: number;
  swap_total: number;
  swap_used: number;
  load_one: number;
  load_five: number;
  load_fifteen: number;
  uptime: number;
}

/** 静态/缓变主机信息，只取一次。 */
export interface HostInfo {
  os: string;
  os_version: string;
  kernel_version: string;
  long_os_version: string;
  arch: string;
  host_name: string;
  cpu_brand: string;
  core_count: number;
  physical_core_count: number | null;
  boot_time: number;
}

export interface DiskInfo {
  name: string;
  mount: string;
  fs: string;
  total: number;
  available: number;
  used: number;
}

const LIVE_INTERVAL = 1500; // CPU/内存快轮询
const SLOW_INTERVAL = 5000; // 温度慢轮询（探测较贵）

/**
 * 系统遥测组合式函数：快轮询实时指标，慢轮询温度，挂载时取一次静态信息与磁盘。
 * 仅在使用它的页面挂载且窗口可见时轮询；离开/隐藏即停。view 层只渲染。
 */
export function useSystemStats() {
  const stats = ref<SystemStats | null>(null);
  const host = ref<HostInfo | null>(null);
  const disks = ref<DiskInfo[]>([]);
  const cpuTemp = ref<number | null>(null);
  const error = ref("");

  let liveTimer: number | undefined;
  let slowTimer: number | undefined;

  async function pollLive() {
    if (document.hidden) return;
    try {
      stats.value = await invoke<SystemStats>("read_system_stats");
      error.value = "";
    } catch (e) {
      error.value = String(e);
    }
  }

  async function pollTemp() {
    if (document.hidden) return;
    try {
      cpuTemp.value = await invoke<number | null>("read_cpu_temp");
    } catch {
      /* 温度尽力而为，失败保持上次值 */
    }
  }

  async function loadHost() {
    try {
      host.value = await invoke<HostInfo>("read_host_info");
    } catch {
      /* 非 Tauri 环境忽略 */
    }
  }

  async function loadDisks() {
    try {
      disks.value = await invoke<DiskInfo[]>("read_disks");
    } catch {
      /* 非 Tauri 环境忽略 */
    }
  }

  /** 手动刷新：实时指标 + 温度 + 磁盘一起拉一次。 */
  async function refresh() {
    await Promise.all([pollLive(), pollTemp(), loadDisks()]);
  }

  function startTimers() {
    stopTimers();
    liveTimer = window.setInterval(pollLive, LIVE_INTERVAL);
    slowTimer = window.setInterval(pollTemp, SLOW_INTERVAL);
  }
  function stopTimers() {
    if (liveTimer !== undefined) {
      clearInterval(liveTimer);
      liveTimer = undefined;
    }
    if (slowTimer !== undefined) {
      clearInterval(slowTimer);
      slowTimer = undefined;
    }
  }
  function onVisibility() {
    if (document.hidden) {
      stopTimers();
    } else {
      void pollLive();
      void pollTemp();
      startTimers();
    }
  }

  onMounted(() => {
    void loadHost();
    void loadDisks();
    void pollLive();
    void pollTemp();
    startTimers();
    document.addEventListener("visibilitychange", onVisibility);
  });
  onUnmounted(() => {
    stopTimers();
    document.removeEventListener("visibilitychange", onVisibility);
  });

  return { stats, host, disks, cpuTemp, error, refresh };
}
