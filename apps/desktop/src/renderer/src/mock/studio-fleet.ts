// A Fleet keeping this repository's Studios: started, a node moved or removed, a relation decided,
// each written and published whole the way `studio.changed` carries it — #1287. `helmProposes` is
// the one write a person does not make, standing in for Helm adding nodes and proposing a relation.
//
// **`keeping()` is every scenario's, and `studying()` is one scenario** — #1341. The store answers
// the Studio reads and writes wherever `fake.ts` puts it, so a scenario that keeps none answers an
// empty list and the surface draws its empty state; the `studios` scenario adds the Helm write.

import type { Studio, StudioCapture, StudioEdge, StudioNode } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { foldStudio } from "@armada/screens/src/studio-reads";

import type { BridgeApi } from "../../../shared/api";
import { onBoard, unanswered } from "./moment";
import type { FleetHandle, Scenario } from "./moment";

const OK = { ok: true } as const;
const MANIFEST_ID = repository().manifest!.id;

/**
 * Every Studio call a mock Fleet answers. **Named one by one**, so a Studio capability added to
 * `BridgeApi` fails typecheck here rather than going unanswered at runtime — `fake.ts`'s own rule.
 */
export type StudioRoutes = Pick<
  BridgeApi,
  | "watchStudios"
  | "watchStudio"
  | "createStudio"
  | "captureStudioNote"
  | "readStudioFrame"
  | "moveStudioNode"
  | "removeStudioNode"
  | "decideStudioEdge"
>;

/** The Studios a mock Fleet keeps, and what Helm writes into one. */
export type StudioKeeping = {
  /** What this Fleet answers the Studio calls with, over the window it publishes to. */
  routes: (fleet: FleetHandle) => StudioRoutes;
  /** Every Studio as Fleet holds it now. */
  studios: () => readonly Studio[];
  /** Helm adds a Note and a Finding to a Studio, and proposes that the Finding answers the Note. */
  helmProposes: (studioId: string) => void;
};

/** One scenario's own Fleet, keeping Studios. */
export type StudioFleet = StudioKeeping & { scenario: Scenario };

let minted = 0;
const mint = (prefix: string) => `${prefix}${String(++minted).padStart(4, "0")}`;
/** Each write a second later than the last, so last touched first is a real order. */
let clock = Date.parse("2026-09-17T09:00:00Z");
const tick = () => new Date((clock += 1000)).toISOString();

const at = "2026-09-16T15:30:00Z";

/**
 * The picture a Note kept, as a mock Fleet answers it — #1352.
 *
 * **Drawn here rather than kept as a file.** A mock Fleet has no disk and no
 * window to photograph, and what a scenario has to answer is bytes an `img` can
 * draw; a PNG committed beside this would be a binary nobody can read a diff of.
 * A window's bands, not its words: the colours and the boxes are a photograph's
 * and no token of the design system applies to one.
 */
async function aFrame(): Promise<Uint8Array> {
  const canvas = new OffscreenCanvas(1440, 900);
  const ink = canvas.getContext("2d")!;
  ink.fillStyle = "darkslategray";
  ink.fillRect(0, 0, 1440, 900);
  ink.fillStyle = "slategray";
  ink.fillRect(0, 0, 1440, 72);
  ink.fillStyle = "gainsboro";
  ink.fillRect(64, 160, 420, 560);
  ink.fillRect(548, 160, 828, 260);
  const png = await canvas.convertToBlob({ type: "image/png" });
  return new Uint8Array(await png.arrayBuffer());
}

/** What a Note's `capture` carries where the mock kept a frame for it. */
function pointedAt(nodeId: string): StudioCapture {
  return {
    selector: "button.armada-chip",
    element: { tag: "button", text: "Queued 3" },
    location: "/",
    bounds: { x: 312, y: 148, width: 96, height: 28 },
    window: { width: 1440, height: 900 },
    markup: '<button class="armada-chip">Queued 3</button>',
    frame: { filename: `${nodeId}.png`, byte_size: 41_000, width: 1440, height: 900 },
  };
}

/** A Studio already kept, so the list and the whiteboard have something to draw on the mock page. */
function legend(): Studio {
  return {
    id: "01STUDIOLEGEND0000000000000",
    manifest_id: MANIFEST_ID,
    name: "The Board's legend",
    created_at: at,
    touched_at: at,
    nodes: [
      // One Note with the picture it kept and one without: both are Notes, and only one draws a plate.
      { id: "legend-note", kind: "note", said: "The legend under the step bar is unreadable", capture: pointedAt("legend-note"), position: { x: 0, y: 0 }, created_at: at },
      { id: "legend-width", kind: "note", said: "It wraps at 720 wide", position: { x: 0, y: 300 }, created_at: at },
      { id: "legend-finding", kind: "finding", asked: "Where do the legend's colours come from?", state: "frozen", position: { x: 340, y: 0 }, created_at: at },
      { id: "legend-draft", kind: "issue_draft", title: "The Board's legend is illegible", body: "…", state: "draft", position: { x: 680, y: 110 }, created_at: at },
    ],
    edges: [
      { id: "legend-e1", from: "legend-note", to: "legend-finding", kind: "produced", standing: "accepted", created_at: at },
      { id: "legend-e2", from: "legend-width", to: "legend-note", kind: "same_as", standing: "proposed", created_at: at },
      { id: "legend-e3", from: "legend-finding", to: "legend-draft", kind: "produced", standing: "accepted", created_at: at },
    ],
  };
}

/** A Fleet keeping these Studios, answering every read and write on them. */
export function keeping(seeded: readonly Studio[] = []): StudioKeeping {
  const store = new Map<string, Studio>(seeded.map((one) => [one.id, one]));
  let fleet: FleetHandle | null = null;
  let listing: string | null = null;
  let opened: string | null = null;

  const listOf = (manifestId: string) =>
    [...store.values()].filter((one) => one.manifest_id === manifestId).reduce(foldStudio, []);

  /** What main publishes after a write it heard: the open Studio whole, and the list's row. */
  function published(studio: Studio): void {
    store.set(studio.id, studio);
    if (fleet === null) return;
    if (opened === studio.id) fleet.publish({ studio: { state: "read", studio } });
    if (listing === studio.manifest_id) {
      fleet.publish({ studios: { state: "read", manifestId: listing, list: { studios: listOf(listing) } } });
    }
  }

  function write(studioId: string, change: (studio: Studio) => Studio) {
    const held = store.get(studioId);
    if (held === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
    const studio = { ...change(held), touched_at: tick() };
    published(studio);
    return { ok: true, studio } as const;
  }

  const routes = (handle: FleetHandle): StudioRoutes => {
    fleet = handle;
    return {
      watchStudios: async (manifestId) => {
        listing = manifestId;
        handle.publish({
          studios: manifestId === null ? { state: "none" } : { state: "read", manifestId, list: { studios: listOf(manifestId) } },
        });
      },
      watchStudio: async (studioId) => {
        opened = studioId;
        const studio = studioId === null ? undefined : store.get(studioId);
        handle.publish({ studio: studio === undefined ? { state: "none" } : { state: "read", studio } });
      },
      createStudio: async (manifestId) => {
        const now = tick();
        const studio: Studio = { id: mint("01STUDIO"), manifest_id: manifestId, created_at: now, touched_at: now, nodes: [], edges: [] };
        published(studio);
        return { ok: true, studio };
      },
      // The frame is main's, and the mock has no window to take one of, so a
      // captured Note here carries everything but that.
      captureStudioNote: async (studioId, said, capture: StudioCapture) => {
        const answer = write(studioId, (studio) => {
          const y = studio.nodes.reduce((lowest, node) => Math.max(lowest, node.position.y), -260) + 260;
          const note: StudioNode = {
            id: mint("note-"),
            kind: "note",
            said,
            capture,
            position: { x: 0, y },
            created_at: tick(),
            added_by: "person",
          };
          return { ...studio, nodes: [...studio.nodes, note] };
        });
        return answer.ok ? OK : answer.outcome;
      },
      // The bytes of one Note's picture. A node that kept none is refused the
      // way Fleet refuses it, so the surface draws its sentence rather than an
      // image that never arrives.
      readStudioFrame: async (studioId, nodeId) => {
        const node = store.get(studioId)?.nodes.find((one) => one.id === nodeId);
        if (node?.kind !== "note" || node.capture?.frame === undefined) {
          return { ok: false, outcome: unanswered(`/studios/${studioId}/frames/${nodeId}`) };
        }
        return { ok: true, bytes: await aFrame(), type: "image/png" };
      },
      moveStudioNode: async (studioId, nodeId, position) => {
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: studio.nodes.map((node) => (node.id === nodeId ? { ...node, position } : node)),
        }));
        return answer.ok ? OK : answer.outcome;
      },
      removeStudioNode: async (studioId, nodeId) => {
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: studio.nodes.filter((node) => node.id !== nodeId),
          edges: studio.edges.filter((edge) => edge.from !== nodeId && edge.to !== nodeId),
        }));
        return answer.ok ? OK : answer.outcome;
      },
      decideStudioEdge: async (studioId, edgeId, accepted) => {
        const answer = write(studioId, (studio) => ({
          ...studio,
          edges: accepted
            ? studio.edges.map((edge): StudioEdge => (edge.id === edgeId ? { ...edge, standing: "accepted" } : edge))
            : studio.edges.filter((edge) => edge.id !== edgeId),
        }));
        return answer.ok ? OK : answer.outcome;
      },
    };
  };

  return {
    routes,
    studios: () => [...store.values()],
    helmProposes: (studioId) =>
      void write(studioId, (studio) => {
        const now = tick();
        const note: StudioNode = { id: mint("note-"), kind: "note", said: "Drag me somewhere", position: { x: 0, y: 0 }, created_at: now };
        const finding: StudioNode = {
          id: mint("finding-"),
          kind: "finding",
          asked: "What does the note point at?",
          state: "proposed",
          position: { x: 360, y: 0 },
          created_at: now,
        };
        const edge: StudioEdge = { id: mint("edge-"), from: finding.id, to: note.id, kind: "answers", standing: "proposed", created_at: now };
        return { ...studio, nodes: [...studio.nodes, note, finding], edges: [...studio.edges, edge] };
      }),
  };
}

/** The `studios` scenario: this repository picked, and a Fleet keeping these Studios. */
export function studying(seeded: readonly Studio[] = [legend()]): StudioFleet {
  const fleet = keeping(seeded);
  return {
    ...fleet,
    scenario: {
      ...onBoard([], { picked: repository().root }),
      name: "studios",
      says: "This repository's Studios, on a Fleet that keeps them",
      behaves: fleet.routes,
    },
  };
}

/**
 * The grid `everyKind()` is laid out on, wide enough for a node card and a gap. **Taller than it
 * is wide, and its top-right corner left empty**: the whiteboard fits the graph to the window and
 * draws the Proposed panel over that corner, so a node placed there is read through glass.
 *
 * The row pitch clears a Note carrying its frame, which is the tallest card there is — #1352.
 */
const place = (column: number, row: number) => ({ x: column * 340, y: row * 300 });

const MADE = "2026-09-15T11:00:00Z";
const TOUCHED = "2026-09-17T08:40:00Z";

/**
 * A Studio holding a node of every kind and an edge of every kind, proposed and accepted — what
 * `every-state` carries for Studios, the way it carries a Job in every state. `jobId` is a row on
 * that Board, so the Job node draws the state the Board holds rather than a bare id.
 */
export function everyKind(jobId: string): Studio {
  const nodes: StudioNode[] = [
    { id: "every-link", kind: "link", address: "docs/contracts/design-system.md#the-board", position: place(0, 0), created_at: MADE },
    // The one Note here that kept a picture — #1352. The rest draw no plate.
    { id: "every-note", kind: "note", said: "The legend under the step bar is unreadable", capture: pointedAt("every-note"), position: place(1, 0), created_at: MADE },
    { id: "every-note-wide", kind: "note", said: "It wraps at 720 wide", position: place(0, 1), created_at: MADE },
    { id: "every-note-states", kind: "note", said: "Queued and preparing read the same at a glance", position: place(1, 1), created_at: MADE },
    { id: "every-cluster", kind: "cluster", title: "The legend cannot be read", position: place(0, 2), created_at: MADE },
    {
      id: "every-run",
      kind: "run",
      run_id: "01RUNEVERYKIND000000000000",
      // Swept by retention, so the Studio kept the run whole — #1289. A Studio outlives a run's log.
      kept: {
        name: "typecheck",
        command: "pnpm typecheck",
        exit_code: 2,
        expect_exit_code: 0,
        stopped: false,
        duration_ms: 8400,
        lines: ["src/renderer/src/Board.tsx(212,9): error TS2322", "Found 1 error."],
        total_lines: 96,
        whole: false,
      },
      position: place(1, 2),
      created_at: MADE,
    },
    { id: "every-finding", kind: "finding", asked: "Where do the legend's colours come from?", state: "frozen", position: place(2, 2), created_at: MADE },
    { id: "every-finding-asked", kind: "finding", asked: "Which states share a token?", state: "proposed", position: place(2, 3), created_at: MADE },
    {
      id: "every-contradiction",
      kind: "contradiction",
      first: "The design contract gives the legend its own row",
      second: "The Board draws it inside the step bar",
      state: "reported",
      position: place(0, 3),
      created_at: MADE,
    },
    { id: "every-sketch", kind: "sketch", body: "Legend on its own row\nunder the step bar", state: "frozen", position: place(1, 3), created_at: MADE },
    { id: "every-deferral", kind: "deferral", what: "Whether the legend collapses under 720", state: "open", position: place(2, 4), created_at: MADE },
    { id: "every-outline", kind: "outline", body: "Give the legend its own row\nThen fix the contrast", state: "draft", position: place(1, 4), created_at: MADE },
    { id: "every-draft", kind: "issue_draft", title: "The Board's legend is illegible", body: "…", state: "draft", position: place(0, 4), created_at: MADE },
    { id: "every-job", kind: "job", job_id: jobId, position: place(1, 5), created_at: MADE },
  ];
  // `produced` is drawn by the Studio and never proposed (`docs/concepts/studio.md`, Edges), so
  // only the three relations are here twice, once waiting on a person and once decided.
  const edges: StudioEdge[] = ([
    ["every-produced-note", "every-link", "every-note", "produced", "accepted"],
    ["every-produced-contradiction", "every-link", "every-contradiction", "produced", "accepted"],
    ["every-produced-capture", "every-run", "every-note-states", "produced", "accepted"],
    ["every-produced-finding", "every-note", "every-finding", "produced", "accepted"],
    ["every-produced-cluster", "every-note-wide", "every-cluster", "produced", "accepted"],
    ["every-produced-outline", "every-finding", "every-outline", "produced", "accepted"],
    ["every-produced-draft", "every-outline", "every-draft", "produced", "accepted"],
    ["every-produced-sketch", "every-sketch", "every-draft", "produced", "accepted"],
    ["every-produced-job", "every-draft", "every-job", "produced", "accepted"],
    ["every-same-as", "every-note", "every-note-wide", "same_as", "accepted"],
    ["every-same-as-asked", "every-note", "every-note-states", "same_as", "proposed"],
    ["every-blocks", "every-deferral", "every-outline", "blocks", "accepted"],
    ["every-blocks-asked", "every-contradiction", "every-draft", "blocks", "proposed"],
    ["every-answers", "every-finding", "every-deferral", "answers", "accepted"],
    ["every-answers-asked", "every-finding-asked", "every-deferral", "answers", "proposed"],
  ] as const).map(([id, from, to, kind, standing]) => ({ id, from, to, kind, standing, created_at: MADE }));
  return {
    id: "01STUDIOEVERYKIND0000000000",
    manifest_id: MANIFEST_ID,
    name: "Every kind of node and edge",
    created_at: MADE,
    touched_at: TOUCHED,
    nodes,
    edges,
  };
}

/** A Studio nobody has named, so the list draws what an untitled one is called. */
export function untitled(): Studio {
  const at = "2026-09-16T17:05:00Z";
  return {
    id: "01STUDIOUNTITLED00000000000",
    manifest_id: MANIFEST_ID,
    created_at: at,
    touched_at: at,
    nodes: [
      { id: "untitled-link", kind: "link", address: "docs/concepts/studio.md#notes", position: place(0, 0), created_at: at },
      { id: "untitled-note", kind: "note", said: "Capture on another repository's web app waits on a security review", position: place(0, 1), created_at: at },
    ],
    edges: [{ id: "untitled-produced", from: "untitled-link", to: "untitled-note", kind: "produced", standing: "accepted", created_at: at }],
  };
}
