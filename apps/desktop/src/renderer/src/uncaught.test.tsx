// What Bridge's uncaught surface says, and what it does not.
//
// **This file is here and not beside `uncaught.ts` because `packages/shell` has
// no runner**, and because the second test needs the component the notice comes
// from — `@armada/components` — which only this app depends on beside the shell.
//
// The first test is the contract. The second is the reproduction that found it,
// kept so the next person does not have to hunt for it again: it fails if the
// board stops raising the notice, which would mean the repro has gone stale.

import { afterEach, expect, test } from "vitest";
import { StudioWhiteboard, type StudioWhiteboardNode } from "@armada/components";
import { watchUncaught, type Uncaught } from "@armada/shell";
import { createRoot, type Root } from "react-dom/client";
import { useEffect, useState } from "react";

/** Chromium's wording, which is Electron's, and what the owner was shown at 20:52Z on 17 Sep 2026. */
const NOTICE = "ResizeObserver loop completed with undelivered notifications.";

const stops: (() => void)[] = [];
const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const stop of stops.splice(0)) stop();
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

/** Watch, and collect. The unsubscribe is taken so no two tests hear each other's events. */
function caught(): Uncaught[] {
  const seen: Uncaught[] = [];
  stops.push(watchUncaught((one) => seen.push(one)));
  return seen;
}

test("the browser's ResizeObserver notice raises nothing, and a real throw still does", () => {
  const seen = caught();

  // The notice, exactly as Chromium sends it: no exception behind it.
  window.dispatchEvent(new ErrorEvent("error", { message: NOTICE }));
  expect(seen).toEqual([]);

  // A throw is still a throw.
  window.dispatchEvent(new ErrorEvent("error", { message: "boom", error: new Error("boom") }));
  expect(seen).toHaveLength(1);
  expect(seen[0]?.from).toBe("throw");
  expect(seen[0]?.message).toBe("boom");
  expect(seen[0]?.stack).not.toBeNull();

  // **The message alone does not buy silence.** An `Error` carrying the same
  // sentence is something Bridge did, and is reported.
  window.dispatchEvent(new ErrorEvent("error", { message: NOTICE, error: new Error(NOTICE) }));
  expect(seen).toHaveLength(2);
  expect(seen[1]?.message).toBe(NOTICE);

  // A rejection is untouched by any of this.
  window.dispatchEvent(
    new PromiseRejectionEvent("unhandledrejection", { promise: Promise.reject(new Error("nope")).catch(() => undefined), reason: new Error("nope") }),
  );
  expect(seen).toHaveLength(3);
  expect(seen[2]?.from).toBe("rejection");
});

const KINDS = ["link", "run", "note", "contradiction", "finding", "cluster", "sketch", "outline", "issue_draft", "job"] as const;

/** Nodes whose own boxes change on every pass — a Studio's cards as their titles and facts change. */
function board(pass: number): StudioWhiteboardNode[] {
  return KINDS.map((kind, i) => ({
    id: `n${i}`,
    position: { x: (i % 4) * 320, y: Math.floor(i / 4) * 240 },
    node: {
      kind,
      title: `${kind} ${"word ".repeat(1 + ((pass + i) % 14))}`,
      facts: ["one", "two"].slice(0, 1 + ((pass + i) % 2)),
    },
  })) as StudioWhiteboardNode[];
}

test("the whiteboard's nodes resizing raise the notice, and the surface stays quiet", async () => {
  const seen = caught();
  const notices: string[] = [];
  const listen = (event: ErrorEvent): void => {
    if (event.message.startsWith("ResizeObserver loop")) {
      notices.push(event.message);
      // The runner fails a test on an unhandled window error, and this one is
      // the thing under test rather than a failure.
      event.preventDefault();
    }
  };
  window.addEventListener("error", listen);
  stops.push(() => window.removeEventListener("error", listen));

  // The whole viewport, so the board has a box to lay nodes out in without this
  // file naming a length — `armada.yml`'s design rule reads every file here.
  const host = document.createElement("div");
  host.style.cssText = "position:fixed;inset:0";
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });

  // **Drawn from inside a frame, not awaited pass by pass.** React Flow measures
  // each node with a `ResizeObserver` and writes what it measured into its store,
  // which is read through `useSyncExternalStore` — a sync-lane update, flushed in
  // the microtask between two of the observer's own callbacks, so the commit lands
  // inside the observer's own delivery loop. A test that lets each pass settle
  // first never leaves the loop unsettled, and sees nothing.
  const PASSES = 150;
  let drawn = 0;
  function Churn({ done }: { done: () => void }) {
    const [pass, setPass] = useState(0);
    useEffect(() => {
      drawn = pass;
      if (pass >= PASSES) {
        done();
        return undefined;
      }
      const frame = requestAnimationFrame(() => setPass(pass + 1));
      return () => cancelAnimationFrame(frame);
    }, [pass, done]);
    return <StudioWhiteboard nodes={board(pass)} edges={[]} onNodeMoved={() => undefined} onSelectionChange={() => undefined} />;
  }

  await new Promise<void>((done) => root.render(<Churn done={done} />));
  await new Promise((settle) => setTimeout(settle, 400));
  expect(drawn).toBe(PASSES);
  expect(host.querySelectorAll(".react-flow__node")).toHaveLength(KINDS.length);

  expect(notices.length, "the board no longer raises the notice — the reproduction has gone stale").toBeGreaterThan(0);
  expect(seen, "Bridge reported a browser notice as a failure").toEqual([]);
});
