import { describe, expect, it } from "vitest";

import { SOURCE_ATTRIBUTE } from "../shared/annotations";
import { annotationSource, stampFor, type OpeningElement } from "./annotations-source";

const ROOT = "/Users/user/armada";
const FILE = `${ROOT}/packages/screens/src/Board.tsx`;

function element(tag: string, extra: Partial<OpeningElement> = {}): OpeningElement {
  return { name: { type: "JSXIdentifier", name: tag }, attributes: [], loc: { start: { line: 42 } }, ...extra };
}

describe("what an element is stamped with", () => {
  it("is the file from the repository root and the line its JSX opens on", () => {
    expect(stampFor(element("div"), FILE, ROOT)).toBe("packages/screens/src/Board.tsx:42");
  });

  it("is read off the file, not the vite id, so a query does not become part of the path", () => {
    expect(stampFor(element("div"), `${FILE}?t=1730000000`, ROOT)).toBe("packages/screens/src/Board.tsx:42");
  });

  it("is nothing for a component element, whose stamp would be a prop and not an attribute", () => {
    expect(stampFor(element("Button"), FILE, ROOT)).toBeNull();
    expect(stampFor(element("Tabs", { name: { type: "JSXMemberExpression" } }), FILE, ROOT)).toBeNull();
  });

  it("is nothing where a stamp is already there, so the innermost one written stands", () => {
    const already = [{ name: { name: SOURCE_ATTRIBUTE } }];
    expect(stampFor(element("div", { attributes: already }), FILE, ROOT)).toBeNull();
  });

  it("is nothing without a position, a repository, or a file inside it", () => {
    expect(stampFor(element("div", { loc: null }), FILE, ROOT)).toBeNull();
    expect(stampFor(element("div"), FILE, null)).toBeNull();
    expect(stampFor(element("div"), "/elsewhere/App.tsx", ROOT)).toBeNull();
    expect(stampFor(element("div"), `${ROOT}/node_modules/thing/index.jsx`, ROOT)).toBeNull();
  });
});

describe("the plugin", () => {
  /** Babel's builders, as far as this uses them: enough to read what was pushed. */
  const types = {
    jsxAttribute: (name: unknown, value: unknown) => ({ name, value }),
    jsxIdentifier: (name: string) => ({ name }),
    stringLiteral: (value: string) => ({ value }),
  };

  it("adds one attribute to a host element and none to a component", () => {
    const plugin = annotationSource({ root: ROOT })({ types });
    const visit = (node: OpeningElement): void =>
      plugin.visitor.JSXOpeningElement({ node }, { filename: FILE });

    const host = element("div");
    visit(host);
    expect(host.attributes).toEqual([
      { name: { name: SOURCE_ATTRIBUTE }, value: { value: "packages/screens/src/Board.tsx:42" } },
    ]);

    const component = element("Button");
    visit(component);
    expect(component.attributes).toEqual([]);
  });
});
