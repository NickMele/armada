import { describe, expect, it, vi } from "vitest";
import type { JobSummary, Proposed } from "@armada/protocol";

import { isAnnotation, requestOf, serializeAnnotation, type Annotation } from "../../../shared/annotations";
import { sendToFleet, unsendable, type Proposer } from "./send";
import type { Sink } from "./sink";

function note(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: "20260917-135315-tsfq",
    status: "open",
    text: "Can we add an illustration here?",
    component: "BoardEmptyState",
    owners: ["BoardEmpty", "ActiveJobsList", "OverviewLists"],
    ownersFrom: "parent",
    selector: "#root div.armada-board-empty",
    element: { tag: "div", text: "No jobs. Propose one above.", label: null },
    screen: "Overview",
    layer: null,
    location: "/index.html",
    scenario: null,
    box: { x: 254, y: 184, width: 557, height: 200 },
    window: { width: 1280, height: 800 },
    createdAt: "2026-09-17T13:53:15.999Z",
    updatedAt: "2026-09-17T13:53:26.658Z",
    ...overrides,
  };
}

function sink(overrides: Partial<Sink> = {}): Sink {
  return {
    via: "main",
    list: async () => [],
    save: async () => undefined,
    remove: async () => undefined,
    root: async () => "/Users/user/armada",
    capture: async () => new Uint8Array([137, 80, 78, 71]).buffer,
    ...overrides,
  };
}

const JOB = { id: "01M3JOB00000000000000000000", handle: "42-add-an-illustration" } as JobSummary;

function fleet(answer: Proposed): Proposer & { proposeFromRequest: ReturnType<typeof vi.fn> } {
  return {
    stageAttachment: vi.fn(async (_bytes: ArrayBuffer, filename: string) => ({ path: `/tmp/staged/${filename}` })),
    proposeFromRequest: vi.fn(async () => answer),
  };
}

const AT = new Date("2026-09-17T14:00:00.000Z");

describe("a note's request", () => {
  it("leads with the person's words, then says where in Bridge they point", () => {
    const text = requestOf(note());
    expect(text.split("\n")[0]).toBe("Can we add an illustration here?");
    expect(text).toContain("Component: BoardEmptyState inside BoardEmpty inside ActiveJobsList inside OverviewLists");
    expect(text).toContain('Element: <div> reading "No jobs. Propose one above."');
    expect(text).toContain("Screen: Overview");
    expect(text).toContain("Note file: .armada/annotations/20260917-135315-tsfq.json");
    expect(text).not.toContain("Mock scenario");
  });
});

describe("a note's source", () => {
  it("is written beside the component it names, and says where the code is in the request", () => {
    const located = note({ source: { file: "packages/screens/src/Board.tsx", line: 88 } });
    expect(Object.keys(JSON.parse(serializeAnnotation(located)) as object).slice(3, 6)).toEqual([
      "component",
      "source",
      "owners",
    ]);
    expect(requestOf(located)).toContain("Source: packages/screens/src/Board.tsx:88");
  });

  it("is absent from a note written before the build stamped anything, which still reads", () => {
    expect(isAnnotation(note())).toBe(true);
    expect(Object.keys(JSON.parse(serializeAnnotation(note())) as object)).not.toContain("source");
    expect(requestOf(note())).not.toContain("Source:");
    expect(isAnnotation({ ...note(), source: { file: "packages/screens/src/Board.tsx" } })).toBe(false);
  });
});

describe("a sent note", () => {
  it("is still a note, and keeps where it went just after its status", () => {
    const sent = note({ sent: { jobId: JOB.id, handle: JOB.handle, at: AT.toISOString() } });
    expect(isAnnotation(sent)).toBe(true);
    const keys = Object.keys(JSON.parse(serializeAnnotation(sent)) as object);
    expect(keys.slice(0, 3)).toEqual(["id", "status", "sent"]);
  });

  it("is refused with a sent record missing its Job", () => {
    expect(isAnnotation({ ...note(), sent: { handle: "42-x", at: AT.toISOString() } })).toBe(false);
  });
});

describe("sending to Fleet", () => {
  it("proposes the request against the notes' repository, with the screenshot staged, and names the Job", async () => {
    const proposer = fleet({ ok: true, jobs: [JOB] });
    const answer = await sendToFleet(note(), note().box, sink(), proposer, AT);
    expect(answer).toEqual({ ok: true, sent: { jobId: JOB.id, handle: JOB.handle, at: AT.toISOString() } });
    expect(proposer.stageAttachment).toHaveBeenCalledWith(expect.any(ArrayBuffer), "annotation-20260917-135315-tsfq.png", "image/png");
    expect(proposer.proposeFromRequest).toHaveBeenCalledWith(
      requestOf(note()),
      [{ path: "/tmp/staged/annotation-20260917-135315-tsfq.png", filename: "annotation-20260917-135315-tsfq.png", mimeType: "image/png" }],
      "/Users/user/armada",
    );
  });

  it("sends without a screenshot where none can be taken", async () => {
    const proposer = fleet({ ok: true, jobs: [JOB] });
    await sendToFleet(note(), note().box, sink({ capture: async () => null }), proposer, AT);
    expect(proposer.stageAttachment).not.toHaveBeenCalled();
    expect(proposer.proposeFromRequest.mock.calls[0]![1]).toEqual([]);
  });

  it("says what Fleet said when it takes nothing, and sends nothing when there is no repository", async () => {
    const refused = await sendToFleet(note(), note().box, sink(), fleet({ ok: false, why: "refused", outcome: { ok: false, why: "not_connected" } }), AT);
    expect(refused).toEqual({ ok: false, saying: "Fleet is not connected. Nothing was sent." });

    const proposer = fleet({ ok: true, jobs: [JOB] });
    const rootless = await sendToFleet(note(), note().box, sink({ root: async () => null }), proposer, AT);
    expect(rootless.ok).toBe(false);
    expect(proposer.proposeFromRequest).not.toHaveBeenCalled();
  });

  it("is not offered where there is no Fleet behind the layer", () => {
    expect(unsendable(sink())).toBeNull();
    expect(unsendable(sink({ via: "dev server" }))).toMatch(/mock/);
  });
});
