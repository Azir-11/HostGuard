# HostGuard 开发计划

> 一款 macOS 桌面工具：管理 **hosts**、**shell / 系统配置**，并查看**系统用量**。
> 技术栈：Tauri 2 · Vue 3.5 · TypeScript · Vite · Pinia · Vue Router 4 · Naive UI · UnoCSS · oxlint · oxfmt · pnpm。

---

## ✅ 阶段 0 — 项目初始化（已完成）

- [x] 连接远程仓库 `github.com/Azir-11/HostGuard`，`git init`（分支 `main`）
- [x] `create-tauri-app` 脚手架（Tauri 2 + Vue-TS + Vite）
- [x] 接入 Pinia / Vue Router 4（hash 模式）/ Naive UI / UnoCSS
- [x] 自动导入：`unplugin-auto-import`（vue/router/pinia）、`unplugin-vue-components`（Naive UI 按需）
- [x] 代码规范：oxlint + oxfmt，路径别名 `@ -> src`
- [x] 通过 `pnpm build`（Vite + vue-tsc）与 Rust `tauri build` 验证可运行
- [x] 首页 Demo：调用 Rust `greet` 命令、深/浅色切换，验证全链路打通

---

## 🧭 建议实现顺序

`布局` → `Hosts 管理` → `Zsh / Shell 配置` → `系统配置` → `系统用量监控` → `工程化收尾`

每个模块自带「Rust 后端命令 + 前端页面」两层，逐条交付、独立可验证。

---

## 阶段 1 — 应用布局与框架 ✅（核心完成）

CleanMyMac X 风格「Sentinel」深色玻璃拟态界面，已落地：

- [x] 无边框透明窗口（`decorations:false` / `transparent:true` / `macos-private-api`）
- [x] **异形分层窗口**：深色主窗口（base）+ 悬浮菜单面板（panel，向左溢出、约 80% 主体高度、垂直居中），CleanMyMac 风格非矩形轮廓
- [x] 移除窗口外发光阴影（仅保留面板柔和投影，不再有"光圈"）
- [x] 自定义 macOS 红绿灯窗口控件（关闭 / 最小化 / 缩放）+ `data-tauri-drag-region` 拖拽
- [x] 菜单分组（概览置顶高亮 + 配置 / 监控 分组）
- [x] 样式全部 UnoCSS（tokens + shortcuts + rules + preflights，无 `<style>` 块）；图标 Iconify（Phosphor，经 presetIcons）
- [x] 主布局：侧边栏（品牌 + 导航 + 状态栏）+ 顶部栏 + 内容区
- [x] 各功能模块路由与页面骨架（dashboard / hosts / shell / system / monitor / settings 占位页）
- [x] 概览页：系统健康环形图 + CPU/内存/磁盘/Hosts 统计卡（数据为占位）
- [x] 主题系统：亮 / 暗切换（设置页），CSS 变量 design tokens 与 Naive UI `darkTheme` 联动
- [x] 悬浮操作按钮（FAB）：居中于内容区（已计算菜单偏移）、随页面切换功能（重载 / 保存 / 授予权限…），由 fab store 驱动
- [x] 字体（Sora / JetBrains Mono 自托管离线）+ Phosphor 图标 + 入场动画
- [ ] 主题跟随系统（prefers-color-scheme）
- [ ] 可折叠侧栏
- [ ] 全局 Provider 封装为可复用 hook（message / dialog / notification / loadingBar）
- [ ] 应用设置持久化（`@tauri-apps/plugin-store` 或本地 JSON）
- [ ] 系统托盘 Tray（可选）

## 阶段 2 — Hosts 管理（`/etc/hosts`）✅（核心完成）

- [x] Rust：读取 `/etc/hosts` 并解析为结构化条目（IP / 域名 / 注释 / 启用-停用，注释行保留）
- [x] **权限模型**：一次性 ACL 授权（`chmod +a`，单次管理员弹窗，**async 不卡 UI**）→ 之后直接写入、无需重复输入密码；无权限时横幅 + FAB 提示「授予权限」
- [x] Rust：写回 `/etc/hosts`（授权后直接写入，保存前备份到 `~/.hostguard.hosts.bak`）
- [x] 前端：条目表格，增 / 删 / 改 / 启用停用 + 搜索过滤
- [x] FAB「保存」提交更改、工具栏「重载」从磁盘重读
- [x] 工具栏「刷新 DNS 缓存」：`dscacheutil -flushcache` + `killall -HUP mDNSResponder`；**一次性 sudoers 免密授权**（首次单次管理员弹窗，之后静默、跨重启保留；安装前 `visudo -cf` 校验，仅放行这两条精确命令）
- [ ] 多方案 Profiles（开发 / 测试 / 生产）一键切换
- [ ] 原文编辑模式（语法高亮）与结构化模式互转
- [ ] 导入 / 导出、备份回滚 UI
- [ ] 校验：重复域名、非法 IP 提示

## 阶段 3 — Zsh / Shell 配置 ✅（核心完成）

- [x] Rust：读写 `~/.zshrc` / `~/.zprofile` / `~/.zshenv` / `~/.zlogin`（用户文件，无需提权；保存前备份到 `~/.hostguard.<name>.bak`）
- [x] 前端：CodeMirror 6 语法高亮编辑器（行号 / 折行 / 撤销重做 / 深浅色主题切换）
- [x] 多文件切换（带"未创建"标记）+ 未保存(dirty)提示 + 切换前确认对话框
- [x] 保存（FAB）/ 重载；`source` 命令提示 + 一键复制
- [ ] 片段管理：alias / 环境变量 / `PATH` / 函数（结构化增删改）
- [ ] Oh My Zsh 检测：主题、插件 启用 / 停用
- [ ] 一键执行 `source`（注：仅影响子进程，已有终端仍需各自 source）

## 阶段 4 — 系统配置

- [ ] 用户级环境变量查看 / 管理
- [ ] DNS / 代理（proxy）查看（可选设置）
- [ ] 常用 macOS `defaults` 开关（显示隐藏文件、Dock 等，操作前二次确认）
- [ ] `~/.ssh/config` 管理（可选）

## 阶段 5 — 系统用量监控 🟡（核心实时监控完成，按"最省"范围）

- [x] Rust：接入 `sysinfo` crate（`default-features = false`，仅 `system` 特性），常驻 `System` 句柄置于 Tauri state，单命令仅刷新 CPU + 内存
- [x] CPU：总体 + 每核使用率
- [x] 内存：已用 / 可用 / swap
- [x] 平均负载（1 / 5 / 15 分钟）+ 运行时长 + 主机名
- [x] 前端定时轮询（~1.5s，仅监控页挂载且窗口可见时；其它页面零轮询，离开即停）
- [ ] 磁盘：各挂载点容量与使用率
- [ ] 网络：实时上下行速率
- [ ] 进程：CPU / 内存 Top N，支持结束进程
- [ ] 趋势图表；电池 / 温度（可选）

## 阶段 6 — 工程化收尾

- [ ] Rust 命令统一封装 + 前端 `invoke` 类型安全封装
- [ ] 提权 / 权限统一处理（Hosts、系统设置）
- [ ] 统一错误处理与通知
- [ ] 国际化中 / 英（可选）
- [x] 关于页（设置页内）
- [ ] 自定义应用图标（当前为 Tauri 默认）、自动更新（可选）
- [x] 打包：`.app` + DMG 安装包（`pnpm tauri build` / `pnpm app:build`），README 含安装说明
- [ ] 代码签名 / 公证（Apple Developer ID，面向他人分发）

---

## ⚠️ 技术决策与待确认

- **Vue Router 4**：未用最新 v5（v5 强依赖 Vite 7/8 并引入 `@pinia/colada`，与 Tauri 的 Vite 6 基线冲突）。如需 v5 的类型化路由，可后续升级 Vite 8 一并切换。
- **"Vite+"**：当前按「最新 Vite + oxc 工具链（oxlint/oxfmt）」理解；若指 VoidZero 商业版 Vite+，待其可公开安装后再切换。
- **提权方案**：Hosts / 系统设置写操作需管理员权限，计划用 `osascript` 弹窗授权，待阶段 2 落地时确认交互细节。
