import { defineStore } from "pinia";
import { ref } from "vue";

/** Global application state (theme, layout-level flags). */
export const useAppStore = defineStore("app", () => {
  // Dark-first, matching the CleanMyMac-style premium aesthetic.
  const isDark = ref(true);

  function toggleDark() {
    isDark.value = !isDark.value;
  }

  function setDark(value: boolean) {
    isDark.value = value;
  }

  return { isDark, toggleDark, setDark };
});
