// A Studio as the whiteboard draws it, off the wire — #1287. No React: every rule here is a function
// of the Studio Fleet sent and the Board this window holds, so it is tested as one.

import type { StudioNodeFrame, StudioWhiteboardEdge, StudioWhiteboardNode } from "@armada/components";
import type {
  EpicRead,
  JobSummary,
  ScoutSource,
  Studio,
  StudioNode,
  StudioRunKept,
  StudioSummary,
} from "@armada/protocol";
import { STUDIO_EDGE_LABEL, STUDIO_NODE_KIND } from "@armada/components";

import { runOutcomeOf } from "./rehearsal";

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

/**
 * What a kept run says beside its name: the command, its exit against what was expected, and how
 * long it took. **The run sheet's own wording**, so a run reads the same on a Studio as it does
 * there — `pastRunOf` in `rehearsal.ts`. A run killed before it exited has no exit line, and its
 * state already says it stopped.
 */
function runFacts(kept: StudioRunKept): string[] {
  const took = `${(kept.duration_ms / 1000).toFixed(1)}s`;
  return kept.exit_code === undefined
    ? [kept.command, took]
    : [kept.command, `exit ${kept.exit_code} (expects ${kept.expect_exit_code})`, took];
}

/** What a scout was handed, as one chip: what it was read as, and what was cut. */
function sourceRead(source: ScoutSource): string {
  const read = SOURCE_READ[source.kind] ?? source.kind;
  return source.cut === 0 ? read : `${read}, ${source.cut.toLocaleString()} characters cut`;
}

/** What each source kind is called where it is read. `docs/concepts/scout.md`. */
const SOURCE_READ: Readonly<Record<string, string>> = {
  issue: "Read an issue",
  pull_request: "Read a pull request",
  milestone: "Read a milestone",
  page: "Read a page",
  session: "Read a session",
  thread: "Read this repository's Helm thread",
};

/** The first line of a body, for a node whose title is prose. */
function firstLine(body: string): string {
  return body.split("\n").find((line) => line.trim() !== "")?.trim() ?? body;
}

/**
 * Where something on a forge stands, in the words a card draws. #1394.
 *
 * **A word this build does not know is drawn as itself**, `StudioNode`'s rule
 * for a state: a newer Fleet's word said plainly beats no word at all.
 */
function forgeState(state: string): string {
  return { open: "Open", closed: "Closed", merged: "Merged" }[state] ?? state;
}

/**
 * How much of an Epic is on the Studio. **An Epic that fits says how many it
 * holds**, rather than saying the same number twice — the two-number form is
 * there to say a read was capped, and drawing it always would make a whole
 * milestone read as a partial one.
 */
function epicRead(read: EpicRead): string {
  return read.issues === read.total
    ? `${read.total} issues`
    : `${read.issues} of ${read.total} issues`;
}

/**
 * How many of a Studio's frames are drawn at once — #1352.
 *
 * **A bound, because a frame is a file and a Studio is kept until it is
 * deleted.** Every Note on a board fetching its own picture is what freezes a
 * window, which is the v1 failure Bridge exists against; the selected Note is
 * always among them, so a frame past the bound is one press away.
 */
export const MOST_FRAMES_DRAWN = 24;

/** Whether this node kept a picture, which is what decides that a plate is drawn at all. */
function keptAFrame(node: StudioNode): boolean {
  return node.kind === "note" && node.capture?.frame !== undefined;
}

/** The Notes whose frames this window asks for: the first `MOST_FRAMES_DRAWN`, and the selected one. */
export function framesDrawn(studio: Studio, selected: string | null): ReadonlySet<string> {
  const kept = studio.nodes.filter(keptAFrame).map((node) => node.id);
  const drawn = new Set(kept.slice(0, MOST_FRAMES_DRAWN));
  if (selected !== null && kept.includes(selected)) drawn.add(selected);
  return drawn;
}

/** What a node's frame is, as the window holds it. A node that kept none takes none. */
export type FrameOf = (nodeId: string) => StudioNodeFrame;

/** For a caller whose subject is not the pictures — every Note reads as one still being fetched. */
export const NO_FRAME_HELD: FrameOf = () => ({});

/**
 * The node as a card: its kind, its state where it has one, a title, and facts.
 * **`null` on a kind this build does not know** — `whiteboardEdges`' rule for an
 * edge, one scope over: a node drawn as a kind it is not is a claim nobody made,
 * and a newer Fleet may send one (#1394 added three).
 */
function cardOf(
  node: StudioNode,
  jobs: readonly JobSummary[],
  frameOf: FrameOf,
): StudioWhiteboardNode["node"] | null {
  switch (node.kind) {
    case "run":
      // What was run, the way the run sheet names it — the Check's name, its command and its
      // result. **A run whose record the Studio has not kept says so** rather than drawing its
      // id as a title: the id names the run to Fleet and says nothing to a person, and the run
      // is still Fleet's to read (#1289). It stays as a fact, where an identifier belongs.
      return node.kept === undefined
        ? { kind: "run", title: "Not read yet", facts: [node.run_id] }
        : { kind: "run", state: runOutcomeOf(node.kept), title: node.kept.name, facts: runFacts(node.kept) };
    case "job": {
      // The Job's state is the Board's, read off the row this window already holds.
      const job = jobs.find((one) => one.id === node.job_id);
      return job === undefined
        ? { kind: "job", title: node.job_id }
        : { kind: "job", state: job.status, title: job.title, facts: [job.handle] };
    }
    case "note":
      // **The plate is drawn only where the Note kept a picture.** One that was
      // typed, or captured where no frame could be taken, is an ordinary Note.
      return keptAFrame(node)
        ? { kind: "note", title: node.said, frame: frameOf(node.id) }
        : { kind: "note", title: node.said };
    case "cluster":
      return { kind: "cluster", title: node.title };
    case "finding":
      // What it was handed beyond the checkout, and what did not fit. A scout
      // asked about the code alone has none and says nothing about sources.
      return {
        kind: "finding",
        state: stateOf(node, "proposed"),
        title: node.asked,
        facts: (node.sources ?? []).map(sourceRead),
      };
    case "contradiction":
      // The answer is a fact beside the two statements, not in place of either: `Resolved here`
      // records what was decided, and the disagreement it settles stays readable under it.
      return {
        kind: "contradiction",
        state: stateOf(node, "reported"),
        title: node.first,
        facts: node.answer === undefined ? [node.second] : [node.second, node.answer],
      };
    case "sketch":
      return { kind: "sketch", state: "frozen", title: firstLine(node.body) };
    case "link":
      // **The title is the person's own line first, then what a read-in
      // learned the source calls itself, then the address** — #1378, #1293.
      // A person's line wins because it is theirs, and the address is drawn
      // under whichever it was, so nothing is said twice.
      return { kind: "link", address: node.address, title: node.said ?? node.named ?? node.address };
    // The three kinds a forge address makes — #1394. The title follows a
    // Link's rule, with what the forge calls it where a read-in learned one;
    // everything else the kind holds is a fact rather than a sentence.
    case "issue":
    case "pull_request":
      return {
        kind: node.kind,
        address: node.address,
        title: node.said ?? node.title ?? node.address,
        facts: [`#${node.number}`, ...(node.state === undefined ? [] : [forgeState(node.state)])],
      };
    case "epic":
      return {
        kind: "epic",
        address: node.address,
        title: node.said ?? node.title ?? node.address,
        facts: [`#${node.number}`, ...(node.read_in === undefined ? [] : [epicRead(node.read_in)])],
      };
    case "deferral":
      return { kind: "deferral", state: stateOf(node, "open"), title: node.what };
    case "outline":
      return { kind: "outline", state: stateOf(node, "draft"), title: firstLine(node.body) };
    case "issue_draft":
      return { kind: "issue_draft", state: "draft", title: node.title };
    default:
      return null;
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
export function whiteboardNodes(
  studio: Studio,
  jobs: readonly JobSummary[],
  frameOf: FrameOf = NO_FRAME_HELD,
): StudioWhiteboardNode[] {
  return studio.nodes.flatMap((node) => {
    const card = cardOf(node, jobs, frameOf);
    return card === null ? [] : [{ id: node.id, position: node.position, node: card }];
  });
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
  const card = cardOf(node, jobs, NO_FRAME_HELD);
  if (card === null) return nodeId;
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
