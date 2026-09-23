// Where each element's JSX is, stamped onto the element the build emits, so a
// note names a file and a line (#1584). Node, as `annotations-server.ts` is.
//
// **React no longer knows.** 19.2.8 declares
// `jsxDEV(type, config, maybeKey, isStaticChildren)` — the `source` the
// automatic development runtime passes is taken nowhere, and `_debugSource`
// appears zero times in `react-dom`'s development build. The compiler that read
// the JSX is the last thing that knows, so this is where it is read.

// **Host elements only.** A stamp on `<Button>` is a prop, which a component
// that spreads forwards and one that does not drops — one field with two
// meanings. On the host element it means one thing: the JSX that produced this
// DOM node.

// **`vite.mock.config.ts` alone.** `pnpm dev` and `pnpm package` both build
// through `electron-vite build`, so a stamp there would ship repository paths.

import { dirname, isAbsolute, relative, sep } from "node:path";

import { SOURCE_ATTRIBUTE } from "../shared/annotations";
import { repositoryRoot } from "./annotations";

/** The Babel node this reads. Structural, so this module needs no `@babel/core`. */
export type OpeningElement = {
  name?: { type?: string; name?: unknown } | null;
  attributes?: unknown[];
  loc?: { start?: { line?: unknown } | null } | null;
};

/** The three builders this calls on Babel's `types`, and nothing else. */
type Types = {
  jsxAttribute: (name: unknown, value: unknown) => unknown;
  jsxIdentifier: (name: string) => unknown;
  stringLiteral: (value: string) => unknown;
};

/** A host element: React's own rule for what it renders as a tag rather than a component. */
const HOST = /^[a-z]/;

// **A primitive is nobody's complaint.** Every button in the app draws the same
// `<button>`, so stamping it sends a note about one screen to a file shared by
// all of them. Unstamped, the nearest stamp is the composition that used it,
// which is where the change goes. `component` still says `Button`.
const PRIMITIVES = "packages/components/src/primitives/";

/**
 * What the element's stamp says, or null where it must not carry one — a
 * component rather than a host element, a primitive, a file outside the
 * repository, a node Babel gave no position, or one already stamped.
 */
export function stampFor(node: OpeningElement, filename: string, root: string | null): string | null {
  if (root === null) return null;
  const name = node.name;
  if (name?.type !== "JSXIdentifier" || typeof name.name !== "string" || !HOST.test(name.name)) return null;
  if ((node.attributes ?? []).some(isStamped)) return null;
  const line = node.loc?.start?.line;
  if (typeof line !== "number" || !Number.isInteger(line) || line < 1) return null;
  // Vite ids carry `?worker`, `?raw` and the like; the file is what is before one.
  const path = filename.split("?")[0] ?? "";
  if (!isAbsolute(path)) return null;
  const from = relative(root, path).split(sep).join("/");
  if (from === "" || from.startsWith("../") || from.includes("node_modules/")) return null;
  if (from.startsWith(PRIMITIVES)) return null;
  return `${from}:${line}`;
}

function isStamped(attribute: unknown): boolean {
  const name = (attribute as { name?: { name?: unknown } } | null)?.name?.name;
  return name === SOURCE_ATTRIBUTE;
}

/**
 * The Babel plugin, for `@vitejs/plugin-react`'s `babel.plugins`. The
 * repository is found from each file rather than passed in, so a file in
 * `packages/` reports a path from the same root as one in `apps/`, and a
 * worktree's Bridge reports paths inside that worktree.
 */
export function annotationSource(options: { root?: string } = {}) {
  const roots = new Map<string, string | null>();
  const rootOf = (filename: string): string | null => {
    if (options.root !== undefined) return options.root;
    const dir = dirname(filename.split("?")[0] ?? "");
    const known = roots.get(dir);
    if (known !== undefined) return known;
    const found = repositoryRoot(dir);
    roots.set(dir, found);
    return found;
  };
  return ({ types }: { types: Types }) => ({
    name: "armada-annotation-source",
    visitor: {
      JSXOpeningElement(path: { node: OpeningElement }, state: { filename?: string | null }) {
        const filename = state.filename;
        if (typeof filename !== "string") return;
        const stamp = stampFor(path.node, filename, rootOf(filename));
        if (stamp === null) return;
        (path.node.attributes ??= []).push(
          types.jsxAttribute(types.jsxIdentifier(SOURCE_ATTRIBUTE), types.stringLiteral(stamp)),
        );
      },
    },
  });
}
