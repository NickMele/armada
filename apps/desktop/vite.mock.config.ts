// Bridge's renderer in a plain browser, on a fake `window.armada` — `pnpm mock`.
// Its own Vite config because electron-vite's builds a main process and a
// preload this page has neither of. `docs/practices/running-locally.md`.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  root: "src/renderer/src/mock",
  plugins: [react(), tailwindcss()],
  server: { open: true },
});
