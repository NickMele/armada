// Main's own state machines, run in node with no Electron and no window.
//
// **The fourth runner, and the first that reaches `src/main`.** The other three
// are `packages/screens`, twice — its modules in node and its screens in a
// browser — and `packages/components`' stories. None of them can reach a
// socket, and everything main holds is a socket's messages folded into state:
// `observe.ts` published a `failed` that dropped every row it had already
// received, and no Check could have said so. `typecheck` compiles main without
// running it and `bridge_build` bundles it.
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
