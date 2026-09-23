// Bridge's renderer in a plain browser, on a fake `window.armada` — `pnpm mock`.
// Its own Vite config because electron-vite's builds a main process and a
// preload this page has neither of. `docs/practices/running-locally.md`.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { annotationsServer } from "./src/main/annotations-server";
import { annotationSource } from "./src/main/annotations-source";

export default defineConfig({
  root: "src/renderer/src/mock",
  // `annotationsServer` saves the dev annotation layer's notes (#1226) to
  // `.armada/annotations/`, as main does inside Electron. `annotationSource`
  // stamps each element with the JSX that drew it, which is what lets a note
  // name a file and a line (#1584) — this config alone, see that module.
  plugins: [react({ babel: { plugins: [annotationSource()] } }), tailwindcss(), annotationsServer()],
  // No `server.open`: `pnpm mock` asks for a browser with `--open`, and a Job
  // serving this for Evidence (`armada.yml`) must not open one on the owner's screen.
});
