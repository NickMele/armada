// A Studio's cards redrawn on every frame with something different on them, and
// what the window hears while they are.
//
// A card whose box changes with its words raises Chromium's `ResizeObserver`
// loop notice off React Flow's per-node observer. See
// `docs/spikes/019-what-raises-the-resize-loop-notice-on-a-studio.md`.
//
// Here rather than beside the component because `packages/components` runs only
// its stories, and the app's stylesheet is what sizes the card.

import { afterEach, expect, test } from "vitest";
import { StudioWhiteboard, type StudioWhiteboardNode } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { useEffect, useState } from "react";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const KINDS = ["link", "run", "note", "contradiction", "finding", "cluster", "sketch", "outline", "issue_draft", "job"] as const;

/**
 * A board whose cards say something different on every pass — a title from one
 * word to fourteen, and one fact or two. **Faster than any Studio changes**,
 * deliberately: the rule is that no rate of change in a card's words moves that
 * card's box.
 */
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

const PASSES = 150;

test("a board whose cards keep changing what they say holds its boxes and raises nothing", async () => {
  const raised: string[] = [];
  const listen = (event: ErrorEvent): void => {
    raised.push(event.message);
    // Read here rather than left to the runner, which fails a file on an
    // unhandled window error — and none arriving is what is under test.
    event.preventDefault();
  };
  window.addEventListener("error", listen);

  // The whole viewport, so the board has a box to lay nodes out in without this
  // file naming a length — `armada.yml`'s design rule reads every file here.
  const host = document.createElement("div");
  host.style.cssText = "position:fixed;inset:0";
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });

  // **Drawn from inside a frame, not awaited pass by pass.** A test that lets
  // each pass settle never leaves the observer's loop unsettled, and would see
  // nothing whatever the card did.
  let drawn = 0;
  const boxes = new Set<string>();
  function Churn({ done }: { done: () => void }) {
    const [pass, setPass] = useState(0);
    useEffect(() => {
      drawn = pass;
      for (const node of host.querySelectorAll<HTMLElement>(".react-flow__node")) {
        boxes.add(`${node.dataset.id ?? "?"}:${node.offsetHeight}`);
      }
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
  window.removeEventListener("error", listen);

  expect(drawn).toBe(PASSES);
  expect(host.querySelectorAll(".react-flow__node")).toHaveLength(KINDS.length);
  // The cause: one height per node over every pass. A card's box is its kind's,
  // never its words'.
  expect([...boxes], "a card's box changed with what it said").toHaveLength(KINDS.length);
  // The symptom it drove, which is what the owner was shown.
  expect(raised, "the board raised a window error while its cards changed").toEqual([]);
});
