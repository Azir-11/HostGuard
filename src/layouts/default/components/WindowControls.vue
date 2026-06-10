<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";

// Resolved lazily so the component renders fine outside Tauri (plain vite dev).
async function run(action: "close" | "minimize" | "toggleMaximize") {
  try {
    await getCurrentWindow()[action]();
  } catch {
    /* not running inside Tauri */
  }
}

const controls = [
  { color: "bg-tlclose", icon: "i-ph-x-bold", label: "关闭", fn: () => run("close") },
  {
    color: "bg-tlmin",
    icon: "i-ph-minus-bold",
    label: "最小化",
    fn: () => run("minimize"),
  },
  {
    color: "bg-tlzoom",
    icon: "i-ph-plus-bold",
    label: "缩放",
    fn: () => run("toggleMaximize"),
  },
];
</script>

<template>
  <div class="group flex items-center gap-8px">
    <button
      v-for="c in controls"
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
