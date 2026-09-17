import { describe, expect, it } from "vitest";

import { selectorFor, type NodeLike } from "./selector";

type Spec = { tag: string; id?: string; classes?: string[]; attrs?: Record<string, string>; children?: Spec[] };

/** Builds a tree of `NodeLike`s and returns every node by a path of child indexes. */
function build(spec: Spec, parent: NodeLike | null = null): NodeLike {
  const node: NodeLike = {
    tagName: spec.tag.toUpperCase(),
    id: spec.id ?? "",
    classList: spec.classes ?? [],
    parentElement: parent,
    children: [],
    getAttribute: (name) => spec.attrs?.[name] ?? null,
  };
  node.children = (spec.children ?? []).map((child) => build(child, node));
  return node;
}

const at = (node: NodeLike, ...path: number[]): NodeLike =>
  path.reduce((n, i) => n.children[i] as NodeLike, node);

describe("selectorFor", () => {
  it("anchors on a stable id, keeps armada- classes, and drops utility classes", () => {
    const body = build({
      tag: "body",
      children: [
        {
          tag: "div",
          id: "root",
          children: [{ tag: "div", classes: ["armada-shell", "flex"], children: [{ tag: "span" }] }],
        },
      ],
    });
    expect(selectorFor(at(body, 0, 0, 0))).toBe("#root > div.armada-shell > span");
  });

  it("skips an id React generated, which changes between renders", () => {
    const body = build({ tag: "body", children: [{ tag: "div", id: "«r3»", children: [{ tag: "p" }] }] });
    expect(selectorFor(at(body, 0, 0))).toBe("div > p");
  });

  it("tells siblings apart by position only when they would otherwise match", () => {
    const body = build({
      tag: "body",
      children: [
        {
          tag: "ul",
          id: "list",
          children: [{ tag: "li", classes: ["armada-row"] }, { tag: "li", classes: ["armada-row"] }, { tag: "p" }],
        },
      ],
    });
    expect(selectorFor(at(body, 0, 1))).toBe("#list > li.armada-row:nth-of-type(2)");
    expect(selectorFor(at(body, 0, 2))).toBe("#list > p");
  });

  it("prefers an accessible name, which is text the source spells", () => {
    const body = build({
      tag: "body",
      children: [
        {
          tag: "nav",
          id: "rail",
          children: [
            { tag: "button", attrs: { "aria-label": 'Job "Board"' } },
            { tag: "button", attrs: { "aria-label": "Alerts" } },
          ],
        },
      ],
    });
    expect(selectorFor(at(body, 0, 0))).toBe('#rail > button[aria-label="Job \\"Board\\""]');
  });

  it("shortens a deep chain to its two outer steps and the steps nearest the element", () => {
    let spec: Spec = { tag: "b" };
    for (let i = 0; i < 12; i++) spec = { tag: "div", classes: [`armada-d${i}`], children: [spec] };
    const body = build({ tag: "body", children: [spec] });
    let leaf = at(body, 0);
    while (leaf.children.length > 0) leaf = at(leaf, 0);
    const short = selectorFor(leaf);
    expect(short).toBe(
      "div.armada-d11 > div.armada-d10 div.armada-d4 > div.armada-d3 > div.armada-d2 > div.armada-d1 > div.armada-d0 > b",
    );
    expect(selectorFor(leaf, Number.POSITIVE_INFINITY).split(" > ")).toHaveLength(13);
  });
});
