import type { Preview } from "@storybook/react-vite";
import "@armada/tokens/tokens.css";
// What Bridge loads under every component, in the order it loads them, so a
// story is sized and spaced the way the app draws it. Without these every box
// here was `content-box` and every heading kept the browser's margins, while
// the app was `border-box` with none — a component could read right here and
// overflow there by exactly its own padding. Tailwind's preflight is a plain
// stylesheet; its utilities are still not loaded, which is why they stay inert.
import "tailwindcss/preflight.css";
import "@armada/tokens/base.css";
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
