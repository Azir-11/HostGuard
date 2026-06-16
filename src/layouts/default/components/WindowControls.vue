<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isWindows } from "@/composables/usePlatform";

// Resolved lazily so the component renders fine outside Tauri (plain vite dev).
async function run(action: "close" | "minimize" | "toggleMaximize") {
  try {
    await getCurrentWindow()[action]();
  } catch {
    /* not running inside Tauri */
  }
}

// macOS：红绿灯（关闭 / 最小化 / 缩放，左上）。
const macControls = [
  { color: "bg-tlclose", icon: "i-ph-x-bold", label: "关闭", fn: () => run("close") },
  { color: "bg-tlmin", icon: "i-ph-minus-bold", label: "最小化", fn: () => run("minimize") },
  { color: "bg-tlzoom", icon: "i-ph-plus-bold", label: "缩放", fn: () => run("toggleMaximize") },
];

// Windows：方形控件（最小化 / 最大化 / 关闭，右上；关闭悬停转红）。
const winControls = [
  { icon: "i-ph-minus-bold", label: "最小化", danger: false, fn: () => run("minimize") },
  { icon: "i-ph-square-bold", label: "最大化", danger: false, fn: () => run("toggleMaximize") },
  { icon: "i-ph-x-bold", label: "关闭", danger: true, fn: () => run("close") },
];
</script>

<template>
  <!-- Windows：右上方形控件 -->
  <div v-if="isWindows" class="flex items-center">
    <button
      v-for="c in winControls"
      :key="c.label"
      class="w-44px h-32px grid place-items-center border-none bg-transparent cursor-pointer text-fg-2 transition-colors"
      :class="c.danger ? 'hover:bg-tlclose hover:text-white' : 'hover:bg-elevated hover:text-fg'"
      :aria-label="c.label"
      @click="c.fn"
    >
      <span class="text-12px" :class="c.icon" />
    </button>
  </div>

  <!-- macOS：红绿灯 -->
  <div v-else class="group flex items-center gap-8px">
    <button
      v-for="c in macControls"
      :key="c.label"
      class="w-13px h-13px p-0 border-none rounded-full grid place-items-center cursor-pointer active:brightness-90"
      :class="c.color"
      :aria-label="c.label"
      @click="c.fn"
    >
      <span
        class="text-[8px] text-black/55 op-0 transition-opacity duration-100 group-hover:op-100"
        :class="c.icon"
      />
    </button>
  </div>
</template>
