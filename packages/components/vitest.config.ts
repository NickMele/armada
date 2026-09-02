// Every story in this package, run as a test.
//
// **The stories are the test suite.** `@storybook/addon-vitest` mounts each one
// in a real browser: rendering it at all is the smoke test, and a story with a
// `play` function has its assertions run against the mounted DOM. So a state
// worth drawing is already a state that is checked, and there is no second
// fixture to keep in step with the first.
//
// **A real browser, not jsdom.** These components are drawn — geometry, focus
// order, a `<dialog>` only a browser opens — and a DOM implementation that
// approximates layout would pass stories that do not render. Playwright's
// headless shell is what the `browsers` Command in `armada.yml` installs; its
// cache is per machine rather than per worktree, so it is fetched once.
//
// **A rendered assertion reads roles and text, never class names.** A test
// naming a component's internals fails on every refactor and says nothing about
// what a person saw. `docs/practices/react.md` is the standard.
//
// Nothing here names a story file. `configDir` points at the Storybook config
// and the `stories` glob there is the one list — a second glob would drift.
import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

export default defineConfig({
  // Resolved against the working directory, which is why the Check runs this
  // with `-C packages/components` rather than from the repository root.
  plugins: [storybookTest({ configDir: ".storybook" })],
  test: {
    name: "storybook",
    browser: {
      enabled: true,
      headless: true,
      provider: playwright({}),
      instances: [{ browser: "chromium" }],
    },
  },
});
