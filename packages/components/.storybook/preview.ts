import type { Preview } from "@storybook/react-vite";
import "@armada/tokens/tokens.css";
import "./preview.css";
import "../src/index.css";

// Dark is primary. A light story is the secondary case, never the default.
const preview: Preview = {
  parameters: {
    backgrounds: { disable: true },
    controls: { matchers: { color: /(background|color)$/i } },
  },
};

export default preview;
