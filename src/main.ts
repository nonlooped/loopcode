import { mount } from "svelte";
import "./app.css";

const target = document.getElementById("app");

if (!target) {
  throw new Error("LoopCode could not find its app root");
}

async function start(appTarget: HTMLElement) {
  const platform = import.meta.env.TAURI_ENV_PLATFORM;
  document.documentElement.dataset.platform = platform ?? "web";
  if (!platform) {
    const { setupWebPreview } = await import("./services/web-preview.ts");
    setupWebPreview();
  }
  const { default: App } = await import("./App.svelte");
  mount(App, { target: appTarget });
}

void start(target);
