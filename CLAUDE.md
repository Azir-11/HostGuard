# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

HostGuard is a Tauri 2 desktop app (Vue 3 frontend + Rust backend) that manages `hosts`, shell startup configs, and shows system telemetry, in a CleanMyMac-style frameless glassy UI.

Note: the README/TASKS.md describe it as a "macOS tool", but the code is **cross-platform (macOS + Windows)** — there is a full `windows.rs` platform backend and the CI builds/ships both. Recent work has been mostly Windows parity. Treat both platforms as first-class; never hardcode a platform assumption in shared code.

## Commands

```bash
pnpm install            # bootstrap (pnpm is pinned via packageManager)
pnpm tauri dev          # run the desktop app with hot reload (frontend + Rust)
pnpm dev                # frontend only (vite, no Tauri backend — invoke() calls fail gracefully)

pnpm build              # vite build + vue-tsc --noEmit (type check). The lib-level check.
pnpm typecheck          # vue-tsc --noEmit only
pnpm lint               # oxlint .
pnpm lint:fix           # oxlint . --fix
pnpm format             # oxfmt .  (run before committing)
pnpm format:check       # oxfmt --check . (what CI runs)

pnpm tauri build        # full package: compiles Rust + produces installers (dmg / nsis)
```

There is **no test suite**. CI (`.github/workflows/ci.yml`) runs on macOS + Windows and gates on: `pnpm build`, `pnpm lint`, `pnpm format:check`, then a full `pnpm tauri build`. Match those locally before pushing.

## Architecture

### Frontend ↔ backend boundary

The frontend never touches the filesystem or runs privileged commands directly. Everything goes through Tauri IPC: the frontend calls `invoke("command_name", { args })` (`@tauri-apps/api/core`), and each command is a `#[tauri::command]` registered in the `generate_handler!` list in `src-tauri/src/lib.rs`. **Adding a backend capability = define the command in `lib.rs` + register it in that list + (usually) implement it per-platform in `platform/`.**

### Platform abstraction (Rust)

`src-tauri/src/platform/mod.rs` holds the cross-platform logic (read/write/backup orchestration, the `ShellConfig` struct) and `#[cfg(target_os = ...)]`-dispatches to `macos.rs` or `windows.rs` via `use self::<os> as sys`. Each platform module implements the same surface: `hosts_path`, `hosts_writable`, `grant_hosts_access`, `shell_configs`, `shell_path`, `on_shell_saved`, `backup_path`, and the DNS-flush trio. When adding a platform-specific entry point, export it from both modules and re-export in `mod.rs`'s `pub use sys::{...}`.

Key platform divergences already encoded (preserve these — they fix real bugs, see git log):
- **Hosts write permission** is a one-time elevation that grants the *current user* persistent write access, so later saves need no prompt: macOS `chmod +a` ACL via `osascript ... with administrator privileges`; Windows `icacls /grant` via UAC-elevated `Start-Process -Verb RunAs`. Windows grants to the user's **SID** (resolved in the un-elevated process), not the bare username, to work on domain/AzureAD machines.
- **Shell configs**: macOS = `~/.zshrc`/`.zprofile`/`.zshenv`/`.zlogin`; Windows = PowerShell `$PROFILE` (queried from PowerShell itself so it follows version + OneDrive redirection) + a cmd AutoRun batch file (`on_shell_saved` points `HKCU\...\Command Processor\AutoRun` at it via `call "<path>"`).
- **DNS flush**: macOS needs a one-time narrowly-scoped passwordless sudoers rule (`dscacheutil` + `killall mDNSResponder`); Windows `ipconfig /flushdns` needs no elevation (its grant/granted commands are no-ops returning true).

All write paths back up the previous file to `~/.hostguard.<name>.bak` before overwriting. Privileged shell-outs use `spawn_blocking` so the modal UAC/osascript dialog never blocks the UI thread.

### System telemetry

A single `sysinfo::System` lives in Tauri managed state (`Mutex<System>`) so CPU deltas survive across polls (sysinfo needs two samples). `read_system_stats` refreshes only CPU + memory. The frontend polls only while the monitor page is mounted. Note: Windows reports no load average (returns 0).

### Frontend conventions

- **Auto-imports**: `vue`, `vue-router`, `pinia` APIs and all Naive UI components (`NButton`, `NInput`, `useMessage`, …) are auto-imported via `unplugin-auto-import` / `unplugin-vue-components`. Don't add explicit imports for them. Generated dts: `src/types/auto-imports.d.ts`, `src/types/components.d.ts` (don't edit; they're lint/format-ignored).
- **Path alias**: `@` → `src`.
- **Router**: hash history (`createWebHashHistory`) to avoid 404 on reload inside the webview. Views are lazy-loaded under `src/views/<name>/index.vue`; nav model lives in `src/layouts/default/menu.ts`.
- **Styling is UnoCSS-only — no `<style>` blocks.** All design tokens are CSS variables in `src/styles/theme.css` (dark default + `[data-theme="light"]`), surfaced to UnoCSS as theme colors/shortcuts/rules in `uno.config.ts`. Theme switches by setting `document.documentElement.dataset.theme` (App.vue), which also drives Naive UI's `darkTheme`.
- **Icons**: Phosphor (`i-ph-*`) / Carbon via `presetIcons`. Icons referenced only in `.ts` data files (e.g. `menu.ts`) aren't scanned by UnoCSS's pipeline — they must be added to the `safelist` in `uno.config.ts` or they won't render.
- **Frameless layered window**: `decorations:false` + `transparent:true` (tauri.conf.json). The layout (`layouts/default/index.vue`) hand-builds the window: an inset base window + an overhanging floating menu panel, geometry driven by `--base-*` / `--panel-*` / `--content-ml` / `--fab-center-x` CSS vars. Custom window controls (`WindowControls.vue`) render macOS traffic-lights vs Windows square buttons, branched on `isWindows`/`isMacOS` from `composables/usePlatform.ts` (UA sniffing — cheap, sync, for chrome appearance only). `data-tauri-drag-region` marks draggable areas.
- **Context-aware FAB**: the floating action button is global, driven by `store/fab.ts`. Each page registers its primary action on `onMounted` (`fab.set({ label, icon, run })`) and `fab.clear()`s on `onUnmounted`. `run` returning `false` = silent abort (idle); throwing = error state. The store runs an idle→loading→success/error→idle machine with deliberate minimum timings.
- **Page action pattern** (see `views/hosts/index.vue`, `views/shell/index.vue`): load state via `invoke`, mutate locally, persist via the FAB's `run`. After a successful write, do **not** re-`invoke` a read — the in-memory list already reflects what was written, and re-parsing rebuilds rows and flickers the UI.

### Hosts parsing

`src/utils/hosts.ts` models the file as an ordered list of lines so comments/blank lines/headers round-trip on save. Only `entry` lines surface in the structured editor; `raw` lines pass through untouched. A commented-out line that still looks like a host entry is parsed as a disabled `entry` (toggling `enabled` re-comments it).

### Shell editor

`components/CodeEditor.vue` wraps CodeMirror 6 with compartments for hot-swapping theme (oneDark vs light) and language without rebuilding the editor. Language is picked from the shell config's stable `name`: `powershell` → PowerShell mode, `cmd` → plain (no good legacy mode), everything else → shell mode.

## Conventions

- Code comments in this repo are frequently in Chinese (especially Rust platform code explaining *why* a particular elevation/escaping approach is used). Match the surrounding language and keep the rationale when editing — several comments document non-obvious cross-platform bug fixes.
- Commit messages follow Conventional Commits with a Chinese subject, e.g. `fix(hosts/win): ...`.
