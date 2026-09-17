// Which React components drew an element, read off React's internal fiber.
// Undocumented, dev-only, and allowed to come back empty: a note without a
// component still carries its selector.

/** The fields of a fiber this reads. Everything is optional; React does not promise any of it. */
export type FiberLike = {
  type?: unknown;
  return?: FiberLike | null;
  _debugOwner?: FiberLike | { name?: unknown } | null;
};

const MEMO = Symbol.for("react.memo");
const FORWARD_REF = Symbol.for("react.forward_ref");

/** A component's name, or null for a host element, a fragment, or an anonymous function. */
export function componentName(type: unknown): string | null {
  if (typeof type === "function") {
    const named = type as { displayName?: unknown; name?: unknown };
    const name = typeof named.displayName === "string" ? named.displayName : named.name;
    return typeof name === "string" && name !== "" ? name : null;
  }
  if (typeof type === "object" && type !== null) {
    const wrapped = type as { $$typeof?: unknown; displayName?: unknown; type?: unknown; render?: unknown };
    if (typeof wrapped.displayName === "string" && wrapped.displayName !== "") return wrapped.displayName;
    if (wrapped.$$typeof === MEMO) return componentName(wrapped.type);
    if (wrapped.$$typeof === FORWARD_REF) return componentName(wrapped.render);
  }
  return null;
}

/** The fiber React hung on a DOM node, walking up to the nearest node that has one. */
export function fiberOf(node: object | null): FiberLike | null {
  let at: (object & { parentNode?: object | null }) | null = node;
  while (at !== null && at !== undefined) {
    for (const key of Object.keys(at)) {
      if (key.startsWith("__reactFiber$")) return (at as Record<string, FiberLike>)[key] ?? null;
    }
    at = at.parentNode ?? null;
  }
  return null;
}

/** Collapses `Button > Button` from a wrapper and its inner function to one name. */
function pushName(names: string[], name: string | null): void {
  if (name !== null && names[names.length - 1] !== name) names.push(name);
}

const DEPTH = 16;

/**
 * The component under the click and the chain above it, nearest first.
 *
 * The owner chain starts at whichever component's render wrote the clicked
 * element, which is where its JSX is — often not the nearest parent, when that
 * parent is a primitive it was passed into. React keeps owners only in a
 * development build; without them the parent chain stands in, and `from` says so.
 */
export function componentsOf(fiber: FiberLike | null): {
  component: string | null;
  owners: string[];
  from: "owner" | "parent";
} {
  const parents: string[] = [];
  for (let at = fiber; at != null && parents.length < DEPTH; at = at.return ?? null) {
    pushName(parents, componentName(at.type));
  }
  const component = parents[0] ?? null;
  if (component === null) return { component: null, owners: [], from: "parent" };

  const owners: string[] = [component];
  let owner = fiber?._debugOwner;
  while (owner != null && owners.length < DEPTH) {
    // A server component's owner is `{ name }` rather than a fiber.
    const named = (owner as { name?: unknown }).name;
    pushName(owners, "type" in owner ? componentName(owner.type) : typeof named === "string" ? named : null);
    owner = "_debugOwner" in owner ? (owner as FiberLike)._debugOwner : null;
  }
  if (owners.length > 1) return { component, owners: owners.slice(1), from: "owner" };
  return { component, owners: parents.slice(1), from: "parent" };
}
