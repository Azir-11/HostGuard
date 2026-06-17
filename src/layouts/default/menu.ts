export interface MenuItem {
  key: string;
  label: string;
  icon: string;
  path: string;
}

export interface MenuGroup {
  title: string;
  items: MenuItem[];
}

/** Featured item shown at the very top of the panel (like CleanMyMac's 智能扫描). */
export const topItem: MenuItem = {
  key: "dashboard",
  label: "概览",
  icon: "i-ph-gauge-duotone",
  path: "/dashboard",
};

/** Labelled navigation groups. */
export const groups: MenuGroup[] = [
  {
    title: "配置",
    items: [
      {
        key: "hosts",
        label: "Hosts",
        icon: "i-ph-globe-hemisphere-west-duotone",
        path: "/hosts",
      },
      {
        key: "shell",
        label: "Shell 配置",
        icon: "i-ph-terminal-window-duotone",
        path: "/shell",
      },
      {
        key: "system",
        label: "系统配置",
        icon: "i-ph-sliders-horizontal-duotone",
        path: "/system",
      },
    ],
  },
];

/** Pinned to the bottom of the panel. */
export const bottomItem: MenuItem = {
  key: "settings",
  label: "设置",
  icon: "i-ph-gear-six-duotone",
  path: "/settings",
};

/** Flat list of every navigable item (used to resolve the current page). */
export const allItems: MenuItem[] = [topItem, ...groups.flatMap((g) => g.items), bottomItem];
