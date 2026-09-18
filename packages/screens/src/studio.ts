// A Studio as the whiteboard draws it, off the wire — #1287. No React: every rule here is a function
// of the Studio Fleet sent and the Board this window holds, so it is tested as one.

import type { StudioNodeFrame, StudioWhiteboardEdge, StudioWhiteboardNode } from "@armada/components";
import type {
  EpicRead,
  JobSummary,
  ScoutSource,
  ServerState,
  Studio,
  StudioNode,
  StudioRunKept,
  StudioSummary,
} from "@armada/protocol";
import { STUDIO_EDGE_LABEL, STUDIO_NODE_KIND } from "@armada/components";

import { runOutcomeOf } from "./rehearsal";
import { span } from "./duration";

/**
 * What the window knows that the Studio's own record does not — #1345.
 *
 * **A Run node holding a server reads the live holder**, so the instance has to
 * reach the card from somewhere: Fleet holds one instance per checkout in
 * memory, `list_servers` publishes them, and this window already keeps that
 * list. Nothing about a server is copied onto the node while it is up.
 */
export type StudioLive = {
  /** Every server Fleet holds, and the last of each that ended. */
  servers?: readonly ServerState[];
  /** The clock this window ticks on, for how long a server has been up. */
  now?: number;
};

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

/**
 * What a kept **server** says beside its name: the `serve` line, how it ended,
 * and how long it was up.
 *
 * **Never "expects"**, which every other run's result carries. A server that
 * exits on its own has failed whatever its code (`docs/concepts/manifest.md`),
 * so there was no code it was expected to reach and printing one would say
 * there was.
 */
function serverKeptFacts(kept: StudioRunKept): string[] {
  const up = `up ${(kept.duration_ms / 1000).toFixed(1)}s`;
  return kept.exit_code === undefined ? [kept.command, up] : [kept.command, `exit ${kept.exit_code}`, up];
}

/**
 * How a kept server reads: **stopped, or failed, and never passed**. Staying up
 * is the whole of what a server is for, so the only good ending is a person
 * ending it.
 */
export function serverOutcomeOf(kept: Pick<StudioRunKept, "stopped">): "stopped" | "failed" {
  return kept.stopped ? "stopped" : "failed";
}

/** Where a server is, as one chip: the first port it declared, else its line. */
function serverAddress(instance: ServerState): string {
  const first = instance.ports[0];
  return first === undefined ? instance.serve : `localhost:${first.port}`;
}

/**
 * A Run node holding a server — #1345. **The live instance, then what the node
 * kept, then neither.**
 *
 * The order is the rule: while Fleet holds it the card is drawn off the holder,
 * and a node that kept a result is one whose server is gone. A node that has
 * neither says so rather than drawing its id as a title, which is what a Run
 * node holding a run nobody read already does.
 */
function serverCard(
  node: Extract<StudioNode, { kind: "run" }>,
  live: StudioLive,
): StudioWhiteboardNode["node"] {
  const instance = (live.servers ?? []).find((one) => one.id === node.run_id);
  if (instance !== undefined && instance.phase !== "exited") {
    return instance.phase === "starting"
      ? { kind: "run", state: "starting", title: instance.name, facts: [instance.serve] }
      : {
          kind: "run",
          state: "serving",
          title: instance.name,
          facts: [upFor(instance, live.now), serverAddress(instance)].filter((fact) => fact !== ""),
        };
  }
  if (node.kept !== undefined) {
    return {
      kind: "run",
      state: serverOutcomeOf(node.kept),
      title: node.kept.name,
      facts: serverKeptFacts(node.kept),
    };
  }
  return { kind: "run", title: "Not read yet", facts: [node.run_id] };
}

/** How long it has been up. **Nothing ticks on the wire**, so the window's own
 * clock is what counts it — `serverStatusOf`'s rule on the run sheet. */
function upFor(instance: ServerState, now: number | undefined): string {
  if (instance.serving_since === undefined || now === undefined) return "";
  const lasting = span(instance.serving_since, now);
  return lasting === null ? "" : `up ${lasting}`;
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
 * What an Epic says about itself, as the facts a card draws — #1394, #1405.
 *
 * **An Epic that fits says how many it holds**, rather than saying the same
 * number twice: the two-number form is there to say a read did not take
 * everything, and drawing it always would make a whole milestone read as a
 * partial one.
 *
 * **Which issues it took is drawn beside the count, and only where it was
 * recorded.** An Epic read in before 14.20 kept no answer, so it says nothing
 * about one rather than claiming it took everything.
 */
function epicRead(read: EpicRead): string[] {
  const count = read.issues === read.total ? `${read.total} issues` : `${read.issues} of ${read.total} issues`;
  if (read.took === undefined) return [count];
  // **One fact each, because a chip is one fact.** A card is 240 wide and a
  // chip that does not fit is clipped, so a sentence written across three of
  // them reads and a sentence written into one does not.
  return [
    count,
    EPIC_TOOK[read.took] ?? read.took,
    ...(read.left_out ? [`${read.left_out} left out`] : []),
    ...(read.kept ? [`${read.kept} kept, worked on`] : []),
  ];
}

/**
 * What each answer is called where a card draws it. **A word this build does
 * not know is drawn as itself**, `forgeState`'s rule: a newer Fleet's word said
 * plainly beats no word at all.
 */
const EPIC_TOOK: Readonly<Record<string, string>> = {
  everything: "Every issue",
  open: "Open issues only",
};

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
 * What a replacement says about where the work came from — #1440. The `produced` edge draws the
 * link and this names the other end, so a Job that replaced another reads as one rather than as a
 * second dispatch somebody asked for.
 *
 * **Read off the row, like the status beside it.** A Job node holds a reference and no copy of
 * anything, and `redispatched_from` is the one record of a redispatch.
 *
 * Nothing is said where the window does not hold the predecessor: a handle nobody can resolve is
 * an id drawn as a fact, which says less than silence.
 */
function carriedOnFrom(job: JobSummary, jobs: readonly JobSummary[]): string[] {
  const before = jobs.find((one) => one.id === job.redispatched_from);
  return before === undefined ? [] : [`carried on from ${before.handle}`];
}

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
  live: StudioLive,
): StudioWhiteboardNode["node"] | null {
  switch (node.kind) {
    case "run":
      // A server is a Run node whose id names an instance Fleet holds rather
      // than a directory under `.armada/runs` — #1345, and two readers.
      if (node.held === "server") return serverCard(node, live);
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
        : { kind: "job", state: job.status, title: job.title, facts: [job.handle, ...carriedOnFrom(job, jobs)] };
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
        facts: [`#${node.number}`, ...(node.read_in === undefined ? [] : epicRead(node.read_in))],
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
  live: StudioLive = {},
): StudioWhiteboardNode[] {
  return studio.nodes.flatMap((node) => {
    const card = cardOf(node, jobs, frameOf, live);
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
export function nodeNamed(
  studio: Studio,
  nodeId: string,
  jobs: readonly JobSummary[],
  live: StudioLive = {},
): string {
  const node = studio.nodes.find((one) => one.id === nodeId);
  if (node === undefined) return nodeId;
  const card = cardOf(node, jobs, NO_FRAME_HELD, live);
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
