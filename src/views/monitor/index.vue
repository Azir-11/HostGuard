<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useFabStore } from "@/store/fab";

interface SystemStats {
  cpu_usage: number;
  per_core: number[];
  mem_total: number;
  mem_used: number;
  mem_available: number;
  swap_total: number;
  swap_used: number;
  load_one: number;
  load_five: number;
  load_fifteen: number;
  uptime: number;
  core_count: number;
  cpu_brand: string;
  host_name: string;
}

const fab = useFabStore();

const stats = ref<SystemStats | null>(null);
const error = ref("");

// Real but light: poll a cheap CPU+RAM sample, and only while this page is
// mounted AND the window is visible. Other pages never poll.
const INTERVAL = 1500;
let timer: number | undefined;

async function poll() {
  if (document.hidden) return;
  try {
    stats.value = await invoke<SystemStats>("read_system_stats");
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

function start() {
  stop();
  void poll();
  timer = window.setInterval(poll, INTERVAL);
}
function stop() {
  if (timer !== undefined) {
    clearInterval(timer);
    timer = undefined;
  }
}
function onVisibility() {
  if (document.hidden) stop();
  else start();
}

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

const memPct = computed(() =>
  stats.value && stats.value.mem_total ? (stats.value.mem_used / stats.value.mem_total) * 100 : 0,
);
const swapPct = computed(() =>
  stats.value && stats.value.swap_total
    ? (stats.value.swap_used / stats.value.swap_total) * 100
    : 0,
);
const loads = computed(() => {
  const s = stats.value;
  return [
    { label: "1 分钟", v: s?.load_one ?? 0 },
    { label: "5 分钟", v: s?.load_five ?? 0 },
    { label: "15 分钟", v: s?.load_fifteen ?? 0 },
  ];
});

onMounted(() => {
  start();
  document.addEventListener("visibilitychange", onVisibility);
  fab.set({ label: "立即刷新", icon: "i-ph-radar-duotone", run: poll });
});
onUnmounted(() => {
  stop();
  document.removeEventListener("visibilitychange", onVisibility);
  fab.clear();
});
</script>

<template>
  <div class="h-full flex flex-col gap-16px overflow-auto pr-2px">
    <!-- loading (first sample not in yet) -->
    <div v-if="!stats && !error" class="flex-1 grid place-items-center text-fg-3">
      <span class="i-ph-circle-notch-bold animate-spin text-28px" />
    </div>

    <!-- hard error before any data -->
    <div
      v-else-if="!stats && error"
      class="flex-1 flex flex-col items-center justify-center gap-12px p-24px text-center"
    >
      <span class="i-ph-warning-circle-duotone text-44px text-amber" />
      <p class="m-0 max-w-420px text-fg-2 leading-[1.6]">读取系统状态失败：{{ error }}</p>
    </div>

    <template v-else-if="stats">
      <!-- ═══ CPU ═══ -->
      <section class="card p-20px">
        <div class="flex items-center gap-10px mb-14px">
          <span class="i-ph-cpu-duotone text-22px text-accent-2 shrink-0" />
          <span class="text-14px font-600 shrink-0">处理器</span>
          <span class="text-12px text-fg-3 truncate">{{ stats.cpu_brand }}</span>
          <div class="flex-1" />
          <span class="font-display font-mono text-28px font-700 leading-none">
            {{ stats.cpu_usage.toFixed(0)
            }}<span class="ml-1px text-15px font-500 text-fg-2">%</span>
          </span>
        </div>

        <div class="h-8px rounded-full bg-hairline overflow-hidden mb-18px">
          <span
            class="block h-full rounded-full transition-[width,background] duration-500 ease-out"
            :style="{
              width: `${clamp(stats.cpu_usage)}%`,
              background: usageColor(stats.cpu_usage),
            }"
          />
        </div>

        <div class="grid gap-x-14px gap-y-9px grid-cols-[repeat(auto-fill,minmax(78px,1fr))]">
          <div v-for="(c, i) in stats.per_core" :key="i" class="flex items-center gap-6px">
            <span class="w-16px shrink-0 text-10px text-fg-3 font-mono">{{ i + 1 }}</span>
            <div class="flex-1 h-5px rounded-full bg-hairline overflow-hidden">
              <span
                class="block h-full rounded-full transition-[width,background] duration-500"
                :style="{ width: `${clamp(c)}%`, background: usageColor(c) }"
              />
            </div>
          </div>
        </div>
      </section>

      <!-- ═══ Memory + Load ═══ -->
      <section class="grid grid-cols-2 gap-16px">
        <!-- memory -->
        <article class="card p-20px flex flex-col gap-14px">
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

        <!-- load + uptime -->
        <article class="card p-20px flex flex-col gap-14px">
          <div class="flex items-center gap-10px">
            <span class="i-ph-gauge-duotone text-22px text-accent-2 shrink-0" />
            <span class="text-14px font-600">平均负载</span>
            <div class="flex-1" />
            <span class="text-12px text-fg-3">{{ stats.core_count }} 核</span>
          </div>

          <div class="grid grid-cols-3 gap-10px">
            <div
              v-for="l in loads"
              :key="l.label"
              class="flex flex-col items-center gap-3px py-8px rounded-md bg-elevated border border-hairline"
            >
              <span class="font-display font-mono text-20px font-700">{{ l.v.toFixed(2) }}</span>
              <span class="text-11px text-fg-3">{{ l.label }}</span>
            </div>
          </div>

          <div class="mt-auto pt-4px flex flex-col gap-7px text-12px">
            <div class="flex items-center gap-8px text-fg-3">
              <span class="i-ph-timer-duotone text-15px text-fg-2 shrink-0" />
              <span class="text-fg-2">已运行</span>
              <span class="font-mono">{{ fmtUptime(stats.uptime) }}</span>
            </div>
            <div v-if="stats.host_name" class="flex items-center gap-8px text-fg-3">
              <span class="i-ph-desktop-duotone text-15px text-fg-2 shrink-0" />
              <span class="text-fg-2">主机</span>
              <span class="font-mono truncate">{{ stats.host_name }}</span>
            </div>
          </div>
        </article>
      </section>
    </template>
  </div>
</template>
