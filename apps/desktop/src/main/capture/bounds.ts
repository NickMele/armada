// Every bound re-applied on what the page answered — #1294,
// `docs/practices/capture-window.md`, *What may come back*.
//
// **The bound belongs on the side that is not the page.** The layer's source
// asks for a trimmed markup and a declared style list, and the page can answer
// with anything at all, fifty megabytes of markup included — so what the layer
// asks for is a courtesy and this is the rule.
//
// Pure, and tested as such.

import type { CaptureServed, StudioCapture } from "@armada/protocol";
import type { CaptureAimed, CaptureRect } from "../../shared/capture-window";
import { CAPTURED_STYLES, MOST_MARKUP, MOST_SELECTOR, MOST_TEXT } from "./layer";

/** The most owners a chain keeps, and the most a name may run to. */
const MOST_OWNERS = 16;
const MOST_NAME = 120;
/** The most style properties, which is the declared list and cannot exceed it. */
const MOST_STYLE_VALUE = 200;

const text = (value: unknown, most: number): string | null =>
  typeof value === "string" ? value.slice(0, most) : null;

const whole = (value: unknown): number => (typeof value === "number" && Number.isFinite(value) ? Math.round(value) : 0);

/** A rectangle, clamped so a negative or an infinite one cannot reach a crop. */
export function rectOf(value: unknown, viewport: { width: number; height: number }): CaptureRect {
  const box = (value ?? {}) as Record<string, unknown>;
  const x = Math.min(Math.max(whole(box["x"]), 0), viewport.width);
  const y = Math.min(Math.max(whole(box["y"]), 0), viewport.height);
  return {
    x,
    y,
    width: Math.min(Math.max(whole(box["width"]), 0), viewport.width - x),
    height: Math.min(Math.max(whole(box["height"]), 0), viewport.height - y),
  };
}

/** The box and the name an aim answers with, or `null` where it answered nothing. */
export function aimed(answer: unknown, viewport: { width: number; height: number }): CaptureAimed | null {
  if (typeof answer !== "object" || answer === null) return null;
  const said = answer as Record<string, unknown>;
  return { box: rectOf(said["box"], viewport), named: text(said["named"], MOST_NAME) ?? "element" };
}

/**
 * The capture a take answered with, every field re-typed and re-bounded, or
 * `null` where it is not one.
 *
 * **`served` is main's and never the page's.** It names the Run and the origin
 * main pinned the window to, so a page that answers with one of its own is
 * answering a field this drops.
 */
export function bounded(
  answer: unknown,
  viewport: { width: number; height: number },
  served: CaptureServed,
): StudioCapture | null {
  if (typeof answer !== "object" || answer === null) return null;
  const said = (answer as Record<string, unknown>)["capture"];
  if (typeof said !== "object" || said === null) return null;
  const capture = said as Record<string, unknown>;
  const element = (capture["element"] ?? {}) as Record<string, unknown>;
  const tag = text(element["tag"], 40);
  const selector = text(capture["selector"], MOST_SELECTOR);
  const markup = text(capture["markup"], MOST_MARKUP);
  if (tag === null || selector === null || markup === null) return null;

  const component = text(capture["component"], MOST_NAME);
  const label = text(element["label"], MOST_TEXT);
  const screen = text(capture["screen"], MOST_NAME);
  const layer = text(capture["layer"], MOST_NAME);
  const owners = Array.isArray(capture["owners"])
    ? capture["owners"]
        .slice(0, MOST_OWNERS)
        .map((one) => text(one, MOST_NAME))
        .filter((one): one is string => one !== null)
    : [];

  return {
    ...(component === null ? {} : { component }),
    ...(owners.length === 0 ? {} : { owners }),
    selector,
    element: {
      tag,
      text: text(element["text"], MOST_TEXT) ?? "",
      ...(label === null ? {} : { label }),
    },
    ...(screen === null ? {} : { screen }),
    ...(layer === null ? {} : { layer }),
    location: text(capture["location"], 600) ?? "/",
    bounds: rectOf(capture["bounds"], viewport),
    window: { width: whole(viewport.width), height: whole(viewport.height) },
    styles: styled(capture["styles"]),
    markup,
    served,
  };
}

/**
 * The declared properties and no others.
 *
 * **Filtered against Bridge's own list rather than the page's answer**, which
 * is the whole point: a page that answers with three hundred and fifty
 * properties, or with one nobody declared, contributes the ones on the list.
 */
export function styled(value: unknown): Record<string, string> {
  if (typeof value !== "object" || value === null) return {};
  const said = value as Record<string, unknown>;
  const styles: Record<string, string> = {};
  for (const property of CAPTURED_STYLES) {
    const resolved = text(said[property], MOST_STYLE_VALUE);
    if (resolved !== null && resolved !== "") styles[property] = resolved;
  }
  return styles;
}

/**
 * The card's one line, as the bar draws it: `Nav ← Header · nav[role="navigation"]`.
 *
 * `capture/note.ts`'s `chainOf`, which this cannot import — it is renderer
 * code, and the seam between the two builds is not an import path.
 */
export function chainOf(capture: StudioCapture): string {
  const names = [capture.component, ...(capture.owners ?? [])].filter((name): name is string => name !== undefined);
  const steps = capture.selector.split(" > ");
  const shown = steps.length > 2 ? `… > ${steps.slice(-2).join(" > ")}` : capture.selector;
  return `${names.length > 0 ? names.slice(0, 4).join(" ← ") : capture.element.tag} · ${shown}`;
}
