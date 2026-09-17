import { expect, test } from "vitest";
import type { JobSummary, Studio, StudioSummary } from "@armada/protocol";

import { nodeNamed, proposedRelations, studioName, UNTITLED_STUDIO, whiteboardEdges, whiteboardNodes } from "./studio";
import { foldStudio } from "./studio-reads";

const AT = "2026-09-17T10:00:00Z";

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
  ],
  edges: [
    { id: "e1", from: "n1", to: "n2", kind: "produced", standing: "accepted", created_at: AT },
    { id: "e2", from: "n2", to: "n5", kind: "answers", standing: "proposed", created_at: AT },
    { id: "e3", from: "n1", to: "n5", kind: "blocks", standing: "accepted", created_at: AT },
    { id: "e4", from: "n1", to: "n3", kind: "a_kind_from_a_newer_fleet", standing: "proposed", created_at: AT },
  ],
};

const JOB = { id: "j1", status: "running", title: "Fix the legend", handle: "12-fix-the-legend" } as JobSummary;

test("an untitled Studio is called one, and a named one by its name", () => {
  expect(studioName({})).toBe(UNTITLED_STUDIO);
  expect(studioName({ name: "The legend" })).toBe("The legend");
});

test("a node draws where a person left it, titled from what it holds", () => {
  const nodes = whiteboardNodes(STUDIO, [JOB]);
  expect(nodes.map((one) => one.position)).toEqual(STUDIO.nodes.map((one) => one.position));
  expect(nodes[0]!.node).toEqual({ kind: "note", title: "The legend is unreadable" });
  expect(nodes[1]!.node).toEqual({ kind: "finding", state: "gathering", title: "Where do its colours come from?" });
  expect(nodes[4]!.node).toMatchObject({ kind: "outline", state: "draft", title: "Legend, then width" });
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

test("a Run node carries no state, because the Studio holds only a reference", () => {
  expect(whiteboardNodes(STUDIO, [])[3]!.node).toEqual({ kind: "run", title: "r1" });
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
