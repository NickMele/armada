// Bridge's renderer in a plain browser, on a fake `window.armada` — `pnpm mock`.
// Its own Vite config because electron-vite's builds a main process and a
// preload this page has neither of. `docs/practices/running-locally.md`.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { annotationsServer } from "./src/main/annotations-server";

export default defineConfig({
  root: "src/renderer/src/mock",
  // `annotationsServer` saves the dev annotation layer's notes (#1226) to
  // `.armada/annotations/`, as main does inside Electron.
  plugins: [react(), tailwindcss(), annotationsServer()],
  server: { open: true },
});
