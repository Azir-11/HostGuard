# GitHub 自动更新 + Tag 发布 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 HostGuard 通过打 tag 触发 CI 发布 GitHub Release，并内置基于 GitHub Releases 的应用内更新能力；先发内置更新器的 `0.0.10`，再发 `0.1.0` 并真实测试 `0.0.10 → 0.1.0` 自动升级。

**Architecture:** Tauri v2 `tauri-plugin-updater` + `tauri-plugin-process`，更新源为 GitHub Releases 的 `latest.json`（`tauri-action` 自动生成上传）；前端在 Settings 页手动触发，逻辑收敛在 `useUpdater` 组合式函数；新增 `release.yml` 工作流用 `tauri-apps/tauri-action@v0` 在 macOS(arm64)+Windows 矩阵上构建、签名、发布。

**Tech Stack:** Tauri 2 (Rust), Vue 3 + Naive UI + UnoCSS, pnpm, GitHub Actions, `tauri-apps/tauri-action@v0`, Tauri minisign 更新签名。

**无测试套件：** 本仓库无单元测试（见 CLAUDE.md）。每个任务的验证门用 `pnpm build` / `pnpm lint` / `pnpm format:check`，涉及 Rust 处用 `cargo check`（在 `src-tauri`）或本地 `pnpm tauri dev`/`pnpm tauri build`；更新链路的端到端测试在 Phase 2 手动完成。

**关键事实（已核实，2026-06）：**

- `tauri-apps/tauri-action@v0`（最新 `action-v0.6.2`）。
- `macos-latest` 现为 Apple Silicon(arm64)；用 `--target aarch64-apple-darwin` 构建 M 芯片包。
- `tauri-action` 的 `uploadUpdaterJson` 默认 `true` → 当 `createUpdaterArtifacts:true` 时自动生成并上传 `latest.json`；`releaseDraft` 默认 `false`。
- 更新器签名（minisign）与 Apple 公证无关；macOS 包未公证，首次打开需右键→打开。

---

## 文件结构

| 文件                                  | 责任                                                      | 动作 |
| ------------------------------------- | --------------------------------------------------------- | ---- |
| `src-tauri/Cargo.toml`                | 后端依赖、版本号                                          | 改   |
| `src-tauri/src/lib.rs`                | 注册 updater / process 插件                               | 改   |
| `src-tauri/tauri.conf.json`           | `plugins.updater` + `createUpdaterArtifacts` + 版本号     | 改   |
| `src-tauri/capabilities/default.json` | 放行 updater / restart 权限                               | 改   |
| `package.json`                        | 版本号                                                    | 改   |
| `src/composables/useUpdater.ts`       | 更新检查/下载/安装/重启的全部逻辑（单一职责，可独立测试） | 建   |
| `src/views/settings/index.vue`        | 动态版本号 + 「检查更新」UI，调用 `useUpdater`            | 改   |
| `.github/workflows/release.yml`       | 打 tag 触发的发布工作流                                   | 建   |
| `.gitignore`                          | 忽略本地签名私钥                                          | 改   |

---

# Phase 0 — 基础设施 + 功能（不发布）

## Task 1: 添加后端依赖并注册插件

**Files:**

- Modify: `src-tauri/Cargo.toml`（`[dependencies]` 段）
- Modify: `src-tauri/src/lib.rs:132-150`（builder 链）

- [ ] **Step 1: 添加 Rust 依赖**

编辑 `src-tauri/Cargo.toml`，在 `[dependencies]` 段 `tauri-plugin-opener = "2"` 下一行加入：

```toml
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

- [ ] **Step 2: 注册插件**

编辑 `src-tauri/src/lib.rs` 的 `run()`，在 `.plugin(tauri_plugin_opener::init())` 之后追加两行：

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Mutex::new(init_system()))
```

- [ ] **Step 3: 编译验证（依赖能解析、插件注册无误）**

Run（PowerShell，仓库根目录）：

```
pnpm --filter . exec true; cargo check --manifest-path src-tauri/Cargo.toml
```

或直接：`cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过（首次会下载 `tauri-plugin-updater` / `tauri-plugin-process` 及依赖），无 error。

- [ ] **Step 4: 添加前端依赖**

Run：

```
pnpm add @tauri-apps/plugin-updater @tauri-apps/plugin-process
```

Expected: `package.json` 的 `dependencies` 新增这两个包，`pnpm-lock.yaml` 更新。

- [ ] **Step 5: Commit**

```
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/Cargo.lock package.json pnpm-lock.yaml
git commit -m "feat(update): 接入 tauri updater 与 process 插件"
```

---

## Task 2: 生成签名密钥并配置 tauri.conf.json

**Files:**

- Create: `src-tauri/.tauri/hostguard.key`（私钥，**不提交**）+ `hostguard.key.pub`（公钥）
- Modify: `src-tauri/tauri.conf.json`
- Modify: `.gitignore`

- [ ] **Step 1: 先忽略私钥目录（防止误提交）**

编辑仓库根 `.gitignore`（若无则创建），追加：

```
# Tauri updater signing keys — 私钥绝不入库
src-tauri/.tauri/
*.key
*.key.pub
```

- [ ] **Step 2: 生成密钥对**

Run（PowerShell，仓库根目录；`-p` 后是私钥口令，自行设定并记住）：

```
pnpm tauri signer generate -w src-tauri/.tauri/hostguard.key -p "REPLACE_WITH_A_PASSWORD"
```

Expected: 生成 `src-tauri/.tauri/hostguard.key`（私钥）与 `src-tauri/.tauri/hostguard.key.pub`（公钥），终端打印 public key。**记下你设的口令。**

> 说明：`-w` 写入私钥文件，命令同时输出公钥到 `<path>.pub`。私钥内容 + 口令稍后作为 GitHub Secret（Task 7）。

- [ ] **Step 3: 读取公钥内容**

Run：

```
Get-Content src-tauri/.tauri/hostguard.key.pub
```

复制其完整内容（一行 base64）。

- [ ] **Step 4: 写入 updater 配置 + 开启更新产物**

编辑 `src-tauri/tauri.conf.json`：

1. 在 `bundle` 对象内加入 `"createUpdaterArtifacts": true`：

```json
  "bundle": {
    "active": true,
    "createUpdaterArtifacts": true,
    "targets": ["app", "dmg", "nsis"],
```

2. 在顶层（与 `app`、`bundle` 同级）新增 `plugins` 段，`PASTE_PUBKEY_HERE` 换成 Step 3 的公钥内容：

```json
  "plugins": {
    "updater": {
      "pubkey": "PASTE_PUBKEY_HERE",
      "endpoints": [
        "https://github.com/Azir-11/HostGuard/releases/latest/download/latest.json"
      ]
    }
  },
```

- [ ] **Step 5: 验证配置合法 + 私钥未被追踪**

Run：

```
git status --porcelain
```

Expected: 输出里**不得**出现 `src-tauri/.tauri/` 或任何 `.key` 文件（被忽略了）；应出现 `tauri.conf.json`、`.gitignore` 改动。

Run（确认 JSON 合法 / 类型检查仍过）：

```
pnpm build
```

Expected: vite build + vue-tsc 通过（此步不依赖 updater，仅确认无 JSON 语法破坏）。

- [ ] **Step 6: Commit**

```
git add src-tauri/tauri.conf.json .gitignore
git commit -m "feat(update): 配置 GitHub Releases 更新源与签名公钥"
```

---

## Task 3: 放行更新器与重启权限

**Files:**

- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 添加权限**

编辑 `src-tauri/capabilities/default.json`，在 `permissions` 数组末尾（`"core:window:allow-is-maximized"` 之后）追加：

```json
    "core:window:allow-is-maximized",
    "updater:default",
    "process:allow-restart"
```

完整 `permissions` 应为：

```json
  "permissions": [
    "core:default",
    "opener:default",
    "core:window:allow-start-dragging",
    "core:window:allow-close",
    "core:window:allow-minimize",
    "core:window:allow-maximize",
    "core:window:allow-unmaximize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-is-maximized",
    "updater:default",
    "process:allow-restart"
  ]
```

> `relaunch()` 对应底层 `restart` 命令，由 `process:allow-restart` 放行（仅需重启，不放行 exit）。`updater:default` 含 check/download/install。

- [ ] **Step 2: 编译验证（权限标识符有效）**

Run：

```
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 通过。若权限名拼错，capability 校验会在编译期报错。

- [ ] **Step 3: Commit**

```
git add src-tauri/capabilities/default.json
git commit -m "feat(update): 放行 updater 与进程重启权限"
```

---

## Task 4: 新增 `useUpdater` 组合式函数

**Files:**

- Create: `src/composables/useUpdater.ts`

- [ ] **Step 1: 创建文件**

写入 `src/composables/useUpdater.ts`（完整内容）：

```ts
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
```

- [ ] **Step 2: 类型检查**

Run：

```
pnpm typecheck
```

Expected: 通过（vue-tsc 无 error）。若报 `@tauri-apps/plugin-updater` 找不到类型，确认 Task 1 Step 4 已安装依赖。

- [ ] **Step 3: 格式 + lint**

Run：

```
pnpm format && pnpm lint
```

Expected: 无错误。

- [ ] **Step 4: Commit**

```
git add src/composables/useUpdater.ts
git commit -m "feat(update): 新增 useUpdater 组合式函数封装更新逻辑"
```

---

## Task 5: Settings 页接入动态版本号与「检查更新」UI

**Files:**

- Modify: `src/views/settings/index.vue`

- [ ] **Step 1: 重写文件**

将 `src/views/settings/index.vue` 整体替换为（保留原「外观」段，关于段改为动态版本 + 检查更新行）：

```vue
<script setup lang="ts">
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { onMounted, ref } from "vue";
import { useUpdater } from "@/composables/useUpdater";
import { useAppStore } from "@/store/app";

const appStore = useAppStore();
const message = useMessage();

const version = ref("");
onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    /* web dev 环境下无 Tauri，忽略 */
  }
});

const { status, newVersion, releaseNotes, progress, errorMsg, checkForUpdate, downloadAndInstall } =
  useUpdater();

async function onCheck() {
  await checkForUpdate();
  if (status.value === "uptodate") message.success("已是最新版本");
  else if (status.value === "error") message.error(`检查更新失败：${errorMsg.value}`);
}

async function onInstall() {
  await downloadAndInstall();
  if (status.value === "error") message.error(`更新失败：${errorMsg.value}`);
}

async function openRepo() {
  try {
    await openUrl("https://github.com/Azir-11/HostGuard");
  } catch {
    /* noop outside Tauri */
  }
}
</script>

<template>
  <div class="flex flex-col gap-18px max-w-680px">
    <section class="rounded-lg border border-hairline bg-elevated overflow-hidden">
      <h3 class="px-18px pt-15px pb-9px text-13px font-600 text-fg-2 tracking-[0.3px]">外观</h3>
      <div
        class="flex items-center justify-between gap-16px px-18px py-14px border-t border-hairline"
      >
        <div class="flex flex-col gap-3px min-w-0">
          <span class="text-14px font-500">主题</span>
          <span class="text-[12.5px] text-fg-3">在深色与浅色界面之间切换</span>
        </div>
        <div class="flex gap-4px p-4px rounded-md border border-hairline bg-[var(--c-bg-0)]">
          <button
            class="inline-flex items-center gap-6px px-14px py-7px rounded-sm border-none cursor-pointer text-13px transition-colors"
            :class="
              appStore.isDark
                ? 'bg-grad-accent !text-[#04130b] font-600'
                : 'bg-transparent text-fg-2'
            "
            @click="appStore.setDark(true)"
          >
            <span class="i-ph-moon-stars-duotone" /> 深色
          </button>
          <button
            class="inline-flex items-center gap-6px px-14px py-7px rounded-sm border-none cursor-pointer text-13px transition-colors"
            :class="
              !appStore.isDark
                ? 'bg-grad-accent !text-[#04130b] font-600'
                : 'bg-transparent text-fg-2'
            "
            @click="appStore.setDark(false)"
          >
            <span class="i-ph-sun-duotone" /> 浅色
          </button>
        </div>
      </div>
    </section>

    <section class="rounded-lg border border-hairline bg-elevated overflow-hidden">
      <h3 class="px-18px pt-15px pb-9px text-13px font-600 text-fg-2 tracking-[0.3px]">关于</h3>
      <div
        class="flex items-center justify-between gap-16px px-18px py-14px border-t border-hairline"
      >
        <div class="flex flex-col gap-3px min-w-0">
          <span class="text-14px font-500">HostGuard</span>
          <span class="text-[12.5px] text-fg-3">hosts、Shell 与系统配置管理工具</span>
        </div>
        <span class="px-10px py-4px rounded-[8px] font-mono text-13px text-fg-2 bg-elevated-2">
          v{{ version || "—" }}
        </span>
      </div>

      <div class="flex flex-col gap-10px px-18px py-14px border-t border-hairline">
        <div class="flex items-center justify-between gap-16px">
          <div class="flex flex-col gap-3px min-w-0">
            <span class="flex items-center gap-7px text-14px font-500">
              <span class="i-ph-arrows-clockwise-duotone" /> 软件更新
            </span>
            <span class="text-[12.5px] text-fg-3">
              {{
                status === "available"
                  ? `发现新版本 v${newVersion}`
                  : status === "downloading"
                    ? `正在下载更新 ${progress}%`
                    : "从 GitHub 检查最新版本"
              }}
            </span>
          </div>
          <button
            v-if="status !== 'available' && status !== 'downloading'"
            class="inline-flex items-center gap-6px px-14px py-7px rounded-sm border border-hairline bg-[var(--c-bg-0)] cursor-pointer text-13px text-fg-2 transition-colors hover:bg-elevated-2 disabled:opacity-50 disabled:cursor-default"
            :disabled="status === 'checking'"
            @click="onCheck"
          >
            <span
              :class="
                status === 'checking'
                  ? 'i-ph-spinner-duotone animate-spin'
                  : 'i-ph-arrows-clockwise-bold'
              "
            />
            {{ status === "checking" ? "检查中…" : "检查更新" }}
          </button>
          <button
            v-else-if="status === 'available'"
            class="inline-flex items-center gap-6px px-14px py-7px rounded-sm border-none cursor-pointer text-13px font-600 bg-grad-accent !text-[#04130b] transition-opacity hover:opacity-90"
            @click="onInstall"
          >
            <span class="i-ph-download-simple-bold" /> 下载并安装
          </button>
          <span v-else class="inline-flex items-center gap-6px px-14px py-7px text-13px text-fg-3">
            <span class="i-ph-spinner-duotone animate-spin" /> {{ progress }}%
          </span>
        </div>

        <div
          v-if="status === 'available' && releaseNotes"
          class="rounded-md border border-hairline bg-[var(--c-bg-0)] px-12px py-9px text-[12.5px] text-fg-3 whitespace-pre-wrap max-h-160px overflow-auto"
        >
          {{ releaseNotes }}
        </div>
      </div>

      <div
        class="flex items-center justify-between gap-16px px-18px py-14px border-t border-hairline cursor-pointer transition-colors hover:bg-elevated-2"
        @click="openRepo"
      >
        <div class="flex flex-col gap-3px min-w-0">
          <span class="flex items-center gap-7px text-14px font-500">
            <span class="i-ph-github-logo-duotone" /> 源代码
          </span>
          <span class="text-[12.5px] text-fg-3">github.com/Azir-11/HostGuard</span>
        </div>
        <span class="i-ph-arrow-up-right-bold text-15px text-fg-3" />
      </div>
    </section>
  </div>
</template>
```

> 用到的图标 `i-ph-arrows-clockwise-duotone`、`i-ph-arrows-clockwise-bold`、`i-ph-spinner-duotone`、`i-ph-download-simple-bold` 均在 `.vue` 模板里直接出现，会被 UnoCSS 扫描，无需加 safelist。`useMessage` 自动导入。`animate-spin` 是 UnoCSS preset 内置动画。

- [ ] **Step 2: 类型检查 + 构建**

Run：

```
pnpm build
```

Expected: vite build + vue-tsc 通过。

- [ ] **Step 3: 格式 + lint**

Run：

```
pnpm format && pnpm lint
```

Expected: 无错误。

- [ ] **Step 4: 本地肉眼验证（可选但推荐）**

Run：

```
pnpm tauri dev
```

打开「设置 → 关于」：应显示动态版本号 `v0.1.0`（当前文件里仍是 0.1.0），点「检查更新」按钮无崩溃（此时尚无 release，会走 error/uptodate 分支，提示即可）。关闭。

- [ ] **Step 5: Commit**

```
git add src/views/settings/index.vue
git commit -m "feat(update): 设置页接入动态版本号与检查更新入口"
```

---

## Task 6: 新增发布工作流 `release.yml`

**Files:**

- Create: `.github/workflows/release.yml`

- [ ] **Step 1: 创建工作流**

写入 `.github/workflows/release.yml`（完整内容）：

```yaml
name: Release

# 打 v* tag 时触发：构建 macOS(arm64) + Windows 安装包并发布到 GitHub Release。
on:
  push:
    tags:
      - "v*"

jobs:
  release:
    name: release (${{ matrix.platform }})
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest # 现为 Apple Silicon(arm64)，对应 M 芯片
            args: --target aarch64-apple-darwin
          - platform: windows-latest
            args: ""
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v5

      - name: Install pnpm
        uses: pnpm/action-setup@v5

      - name: Setup Node
        uses: actions/setup-node@v5
        with:
          node-version: 22
          cache: pnpm

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin' || '' }}

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Install JS deps
        run: pnpm install --frozen-lockfile

      # tauri-action 会跑 beforeBuildCommand(pnpm build) → tauri build，
      # 用签名私钥签名更新产物，创建并发布 Release，自动上传 dmg/nsis/
      # 更新归档/.sig 以及 latest.json（uploadUpdaterJson 默认 true）。
      - name: Build and publish release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: HostGuard ${{ github.ref_name }}
          releaseBody: |
            ## 下载
            - **Windows**：`HostGuard_*_x64-setup.exe`
            - **macOS（Apple Silicon / M 芯片）**：`HostGuard_*_aarch64.dmg`

            > macOS 包未做 Apple 公证，首次打开请右键 →「打开」以绕过 Gatekeeper。
          releaseDraft: false
          prerelease: false
          args: ${{ matrix.args }}
```

> 说明：去掉了设计稿里提到的 `workflow_dispatch`——手动从分支触发时 `github.ref_name` 是分支名而非 tag，`tauri-action` 的 `tagName` 会拿到错误值。发布统一走「打 tag」。`releaseDraft:false` 让 Release 立即发布并成为 latest，`/releases/latest/download/latest.json` 才能被更新器解析。

- [ ] **Step 2: 校验 YAML 语法**

Run（PowerShell）：

```
pnpm dlx js-yaml .github/workflows/release.yml > $null; if ($?) { "yaml ok" }
```

Expected: 打印 `yaml ok`（无解析异常）。若 `js-yaml` 不可用，跳过——GitHub 会在推送后校验。

- [ ] **Step 3: Commit**

```
git add .github/workflows/release.yml
git commit -m "ci(release): 新增打 tag 触发的 GitHub Release 发布工作流"
```

---

## Task 7: 【用户操作】添加 GitHub Secrets（人工，非代码）

> 这一步**只能由仓库所有者完成**，且必须在 push tag 之前完成，否则 CI 签名步骤会失败。

- [ ] **Step 1: 取私钥内容**

Run：

```
Get-Content src-tauri/.tauri/hostguard.key
```

复制整段内容。

- [ ] **Step 2: 在 GitHub 网页添加两个 Secret**

打开 `https://github.com/Azir-11/HostGuard/settings/secrets/actions` → New repository secret，添加：

- `TAURI_SIGNING_PRIVATE_KEY` = Step 1 的私钥完整内容
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = Task 2 Step 2 设定的口令（若没设口令则留空字符串）

- [ ] **Step 3: 确认**

在 Secrets 列表中能看到这两项即可。

---

# Phase 1 — 发布 0.0.10

## Task 8: 版本号统一改为 0.0.10 并打 tag

**Files:**

- Modify: `package.json:3`、`src-tauri/tauri.conf.json:4`、`src-tauri/Cargo.toml:3`

- [ ] **Step 1: 改三处版本号**

- `package.json`: `"version": "0.1.0"` → `"version": "0.0.10"`
- `src-tauri/tauri.conf.json`: `"version": "0.1.0"` → `"version": "0.0.10"`
- `src-tauri/Cargo.toml`: `version = "0.1.0"` → `version = "0.0.10"`

- [ ] **Step 2: 更新 Cargo.lock 中的本包版本**

Run：

```
cargo update -p hostguard --manifest-path src-tauri/Cargo.toml --precise 0.0.10
```

若该命令不适用（本地包），改用 `cargo check --manifest-path src-tauri/Cargo.toml` 让 lock 自动同步。
Expected: `src-tauri/Cargo.lock` 里 `name = "hostguard"` 的 `version` 变为 `0.0.10`。

- [ ] **Step 3: 构建验证**

Run：

```
pnpm build
```

Expected: 通过。

- [ ] **Step 4: 提交并打 tag**

```
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore(release): 0.0.10"
git tag v0.0.10
```

- [ ] **Step 5: 确认 Secrets 已就绪（Task 7 完成）后推送**

```
git push origin main
git push origin v0.0.10
```

Expected: push tag 触发 `Release` 工作流。

---

## Task 9: 验证 0.0.10 Release

- [ ] **Step 1: 看 CI**

打开 `https://github.com/Azir-11/HostGuard/actions`，确认 `Release` 工作流 macOS + Windows 两个 job 均成功。
若签名步骤报 `TAURI_SIGNING_PRIVATE_KEY` 相关错误 → 回到 Task 7 检查 Secrets。

- [ ] **Step 2: 看 Release 资产**

打开 `https://github.com/Azir-11/HostGuard/releases/tag/v0.0.10`，确认存在：

- `HostGuard_0.0.10_x64-setup.exe`（Windows）
- `HostGuard_0.0.10_aarch64.dmg`（macOS M 芯片）
- `latest.json`、以及 `.sig` / `.app.tar.gz` 等更新产物

- [ ] **Step 3: 验证 latest.json 可达**

浏览器打开 `https://github.com/Azir-11/HostGuard/releases/latest/download/latest.json`，应能下载/显示 JSON，且 `version` 为 `0.0.10`。

> **Part 1 交付完成**：Release 页含 Win + Mac(M 芯片) 可下载链接。

---

# Phase 2 — 发布 0.1.0 并测试自动更新

## Task 10: 版本号 bump 到 0.1.0 并打 tag

**Files:**

- Modify: `package.json:3`、`src-tauri/tauri.conf.json:4`、`src-tauri/Cargo.toml:3`

- [ ] **Step 1: 改三处版本号为 0.1.0**

- `package.json`: `0.0.10` → `0.1.0`
- `src-tauri/tauri.conf.json`: `0.0.10` → `0.1.0`
- `src-tauri/Cargo.toml`: `0.0.10` → `0.1.0`

- [ ] **Step 2: 同步 lock + 构建**

```
cargo check --manifest-path src-tauri/Cargo.toml
pnpm build
```

Expected: 通过，`Cargo.lock` 中本包版本变 `0.1.0`。

- [ ] **Step 3: 提交、打 tag、推送**

```
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore(release): 0.1.0"
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

Expected: 触发 `Release` 工作流。

---

## Task 11: 验证 0.1.0 Release

- [ ] **Step 1: 确认工作流成功 + 资产齐全**

`https://github.com/Azir-11/HostGuard/releases/tag/v0.1.0` 含 `HostGuard_0.1.0_x64-setup.exe`、`HostGuard_0.1.0_aarch64.dmg`、`latest.json`、`.sig`。

- [ ] **Step 2: 确认 latest.json 指向 0.1.0**

`https://github.com/Azir-11/HostGuard/releases/latest/download/latest.json` 的 `version` 现为 `0.1.0`。

---

## Task 12: 端到端测试自动更新（0.0.10 → 0.1.0）

> 这是用户最初想要的「基于上一个 release 测试更新」。

- [ ] **Step 1: 安装 0.0.10**

从 0.0.10 Release 下载并安装对应平台的包（Windows 装 `setup.exe`；macOS 装 dmg，首次右键→打开）。**不要**用本地 dev 版本——要用真正发布的 0.0.10 二进制。

- [ ] **Step 2: 触发更新**

打开已安装的 HostGuard（0.0.10）→ 设置 → 关于 → 点「检查更新」。
Expected: 提示「发现新版本 v0.1.0」，显示 release notes 与「下载并安装」按钮。

- [ ] **Step 3: 安装并重启**

点「下载并安装」→ 进度条到 100% → 应用自动重启。
Expected: 重启后「关于」里版本显示 `v0.1.0`。

- [ ] **Step 4: 记录结果**

- 成功：更新链路打通，全部完成。
- 失败排查：
  - 签名校验失败 → 0.0.10 内嵌 pubkey 与 0.1.0 签名私钥不匹配（Task 2/7 用了不同密钥）。两个版本必须用同一密钥对。
  - 404 latest.json → Release 不是 published-latest（检查 `releaseDraft:false`、非 prerelease）。
  - macOS 更新后无法启动 → 未公证的已知边角情况；本测试中通常可正常完成（仅首次安装需右键打开）。

---

## 自检（写计划后回查 spec）

- **Spec 覆盖**：①打 tag 发 0.0.10 含 Win+Mac(M) 下载 → Task 6/8/9。②基于 GitHub 的应用内更新 → Task 1–5。③以上一 release 测试 → Task 12。④发带更新能力的 0.1.0 → Task 10/11。⑤两个 Secret 由用户加 → Task 7。⑥版本三处同步 → Task 8/10。全部命中。
- **占位符**：无 TBD/TODO；所有代码块为完整内容。
- **类型一致**：`useUpdater` 暴露的 `status/newVersion/releaseNotes/progress/errorMsg/checkForUpdate/downloadAndInstall` 与 settings 解构使用名一致；`UpdaterStatus` 字面量在 view 比较处一致（`idle/checking/available/downloading/uptodate/error`）。
- **与 spec 的偏差**：发布工作流去掉 `workflow_dispatch`（理由见 Task 6 Step 1 注），其余一致。

```

```
