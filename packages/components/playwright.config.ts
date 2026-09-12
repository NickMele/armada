// What `evidence.run` in `armada.yml` invokes. Here rather than in
// `packages/screens`, though the screens are what it shoots: Storybook,
// the `browsers` Command and the Playwright pin are all this package's,
// and the spec reaches the screens over a URL rather than an import.
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "frames",
  // Traces and whatever else a run leaves that is not a frame. Under
  // `.armada/`, which is ignored whole, and never beside the frames —
  // `crates/fleet/src/showing.rs` reads that directory files-only and one
  // level deep, so a runner's own directory there is skipped in silence.
  outputDir: "../../.armada/playwright",
  // A capture is not a measurement, so nothing retries to stabilise one.
  retries: 0,
  reporter: [["list"]],
  use: {
    // The headless shell, named rather than inferred: `browsers` installs
    // `chromium --only-shell`, so asking for the full build asks for a
    // browser this repository never downloads.
    channel: "chromium-headless-shell",
    // A window the app is laid out for. Bridge's floor is narrower and one
    // story asks for it, so this is the wide reading and that one the narrow.
    viewport: { width: 1440, height: 900 },
  },
  // **No `webServer`, and no snapshot comparison.** Armada starts
  // `evidence.serve` and holds it, so a runner booting its own Storybook
  // would start a second on a taken port; and `toHaveScreenshot()` keeps
  // baselines and fails on any difference, where here a difference is the
  // answer. `docs/concepts/manifest.md` carries both rules.
});
