/* Renders every Bridge screen to static markup, once, into one page.
 *
 * The same shape as `packages/components/gallery/render.tsx` and deliberately
 * not the same file: that one collects Storybook stories out of the component
 * library, this one collects `*.screens.tsx` out of the app. They answer
 * different questions — is the component drawn right, and is the screen the app
 * assembles drawn right — and the second had no answer at all until this
 * existed.
 *
 * It imports what the app imports, so it cannot disagree with it. */
import { renderToStaticMarkup } from "react-dom/server";
import type { ReactElement } from "react";

type Screen = { mark: string; name: string; render: string; element: ReactElement; width?: number };

const modules = import.meta.glob<Record<string, unknown>>("../src/renderer/src/**/*.screens.tsx", {
  eager: true,
});

export type Rendered = {
  title: string;
  shots: { mark: string; name: string; render: string; html: string; width?: number }[];
};

export function collect(): Rendered[] {
  const out: Rendered[] = [];
  for (const mod of Object.values(modules)) {
    const title = typeof mod.title === "string" ? mod.title : undefined;
    const screens = mod.screens as Screen[] | undefined;
    if (title === undefined || !Array.isArray(screens)) continue;
    const shots = screens.map((screen) => ({
      mark: screen.mark,
      name: screen.name,
      render: screen.render,
      width: screen.width,
      /* A screen that throws is drawn as the failure rather than dropped. A
         missing figure reads as a state nobody built, which is a different
         defect and a false one. */
      html: render(screen),
    }));
    if (shots.length) out.push({ title, shots });
  }
  return out.sort((a, b) => a.title.localeCompare(b.title));
}

function render(screen: Screen): string {
  try {
    return renderToStaticMarkup(screen.element);
  } catch (e) {
    return `<p style="color:var(--status-completed-failed)">did not render: ${String(e)}</p>`;
  }
}
