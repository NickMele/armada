import { useLayoutEffect } from "react";
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
  // **A `layout: "fullscreen"` story loses `body`'s own padding, in the test
  // run and not only in `storybook dev`.** Storybook's own preview toggles a
  // class for this — `WebView.applyLayout` — but `@storybook/addon-vitest`'s
  // test renderer never calls it, so a story that mounts a wired screen at the
  // real window's own size (`Setup.tsx` and its siblings, each `height:
  // "100vh"`) still sat inside this stylesheet's padding: offset from the
  // viewport's top edge by it, and running past the viewport's bottom edge by
  // the same amount. #1192. The attribute this sets is `preview.css`'s own,
  // rather than a class Storybook already owns the name of.
  decorators: [
    (Story, context) => {
      const fullscreen = context.parameters.layout === "fullscreen";
      useLayoutEffect(() => {
        if (!fullscreen) return undefined;
        document.body.setAttribute("data-storybook-layout", "fullscreen");
        return () => document.body.removeAttribute("data-storybook-layout");
      }, [fullscreen]);
      return <Story />;
    },
  ],
};

export default preview;
