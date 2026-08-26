/* Renders every story to static markup, once, into a single page.
 *
 * This is an output, not a rendering of its own: it imports the same modules
 * the app imports, so it cannot disagree with them. Regenerate it and the
 * question of whether it is current stops being a question. */
import { renderToStaticMarkup } from "react-dom/server";
import { createElement, type ReactElement } from "react";

const stories = import.meta.glob<Record<string, any>>("../src/**/*.stories.tsx", {
  eager: true,
});

type Rendered = { title: string; stories: { name: string; html: string }[] };

function label(key: string) {
  return key
    .replace(/([A-Z])/g, " $1")
    .replace(/^./, (c) => c.toUpperCase())
    .trim();
}

export function collect(): Rendered[] {
  const out: Rendered[] = [];
  for (const mod of Object.values(stories)) {
    const meta = mod.default;
    if (!meta?.title) continue;
    const Component = meta.component;
    const rendered: { name: string; html: string }[] = [];
    for (const [key, story] of Object.entries(mod)) {
      if (key === "default" || !story || typeof story !== "object") continue;
      const s = story as { render?: (a: any) => ReactElement; args?: any };
      try {
        const el = s.render
          ? s.render({ ...meta.args, ...s.args })
          : Component
            ? createElement(Component, { ...meta.args, ...s.args })
            : null;
        if (el) rendered.push({ name: label(key), html: renderToStaticMarkup(el) });
      } catch (e) {
        rendered.push({
          name: label(key),
          html: `<p style="color:var(--status-failed)">did not render: ${String(e)}</p>`,
        });
      }
    }
    if (rendered.length) out.push({ title: meta.title, stories: rendered });
  }
  return out.sort((a, b) => a.title.localeCompare(b.title));
}
