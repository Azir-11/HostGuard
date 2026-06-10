import { createApp } from "vue";
import { createPinia } from "pinia";
import "@fontsource-variable/sora";
import "@fontsource-variable/jetbrains-mono";
import App from "./App.vue";
import { router } from "@/router";
import "@/styles/theme.css";
import "virtual:uno.css";

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.mount("#app");
