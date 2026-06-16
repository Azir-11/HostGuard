<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useMessage } from "naive-ui";
import { useFabStore } from "@/store/fab";
import { type HostEntry, type HostLine, newEntry, parseHosts, serializeHosts } from "@/utils/hosts";

const message = useMessage();
const fab = useFabStore();

const lines = ref<HostLine[]>([]);
const loading = ref(true);
const error = ref("");
const search = ref("");
const writable = ref(false);
const flushing = ref(false);
const hostsPath = ref("");
// Display label: never seed a platform-specific literal. Falls back to a neutral
// term until the backend `hosts_path` resolves (and if it ever fails to).
const hostsLabel = computed(() => hostsPath.value || "系统 hosts 文件");

const entries = computed(() => lines.value.filter((l): l is HostEntry => l.type === "entry"));

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return entries.value;
  return entries.value.filter(
    (e) =>
      e.ip.toLowerCase().includes(q) ||
      e.hosts.toLowerCase().includes(q) ||
      e.comment.toLowerCase().includes(q),
  );
});

const enabledCount = computed(() => entries.value.filter((e) => e.enabled).length);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    lines.value = parseHosts(await invoke<string>("read_hosts"));
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function checkPerm() {
  try {
    writable.value = await invoke<boolean>("hosts_writable");
  } catch {
    writable.value = false;
  }
}

function addEntry() {
  lines.value.push(newEntry());
}

function removeEntry(id: string) {
  lines.value = lines.value.filter((l) => l.id !== id);
}

async function grant(): Promise<boolean> {
  try {
    await invoke("grant_hosts_access");
    await checkPerm();
    if (writable.value) {
      message.success("已获得写入权限");
      return true;
    }
    return false;
  } catch (e) {
    const msg = String(e);
    if (msg.includes("已取消")) message.info("已取消授权");
    else message.error(`授权失败：${msg}`);
    return false;
  }
}

async function save(): Promise<boolean> {
  if (!writable.value && !(await grant())) return false;
  try {
    // No reload afterwards: the in-memory list already reflects what we wrote,
    // so re-parsing would rebuild every row and make the list flicker.
    await invoke("write_hosts", { content: serializeHosts(lines.value) });
    message.success(`已保存到 ${hostsLabel.value}`);
    return true;
  } catch (e) {
    const msg = String(e);
    if (msg.includes("NO_PERMISSION")) {
      writable.value = false;
      message.warning("没有写入权限，请先授予权限");
    } else {
      message.error(`保存失败：${msg}`);
    }
    return false;
  }
}

// Flush the DNS resolver cache. First use installs a one-time, narrowly-scoped
// passwordless rule (single admin prompt); every flush after that is silent and
// survives app restarts.
async function flushDns() {
  if (flushing.value) return;
  flushing.value = true;
  try {
    let granted = await invoke<boolean>("dns_flush_granted");
    if (!granted) {
      await invoke("grant_dns_flush_access"); // one-time admin prompt
      granted = await invoke<boolean>("dns_flush_granted");
      if (!granted) {
        message.error("授权未完成，无法刷新");
        return;
      }
      message.success("已授权，之后刷新无需再次输入密码");
    }
    await invoke("flush_dns_cache");
    message.success("已刷新 DNS 缓存");
  } catch (e) {
    const msg = String(e);
    if (msg.includes("已取消")) message.info("已取消授权");
    else if (msg.includes("NO_PERMISSION")) message.warning("缺少权限，请重试以授权");
    else message.error(`刷新失败：${msg}`);
  } finally {
    flushing.value = false;
  }
}

const cols = "grid-cols-[44px_1fr_1.6fr_1fr_44px]";

function syncFab() {
  fab.set(
    writable.value
      ? { label: "保存", icon: "i-ph-floppy-disk-duotone", run: save }
      : { label: "授予权限", icon: "i-ph-lock-key-duotone", run: grant },
  );
}

watch(writable, syncFab);

onMounted(async () => {
  try {
    hostsPath.value = await invoke<string>("hosts_path");
  } catch {
    /* keep default */
  }
  await Promise.all([load(), checkPerm()]);
  syncFab();
});
onUnmounted(() => fab.clear());
</script>

<template>
  <div class="h-full flex flex-col gap-16px">
    <!-- toolbar -->
    <div class="flex items-center gap-12px">
      <NButton type="primary" @click="addEntry">
        <template #icon><span class="i-ph-plus-bold" /></template>
        添加条目
      </NButton>
      <NInput
        v-model:value="search"
        class="max-w-300px"
        placeholder="搜索 IP / 域名 / 注释"
        clearable
      >
        <template #prefix><span class="i-ph-magnifying-glass-bold op-50" /></template>
      </NInput>
      <div class="flex-1" />
      <span class="text-13px text-fg-3">{{ enabledCount }} / {{ entries.length }} 启用</span>
      <NButton quaternary :loading="flushing" title="清空系统 DNS 解析缓存" @click="flushDns">
        <template #icon><span class="i-ph-broom-bold" /></template>
        刷新 DNS 缓存
      </NButton>
      <NButton quaternary @click="load">
        <template #icon><span class="i-ph-arrow-clockwise-bold" /></template>
        重载
      </NButton>
    </div>

    <!-- permission banner -->
    <div
      v-if="!writable && !loading && !error"
      class="flex items-center gap-12px px-16px py-12px card"
    >
      <span class="i-ph-lock-key-duotone text-22px text-amber shrink-0" />
      <div class="flex-1 min-w-0">
        <div class="text-[13.5px] font-600">需要写入权限</div>
        <div class="text-12px text-fg-3">
          修改 {{ hostsLabel }} 需一次性管理员授权，授权后即可直接保存，无需重复输入密码。
        </div>
      </div>
      <NButton type="primary" size="small" @click="grant">
        <template #icon><span class="i-ph-lock-key-duotone" /></template>
        授予权限
      </NButton>
    </div>

    <!-- table -->
    <div class="flex-1 min-h-0 card overflow-hidden flex flex-col">
      <div
        class="grid items-center gap-10px px-16px h-42px shrink-0 border-b border-hairline text-12px font-600 text-fg-3"
        :class="cols"
      >
        <span>启用</span>
        <span>IP</span>
        <span>域名</span>
        <span>注释</span>
        <span />
      </div>

      <div v-if="loading" class="flex-1 grid place-items-center text-fg-3">
        <span class="i-ph-circle-notch-bold animate-spin text-28px" />
      </div>

      <div
        v-else-if="error"
        class="flex-1 flex flex-col items-center justify-center gap-12px p-24px text-center"
      >
        <span class="i-ph-warning-circle-duotone text-44px text-amber" />
        <p class="m-0 max-w-420px text-fg-2 leading-[1.6]">
          读取 {{ hostsLabel }} 失败：{{ error }}
        </p>
        <NButton @click="load">重试</NButton>
      </div>

      <div v-else-if="!filtered.length" class="flex-1 grid place-items-center text-fg-3">
        <span>暂无匹配条目</span>
      </div>

      <div v-else class="flex-1 overflow-auto">
        <div
          v-for="e in filtered"
          :key="e.id"
          class="grid items-center gap-10px px-16px py-8px border-b border-hairline transition-colors hover:bg-elevated"
          :class="[cols, { 'op-55': !e.enabled }]"
        >
          <NSwitch v-model:value="e.enabled" size="small" />
          <NInput v-model:value="e.ip" size="small" placeholder="127.0.0.1" />
          <NInput v-model:value="e.hosts" size="small" placeholder="example.local" />
          <NInput v-model:value="e.comment" size="small" placeholder="备注" />
          <NButton quaternary circle size="small" @click="removeEntry(e.id)">
            <template #icon>
              <span class="i-ph-trash text-fg-3 hover:text-tlclose" />
            </template>
          </NButton>
        </div>
      </div>
    </div>
  </div>
</template>
