// What the layer reads in the page, and what it refuses to carry out of one —
// #1294, `docs/practices/capture-window.md`, *The injected layer*.
//
// **Run against a stub page rather than a DOM**, because main's project has no
// `lib.dom` and this code is not main's to run — it is source that crosses into
// a foreign document. The stub is exactly `PageElement`, which is exactly what
// the layer may touch, so a reading this cannot express is one the layer cannot
// make either.

import { describe, expect, it } from "vitest";

import { ASK_BOUNDS, askSource, readsThePage } from "./layer";

type Built = {
  tag: string;
  attributes?: Record<string, string>;
  text?: string;
  html?: string;
  children?: Built[];
  fiber?: string[];
};

/** One node of the stub page, with only what `PageElement` declares. */
function node(built: Built, parent: unknown = null): Record<string, unknown> {
  const attributes: Record<string, string> = { ...built.attributes };
  const children: Record<string, unknown>[] = [];
  const it: Record<string, unknown> = {
    tagName: built.tag.toUpperCase(),
    id: attributes["id"] ?? "",
    textContent: built.text ?? "",
    parentElement: parent,
    parentNode: parent,
    children,
    get attributes() {
      return Object.keys(attributes).map((name) => ({ name }));
    },
    getAttribute: (name: string) => attributes[name] ?? null,
    removeAttribute: (name: string) => {
      delete attributes[name];
    },
    closest: (selector: string) => (selector === "svg" ? null : null),
    querySelectorAll: () => children,
    cloneNode: () => node(built),
    replaceChildren: () => {
      children.length = 0;
      it["textContent"] = "";
      it["emptied"] = true;
    },
    getBoundingClientRect: () => ({ x: 12, y: 34, width: 56, height: 78 }),
    get outerHTML(): string {
      const spelled = Object.entries(attributes)
        .map(([name, value]) => ` ${name}="${value}"`)
        .join("");
      const inside =
        it["emptied"] === true
          ? ""
          : (built.html ?? [...(built.children ?? []).map(() => ""), built.text ?? ""].join(""));
      return `<${built.tag}${spelled}>${inside}</${built.tag}>`;
    },
  };
  // React's own expando, which is the whole reason this runs in the page's world.
  if (built.fiber !== undefined) {
    let fiber: Record<string, unknown> | null = null;
    for (const name of [...built.fiber].reverse()) {
      const named = (): null => null;
      Object.defineProperty(named, "name", { value: name });
      fiber = { type: named, return: fiber };
    }
    it["__reactFiber$abc"] = fiber;
  }
  for (const child of built.children ?? []) children.push(node(child, it));
  return it;
}

/** The page the ask runs against, with one element under the pointer. */
function page(under: Built): { answer: (take: boolean) => unknown } {
  const element = node(under);
  const globals = globalThis as Record<string, unknown>;
  globals["document"] = {
    elementFromPoint: () => element,
    querySelector: () => null,
    getElementById: () => null,
  };
  globals["window"] = {
    getComputedStyle: () => ({
      getPropertyValue: (property: string) =>
        ({ color: "rgb(1, 2, 3)", cursor: "pointer" })[property] ?? "",
    }),
    location: { pathname: "/checkout", search: "?step=2", hash: "#pay" },
    innerWidth: 900,
    innerHeight: 600,
  };
  return { answer: (take: boolean) => readsThePage(10, 20, take, ASK_BOUNDS) };
}

type Answer = { box: unknown; named: string; capture?: Record<string, unknown> };

describe("what the layer reads", () => {
  it("names the component off React's own expando, which an isolated world cannot see", () => {
    const answer = page({ tag: "button", text: "Pay", fiber: ["PayButton", "Checkout", "App"] }).answer(true) as Answer;
    expect(answer.named).toBe("PayButton");
    expect(answer.capture).toMatchObject({ component: "PayButton", owners: ["Checkout", "App"] });
  });

  it("falls back to the tag on a page React did not draw, and records everything else", () => {
    const answer = page({ tag: "button", text: "Pay" }).answer(true) as Answer;
    expect(answer.named).toBe("button");
    expect(answer.capture).toMatchObject({ element: { tag: "button", text: "Pay" }, location: "/checkout?step=2#pay" });
    expect(answer.capture?.["component"]).toBeUndefined();
  });

  it("answers the box and the name alone when it is not asked to take one", () => {
    const answer = page({ tag: "button", text: "Pay" }).answer(false) as Answer;
    expect(answer.box).toEqual({ x: 12, y: 34, width: 56, height: 78 });
    expect(answer.capture).toBeUndefined();
  });

  it("keeps only the declared style properties", () => {
    const answer = page({ tag: "button", text: "Pay" }).answer(true) as Answer;
    expect(answer.capture?.["styles"]).toEqual({ color: "rgb(1, 2, 3)" });
  });
});

describe("what the markup never carries", () => {
  it("drops a `value` attribute, which is what a person typed", () => {
    const answer = page({ tag: "div", attributes: { value: "4242 4242 4242 4242", class: "row" } }).answer(
      true,
    ) as Answer;
    expect(answer.capture?.["markup"]).not.toContain("4242");
    expect(answer.capture?.["markup"]).toContain('class="row"');
  });

  it("empties a textarea rather than keeping its contents", () => {
    const answer = page({ tag: "textarea", text: "my whole draft", html: "my whole draft" }).answer(true) as Answer;
    expect(answer.capture?.["markup"]).toBe("<textarea></textarea>");
  });

  it("strips a password field to its type and nothing else", () => {
    const answer = page({
      tag: "input",
      attributes: { type: "password", value: "hunter2", name: "password", autocomplete: "off" },
    }).answer(true) as Answer;
    expect(answer.capture?.["markup"]).toBe('<input type="password"></input>');
  });
});

describe("the source of one ask", () => {
  it("is an expression, which is the only channel the page has to main", () => {
    const source = askSource(10, 20, true);
    expect(source.startsWith("(function")).toBe(true);
    expect(source.endsWith(")")).toBe(true);
  });

  it("serialises every argument rather than naming one, so the page cannot change the ask", () => {
    expect(askSource(10, 20, false)).toContain(")(10, 20, false, {");
    expect(askSource(10, 20, false)).toContain('"markup":2000');
  });

  it("leaves nothing in the page: no global is named and no listener is added", () => {
    const source = askSource(1, 2, true);
    expect(source).not.toContain("addEventListener");
    expect(source).not.toContain("window.__");
    expect(source).not.toContain("localStorage");
    expect(source).not.toContain("cookie");
    expect(source).not.toContain("fetch");
  });
});
