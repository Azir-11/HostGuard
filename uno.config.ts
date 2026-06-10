import { defineConfig, presetAttributify, presetIcons, presetWind3 } from "unocss";

// https://unocss.dev/guide/config-file
export default defineConfig({
  presets: [
    presetWind3(),
    presetAttributify(),
    // Iconify icons (Phosphor / Carbon) rendered via UnoCSS.
    presetIcons({
      scale: 1.2,
      extraProperties: {
        display: "inline-block",
        "vertical-align": "middle",
      },
    }),
  ],

  content: {
    pipeline: {
      // UnoCSS's default pipeline does NOT scan plain `.ts` files, so icon
      // names declared in data modules (e.g. menu.ts) were never generated.
      include: [/\.(vue|svelte|[jt]sx?|mdx?|astro|html)($|\?)/],
    },
  },

  // Belt-and-suspenders: always generate every icon, even data-driven ones.
  safelist: [
    "i-ph-arrow-clockwise-bold",
    "i-ph-arrow-right-bold",
    "i-ph-arrow-up-right-bold",
    "i-ph-bell-bold",
    "i-ph-check-bold",
    "i-ph-circle-notch-bold",
    "i-ph-cpu-duotone",
    "i-ph-floppy-disk-duotone",
    "i-ph-gauge-duotone",
    "i-ph-gear-six-duotone",
    "i-ph-github-logo-duotone",
    "i-ph-globe-hemisphere-west-duotone",
    "i-ph-hard-drives-duotone",
    "i-ph-heartbeat-duotone",
    "i-ph-copy-bold",
    "i-ph-info-duotone",
    "i-ph-lock-key-duotone",
    "i-ph-magnifying-glass-bold",
    "i-ph-memory-duotone",
    "i-ph-minus-bold",
    "i-ph-moon-stars-duotone",
    "i-ph-plus-bold",
    "i-ph-pulse-duotone",
    "i-ph-radar-duotone",
    "i-ph-shield-check-duotone",
    "i-ph-sliders-horizontal-duotone",
    "i-ph-sun-duotone",
    "i-ph-terminal-window-duotone",
    "i-ph-trash",
    "i-ph-warning-circle-duotone",
    "i-ph-wrench-duotone",
    "i-ph-x-bold",
  ],

  theme: {
    colors: {
      primary: "var(--c-accent)",
      accent: {
        DEFAULT: "var(--c-accent)",
        2: "var(--c-accent-2)",
        soft: "var(--c-accent-soft)",
      },
      fg: {
        DEFAULT: "var(--c-text)",
        2: "var(--c-text-2)",
        3: "var(--c-text-3)",
      },
      hairline: {
        DEFAULT: "var(--c-hairline)",
        strong: "var(--c-hairline-strong)",
      },
      elevated: {
        DEFAULT: "var(--c-elevated)",
        2: "var(--c-elevated-2)",
      },
      info: "var(--c-blue)",
      violet: "var(--c-violet)",
      amber: "var(--c-amber)",
      tlclose: "var(--tl-close)",
      tlmin: "var(--tl-min)",
      tlzoom: "var(--tl-zoom)",
    },
    fontFamily: {
      display: "var(--font-display)",
      body: "var(--font-body)",
      mono: "var(--font-mono)",
    },
    borderRadius: {
      window: "var(--radius-window)",
      lg: "var(--radius-lg)",
      md: "var(--radius-md)",
      sm: "var(--radius-sm)",
    },
    boxShadow: {
      panel: "var(--shadow-panel)",
      pop: "var(--shadow-pop)",
      glow: "var(--shadow-glow)",
      fab: "var(--shadow-fab)",
    },
    animation: {
      keyframes: {
        fabring: "{0%{transform:scale(0.86);opacity:0.7}100%{transform:scale(1.28);opacity:0}}",
        spinslow: "{to{transform:rotate(360deg)}}",
      },
      durations: { fabring: "2.6s", spinslow: "26s" },
      timingFns: { fabring: "ease-out", spinslow: "linear" },
      counts: { fabring: "infinite", spinslow: "infinite" },
    },
  },

  shortcuts: {
    "ghost-btn":
      "w-34px h-34px rounded-[10px] border border-hairline bg-elevated text-fg-2 grid place-items-center cursor-pointer transition-colors hover:text-fg hover:bg-elevated-2 hover:border-hairline-strong",
    "nav-item":
      "relative flex items-center gap-12px h-40px px-12px rounded-md text-fg-2 text-[13.5px] font-500 cursor-default transition-colors hover:text-fg hover:bg-elevated",
    "nav-active": "!text-fg bg-accent-soft shadow-[inset_0_0_0_1px_var(--c-hairline)]",
    card: "rounded-lg border border-hairline bg-elevated",
  },

  // Gradient backgrounds: a bracketed `var()` would be read as a color
  // (invalid for a gradient), so expose them as explicit background rules.
  rules: [
    ["bg-base", { background: "var(--c-base-grad)" }],
    ["bg-panel", { background: "var(--c-panel-grad)" }],
    ["bg-grad-accent", { background: "var(--grad-accent)" }],
  ],

  preflights: [
    {
      getCSS: () => `
        * { box-sizing: border-box; }
        html, body, #app { height: 100%; margin: 0; }
        /* Transparent so the frameless window shows the desktop through margins. */
        html, body { background: transparent; }
        body {
          font-family: var(--font-body);
          color: var(--c-text);
          font-size: 14px;
          line-height: 1.5;
          overflow: hidden;
          user-select: none;
          -webkit-font-smoothing: antialiased;
          -moz-osx-font-smoothing: grayscale;
          text-rendering: optimizeLegibility;
        }
        a { color: inherit; text-decoration: none; }
        button { font-family: inherit; }
        ::selection { background: var(--c-accent-soft); }
        ::-webkit-scrollbar { width: 10px; height: 10px; }
        ::-webkit-scrollbar-thumb {
          background: var(--c-hairline-strong);
          border-radius: 99px;
          border: 3px solid transparent;
          background-clip: padding-box;
        }
        ::-webkit-scrollbar-thumb:hover { background: var(--c-text-3); background-clip: padding-box; }
        ::-webkit-scrollbar-track { background: transparent; }
      `,
    },
  ],
});
