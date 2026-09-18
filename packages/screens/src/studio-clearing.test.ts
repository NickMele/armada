import { expect, test } from "vitest";
import type { Studio, StudioCapture } from "@armada/protocol";

import { clearingLabel, clearingOf, clearingSaid } from "./studio-clearing";

const AT = "2026-09-17T10:00:00Z";

/** A capture that kept a picture, which is the only field the count reads. */
const POINTED: StudioCapture = {
  selector: "button.armada-chip",
  element: { tag: "button", text: "Queued 3" },
  location: "/",
  bounds: { x: 0, y: 0, width: 96, height: 28 },
  window: { width: 1440, height: 900 },
  markup: "<button/>",
  frame: { filename: "n1.png", byte_size: 41_000, width: 1440, height: 900 },
};

/**
 * A Studio holding what each sentence is about: a Note with a picture and one
 * without, an Issue that produced a Note, a Job node, and an edge from a node
 * going to a node that stays.
 */
const STUDIO: Studio = {
  id: "s1",
  manifest_id: "armada",
  created_at: AT,
  touched_at: AT,
  nodes: [
    { id: "n1", kind: "note", said: "The legend is unreadable", capture: POINTED, position: { x: 0, y: 0 }, created_at: AT },
    { id: "n2", kind: "note", said: "It wraps at 720 wide", position: { x: 0, y: 300 }, created_at: AT },
    { id: "n3", kind: "issue", address: "https://example.invalid/o/r/issues/1411", number: "1411", position: { x: 300, y: 0 }, created_at: AT },
    { id: "n4", kind: "note", said: "What the issue said", position: { x: 300, y: 300 }, created_at: AT },
    { id: "n5", kind: "job", job_id: "j1", position: { x: 600, y: 0 }, created_at: AT },
  ],
  edges: [
    { id: "e1", from: "n3", to: "n4", kind: "produced", standing: "accepted", created_at: AT },
    { id: "e2", from: "n1", to: "n2", kind: "same_as", standing: "accepted", created_at: AT },
  ],
};

test("everything picked takes every edge, and nothing is left hanging or reaching outside", () => {
  const whole = clearingOf(STUDIO, ["n1", "n2", "n3", "n4", "n5"]);
  expect(whole).toEqual({ nodes: 5, edges: 2, toWhatStays: 0, frames: 1, leftHanging: 0, jobs: 1 });
  expect(clearingSaid(whole)).toEqual([
    "5 nodes go from this Studio, with the 2 edges on them.",
    "1 Note going keeps a picture, and it goes with the Note.",
    "1 Job node goes. The Job it names is untouched — a Studio holds a reference to a Job, never the Job.",
    "There is no undo.",
  ]);
});

test("an Issue picked without what it produced says the Note stays and the edge goes", () => {
  const some = clearingOf(STUDIO, ["n3"]);
  expect(some).toEqual({ nodes: 1, edges: 1, toWhatStays: 1, frames: 0, leftHanging: 1, jobs: 0 });
  expect(clearingSaid(some)).toEqual([
    "1 node goes from this Studio, with the 1 edge on it.",
    "1 of those edges reaches a node that stays. The node stays; the edge goes.",
    "1 node hanging off what is going — what a read-in or a scout produced — stays behind, with nothing left saying where it came from.",
    "There is no undo.",
  ]);
});

/** A node carrying no edge says so, rather than counting nothing at a person. */
test("a selection carrying no edges is told there are none", () => {
  const bare = clearingOf({ ...STUDIO, edges: [] }, ["n1", "n2"]);
  expect(bare.edges).toBe(0);
  expect(clearingSaid(bare)[0]).toBe("2 nodes go from this Studio, carrying no edges.");
});

/** Two things produced by picked nodes reaching one node that stays is one node left hanging. */
test("a node two picked nodes produced is one node left hanging, not two", () => {
  const twice: Studio = {
    ...STUDIO,
    edges: [
      { id: "e1", from: "n1", to: "n4", kind: "produced", standing: "accepted", created_at: AT },
      { id: "e2", from: "n2", to: "n4", kind: "produced", standing: "accepted", created_at: AT },
    ],
  };
  expect(clearingOf(twice, ["n1", "n2"]).leftHanging).toBe(1);
});

test("the act and the confirm are named the same, and one node is not pluralised", () => {
  expect(clearingLabel(18)).toBe("Delete 18 nodes");
  expect(clearingLabel(1)).toBe("Delete 1 node");
});
