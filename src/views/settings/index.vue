<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { useAppStore } from "@/store/app";

const appStore = useAppStore();

async function openRepo() {
  try {
    await openUrl("https://github.com/Azir-11/HostGuard");
  } catch {
    /* noop outside Tauri */
  }
}
</script>

<template>
  <div class="flex flex-col gap-18px max-w-680px">
    <section class="rounded-lg border border-hairline bg-elevated overflow-hidden">
      <h3 class="px-18px pt-15px pb-9px text-13px font-600 text-fg-2 tracking-[0.3px]">外观</h3>
      <div
        class="flex items-center justify-between gap-16px px-18px py-14px border-t border-hairline"
      >
        <div class="flex flex-col gap-3px min-w-0">
          <span class="text-14px font-500">主题</span>
          <span class="text-[12.5px] text-fg-3">在深色与浅色界面之间切换</span>
        </div>
        <div class="flex gap-4px p-4px rounded-md border border-hairline bg-[var(--c-bg-0)]">
          <button
            class="inline-flex items-center gap-6px px-14px py-7px rounded-sm border-none cursor-pointer text-13px transition-colors"
            :class="
              appStore.isDark
                ? 'bg-grad-accent !text-[#04130b] font-600'
                : 'bg-transparent text-fg-2'
            "
            @click="appStore.setDark(true)"
          >
            <span class="i-ph-moon-stars-duotone" /> 深色
          </button>
          <button
            class="inline-flex items-center gap-6px px-14px py-7px rounded-sm border-none cursor-pointer text-13px transition-colors"
            :class="
              !appStore.isDark
                ? 'bg-grad-accent !text-[#04130b] font-600'
                : 'bg-transparent text-fg-2'
            "
            @click="appStore.setDark(false)"
          >
            <span class="i-ph-sun-duotone" /> 浅色
          </button>
        </div>
      </div>
    </section>

    <section class="rounded-lg border border-hairline bg-elevated overflow-hidden">
      <h3 class="px-18px pt-15px pb-9px text-13px font-600 text-fg-2 tracking-[0.3px]">关于</h3>
      <div
        class="flex items-center justify-between gap-16px px-18px py-14px border-t border-hairline"
      >
        <div class="flex flex-col gap-3px min-w-0">
          <span class="text-14px font-500">HostGuard</span>
          <span class="text-[12.5px] text-fg-3">hosts、Shell 与系统配置管理工具</span>
        </div>
        <span class="px-10px py-4px rounded-[8px] font-mono text-13px text-fg-2 bg-elevated-2"
          >v0.1.0</span
        >
      </div>
      <div
        class="flex items-center justify-between gap-16px px-18px py-14px border-t border-hairline cursor-pointer transition-colors hover:bg-elevated-2"
        @click="openRepo"
      >
        <div class="flex flex-col gap-3px min-w-0">
          <span class="flex items-center gap-7px text-14px font-500">
            <span class="i-ph-github-logo-duotone" /> 源代码
          </span>
          <span class="text-[12.5px] text-fg-3">github.com/Azir-11/HostGuard</span>
        </div>
        <span class="i-ph-arrow-up-right-bold text-15px text-fg-3" />
      </div>
    </section>
  </div>
</template>
