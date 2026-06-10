<script setup lang="ts">
import { watchEffect } from "vue";
import { darkTheme } from "naive-ui";
import { useAppStore } from "@/store/app";
import DefaultLayout from "@/layouts/default/index.vue";

const appStore = useAppStore();

// Reflect the active theme onto <html> so the CSS design tokens switch.
watchEffect(() => {
  document.documentElement.dataset.theme = appStore.isDark ? "dark" : "light";
});
</script>

<template>
  <NConfigProvider :theme="appStore.isDark ? darkTheme : null">
    <NMessageProvider>
      <NDialogProvider>
        <DefaultLayout />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>
