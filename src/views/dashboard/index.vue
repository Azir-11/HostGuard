<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useSystemStats } from "@/composables/useSystemStats";
import { useFabStore } from "@/store/fab";
import { parseHosts } from "@/utils/hosts";

const { stats, host, disks, cpuTemp, error, refresh } = useSystemStats();
const fab = useFabStore();

// ── hosts 条目数（真实读取并解析）──
const hostsTotal = ref<number | null>(null);
const hostsEnabled = ref(0);
async function loadHosts() {
  try {
    const text = await invoke<string>("read_hosts");
    const entries = parseHosts(text).filter((l) => l.type === "entry");
    hostsTotal.value = entries.length;
    hostsEnabled.value = entries.filter((e) => e.type === "entry" && e.enabled).length;
  } catch {
    hostsTotal.value = null;
  }
}

// ── 格式化 / 配色 ──
const GB = 1024 ** 3;
const gb = (b: number) => (b / GB).toFixed(1);
const clamp = (n: number) => Math.max(0, Math.min(100, n));
function usageColor(p: number) {
  if (p >= 85) return "var(--tl-close)";
  if (p >= 60) return "var(--c-amber)";
  return "var(--c-accent-2)";
}
function fmtUptime(s: number) {
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d) return `${d} 天 ${h} 小时`;
  if (h) return `${h} 小时 ${m} 分`;
  return `${m} 分`;
}

// ── 派生指标 ──
const memPct = computed(() =>
  stats.value && stats.value.mem_total ? (stats.value.mem_used / stats.value.mem_total) * 100 : 0,
);
const swapPct = computed(() =>
  stats.value && stats.value.swap_total
    ? (stats.value.swap_used / stats.value.swap_total) * 100
    : 0,
);
const maxDiskPct = computed(() => {
  if (!disks.value.length) return 0;
  return Math.max(...disks.value.map((d) => (d.total ? (d.used / d.total) * 100 : 0)));
});
const cpuGhz = computed(() => (stats.value ? (stats.value.cpu_freq / 1000).toFixed(2) : "0"));

// 健康分：主要看实时 CPU + 内存压力，磁盘极满（>90%）时额外扣分。
const health = computed(() => {
  if (!stats.value) return 100;
  let h = 100 - stats.value.cpu_usage * 0.5 - memPct.value * 0.5;
  if (maxDiskPct.value > 90) h -= (maxDiskPct.value - 90) * 2;
  return Math.round(clamp(h));
});
const healthLabel = computed(() => {
  const h = health.value;
  if (h >= 85) return "运行良好";
  if (h >= 60) return "略有压力";
  return "负载偏高";
});

const ring = {
  r: 82,
  get c() {
    return 2 * Math.PI * this.r;
  },
};

const loads = computed(() => {
  const s = stats.value;
  return [
    { label: "1 分钟", v: s?.load_one ?? 0 },
    { label: "5 分钟", v: s?.load_five ?? 0 },
    { label: "15 分钟", v: s?.load_fifteen ?? 0 },
  ];
});

const greeting = computed(() => {
  const h = new Date().getHours();
  if (h < 6) return "凌晨好";
  if (h < 12) return "早上好";
  if (h < 14) return "中午好";
  if (h < 18) return "下午好";
  return "晚上好";
});

function bootText() {
  if (!host.value?.boot_time) return "—";
  return new Date(host.value.boot_time * 1000).toLocaleString();
}

// 系统详情键值对
const details = computed(() => {
  const h = host.value;
  if (!h) return [] as { icon: string; label: string; value: string }[];
  const cores = h.physical_core_count
    ? `${h.physical_core_count} 核 / ${h.core_count} 线程`
    : `${h.core_count} 核`;
  return [
    {
      icon: "i-ph-desktop-duotone",
      label: "操作系统",
      value: h.long_os_version || h.os_version || h.os,
    },
    { icon: "i-ph-tree-structure-duotone", label: "内核", value: h.kernel_version || "—" },
    { icon: "i-ph-cpu-duotone", label: "架构", value: h.arch },
    { icon: "i-ph-identification-card-duotone", label: "主机名", value: h.host_name || "—" },
    { icon: "i-ph-circuitry-duotone", label: "处理器", value: h.cpu_brand || "—" },
    { icon: "i-ph-stack-duotone", label: "核心", value: cores },
    { icon: "i-ph-power-duotone", label: "开机时间", value: bootText() },
  ];
});

onMounted(() => {
  void loadHosts();
  fab.set({
    label: "刷新",
    icon: "i-ph-arrows-clockwise-duotone",
    run: async () => {
      await refresh();
      await loadHosts();
    },
  });
});
onUnmounted(() => fab.clear());
</script>

<template>
  <div class="h-full flex flex-col gap-18px overflow-auto pr-2px">
    <!-- 首帧加载 -->
    <div v-if="!stats && !error" class="flex-1 grid place-items-center text-fg-3">
      <span class="i-ph-circle-notch-bold animate-spin text-28px" />
    </div>

    <!-- 取数失败 -->
    <div
      v-else-if="!stats && error"
      class="flex-1 flex flex-col items-center justify-center gap-12px p-24px text-center"
    >
      <span class="i-ph-warning-circle-duotone text-44px text-amber" />
      <p class="m-0 max-w-420px text-fg-2 leading-[1.6]">读取系统状态失败：{{ error }}</p>
    </div>

    <template v-else-if="stats">
      <!-- ════════ Hero：健康分 + 概要 ════════ -->
      <section
        class="relative flex items-center gap-34px p-[26px_32px] rounded-lg border border-hairline bg-elevated overflow-hidden"
      >
        <div
          class="absolute inset-0 pointer-events-none bg-[radial-gradient(90%_120%_at_100%_0%,rgba(56,224,138,0.08),transparent_55%)]"
        />

        <div class="relative w-160px h-160px shrink-0 grid place-items-center">
          <svg class="absolute inset-0 w-full h-full -rotate-90" viewBox="0 0 200 200">
            <defs>
              <linearGradient id="ringGrad" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="#38e08a" />
                <stop offset="100%" stop-color="#14a866" />
              </linearGradient>
            </defs>
            <circle
              class="stroke-hairline-strong"
              cx="100"
              cy="100"
              :r="ring.r"
              fill="none"
              stroke-width="10"
            />
            <circle
              class="drop-shadow-[0_4px_12px_var(--c-accent-glow)] transition-[stroke-dashoffset] duration-1000 ease-out"
              cx="100"
              cy="100"
              :r="ring.r"
              fill="none"
              stroke="url(#ringGrad)"
              stroke-width="10"
              :stroke-dasharray="ring.c"
              :stroke-dashoffset="ring.c * (1 - health / 100)"
              stroke-linecap="round"
            />
          </svg>
          <div class="text-center leading-none">
            <div class="font-display text-42px font-700">
              {{ health }}<span class="ml-2px text-15px font-500 text-fg-2">分</span>
            </div>
            <div class="mt-7px text-12px text-fg-2">系统健康</div>
          </div>
        </div>

        <div class="relative flex-1 min-w-0">
          <span
            class="inline-flex items-center gap-6px px-12px py-5px rounded-full text-[12.5px] text-accent-2 bg-accent-soft"
          >
            <span class="i-ph-shield-check-duotone" /> {{ healthLabel }}
          </span>
          <h2 class="mt-12px mb-6px text-26px font-700 font-display">
            {{ greeting }}{{ host?.host_name ? `，${host.host_name}` : "" }}
          </h2>
          <p class="m-0 max-w-460px text-fg-2 leading-[1.6] text-[13.5px]">
            {{ host?.long_os_version || host?.os_version || "" }} · 已运行
            {{ fmtUptime(stats.uptime) }}
          </p>

          <div class="mt-16px flex flex-wrap gap-10px">
            <div
              class="flex items-center gap-8px px-12px py-8px rounded-md border border-hairline bg-[var(--c-bg-0)]"
            >
              <span class="i-ph-cpu-duotone text-16px text-accent-2" />
              <span class="text-12px text-fg-3">CPU</span>
              <span class="font-mono text-13px font-600">{{ stats.cpu_usage.toFixed(0) }}%</span>
            </div>
            <div
              class="flex items-center gap-8px px-12px py-8px rounded-md border border-hairline bg-[var(--c-bg-0)]"
            >
              <span class="i-ph-memory-duotone text-16px text-info" />
              <span class="text-12px text-fg-3">内存</span>
              <span class="font-mono text-13px font-600">{{ memPct.toFixed(0) }}%</span>
            </div>
            <div
              v-if="disks.length"
              class="flex items-center gap-8px px-12px py-8px rounded-md border border-hairline bg-[var(--c-bg-0)]"
            >
              <span class="i-ph-hard-drives-duotone text-16px text-violet" />
              <span class="text-12px text-fg-3">磁盘</span>
              <span class="font-mono text-13px font-600">{{ maxDiskPct.toFixed(0) }}%</span>
            </div>
            <div
              v-if="hostsTotal !== null"
              class="flex items-center gap-8px px-12px py-8px rounded-md border border-hairline bg-[var(--c-bg-0)]"
            >
              <span class="i-ph-globe-hemisphere-west-duotone text-16px text-amber" />
              <span class="text-12px text-fg-3">Hosts</span>
              <span class="font-mono text-13px font-600">{{ hostsEnabled }}/{{ hostsTotal }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- ════════ CPU + 内存 ════════ -->
      <section class="grid grid-cols-2 gap-16px">
        <!-- CPU -->
        <article class="card p-20px flex flex-col gap-13px">
          <div class="flex items-center gap-10px">
            <span class="i-ph-cpu-duotone text-22px text-accent-2 shrink-0" />
            <span class="text-14px font-600 shrink-0">处理器</span>
            <div class="flex-1" />
            <span
              v-if="cpuTemp !== null"
              class="inline-flex items-center gap-4px px-8px py-3px rounded-full text-11px font-mono border border-hairline text-fg-2"
            >
              <span class="i-ph-thermometer-simple-duotone text-13px" />
              {{ cpuTemp.toFixed(0) }}°C
            </span>
            <span class="font-display font-mono text-26px font-700 leading-none">
              {{ stats.cpu_usage.toFixed(0)
              }}<span class="ml-1px text-14px font-500 text-fg-2">%</span>
            </span>
          </div>

          <div class="h-8px rounded-full bg-hairline overflow-hidden">
            <span
              class="block h-full rounded-full transition-[width,background] duration-500 ease-out"
              :style="{
                width: `${clamp(stats.cpu_usage)}%`,
                background: usageColor(stats.cpu_usage),
              }"
            />
          </div>

          <div class="flex items-center justify-between text-12px text-fg-3">
            <span>{{ stats.per_core.length }} 逻辑核心</span>
            <span v-if="stats.cpu_freq" class="font-mono">{{ cpuGhz }} GHz</span>
          </div>

          <div
            class="grid gap-x-12px gap-y-7px grid-cols-[repeat(auto-fill,minmax(70px,1fr))] mt-2px"
          >
            <div v-for="(c, i) in stats.per_core" :key="i" class="flex items-center gap-5px">
              <span class="w-14px shrink-0 text-10px text-fg-3 font-mono">{{ i + 1 }}</span>
              <div class="flex-1 h-4px rounded-full bg-hairline overflow-hidden">
                <span
                  class="block h-full rounded-full transition-[width,background] duration-500"
                  :style="{ width: `${clamp(c)}%`, background: usageColor(c) }"
                />
              </div>
            </div>
          </div>
        </article>

        <!-- 内存 -->
        <article class="card p-20px flex flex-col gap-13px">
          <div class="flex items-center gap-10px">
            <span class="i-ph-memory-duotone text-22px text-info shrink-0" />
            <span class="text-14px font-600">内存</span>
            <div class="flex-1" />
            <span class="font-mono text-[15px] text-fg-2">
              <b class="text-18px text-fg">{{ gb(stats.mem_used) }}</b> /
              {{ gb(stats.mem_total) }} GB
            </span>
          </div>
          <div class="h-8px rounded-full bg-hairline overflow-hidden">
            <span
              class="block h-full rounded-full transition-[width,background] duration-500 ease-out"
              :style="{ width: `${clamp(memPct)}%`, background: usageColor(memPct) }"
            />
          </div>
          <div class="text-12px text-fg-3">
            可用 {{ gb(stats.mem_available) }} GB · 已用 {{ memPct.toFixed(0) }}%
          </div>

          <template v-if="stats.swap_total > 0">
            <div class="mt-2px h-1px bg-hairline" />
            <div class="flex items-center justify-between text-12px">
              <span class="text-fg-2">交换 (Swap)</span>
              <span class="font-mono text-fg-3">
                {{ gb(stats.swap_used) }} / {{ gb(stats.swap_total) }} GB
              </span>
            </div>
            <div class="h-5px rounded-full bg-hairline overflow-hidden">
              <span
                class="block h-full rounded-full transition-[width] duration-500"
                :style="{ width: `${clamp(swapPct)}%`, background: 'var(--c-violet)' }"
              />
            </div>
          </template>
        </article>
      </section>

      <!-- ════════ 磁盘 ════════ -->
      <section v-if="disks.length" class="card p-20px flex flex-col gap-14px">
        <div class="flex items-center gap-10px">
          <span class="i-ph-hard-drives-duotone text-22px text-violet shrink-0" />
          <span class="text-14px font-600">磁盘占用</span>
          <div class="flex-1" />
          <span class="text-12px text-fg-3">{{ disks.length }} 个卷</span>
        </div>
        <div class="grid gap-x-22px gap-y-13px grid-cols-[repeat(auto-fill,minmax(280px,1fr))]">
          <div v-for="d in disks" :key="d.mount" class="flex flex-col gap-6px">
            <div class="flex items-center gap-8px text-12px">
              <span class="i-ph-hard-drive-duotone text-15px text-fg-2 shrink-0" />
              <span class="font-mono font-600 text-fg truncate">{{ d.mount }}</span>
              <span v-if="d.fs" class="text-fg-3 shrink-0">{{ d.fs }}</span>
              <div class="flex-1" />
              <span class="font-mono text-fg-3 shrink-0">
                {{ gb(d.used) }} / {{ gb(d.total) }} GB
              </span>
            </div>
            <div class="h-6px rounded-full bg-hairline overflow-hidden">
              <span
                class="block h-full rounded-full transition-[width,background] duration-500 ease-out"
                :style="{
                  width: `${clamp(d.total ? (d.used / d.total) * 100 : 0)}%`,
                  background: usageColor(d.total ? (d.used / d.total) * 100 : 0),
                }"
              />
            </div>
          </div>
        </div>
      </section>

      <!-- ════════ 平均负载（仅 macOS）+ 系统详情 ════════ -->
      <section
        class="grid gap-16px"
        :class="host && host.os !== 'windows' ? 'grid-cols-3' : 'grid-cols-1'"
      >
        <!-- 平均负载 -->
        <article v-if="host && host.os !== 'windows'" class="card p-20px flex flex-col gap-14px">
          <div class="flex items-center gap-10px">
            <span class="i-ph-gauge-duotone text-22px text-accent-2 shrink-0" />
            <span class="text-14px font-600">平均负载</span>
          </div>
          <div class="grid grid-cols-3 gap-10px">
            <div
              v-for="l in loads"
              :key="l.label"
              class="flex flex-col items-center gap-3px py-9px rounded-md bg-elevated border border-hairline"
            >
              <span class="font-display font-mono text-19px font-700">{{ l.v.toFixed(2) }}</span>
              <span class="text-11px text-fg-3">{{ l.label }}</span>
            </div>
          </div>
        </article>

        <!-- 系统详情 -->
        <article
          class="card p-20px flex flex-col gap-11px"
          :class="host && host.os !== 'windows' ? 'col-span-2' : ''"
        >
          <div class="flex items-center gap-10px mb-1px">
            <span class="i-ph-info-duotone text-22px text-info shrink-0" />
            <span class="text-14px font-600">系统详情</span>
          </div>
          <div class="grid gap-x-26px gap-y-10px grid-cols-[repeat(auto-fill,minmax(260px,1fr))]">
            <div v-for="row in details" :key="row.label" class="flex items-center gap-9px min-w-0">
              <span :class="row.icon" class="text-16px text-fg-2 shrink-0" />
              <span class="text-12px text-fg-3 shrink-0 w-58px">{{ row.label }}</span>
              <span class="text-[12.5px] font-mono text-fg truncate" :title="row.value">
                {{ row.value }}
              </span>
            </div>
          </div>
        </article>
      </section>
    </template>
  </div>
</template>
