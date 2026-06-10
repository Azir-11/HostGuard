<script setup lang="ts">
import { useRoute } from "vue-router";
import { bottomItem, groups, topItem } from "../menu";

const route = useRoute();

function isActive(path: string) {
  return route.path === path;
}
</script>

<template>
  <nav class="flex-1 flex flex-col gap-2px min-h-0 overflow-auto">
    <RouterLink
      :to="topItem.path"
      class="nav-item h-46px text-[14.5px] font-600 !text-fg bg-elevated mb-2px"
      :class="{ 'nav-active': isActive(topItem.path) }"
    >
      <span
        class="text-20px shrink-0"
        :class="[topItem.icon, isActive(topItem.path) ? 'text-accent-2' : 'op-90']"
      />
      <span class="whitespace-nowrap">{{ topItem.label }}</span>
    </RouterLink>

    <div v-for="group in groups" :key="group.title" class="flex flex-col gap-2px">
      <div class="mt-14px mb-4px px-12px text-[11.5px] font-600 tracking-[0.4px] text-fg-3">
        {{ group.title }}
      </div>
      <RouterLink
        v-for="item in group.items"
        :key="item.key"
        :to="item.path"
        class="nav-item"
        :class="{ 'nav-active': isActive(item.path) }"
      >
        <span
          class="text-20px shrink-0"
          :class="[item.icon, isActive(item.path) ? 'text-accent-2 op-100' : 'op-80']"
        />
        <span class="whitespace-nowrap">{{ item.label }}</span>
      </RouterLink>
    </div>

    <RouterLink
      :to="bottomItem.path"
      class="nav-item mt-auto"
      :class="{ 'nav-active': isActive(bottomItem.path) }"
    >
      <span
        class="text-20px shrink-0"
        :class="[bottomItem.icon, isActive(bottomItem.path) ? 'text-accent-2 op-100' : 'op-80']"
      />
      <span class="whitespace-nowrap">{{ bottomItem.label }}</span>
    </RouterLink>
  </nav>
</template>
