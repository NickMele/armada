// Main's own state machines, and the renderer's own — both in node, with no
// Electron and no window.
//
// **The fourth runner, and the first that reaches `src/main`.** The other three
// are `packages/screens`, twice — its modules in node and its screens in a
// browser — and `packages/components`' stories. `typecheck` compiles main
// without running it and `bridge_build` bundles it.
//
// **`node`, and nothing from Electron.** A test that needed `app` or a
// `BrowserWindow` would be a test of the shell rather than of the state.
//
// **The second project reaches `src/renderer`, on the same terms.**
// `where-open.ts`'s `fold` needs a window no more than anything in `src/main`
// does; a renderer test that needed a real DOM has no runner here yet.
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
    ],
  },
});
