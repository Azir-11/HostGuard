// 轻量平台判定：WebView2(Windows) / WKWebView(macOS) 的 UA 稳定可靠，且同步可用，
// 无需额外 IPC 或插件。仅用于窗口装饰这类纯前端外观分支。
const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;

export const isWindows = /Windows/i.test(ua);
export const isMacOS = /Macintosh|Mac OS X/i.test(ua);
