// The screens, tested two ways — as arithmetic in node, and as screens in a
// browser.
//
// **Two projects here, because this package holds two kinds of thing.** The
// modules owe an answer: which tab a Job is in, which render it takes, what a
// terminal Job does to the step beneath it. Those are functions of the wire, so
// they are tested as functions — no DOM to start, no story to write, and a
// hundred cases cost what one costs. The screens owe a behaviour, and a screen
// is a mounted React component whatever else it is.
//
// **The browser project exists because a screen cannot be storied.**
// `xtask/src/rules_layers.rs` puts `@armada/components` below `@armada/screens`
// and refuses an import the other way, so `packages/components` cannot mount
// `OverruleControl` — what it has under `src/screens/` is a frame assembled from
// primitives, not this package's screens. A story there proves what that frame
// does. Nothing there can prove what a screen wires, and the wiring is where
// `confirmDisabled` either reaches `Dialog` or does not.
//
// That is also the whole point of the layer split, per that file's own comment:
// *a screen can be rendered, storied and tested without an Electron process*.
// This is the "tested" half.
//
// **`.test.ts` is node and `.test.tsx` is the browser**, so which runner a file
// gets is a property of what it imports rather than a list somebody maintains.
// A test that renders needs JSX; one that calls a function does not.
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        // **`node`, not `jsdom`.** A DOM implementation that approximates layout
        // is the wrong answer to both halves: these tests do not need one, and
        // the ones below need a real browser.
        test: {
          name: "screens",
          environment: "node",
          include: ["src/**/*.test.ts"],
        },
      },
      {
        test: {
          name: "screens (browser)",
          include: ["src/**/*.test.tsx"],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright({}),
            instances: [{ browser: "chromium" }],
          },
        },
      },
    ],
  },
});
