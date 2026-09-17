// A Studio as the whiteboard draws it, off the wire — #1287. No React: every rule here is a function
// of the Studio Fleet sent and the Board this window holds, so it is tested as one.

import type { StudioWhiteboardEdge, StudioWhiteboardNode } from "@armada/components";
import type { JobSummary, Studio, StudioNode, StudioSummary } from "@armada/protocol";
import { STUDIO_EDGE_LABEL, STUDIO_NODE_KIND } from "@armada/components";

/**
 * What an untitled Studio is called wherever it is named. A name is optional on the wire and
 * Helm names an untitled Studio (`docs/concepts/studio.md`, Helm on a Studio), so until it does,
 * the list and the footer need a word that says nobody has.
 */
export const UNTITLED_STUDIO = "Untitled Studio";

/** The Studio's name, or what an untitled one is called. */
export function studioName(studio: Pick<StudioSummary, "name">): string {
  return studio.name ?? UNTITLED_STUDIO;
}

/** The first line of a body, for a node whose title is prose. */
function firstLine(body: string): string {
  return body.split("\n").find((line) => line.trim() !== "")?.trim() ?? body;
}

/** The node as a card: its kind, its state where it has one, a title, and facts. */
function cardOf(node: StudioNode, jobs: readonly JobSummary[]): StudioWhiteboardNode["node"] {
  switch (node.kind) {
    case "run":
      // A reference and never a state: the run is read by #1289, so nothing is said of it yet.
      return { kind: "run", title: node.run_id };
    case "job": {
      // The Job's state is the Board's, read off the row this window already holds.
      const job = jobs.find((one) => one.id === node.job_id);
      return job === undefined
        ? { kind: "job", title: node.job_id }
        : { kind: "job", state: job.status, title: job.title, facts: [job.handle] };
    }
    case "note":
      return { kind: "note", title: node.said };
    case "cluster":
      return { kind: "cluster", title: node.title };
    case "finding":
      return { kind: "finding", state: stateOf(node, "proposed"), title: node.asked };
    case "contradiction":
      return { kind: "contradiction", state: stateOf(node, "reported"), title: node.first, facts: [node.second] };
    case "sketch":
      return { kind: "sketch", state: "frozen", title: firstLine(node.body) };
    case "link":
      return { kind: "link", title: node.address };
    case "deferral":
      return { kind: "deferral", state: stateOf(node, "open"), title: node.what };
    case "outline":
      return { kind: "outline", state: stateOf(node, "draft"), title: firstLine(node.body) };
    case "issue_draft":
      return { kind: "issue_draft", state: "draft", title: node.title };
  }
}

/**
 * A node's state, or its kind's first where Fleet sent none. **Typed as the kind's own set on
 * trust**: Fleet only writes a state its kind declares, and `StudioNode` draws a word it does not
 * know as that word rather than failing.
 */
function stateOf<State extends string>(node: StudioNode, first: State): State {
  return (node.state ?? first) as State;
}

/** Every node, where a person left it. */
export function whiteboardNodes(studio: Studio, jobs: readonly JobSummary[]): StudioWhiteboardNode[] {
  return studio.nodes.map((node) => ({ id: node.id, position: node.position, node: cardOf(node, jobs) }));
}

const RELATIONS = ["same_as", "blocks", "answers"] as const;
type Relation = (typeof RELATIONS)[number];
const isRelation = (kind: string): kind is Relation => (RELATIONS as readonly string[]).includes(kind);

/**
 * Every edge the whiteboard can draw. **An edge kind this build does not know is left off** rather
 * than drawn as one it does — a relation drawn as the wrong relation is a claim nobody made.
 */
export function whiteboardEdges(studio: Studio): StudioWhiteboardEdge[] {
  return studio.edges.flatMap((edge): StudioWhiteboardEdge[] => {
    const ends = { id: edge.id, source: edge.from, target: edge.to };
    if (edge.kind === "produced") return [{ ...ends, kind: "produced" }];
    if (!isRelation(edge.kind)) return [];
    return [{ ...ends, kind: edge.kind, proposed: edge.standing === "proposed" }];
  });
}

/** How a node is named in a sentence: its kind, then its title. */
export function nodeNamed(studio: Studio, nodeId: string, jobs: readonly JobSummary[]): string {
  const node = studio.nodes.find((one) => one.id === nodeId);
  if (node === undefined) return nodeId;
  const card = cardOf(node, jobs);
  return `${STUDIO_NODE_KIND[card.kind]} ${card.title}`;
}

/** One proposed relation, waiting on a person, as it is read aloud. */
export type ProposedRelation = { id: string; from: string; relation: string; to: string };

/** Every relation proposed and not yet accepted, oldest first — the order Fleet keeps them in. */
export function proposedRelations(studio: Studio, jobs: readonly JobSummary[]): ProposedRelation[] {
  return studio.edges.flatMap((edge) =>
    edge.standing === "proposed" && isRelation(edge.kind)
      ? [
          {
            id: edge.id,
            from: nodeNamed(studio, edge.from, jobs),
            relation: STUDIO_EDGE_LABEL[edge.kind],
            to: nodeNamed(studio, edge.to, jobs),
          },
        ]
      : [],
  );
}
