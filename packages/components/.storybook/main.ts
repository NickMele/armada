import type { StorybookConfig } from "@storybook/react-vite";

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
};

export default config;
