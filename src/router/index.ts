import type { RouteRecordRaw } from "vue-router";
import { createRouter, createWebHashHistory } from "vue-router";

// Hash history avoids 404 on hard reloads inside the Tauri webview.
const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/dashboard" },
  {
    path: "/dashboard",
    name: "dashboard",
    component: () => import("@/views/dashboard/index.vue"),
  },
  {
    path: "/hosts",
    name: "hosts",
    component: () => import("@/views/hosts/index.vue"),
  },
  {
    path: "/shell",
    name: "shell",
    component: () => import("@/views/shell/index.vue"),
  },
  {
    path: "/system",
    name: "system",
    component: () => import("@/views/system/index.vue"),
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/views/settings/index.vue"),
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
