import { describe, expect, it } from "vitest";

import type { WaveJobView, WaveView } from "./draft/wave";
import { waveReadingOf } from "./tab-wave";
import { waveDepths, waveNeedOf, waveNodeId, waveRunOf, waveSaid, waveStandingOf } from "./wave";

function job(over: Partial<WaveJobView> & Pick<WaveJobView, "job">): WaveJobView {
  return {
    title: `Job ${over.job}`,
    status: "running",
    round: 1,
    waits_on: [],
    ...over,
  };
}

function wave(jobs: WaveJobView[]): WaveView {
  return {
    job: "parent",
    title: "Carry the error contract through every surface",
    rounds: [{ round: 1, says: "The seam, then every surface", live: true }],
    jobs,
  };
}

describe("how far behind the wave a Job sits", () => {
  it("puts a Job one past the furthest of everything it waits on", () => {
    const depths = waveDepths([
      job({ job: "a" }),
      job({ job: "b", waits_on: ["a"] }),
      job({ job: "c", waits_on: ["a"] }),
      job({ job: "d", waits_on: ["b", "c"] }),
    ]);
    expect([...depths]).toEqual([
      ["a", 0],
      ["b", 1],
      ["c", 1],
      ["d", 2],
    ]);
  });

  it("ignores a Job it waits on that the wave does not hold", () => {
    const depths = waveDepths([job({ job: "a", waits_on: ["gone"] })]);
    expect(depths.get("a")).toBe(0);
  });

  // A recorded plan should carry no cycle. One that does draws a wave rather
  // than hanging, which is the only claim here.
  it("draws a wave whose Jobs wait on each other rather than hanging", () => {
    const depths = waveDepths([
      job({ job: "a", waits_on: ["b"] }),
      job({ job: "b", waits_on: ["a"] }),
    ]);
    expect(depths.size).toBe(2);
    // Bounded by the wave's own size, which is what stops the walk.
    for (const at of depths.values()) expect(at).toBeLessThanOrEqual(2);
  });
});

describe("which way a Job needs a person", () => {
  it.each([
    ["escalated", false, "blocked"],
    ["running", true, "blocked"],
    ["running", false, null],
    ["awaiting_review", false, "waiting"],
    ["awaiting_approval", false, "waiting"],
    ["awaiting_repair", false, "waiting"],
    ["awaiting_attestation", false, "waiting"],
    ["queued", false, null],
    ["completed_success", false, null],
    ["piloted", false, null],
  ])("%s, asking %s", (status, asking, want) => {
    expect(waveNeedOf(status, asking)).toBe(want);
  });
});

describe("the wave, placed", () => {
  const view = wave([
    job({ job: "a", status: "completed_success", landed: "merged" }),
    job({ job: "b", status: "awaiting_review", waits_on: ["a"] }),
    job({ job: "c", status: "escalated", waits_on: ["a"] }),
    job({ job: "d", status: "queued", waits_on: ["b", "c"] }),
  ]);

  it("draws the edge from the Job waited on to the Job that waits", () => {
    const { edges } = waveRunOf(view);
    // `waits_on` reads "b waits for a", and the edge has to read the other
    // way: a leads to b, so b is drawn behind it.
    expect(edges.map((edge) => [edge.source, edge.target])).toEqual([
      [waveNodeId("a"), waveNodeId("b")],
      [waveNodeId("a"), waveNodeId("c")],
      [waveNodeId("b"), waveNodeId("d")],
      [waveNodeId("c"), waveNodeId("d")],
    ]);
  });

  it("places a Job that waits further along than the one it waits on", () => {
    const { nodes } = waveRunOf(view);
    const at = (id: string) => nodes.find((node) => node.id === waveNodeId(id))!.position;
    expect(at("b").x).toBeGreaterThan(at("a").x);
    expect(at("d").x).toBeGreaterThan(at("b").x);
  });

  it("keeps two Jobs of one depth apart from each other", () => {
    const { nodes } = waveRunOf(view);
    const at = (id: string) => nodes.find((node) => node.id === waveNodeId(id))!.position;
    expect(at("b").x).toBe(at("c").x);
    expect(at("b").y).not.toBe(at("c").y);
  });

  it("drops an edge to a Job the wave does not hold rather than hanging it", () => {
    const { edges } = waveRunOf(wave([job({ job: "a", waits_on: ["gone"] })]));
    expect(edges).toEqual([]);
  });

  it("says a merged Job merged, and how many each still-out Job waits on", () => {
    const { nodes } = waveRunOf(view);
    const facts = (id: string) => nodes.find((node) => node.id === waveNodeId(id))!.card.facts;
    expect(facts("a")).toEqual(["merged"]);
    expect(facts("b")).toEqual(["waits on 1"]);
    expect(facts("d")).toEqual(["waits on 2"]);
  });

  it("opens a frame too small for the whole wave on the Jobs still out", () => {
    expect(waveRunOf(view).opensOn).toEqual([waveNodeId("b"), waveNodeId("c"), waveNodeId("d")]);
  });

  it("opens on every Job where all of them landed", () => {
    const all = wave([job({ job: "a", landed: "merged" }), job({ job: "b", landed: "merged" })]);
    expect(waveRunOf(all).opensOn).toEqual([waveNodeId("a"), waveNodeId("b")]);
  });

  it("gives a card its opener bound to that Job", () => {
    const opened: string[] = [];
    const { nodes } = waveRunOf(view, { onOpen: (id) => opened.push(id) });
    nodes.find((node) => node.id === waveNodeId("c"))!.card.onOpen!();
    expect(opened).toEqual(["c"]);
  });
});

describe("where each Job stands", () => {
  const view = wave([
    job({ job: "a", status: "completed_success", landed: "merged" }),
    job({ job: "b", status: "completed_success", landed: "merged" }),
    job({ job: "c", status: "awaiting_review" }),
    job({ job: "d", status: "escalated" }),
    job({ job: "e", status: "running" }),
  ]);

  it("tells the ones that merged apart from the ones still out", () => {
    const standing = waveStandingOf(view);
    expect(standing.landed.map((one) => one.job)).toEqual(["a", "b"]);
    expect(standing.out.map((one) => one.job)).toEqual(["c", "d", "e"]);
  });

  it("splits what needs you into blocked and waiting", () => {
    const standing = waveStandingOf(view, new Set(["e"]));
    expect(standing.blocked.map((one) => one.job)).toEqual(["d", "e"]);
    expect(standing.waiting.map((one) => one.job)).toEqual(["c"]);
  });

  // A Job that completed without merging is still out: landing is the pull
  // request merging, not the Job finishing (#1530).
  it("leaves a completed Job whose pull request has not merged among the ones still out", () => {
    const standing = waveStandingOf(wave([job({ job: "a", status: "completed_success" })]));
    expect(standing.landed).toEqual([]);
    expect(standing.out.map((one) => one.job)).toEqual(["a"]);
  });

  it("counts both halves in the sentence", () => {
    expect(waveSaid(waveStandingOf(view))).toBe(
      "5 Jobs under one plan — 2 merged, 3 still out.",
    );
  });

  it("says Job rather than Jobs where the wave dispatched one", () => {
    expect(waveSaid(waveStandingOf(wave([job({ job: "a" })])))).toBe(
      "1 Job under one plan — 0 merged, 1 still out.",
    );
  });
});

// **The wire cannot tell a wave from a landing order**, because both are read
// off `dispatched_by`. A derived wave drew the graph over the members moments,
// where the landing band is the right region — so nothing is derived, and
// `waveOf` waits for the wire to say which a Job is (`#1545`).
describe("which Jobs draw a wave", () => {
  it("draws one only where the moment says this Job is one", () => {
    const view = wave([job({ job: "a" })]);
    expect(waveReadingOf(null, { wave: view }, [])).toBe(view);
  });

  it("draws none from the Board's rows alone, however many a Job dispatched", () => {
    const whole = {
      job: { id: "parent" },
      created_at: "",
      steps: [],
      acceptance_criteria: [],
      dependencies: [],
    } as unknown as Parameters<typeof waveReadingOf>[0];
    const rows = [
      { id: "a", dispatched_by: "parent" },
      { id: "b", dispatched_by: "parent" },
    ] as unknown as Parameters<typeof waveReadingOf>[2];
    expect(waveReadingOf(whole, undefined, rows)).toBeUndefined();
    expect(waveReadingOf(whole, {}, rows)).toBeUndefined();
  });
});
