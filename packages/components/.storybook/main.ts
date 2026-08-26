import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.tsx"],
  addons: ["@storybook/addon-a11y"],
  framework: { name: "@storybook/react-vite", options: {} },
  // Off by default. This repository is public and its posture is that nothing
  // leaves the machine unless somebody asked for it.
  core: { disableTelemetry: true },
};

export default config;
