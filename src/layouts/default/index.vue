<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import SideBar from "./components/SideBar.vue";
import WindowControls from "./components/WindowControls.vue";
import { useFabStore } from "@/store/fab";
import { allItems } from "./menu";

const route = useRoute();
const fab = useFabStore();

const current = computed(() => allItems.find((m) => m.path === route.path));
</script>

<template>
  <div class="fixed inset-0 bg-transparent">
    <!-- ════════ Base window (back layer, no drop shadow) ════════ -->
    <div
      class="absolute top-[var(--base-top)] right-[var(--base-right)] bottom-[var(--base-bottom)] left-[var(--base-left)] rounded-window overflow-hidden border border-hairline bg-base"
    >
      <WindowControls class="absolute top-18px left-20px z-20" />

      <div class="absolute inset-0 ml-[var(--content-ml)] flex flex-col">
        <header
          class="h-64px shrink-0 flex items-center justify-between px-24px border-b border-hairline"
          data-tauri-drag-region
        >
          <div class="flex items-center gap-11px">
            <span class="text-20px text-accent-2" :class="current?.icon" />
            <h1 class="text-18px font-600 font-display">
              {{ current?.label ?? "HostGuard" }}
            </h1>
          </div>
          <div class="flex gap-8px">
            <button class="ghost-btn" title="搜索">
              <span class="i-ph-magnifying-glass-bold" />
            </button>
            <button class="ghost-btn" title="通知">
              <span class="i-ph-bell-bold" />
            </button>
          </div>
        </header>

        <div class="flex-1 overflow-auto p-24px">
          <RouterView v-slot="{ Component }">
            <Transition
              mode="out-in"
              enter-active-class="transition-all duration-200 ease-out"
              enter-from-class="op-0 translate-y-8px"
              leave-active-class="transition-all duration-200 ease-out"
              leave-to-class="op-0 -translate-y-6px"
            >
              <component :is="Component" />
            </Transition>
          </RouterView>
        </div>
      </div>
    </div>

    <!-- ════════ Floating menu panel (overhangs left, ~80% of base height) ════════ -->
    <aside
      class="absolute left-[var(--panel-left)] top-[var(--panel-top)] w-[var(--panel-w)] h-[var(--panel-h)] z-5 flex flex-col pt-14px px-12px pb-12px rounded-[18px] border border-[var(--c-panel-border)] bg-panel shadow-panel overflow-hidden"
    >
      <div class="flex items-center gap-9px px-4px pb-14px" data-tauri-drag-region>
        <div
          class="w-28px h-28px rounded-[8px] grid place-items-center text-17px text-[#04130b] bg-grad-accent shadow-[inset_0_1px_0_rgba(255,255,255,0.35)]"
        >
          <span class="i-ph-shield-check-duotone" />
        </div>
        <span class="font-display font-700 text-15px tracking-[0.2px]">HostGuard</span>
      </div>

      <SideBar class="flex-1 min-h-0" />

      <div
        class="flex items-center gap-9px h-38px mt-8px px-12px rounded-md bg-elevated text-[12.5px] text-fg-2"
      >
        <span
          class="w-8px h-8px rounded-full bg-accent-2 shadow-[0_0_0_4px_var(--c-accent-soft)]"
        />
        <span class="flex-1">系统正常</span>
        <span class="i-ph-heartbeat-duotone text-16px text-accent-2" />
      </div>
    </aside>

    <!-- ════════ Context action button (centred over the content column) ════════ -->
    <Transition
      enter-active-class="transition-all duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)]"
      enter-from-class="op-0 translate-y-8px scale-90"
      leave-active-class="transition-all duration-200 ease-in"
      leave-to-class="op-0 translate-y-6px scale-95"
    >
      <button
        v-if="fab.action"
        class="absolute bottom-[16px] left-[var(--fab-center-x)] -translate-x-1/2 z-10 inline-flex items-center justify-center min-w-[140px] h-46px px-22px rounded-full border-none cursor-pointer text-[#04130b] font-600 text-14px bg-grad-accent shadow-fab transition-transform duration-200 ease-[cubic-bezier(0.34,1.56,0.64,1)] hover:-translate-y-2px active:scale-95"
        @click="fab.trigger()"
      >
        <Transition
          mode="out-in"
          enter-active-class="transition-all duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)]"
          enter-from-class="op-0 scale-75 blur-[3px]"
          leave-active-class="transition-all duration-150 ease-in"
          leave-to-class="op-0 scale-90 blur-[2px]"
        >
          <span :key="fab.status" class="inline-flex items-center gap-8px">
            <span
              v-if="fab.status === 'loading'"
              class="i-ph-circle-notch-bold animate-spin text-20px"
            />
            <template v-else-if="fab.status === 'success'">
              <span class="i-ph-check-bold text-19px" /><span>完成</span>
            </template>
            <template v-else-if="fab.status === 'error'">
              <span class="i-ph-x-bold text-19px" /><span>失败</span>
            </template>
            <template v-else>
              <span class="text-19px" :class="fab.action?.icon" /><span>{{
                fab.action?.label
              }}</span>
            </template>
          </span>
        </Transition>
      </button>
    </Transition>
  </div>
</template>
