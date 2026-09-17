// A Fleet keeping this repository's Studios: started, a node moved or removed, a relation decided,
// each written and published whole the way `studio.changed` carries it — #1287. `helmProposes` is
// the one write a person does not make, standing in for Helm adding nodes and proposing a relation.

import type { Studio, StudioEdge, StudioNode } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { foldStudio } from "@armada/screens/src/studio-reads";

import type { BridgeApi } from "../../../shared/api";
import { onBoard } from "./moment";
import type { FleetHandle, Scenario } from "./moment";

const OK = { ok: true } as const;
const MANIFEST_ID = repository().manifest!.id;

/** The Studios a mock Fleet keeps, and what Helm writes into one. */
export type StudioFleet = {
  scenario: Scenario;
  /** Every Studio as Fleet holds it now. */
  studios: () => readonly Studio[];
  /** Helm adds a Note and a Finding to a Studio, and proposes that the Finding answers the Note. */
  helmProposes: (studioId: string) => void;
};

let minted = 0;
const mint = (prefix: string) => `${prefix}${String(++minted).padStart(4, "0")}`;
/** Each write a second later than the last, so last touched first is a real order. */
let clock = Date.parse("2026-09-17T09:00:00Z");
const tick = () => new Date((clock += 1000)).toISOString();

const at = "2026-09-16T15:30:00Z";

/** A Studio already kept, so the list and the whiteboard have something to draw on the mock page. */
function legend(): Studio {
  return {
    id: "01STUDIOLEGEND0000000000000",
    manifest_id: MANIFEST_ID,
    name: "The Board's legend",
    created_at: at,
    touched_at: at,
    nodes: [
      { id: "legend-note", kind: "note", said: "The legend under the step bar is unreadable", position: { x: 0, y: 0 }, created_at: at },
      { id: "legend-width", kind: "note", said: "It wraps at 720 wide", position: { x: 0, y: 220 }, created_at: at },
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

export function studying(seeded: readonly Studio[] = [legend()]): StudioFleet {
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

  const behaves = (handle: FleetHandle): Partial<BridgeApi> => {
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
    scenario: {
      ...onBoard([], { picked: repository().root }),
      name: "studios",
      says: "This repository's Studios, on a Fleet that keeps them",
      behaves,
    },
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
