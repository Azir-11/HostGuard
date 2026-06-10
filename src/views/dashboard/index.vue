<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useMessage } from "naive-ui";
import { useFabStore } from "@/store/fab";

interface Stat {
  key: string;
  label: string;
  value: string;
  pct: number;
  icon: string;
  color: string;
}

// Placeholder figures — real telemetry arrives with the 系统用量 module.
const stats: Stat[] = [
  {
    key: "cpu",
    label: "CPU",
    value: "18%",
    pct: 18,
    icon: "i-ph-cpu-duotone",
    color: "var(--c-accent-2)",
  },
  {
    key: "mem",
    label: "内存",
    value: "9.4 / 16 GB",
    pct: 59,
    icon: "i-ph-memory-duotone",
    color: "var(--c-blue)",
  },
  {
    key: "disk",
    label: "磁盘",
    value: "312 / 500 GB",
    pct: 63,
    icon: "i-ph-hard-drives-duotone",
    color: "var(--c-violet)",
  },
  {
    key: "hosts",
    label: "Hosts 条目",
    value: "12 条",
    pct: 40,
    icon: "i-ph-globe-hemisphere-west-duotone",
    color: "var(--c-amber)",
  },
];

const ring = {
  r: 82,
  get c() {
    return 2 * Math.PI * this.r;
  },
};
const health = 100;

const fab = useFabStore();
const message = useMessage();

onMounted(() => {
  fab.set({
    label: "重新扫描",
    icon: "i-ph-radar-duotone",
    run: () => {
      message.success("正在重新扫描系统状态…");
    },
  });
});
onUnmounted(() => fab.clear());
</script>

<template>
  <div class="flex flex-col gap-22px h-full">
    <!-- ════════ Hero ════════ -->
    <section
      class="relative flex items-center gap-40px p-[30px_34px] rounded-lg border border-hairline bg-elevated overflow-hidden"
    >
      <div
        class="absolute inset-0 pointer-events-none bg-[radial-gradient(90%_120%_at_100%_0%,rgba(56,224,138,0.08),transparent_55%)]"
      />

      <div class="relative w-186px h-186px shrink-0 grid place-items-center">
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
        <div
          class="absolute inset-[14px] rounded-full border border-dashed border-hairline-strong animate-spinslow"
        />
        <div class="text-center leading-none">
          <div class="font-display text-46px font-700">
            {{ health }}<span class="ml-2px text-16px font-500 text-fg-2">分</span>
          </div>
          <div class="mt-8px text-13px text-fg-2">系统健康</div>
        </div>
      </div>

      <div class="relative flex-1 min-w-0">
        <span
          class="inline-flex items-center gap-6px px-12px py-5px rounded-full text-[12.5px] text-accent-2 bg-accent-soft"
        >
          <span class="i-ph-shield-check-duotone" /> 守护中
        </span>
        <h2 class="mt-14px mb-8px text-28px font-700 font-display">一切运行正常</h2>
        <p class="m-0 max-w-460px text-fg-2 leading-[1.65]">
          HostGuard 正在守护你的 hosts 与系统配置。点击右下角按钮可随时重新扫描。
        </p>
        <button
          class="mt-20px inline-flex items-center gap-8px h-42px px-20px border-none rounded-md text-14px font-600 text-[#04130b] cursor-pointer bg-grad-accent shadow-glow transition-transform hover:-translate-y-2px active:translate-y-0"
        >
          运行完整检查 <span class="i-ph-arrow-right-bold" />
        </button>
      </div>
    </section>

    <!-- ════════ Stats ════════ -->
    <section class="grid grid-cols-4 gap-16px">
      <article
        v-for="s in stats"
        :key="s.key"
        class="p-18px rounded-lg border border-hairline bg-elevated transition-transform hover:-translate-y-3px"
      >
        <div class="flex items-center gap-9px">
          <span class="text-22px" :class="s.icon" :style="{ color: s.color }" />
          <span class="text-13px text-fg-2">{{ s.label }}</span>
        </div>
        <div class="my-12px font-display text-20px font-600">{{ s.value }}</div>
        <div class="h-6px rounded-full bg-hairline overflow-hidden">
          <span
            class="block h-full rounded-full transition-[width] duration-700 ease-out"
            :style="{ width: `${s.pct}%`, background: s.color }"
          />
        </div>
      </article>
    </section>
  </div>
</template>
