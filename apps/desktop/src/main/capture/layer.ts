// What reads the page inside the capture window, and the source main evaluates
// there — #1294, `docs/practices/capture-window.md`, *The injected layer*.
//
// **It runs in the page's own world because a component name is an expando the
// page sets**, and a contextIsolated world cannot see one. That is
// `renderer/src/annotate/fiber.ts`'s reading, which is renderer code only
// because there the page is Bridge.
//
// **Every ask carries its own source and leaves nothing behind**: no global, no
// listener, nothing to re-inject after a navigation, and the promise
// `executeJavaScript` answers on is the page's only channel. The pointer is
// tracked by Bridge's own view over the page, so a listener here could only
// read what it had no way to send. The review was narrowed to this on 18 Sep
// 2026 — its *The injected layer* section records what it gave up and why.

/** The most markup one Note keeps. `capture/note.ts`'s figure, which this matches. */
export const MOST_MARKUP = 2000;

/** The most visible text. `annotate/capture.ts`'s figure. */
export const MOST_TEXT = 80;

/** The most selector. A chain past this keeps its nearest steps. */
export const MOST_SELECTOR = 600;

// Declared twice on purpose: this copy crosses into a foreign page as source
// and cannot import one, and `bounds.ts` re-applies the list on the way back.
/** The computed properties a Note keeps — `capture/note.ts`'s declared list. */
export const CAPTURED_STYLES: readonly string[] = [
  "display",
  "position",
  "width",
  "height",
  "padding",
  "margin",
  "gap",
  "overflow",
  "color",
  "background-color",
  "border-width",
  "border-color",
  "border-radius",
  "opacity",
  "font-family",
  "font-size",
  "font-weight",
  "line-height",
  "letter-spacing",
  "text-align",
  "white-space",
  "visibility",
];

/** The bounds an ask carries, so the page never decides its own. */
export type AskBounds = {
  markup: number;
  text: number;
  selector: number;
  styles: readonly string[];
};

export const ASK_BOUNDS: AskBounds = {
  markup: MOST_MARKUP,
  text: MOST_TEXT,
  selector: MOST_SELECTOR,
  styles: CAPTURED_STYLES,
};

/**
 * Exactly what the layer touches in the page, declared here because main's
 * project carries no `lib.dom` — and that is the better half of the trade:
 * this list *is* `capture-window.md`'s *It reads the page and writes nothing to
 * it*, checked by the compiler rather than asserted in a comment.
 */
type PageElement = {
  tagName: string;
  id: string;
  textContent: string | null;
  outerHTML: string;
  parentElement: PageElement | null;
  parentNode: unknown;
  children: ArrayLike<PageElement>;
  attributes: ArrayLike<{ name: string }>;
  getAttribute: (name: string) => string | null;
  removeAttribute: (name: string) => void;
  closest: (selector: string) => PageElement | null;
  querySelectorAll: (selector: string) => ArrayLike<PageElement>;
  cloneNode: (deep: boolean) => PageElement;
  /** Called with no argument only: what it does here is empty a field. */
  replaceChildren: () => void;
  getBoundingClientRect: () => { x: number; y: number; width: number; height: number };
};

declare const document: {
  elementFromPoint: (x: number, y: number) => PageElement | null;
  querySelector: (selector: string) => PageElement | null;
  getElementById: (id: string) => PageElement | null;
};

declare const window: {
  getComputedStyle: (element: PageElement) => { getPropertyValue: (property: string) => string };
  location: { pathname: string; search: string; hash: string };
  innerWidth: number;
  innerHeight: number;
};

/**
 * The whole of what runs in the page, as one self-contained function.
 *
 * **Every helper is nested and every constant is an argument**, because this is
 * stringified: a reference to this module's scope would be a free identifier in
 * the page and would throw there.
 *
 * `take` false answers the box and what to call it, which is what the outline
 * draws as a person moves. `take` true answers the capture as well.
 *
 * It writes nothing — no node, no attribute, no style. The markup is read off a
 * detached clone, which is what lets the redaction happen with the page's own
 * document untouched.
 */
export function readsThePage(x: number, y: number, take: boolean, bounds: AskBounds): unknown {
  const doc = document;
  const found = doc.elementFromPoint(x, y);
  if (found === null) return null;
  // An SVG is the whole glyph rather than one path, as Bridge's own capture reads it.
  const element = found.closest("svg") ?? found;

  function short(text: string | null | undefined, most: number): string {
    const collapsed = (text ?? "").replace(/\s+/g, " ").trim();
    return collapsed.length > most ? `${collapsed.slice(0, most - 1)}…` : collapsed;
  }

  // React's own expando, walked up to the nearest node that carries one.
  function fiberOf(node: unknown): Record<string, unknown> | null {
    let at = node as { parentNode?: unknown } | null;
    while (at !== null && at !== undefined) {
      for (const key of Object.keys(at)) {
        if (key.startsWith("__reactFiber$")) {
          return ((at as Record<string, unknown>)[key] ?? null) as Record<string, unknown> | null;
        }
      }
      at = (at as { parentNode?: unknown }).parentNode as { parentNode?: unknown } | null;
    }
    return null;
  }

  function nameOf(type: unknown): string | null {
    if (typeof type === "function") {
      const named = type as { displayName?: unknown; name?: unknown };
      const name = typeof named.displayName === "string" ? named.displayName : named.name;
      return typeof name === "string" && name !== "" ? name : null;
    }
    if (typeof type === "object" && type !== null) {
      const wrapped = type as { $$typeof?: unknown; displayName?: unknown; type?: unknown; render?: unknown };
      if (typeof wrapped.displayName === "string" && wrapped.displayName !== "") return wrapped.displayName;
      if (wrapped.$$typeof === Symbol.for("react.memo")) return nameOf(wrapped.type);
      if (wrapped.$$typeof === Symbol.for("react.forward_ref")) return nameOf(wrapped.render);
    }
    return null;
  }

  /** The component chain, nearest first. Empty on a page React did not draw. */
  function chainOf(node: PageElement): string[] {
    const names: string[] = [];
    let at = fiberOf(node);
    for (let depth = 0; at !== null && depth < 16; depth += 1) {
      const name = nameOf(at["type"]);
      if (name !== null && names[names.length - 1] !== name) names.push(name);
      at = (at["return"] ?? null) as Record<string, unknown> | null;
    }
    return names;
  }

  /**
   * One step of a selector. **No class is taken**, where Bridge's own selector
   * takes its `armada-` ones: a foreign app's class is as likely to be a
   * build's hash as a name somebody wrote, and a selector built from one finds
   * nothing after the next build. An id, a `data-testid`, an accessible name
   * and a role are what a person did write.
   */
  function step(node: PageElement): string {
    const id = node.id;
    if (id !== "" && /^[A-Za-z][\w-]*$/.test(id) && !/[«»:]/.test(id)) return `#${id}`;
    const quote = (value: string): string => `"${value.replace(/["\\]/g, "\\$&")}"`;
    const testid = node.getAttribute("data-testid");
    const label = node.getAttribute("aria-label");
    const role = node.getAttribute("role");
    let part = node.tagName.toLowerCase();
    if (testid !== null && testid !== "" && testid.length <= 60) part += `[data-testid=${quote(testid)}]`;
    else if (label !== null && label !== "" && label.length <= 60) part += `[aria-label=${quote(label)}]`;
    else if (role !== null && role !== "") part += `[role=${quote(role)}]`;
    return part;
  }

  function selectorOf(node: PageElement): string {
    const parts: string[] = [];
    let at: PageElement | null = node;
    while (at !== null && parts.length < 12) {
      const here: PageElement = at;
      const part = step(here);
      const tag = here.tagName.toLowerCase();
      if (part.startsWith("#")) {
        parts.unshift(part);
        break;
      }
      if (tag === "body" || tag === "html") break;
      const parent = here.parentElement;
      if (parent === null) {
        parts.unshift(part);
        break;
      }
      const siblings = Array.from<PageElement>(parent.children);
      const same = siblings.filter((one) => step(one) === part).length;
      const sameTag = siblings.filter((one) => one.tagName === here.tagName);
      parts.unshift(same <= 1 ? part : `${part}:nth-of-type(${sameTag.indexOf(here) + 1})`);
      at = parent;
    }
    const whole = parts.join(" > ");
    return whole.length > bounds.selector ? whole.slice(whole.length - bounds.selector) : whole;
  }

  /** The dialog the element sits in, by its accessible name. */
  function layerOf(node: PageElement): string | null {
    const dialog = node.closest('[role="dialog"], [role="alertdialog"], dialog');
    if (dialog === null) return null;
    const by = dialog.getAttribute("aria-labelledby");
    const named = by === null ? null : doc.getElementById(by);
    return dialog.getAttribute("aria-label") ?? (named === null ? null : short(named.textContent, 60)) ?? "dialog";
  }

  /**
   * `outerHTML` off a detached clone, with what a person typed taken out.
   *
   * **No `value` attribute, nothing inside a field, and a password input
   * stripped to its type.** `outerHTML` over a form is a person's own typing,
   * kept for the life of a Studio that nothing expires — and this page is an
   * app they are logged into.
   */
  function markupOf(node: PageElement): string {
    const clone = node.cloneNode(true);
    for (const one of [clone, ...Array.from<PageElement>(clone.querySelectorAll("*"))]) {
      const tag = one.tagName.toLowerCase();
      const type = (one.getAttribute("type") ?? "").toLowerCase();
      if (tag === "input" || tag === "textarea" || tag === "select") one.replaceChildren();
      if (tag === "input" && type === "password") {
        for (const attribute of Array.from<{ name: string }>(one.attributes)) {
          if (attribute.name.toLowerCase() !== "type") one.removeAttribute(attribute.name);
        }
      } else {
        one.removeAttribute("value");
      }
    }
    const collapsed = clone.outerHTML.replace(/\s+/g, " ").trim();
    return collapsed.length > bounds.markup ? `${collapsed.slice(0, bounds.markup - 1)}…` : collapsed;
  }

  const rect = element.getBoundingClientRect();
  const box = {
    x: Math.round(rect.x),
    y: Math.round(rect.y),
    width: Math.round(rect.width),
    height: Math.round(rect.height),
  };
  const chain = chainOf(element);
  const named = chain[0] ?? element.tagName.toLowerCase();
  if (!take) return { box, named };

  const styles: Record<string, string> = {};
  const resolved = window.getComputedStyle(element);
  for (const property of bounds.styles) {
    const value = resolved.getPropertyValue(property);
    if (value !== "") styles[property] = value;
  }

  const current = doc.querySelector('[aria-current="page"]');
  const label = element.getAttribute("aria-label");
  const layer = layerOf(element);
  return {
    box,
    named,
    capture: {
      ...(chain[0] === undefined ? {} : { component: chain[0] }),
      ...(chain.length > 1 ? { owners: chain.slice(1) } : {}),
      selector: selectorOf(element),
      element: {
        tag: element.tagName.toLowerCase(),
        text: short(element.textContent, bounds.text),
        ...(label === null ? {} : { label: short(label, bounds.text) }),
      },
      ...(current === null ? {} : { screen: short(current.getAttribute("aria-label") ?? current.textContent, 60) }),
      ...(layer === null ? {} : { layer }),
      location: `${window.location.pathname}${window.location.search}${window.location.hash}`,
      bounds: box,
      window: { width: window.innerWidth, height: window.innerHeight },
      styles,
      markup: markupOf(element),
    },
  };
}

/**
 * The source of one ask, as main hands it to `executeJavaScript`.
 *
 * **An expression, not a statement**: what `executeJavaScript` answers with is
 * the value of the last one it evaluated, and that promise is the only channel
 * the page has to main. Every argument is serialised rather than named, so
 * nothing the page holds can change what is asked.
 */
export function askSource(x: number, y: number, take: boolean, bounds: AskBounds = ASK_BOUNDS): string {
  const args = [x, y, take, bounds].map((value) => JSON.stringify(value)).join(", ");
  return `(${readsThePage.toString()})(${args})`;
}
