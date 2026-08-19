import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

document.documentElement.dataset.platform = import.meta.env.TAURI_ENV_PLATFORM;

const target = document.getElementById("app");

if (!target) {
  throw new Error("LoopCode could not find its app root");
}

mount(App, { target });
