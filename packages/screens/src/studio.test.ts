import { expect, test } from "vitest";
import type { JobSummary, Studio, StudioRunKept, StudioSummary } from "@armada/protocol";

import {
  framesDrawn,
  MOST_FRAMES_DRAWN,
  nodeNamed,
  proposedRelations,
  studioName,
  UNTITLED_STUDIO,
  whiteboardEdges,
  whiteboardNodes,
} from "./studio";
import { foldStudio } from "./studio-reads";

const AT = "2026-09-17T10:00:00Z";

/** An address of the length the owner pasted, which the card clips. */
const LONG_ADDRESS = "https://example.invalid/armada/issues/1378#issuecomment-2847190034-and-more";

const STUDIO: Studio = {
  id: "s1",
  manifest_id: "armada",
  created_at: AT,
  touched_at: AT,
  nodes: [
    { id: "n1", kind: "note", said: "The legend is unreadable", position: { x: 0, y: 0 }, created_at: AT },
    { id: "n2", kind: "finding", asked: "Where do its colours come from?", state: "gathering", position: { x: 300, y: 0 }, created_at: AT },
    { id: "n3", kind: "job", job_id: "j1", position: { x: 600, y: 0 }, created_at: AT },
    { id: "n4", kind: "run", run_id: "r1", position: { x: 0, y: 300 }, created_at: AT },
    { id: "n5", kind: "outline", body: "\nLegend, then width\nand more", position: { x: 300, y: 300 }, created_at: AT },
    { id: "n6", kind: "link", address: LONG_ADDRESS, said: "the owner's own report", position: { x: 600, y: 300 }, created_at: AT },
    { id: "n7", kind: "link", address: LONG_ADDRESS, position: { x: 900, y: 300 }, created_at: AT },
  ],
  edges: [
    { id: "e1", from: "n1", to: "n2", kind: "produced", standing: "accepted", created_at: AT },
    { id: "e2", from: "n2", to: "n5", kind: "answers", standing: "proposed", created_at: AT },
    { id: "e3", from: "n1", to: "n5", kind: "blocks", standing: "accepted", created_at: AT },
    { id: "e4", from: "n1", to: "n3", kind: "a_kind_from_a_newer_fleet", standing: "proposed", created_at: AT },
  ],
};

const JOB = { id: "j1", status: "running", title: "Fix the legend", handle: "12-fix-the-legend" } as JobSummary;

/** A run the Studio kept once retention swept it. `exit_code` absent is a run killed before it exited. */
const KEPT: StudioRunKept = {
  name: "typecheck",
  command: "pnpm typecheck",
  expect_exit_code: 0,
  stopped: false,
  duration_ms: 8400,
  lines: ["Found 1 error."],
  total_lines: 96,
  whole: false,
};

/** A Studio holding one Run node, kept as given. */
const ran = (kept: StudioRunKept): Studio => ({
  ...STUDIO,
  nodes: [{ id: "n4", kind: "run", run_id: "r1", kept, position: { x: 0, y: 0 }, created_at: AT }],
  edges: [],
});

test("an untitled Studio is called one, and a named one by its name", () => {
  expect(studioName({})).toBe(UNTITLED_STUDIO);
  expect(studioName({ name: "The legend" })).toBe("The legend");
});

test("a node draws where a person left it, titled from what it holds", () => {
  const nodes = whiteboardNodes(STUDIO, [JOB]);
  expect(nodes.map((one) => one.position)).toEqual(STUDIO.nodes.map((one) => one.position));
  expect(nodes[0]!.node).toEqual({ kind: "note", title: "The legend is unreadable" });
  // A scout asked about the code was handed no source, so the card says nothing about one.
  expect(nodes[1]!.node).toEqual({
    kind: "finding",
    state: "gathering",
    title: "Where do its colours come from?",
    facts: [],
  });
  expect(nodes[4]!.node).toMatchObject({ kind: "outline", state: "draft", title: "Legend, then width" });
});

test("a Link is titled by the person's line, and by its address where nobody wrote one", () => {
  const nodes = whiteboardNodes(STUDIO, []);
  expect(nodes[5]!.node).toEqual({ kind: "link", address: LONG_ADDRESS, title: "the owner's own report" });
  // A Link never stops being its address: with no line, the address is the
  // title, and the card says it once rather than twice — #1378.
  expect(nodes[6]!.node).toEqual({ kind: "link", address: LONG_ADDRESS, title: LONG_ADDRESS });
  expect(nodeNamed(STUDIO, "n6", [])).toBe("Link the owner's own report");
});

test("a Job node takes its state off the Board, and says none where the Board has no such Job", () => {
  expect(whiteboardNodes(STUDIO, [JOB])[2]!.node).toEqual({
    kind: "job",
    state: "running",
    title: "Fix the legend",
    facts: ["12-fix-the-legend"],
  });
  expect(whiteboardNodes(STUDIO, [])[2]!.node).toEqual({ kind: "job", title: "j1" });
});

test("a Run the Studio has not kept says so, and keeps its id as a fact rather than a title", () => {
  expect(whiteboardNodes(STUDIO, [])[3]!.node).toEqual({ kind: "run", title: "Not read yet", facts: ["r1"] });
});

test("a kept Run reads as the run sheet reads it: the name, the command, the exit and how long", () => {
  const kept = { ...KEPT, exit_code: 1, duration_ms: 12300 };
  expect(whiteboardNodes(ran(kept), [])[0]!.node).toEqual({
    kind: "run",
    state: "failed",
    title: "typecheck",
    facts: ["pnpm typecheck", "exit 1 (expects 0)", "12.3s"],
  });
  expect(whiteboardNodes(ran({ ...KEPT, exit_code: 0 }), [])[0]!.node).toMatchObject({ state: "passed" });
  // Killed before it exited: no exit line, and the state is what says it stopped.
  expect(whiteboardNodes(ran({ ...KEPT, stopped: true }), [])[0]!.node).toEqual({
    kind: "run",
    state: "stopped",
    title: "typecheck",
    facts: ["pnpm typecheck", "8.4s"],
  });
});

test("edges keep their standing, and a kind this build does not know is left off", () => {
  expect(whiteboardEdges(STUDIO)).toEqual([
    { id: "e1", source: "n1", target: "n2", kind: "produced" },
    { id: "e2", source: "n2", target: "n5", kind: "answers", proposed: true },
    { id: "e3", source: "n1", target: "n5", kind: "blocks", proposed: false },
  ]);
});

test("only a proposed relation waits on a person, named by its two ends", () => {
  expect(proposedRelations(STUDIO, [])).toEqual([
    { id: "e2", from: "Finding Where do its colours come from?", relation: "answers", to: "Outline Legend, then width" },
  ]);
  expect(nodeNamed(STUDIO, "gone", [])).toBe("gone");
});

test("a Studio Fleet wrote replaces its row, and the list stays last touched first", () => {
  const older: StudioSummary = { id: "a", manifest_id: "armada", created_at: AT, touched_at: "2026-09-16T09:00:00Z" };
  const newer: StudioSummary = { id: "b", manifest_id: "armada", created_at: AT, touched_at: "2026-09-17T09:00:00Z" };
  const touched = { ...older, name: "Now named", touched_at: "2026-09-17T11:00:00Z" };
  expect(foldStudio([newer, older], { ...touched, nodes: [], edges: [] } as StudioSummary).map((one) => one.id)).toEqual([
    "a",
    "b",
  ]);
  expect(foldStudio([newer, older], touched)[0]).toEqual(touched);
  expect(foldStudio([], newer)).toEqual([newer]);
});

/** A Note that kept a picture, and one that did not. `#1352`. */
const noted = (id: string, kept: boolean): Studio["nodes"][number] => ({
  id,
  kind: "note",
  said: `said ${id}`,
  ...(kept
    ? {
        capture: {
          selector: "button",
          element: { tag: "button", text: "Queued 3" },
          location: "/",
          bounds: { x: 0, y: 0, width: 10, height: 10 },
          window: { width: 1440, height: 900 },
          markup: "<button/>",
          frame: { filename: `${id}.png`, byte_size: 41_000, width: 1440, height: 900 },
        },
      }
    : {}),
  position: { x: 0, y: 0 },
  created_at: AT,
});

const many = (count: number): Studio => ({
  ...STUDIO,
  nodes: Array.from({ length: count }, (_, at) => noted(`n${at}`, true)),
  edges: [],
});

test("only a Note that kept a frame draws a plate, and the caller resolves what is on it", () => {
  const studio: Studio = { ...STUDIO, nodes: [noted("with", true), noted("without", false)], edges: [] };
  const drawn = framesDrawn(studio, null);
  const nodes = whiteboardNodes(studio, [], (nodeId) => (drawn.has(nodeId) ? { src: `blob:${nodeId}` } : {}));
  expect(nodes[0]!.node).toMatchObject({ kind: "note", frame: { src: "blob:with" } });
  // A Note that kept none carries no `frame` at all, which is what draws no box.
  expect(nodes[1]!.node).not.toHaveProperty("frame");
});

test("a board past the bound draws the first frames, and the selected Note wherever it sits", () => {
  const studio = many(MOST_FRAMES_DRAWN + 4);
  expect(framesDrawn(studio, null).size).toBe(MOST_FRAMES_DRAWN);
  const last = studio.nodes.at(-1)!.id;
  expect(framesDrawn(studio, null).has(last)).toBe(false);
  // Selecting it is what asks for it, and nothing else is dropped to make room.
  const withLast = framesDrawn(studio, last);
  expect(withLast.has(last)).toBe(true);
  expect(withLast.size).toBe(MOST_FRAMES_DRAWN + 1);
  // A selection that is not a Note with a frame changes nothing.
  expect(framesDrawn(studio, "nothing-of-the-sort").size).toBe(MOST_FRAMES_DRAWN);
});

test("a Link read in draws what the source calls itself, keeping its address as a fact", () => {
  const read = whiteboardNodes(
    {
      ...STUDIO,
      nodes: [
        { id: "a", kind: "link", address: "https://x/issues/1293", position: { x: 0, y: 0 }, created_at: AT },
        {
          id: "b",
          kind: "link",
          address: "https://x/issues/1291",
          named: "#1291 Promotion — closed",
          position: { x: 0, y: 200 },
          created_at: AT,
        },
      ],
    },
    [],
  );
  // A Link nobody has read in is its address, because that is all it has.
  expect(read[0]!.node).toEqual({ kind: "link", title: "https://x/issues/1293" });
  // One that was read in reads as what it is, and keeps the address a Job comes from.
  expect(read[1]!.node).toEqual({
    kind: "link",
    title: "#1291 Promotion — closed",
    facts: ["https://x/issues/1291"],
  });
});

test("a Finding says what it was handed beyond the checkout, and what was cut", () => {
  const read = whiteboardNodes(
    {
      ...STUDIO,
      nodes: [
        {
          id: "a",
          kind: "finding",
          asked: "Read in armada:thread",
          state: "frozen",
          sources: [
            { address: "armada:thread", kind: "thread", cut: 0 },
            { address: "https://x/page", kind: "page", cut: 12_400 },
          ],
          position: { x: 0, y: 0 },
          created_at: AT,
        },
      ],
    },
    [],
  );
  expect(read[0]!.node).toMatchObject({
    facts: ["Read this repository's Helm thread", "Read a page, 12,400 characters cut"],
  });
});
