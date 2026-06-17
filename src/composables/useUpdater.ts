import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ref } from "vue";

export type UpdaterStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "uptodate"
  | "error";

/** 是否运行在 Tauri 宿主中（web 端 `pnpm dev` 下为 false，避免插件调用抛错）。 */
function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * 应用内更新：检查 → 展示 → 下载安装 → 重启。
 * 全部状态对外暴露为 ref，view 层只负责渲染，不持有任何更新逻辑。
 */
export function useUpdater() {
  const status = ref<UpdaterStatus>("idle");
  const newVersion = ref("");
  const releaseNotes = ref("");
  const progress = ref(0); // 0..100
  const errorMsg = ref("");

  // check() 返回的待安装更新句柄（无更新时为 null）。
  let pending: Awaited<ReturnType<typeof check>> = null;

  async function checkForUpdate(): Promise<void> {
    if (!inTauri()) {
      status.value = "uptodate";
      return;
    }
    status.value = "checking";
    errorMsg.value = "";
    try {
      const update = await check();
      if (update) {
        pending = update;
        newVersion.value = update.version;
        releaseNotes.value = update.body ?? "";
        status.value = "available";
      } else {
        status.value = "uptodate";
      }
    } catch (e) {
      status.value = "error";
      errorMsg.value = String(e);
    }
  }

  async function downloadAndInstall(): Promise<void> {
    if (!pending) return;
    status.value = "downloading";
    progress.value = 0;
    let total = 0;
    let downloaded = 0;
    try {
      await pending.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            progress.value = total ? Math.round((downloaded / total) * 100) : 0;
            break;
          case "Finished":
            progress.value = 100;
            break;
        }
      });
      // 安装完成后重启进入新版本。
      await relaunch();
    } catch (e) {
      status.value = "error";
      errorMsg.value = String(e);
    }
  }

  return {
    status,
    newVersion,
    releaseNotes,
    progress,
    errorMsg,
    checkForUpdate,
    downloadAndInstall,
  };
}
