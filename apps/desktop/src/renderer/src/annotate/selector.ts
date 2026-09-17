// A selector that finds an element again after a reload, and that an agent can
// search the code for. Stable-ish: it prefers what the source spells — an
// `armada-` class, an accessible name — over position, and uses position only to
// tell siblings apart.

/** The parts of an `Element` this reads, so the rules are testable without a DOM. */
export type NodeLike = {
  tagName: string;
  id: string;
  classList: Iterable<string>;
  parentElement: NodeLike | null;
  children: ArrayLike<NodeLike>;
  getAttribute: (name: string) => string | null;
};

/** React's `useId` spells `«r1»` or `:r1:`, which changes between renders of the tree. */
function isStableId(id: string): boolean {
  return id !== "" && !/[«»:]/.test(id) && /^[A-Za-z][\w-]*$/.test(id);
}

const CSS_IDENT = /^[A-Za-z_][\w-]*$/;

/** A BEM class from the component library, which is what a search of the code finds. */
function sourceClasses(node: NodeLike): string[] {
  return [...node.classList].filter((c) => c.startsWith("armada-") && CSS_IDENT.test(c));
}

const quote = (value: string): string => `"${value.replace(/["\\]/g, "\\$&")}"`;

/** One step: tag, its `armada-` classes, and an accessible name or role if it has one. */
function step(node: NodeLike): string {
  let part = node.tagName.toLowerCase() + sourceClasses(node).map((c) => `.${c}`).join("");
  const label = node.getAttribute("aria-label");
  if (label !== null && label !== "" && label.length <= 60) part += `[aria-label=${quote(label)}]`;
  else {
    const role = node.getAttribute("role");
    if (role !== null && role !== "") part += `[role=${quote(role)}]`;
  }
  return part;
}

/** Whether `part` matches `node`, by the same rules `step` writes it with. */
function sameStep(node: NodeLike, part: string): boolean {
  return step(node) === part;
}

/** `:nth-of-type(n)` only when a sibling would match the same step. */
function disambiguated(node: NodeLike, part: string): string {
  const parent = node.parentElement;
  if (parent === null) return part;
  const siblings = Array.from(parent.children);
  if (siblings.filter((s) => sameStep(s, part)).length <= 1) return part;
  const sameTag = siblings.filter((s) => s.tagName === node.tagName);
  return `${part}:nth-of-type(${sameTag.indexOf(node) + 1})`;
}

const MAX_STEPS = 8;

/**
 * From the element up to the nearest anchor — a stable id, or `body` — joined
 * with `>`. Past `maxSteps` the middle of the chain becomes a descendant
 * combinator, so a wrapper added between two steps does not lose the element.
 */
export function selectorFor(element: NodeLike, maxSteps: number = MAX_STEPS): string {
  const parts: string[] = [];
  let at: NodeLike | null = element;
  while (at !== null) {
    if (isStableId(at.id)) {
      parts.unshift(`#${at.id}`);
      break;
    }
    const tag = at.tagName.toLowerCase();
    if (tag === "body" || tag === "html") break;
    parts.unshift(disambiguated(at, step(at)));
    at = at.parentElement;
  }
  if (parts.length <= maxSteps) return parts.join(" > ");
  const head = parts.slice(0, 2);
  const tail = parts.slice(-(maxSteps - 2));
  return `${head.join(" > ")} ${tail.join(" > ")}`;
}
