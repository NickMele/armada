import { fileURLToPath } from "node:url";
import type { StorybookConfig } from "@storybook/react-vite";
import { notesPlugin } from "./notes.ts";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.tsx"],
  // `addon-vitest` is what turns the stories above into a test suite — see
  // `../vitest.config.ts`. It is listed here as well as there because the panel
  // that reports a failing story is a Storybook panel, and because the tags a
  // story sets are read from this config.
  addons: ["@storybook/addon-a11y", "@storybook/addon-vitest"],
  framework: { name: "@storybook/react-vite", options: {} },
  // Off by default. This repository is public and its posture is that nothing
  // leaves the machine unless somebody asked for it.
  core: { disableTelemetry: true },
  // Registers the picker (`picker.ts`) into the preview bundle without
  // widening `preview.ts`'s default export — it is pure side effect, wiring
  // itself to the channel rather than to decorators or parameters. Absolute,
  // because the virtual module Storybook generates to import this list
  // lives outside `.storybook/` — a relative path resolves against it, not
  // against this file.
  previewAnnotations: [fileURLToPath(new URL("./picker.ts", import.meta.url))],
  // The Notes panel (`manager.tsx`) talks to a dev-server route rather than
  // to Storybook's own channel, because the note has to survive a manager
  // reload and land in a file an agent session reads — a plain Vite plugin
  // owns that route.
  async viteFinal(viteConfig) {
    viteConfig.plugins ??= [];
    viteConfig.plugins.push(notesPlugin());
    return viteConfig;
  },
};

export default config;
