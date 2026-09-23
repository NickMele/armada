// Main's own state machines and the renderer's own, in node, and the renderer
// whole in Chromium — none with Electron.
//
// **The fourth runner, and the first that reaches `src/main`.** The other three
// are `packages/screens`, twice — its modules in node and its screens in a
// browser — and `packages/components`' stories.
//
// **`node`, and nothing from Electron.** A test that needed `app` or a
// `BrowserWindow` would be a test of the shell rather than of the state. The
// renderer's node project is for folds like `where-open.ts`', which need no window.
//
// **The browser project mounts `App` on the mock Fleet**, `src/renderer/src/mock/`.
// `.test.tsx` is the browser here, as it is in `packages/screens`.
import tailwindcss from "@tailwindcss/vite";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: "main",
          environment: "node",
          include: ["src/main/**/*.test.ts"],
        },
      },
      {
        test: {
          name: "renderer",
          environment: "node",
          include: ["src/renderer/**/*.test.ts"],
        },
      },
      {
        // The app's stylesheet imports Tailwind, so the plugin that compiles it is here too.
        plugins: [tailwindcss()],
        // Found only once a test renders JSX, and Vite reloads the test when it finds it.
        optimizeDeps: { include: ["react/jsx-dev-runtime"] },
        test: {
          name: "renderer (browser)",
          include: ["src/renderer/**/*.test.tsx"],
          // Every guide already met, so a card opening on a piece nobody has
          // seen does not sit over the press a test about something else is
          // making. `mock/guides-met.ts` carries the reasoning.
          setupFiles: ["./src/renderer/src/mock/guides-met.ts"],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright({}),
            instances: [{ browser: "chromium" }],
            viewport: { width: 1440, height: 900 },
          },
        },
      },
    ],
  },
});
