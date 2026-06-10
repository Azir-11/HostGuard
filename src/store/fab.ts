import { defineStore } from "pinia";
import { ref } from "vue";

export interface FabAction {
  label: string;
  icon: string;
  /** Return `false` to abort silently (idle); throw to show the error state. */
  run: () => boolean | void | Promise<boolean | void>;
}

export type FabStatus = "idle" | "loading" | "success" | "error";

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

// Deliberate timing so a fast op still reads as a graceful sequence.
const MIN_LOADING = 520;
const SUCCESS_HOLD = 1000;
const ERROR_HOLD = 1300;

/**
 * The floating action button is context-aware: each page registers its
 * primary action (重载 / 保存 …) on mount and clears it on unmount.
 * `trigger` drives an idle → loading → success/error → idle state machine.
 */
export const useFabStore = defineStore("fab", () => {
  const action = ref<FabAction | null>(null);
  const status = ref<FabStatus>("idle");

  function set(a: FabAction) {
    // Only swap the action; never interrupt an in-flight status sequence
    // (e.g. grant→save swap during the success animation).
    action.value = a;
  }

  function clear() {
    action.value = null;
    status.value = "idle";
  }

  async function trigger() {
    if (!action.value || status.value !== "idle") return;
    status.value = "loading";

    const run = async (): Promise<"ok" | "cancel" | "fail"> => {
      try {
        return (await action.value!.run()) === false ? "cancel" : "ok";
      } catch {
        return "fail";
      }
    };

    // Hold the spinner for at least MIN_LOADING for a consistent rhythm.
    const [result] = await Promise.all([run(), wait(MIN_LOADING)]);

    if (result === "ok") {
      status.value = "success";
      await wait(SUCCESS_HOLD);
    } else if (result === "fail") {
      status.value = "error";
      await wait(ERROR_HOLD);
    }
    status.value = "idle";
  }

  return { action, status, set, clear, trigger };
});
