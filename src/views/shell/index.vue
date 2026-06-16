<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useDialog, useMessage } from "naive-ui";
import CodeEditor from "@/components/CodeEditor.vue";
import { useAppStore } from "@/store/app";
import { useFabStore } from "@/store/fab";

interface ShellConfig {
  name: string;
  label: string;
  path: string;
  exists: boolean;
  reload_hint: string;
}

const message = useMessage();
const dialog = useDialog();
const appStore = useAppStore();
const fab = useFabStore();

const files = ref<ShellConfig[]>([]);
const active = ref("");
const content = ref("");
const original = ref("");

const dirty = computed(() => content.value !== original.value);
const activeConfig = computed(() => files.value.find((f) => f.name === active.value));
const activePath = computed(() => activeConfig.value?.path ?? "");
const reloadHint = computed(() => activeConfig.value?.reload_hint ?? "");

async function refreshList() {
  try {
    files.value = await invoke<ShellConfig[]>("list_shell_configs");
  } catch {
    /* not in Tauri */
  }
}

async function loadFile(name: string) {
  try {
    const text = await invoke<string>("read_shell_config", { name });
    content.value = text;
    original.value = text;
    active.value = name;
  } catch (e) {
    message.error(`读取失败：${String(e)}`);
  }
}

function switchFile(name: string) {
  if (name === active.value) return;
  if (dirty.value) {
    dialog.warning({
      title: "未保存的更改",
      content: `「${activeConfig.value?.label ?? active.value}」有未保存的更改，切换将丢弃，确定继续？`,
      positiveText: "丢弃并切换",
      negativeText: "取消",
      onPositiveClick: () => loadFile(name),
    });
  } else {
    loadFile(name);
  }
}

async function save(): Promise<boolean> {
  try {
    await invoke("write_shell_config", { name: active.value, content: content.value });
    original.value = content.value;
    await refreshList();
    message.success(`已保存 ${activePath.value}`);
    return true;
  } catch (e) {
    message.error(`保存失败：${String(e)}`);
    return false;
  }
}

async function copySource() {
  try {
    await navigator.clipboard.writeText(reloadHint.value);
    message.success("已复制命令");
  } catch {
    message.info(reloadHint.value);
  }
}

onMounted(async () => {
  await refreshList();
  const first = files.value[0]?.name;
  if (first) await loadFile(first);
  fab.set({ label: "保存", icon: "i-ph-floppy-disk-duotone", run: save });
});
onUnmounted(() => fab.clear());
</script>

<template>
  <div class="h-full flex flex-col gap-14px">
    <!-- file tabs + toolbar -->
    <div class="flex items-center gap-12px">
      <div class="flex gap-4px p-4px rounded-md border border-hairline bg-[var(--c-bg-0)]">
        <button
          v-for="f in files"
          :key="f.name"
          class="inline-flex items-center gap-6px px-12px py-7px rounded-sm border-none cursor-pointer text-13px font-mono transition-colors"
          :class="
            active === f.name
              ? 'bg-grad-accent !text-[#04130b] font-600'
              : 'bg-transparent text-fg-2 hover:text-fg'
          "
          @click="switchFile(f.name)"
        >
          {{ f.label }}
          <span v-if="!f.exists" class="w-5px h-5px rounded-full bg-fg-3" title="尚未创建" />
        </button>
      </div>
      <div class="flex-1" />
      <span v-if="dirty" class="inline-flex items-center gap-6px text-12px text-amber">
        <span class="w-7px h-7px rounded-full bg-amber" /> 未保存
      </span>
      <NButton quaternary @click="loadFile(active)">
        <template #icon><span class="i-ph-arrow-clockwise-bold" /></template>
        重载
      </NButton>
    </div>

    <!-- reload hint -->
    <div class="flex items-center gap-10px px-14px py-9px card text-12px text-fg-2">
      <span class="i-ph-info-duotone text-16px text-accent-2 shrink-0" />
      <span class="flex-1">
        保存后需在终端执行
        <code class="font-mono text-fg">{{ reloadHint }}</code>
        或重启终端生效。
      </span>
      <button
        class="inline-flex items-center gap-5px border-none bg-transparent cursor-pointer text-12px text-accent-2 hover:text-accent"
        @click="copySource"
      >
        <span class="i-ph-copy-bold" /> 复制
      </button>
    </div>

    <!-- editor -->
    <div class="flex-1 min-h-0 card overflow-hidden">
      <CodeEditor v-model="content" :dark="appStore.isDark" />
    </div>
  </div>
</template>
