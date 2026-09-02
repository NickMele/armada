// Main's own state machines, run in node with no Electron and no window.
//
// **The third runner, and the first that reaches `src/main`.** `packages/screens`
// runs the arithmetic behind the screens and `packages/components` renders its
// stories in a browser; neither can reach the sockets, and everything main holds
// is a socket's messages folded into state. `observe.ts` published a `failed`
// that dropped every row it had already received, and nothing here could have
// said so — `typecheck` compiles main without running it and `bridge_build`
// bundles it.
//
// **`node`, and nothing from Electron.** A test that needed `app` or a
// `BrowserWindow` would be a test of the shell rather than of the state, and
// `src/main/index.ts` is deliberately the only file that holds either.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    name: "main",
    environment: "node",
    include: ["src/main/**/*.test.ts"],
  },
});
