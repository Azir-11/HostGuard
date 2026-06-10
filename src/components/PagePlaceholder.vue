<script setup lang="ts">
defineProps<{
  icon: string;
  title: string;
  desc: string;
}>();
</script>

<template>
  <div class="relative h-full flex flex-col items-center justify-center text-center gap-14px">
    <div
      class="absolute w-280px h-280px rounded-full blur-[8px] pointer-events-none bg-[radial-gradient(circle,var(--c-accent-soft),transparent_65%)]"
    />
    <div
      class="relative text-70px text-accent-2 drop-shadow-[0_8px_22px_var(--c-accent-glow)]"
      :class="icon"
    />
    <h2 class="relative text-22px font-600 font-display">{{ title }}</h2>
    <p class="relative max-w-380px m-0 text-fg-2 leading-[1.6]">{{ desc }}</p>
    <span
      class="relative inline-flex items-center gap-6px mt-6px px-14px py-6px rounded-full text-[12.5px] text-accent-2 bg-accent-soft border border-hairline"
    >
      <span class="i-ph-wrench-duotone" />
      即将上线
    </span>
  </div>
</template>
