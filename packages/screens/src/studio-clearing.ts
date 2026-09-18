// What a delete of everything picked takes with it, said before the press — #1411.
//
// **Counting, never naming.** Eighteen titles is the panel that overflowed the window (#1399), and
// a person clearing a Studio out picked it over whole rather than node by node. What they are owed
// before the press is not the list back — they made it — but what goes that they did not pick:
// the edges reaching nodes that stay, the pictures the Notes going keep, and the nodes one of
// these produced, which stay behind with nothing saying where they came from.
//
// **A Job node is a reference.** Deleting one takes the node and leaves the Job, so the sentence
// says so where one is picked and is absent otherwise.

import type { Studio, StudioNode } from "@armada/protocol";

/** What a delete of one selection takes, and what it leaves. */
export type StudioClearing = {
  /** How many nodes go. Never zero: nothing offers the act on an empty selection. */
  nodes: number;
  /** Every edge with an end on a node going. All of them go. */
  edges: number;
  /** Of those, the ones whose other end stays. */
  toWhatStays: number;
  /** Pictures kept by the Notes going, deleted beside the records. */
  frames: number;
  /** Nodes a picked one produced that are not themselves picked. They stay. */
  leftHanging: number;
  /** A Job node is picked, so the sentence about a Job's own life is said. */
  jobs: number;
};

/** The frame a captured Note keeps, where it kept one. */
function keepsAFrame(node: StudioNode): boolean {
  return node.kind === "note" && node.capture?.frame !== undefined;
}

/** What deleting `picked` takes from `studio`, read off the graph Bridge already holds. */
export function clearingOf(studio: Studio, picked: readonly string[]): StudioClearing {
  const going = new Set(picked);
  const nodes = studio.nodes.filter((node) => going.has(node.id));
  const edges = studio.edges.filter((edge) => going.has(edge.from) || going.has(edge.to));
  return {
    nodes: nodes.length,
    edges: edges.length,
    toWhatStays: edges.filter((edge) => !going.has(edge.from) || !going.has(edge.to)).length,
    frames: nodes.filter(keepsAFrame).length,
    // By node and not by edge: two things a picked node produced reaching one
    // node that stays is one node left hanging, not two.
    leftHanging: new Set(
      studio.edges
        .filter((edge) => edge.kind === "produced" && going.has(edge.from) && !going.has(edge.to))
        .map((edge) => edge.to),
    ).size,
    jobs: nodes.filter((node) => node.kind === "job").length,
  };
}

const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

/**
 * What the confirmation says, a sentence at a time. **The first is always
 * there and the rest only where they are true**: a line reading "0 edges reach
 * a node that stays" is noise on the one screen where noise costs most.
 */
export function clearingSaid(clearing: StudioClearing): string[] {
  const one = clearing.nodes === 1;
  const said = [
    clearing.edges === 0
      ? `${plural(clearing.nodes, "node goes", "nodes go")} from this Studio, carrying no edges.`
      : `${plural(clearing.nodes, "node goes", "nodes go")} from this Studio, with the ${plural(clearing.edges, "edge", "edges")} on ${one ? "it" : "them"}.`,
  ];
  if (clearing.toWhatStays > 0) {
    said.push(
      `${plural(clearing.toWhatStays, "of those edges reaches", "of those edges reach")} a node that stays. The node stays; the edge goes.`,
    );
  }
  if (clearing.leftHanging > 0) {
    const hanging = clearing.leftHanging === 1;
    said.push(
      `${plural(clearing.leftHanging, "node", "nodes")} hanging off what is going — what a read-in or a scout produced — ${hanging ? "stays" : "stay"} behind, with nothing left saying where ${hanging ? "it" : "they"} came from.`,
    );
  }
  if (clearing.frames > 0) {
    const kept = clearing.frames === 1;
    said.push(
      `${plural(clearing.frames, "Note going keeps a picture", "Notes going keep a picture")}, and ${kept ? "it goes" : "each one goes"} with the Note.`,
    );
  }
  if (clearing.jobs > 0) {
    const job = clearing.jobs === 1;
    said.push(
      `${plural(clearing.jobs, "Job node goes", "Job nodes go")}. The ${job ? "Job it names is" : "Jobs they name are"} untouched — a Studio holds a reference to a Job, never the Job.`,
    );
  }
  said.push("There is no undo.");
  return said;
}

/** The act's label, and the confirm's: one name, said the same in both places. */
export const clearingLabel = (count: number): string => `Delete ${plural(count, "node", "nodes")}`;
