import { Badge } from "../../primitives/Badge/Badge";
import { Card } from "../../primitives/Card/Card";
import { JOB_STATUS, type Rendering } from "../../generated/vocabulary";
import { FactChip } from "../FactChip/FactChip";

/**
 * Studio node — one node on a Studio's whiteboard, of any kind on
 * `docs/concepts/studio.md`, Nodes.
 *
 * **Kind, title, state, facts, and nothing else.** A node is a card: the kind
 * and the state on one line, the title under it, and the facts as chips. What a
 * node holds at length is read beside the whiteboard, not on it.
 *
 * **Only Run and Job take status colour.** A Job's is its `Badge`; a Run's is
 * its word in `--run-{state}`, because a bordered pill is a Job state and
 * nothing else. Every other state is plain text in `--fg-muted`.
 *
 * **Every working node pulses** and says so with `aria-busy`, which is what a
 * reader who cannot see the pulse is told.
 */

/**
 * How a run started from the Studio stands. Each takes its hue from
 * `--run-{state}` in `tokens/status.css`, which aliases the Job status it reads
 * as (#1277).
 */
export type StudioRunState = "running" | "passed" | "failed" | "stopped";

export type StudioFindingState = "proposed" | "gathering" | "frozen";
export type StudioContradictionState =
  | "reported"
  | "issue_draft"
  | "deferral"
  | "not_a_problem"
  | "resolved_here";
export type StudioDeferralState = "open" | "answered";
export type StudioOutlineState = "draft" | "frozen";

/**
 * The picture a Note kept, as the caller resolved it — #1352.
 *
 * **The bytes are the caller's problem and the drawing is this one's**, which
 * is `ShownFrame`'s rule: a frame reaches the renderer over the preload and
 * becomes a `blob:`, which needs a live Fleet, so what arrives here is a URL
 * or a reason there is not one. A frame with neither is one still being read.
 *
 * **Absent is a Note that kept no picture**, and it draws no plate at all: a
 * Note typed rather than pointed, or one whose capture could take no frame, is
 * an ordinary Note, and a board of dashed empty boxes would say otherwise.
 */
export type StudioNodeFrame = {
  /** What to draw, once the caller has it. */
  src?: string;
  /** Why there is nothing to draw. Absent beside an absent `src` is a read in flight. */
  why?: string;
};

/** Each kind, with the states `studio.md` gives it. A kind with none takes no `state`. */
export type StudioNodeOf =
  /** `state` absent: the run has not been read, and nothing is said about it. */
  | { kind: "run"; state?: StudioRunState }
  | { kind: "note"; frame?: StudioNodeFrame }
  | { kind: "cluster" }
  | { kind: "finding"; state: StudioFindingState }
  | { kind: "contradiction"; state: StudioContradictionState }
  | { kind: "sketch"; state: "frozen" }
  /**
   * A Link keeps its address, whatever is typed beside it — `#1378`. The
   * card's title is the person's own line, and this is drawn under it; where
   * they typed none the title *is* the address, and it is not said twice.
   */
  | { kind: "link"; address: string }
  | { kind: "deferral"; state: StudioDeferralState }
  | { kind: "outline"; state: StudioOutlineState }
  | { kind: "issue_draft"; state: "draft" }
  /** `state` is the `job_status` wire value, absent where the Job has not been read. */
  | { kind: "job"; state?: string };

export type StudioNodeKind = StudioNodeOf["kind"];

export type StudioNodeProps = StudioNodeOf & {
  title: string;
  /** One-line values: a command, an exit code, a cost, an address. Mono, neutral. */
  facts?: readonly string[];
  /** Selected on the whiteboard. */
  selected?: boolean;
};

/**
 * The kind's name as `studio.md` writes it. A table until code reads it, and
 * this is that code: the move to a data file beside `crates/core-model/domain/`
 * is the rule on that page, and is not made here.
 */
export const STUDIO_NODE_KIND: Readonly<Record<StudioNodeKind, string>> = {
  run: "Run",
  note: "Note",
  cluster: "Cluster",
  finding: "Finding",
  contradiction: "Contradiction",
  sketch: "Sketch",
  link: "Link",
  deferral: "Deferral",
  outline: "Outline",
  issue_draft: "Issue draft",
  job: "Job",
};

/** The neutral states' words. A Contradiction's outcomes use the concept page's names. */
const NEUTRAL_STATE: Readonly<Record<string, string>> = {
  proposed: "proposed",
  gathering: "gathering",
  frozen: "frozen",
  reported: "reported",
  issue_draft: "issue draft",
  deferral: "deferral",
  not_a_problem: "not a problem",
  resolved_here: "resolved here",
  open: "open",
  answered: "answered",
  draft: "draft",
  running: "running",
  passed: "passed",
  failed: "failed",
  stopped: "stopped",
};

/** What the node's state reads as, and whether it is still working. */
export type StudioNodeReading = {
  words: string | null;
  /** A Job status rendering, for Job only. */
  status: Rendering | null;
  /** The run's state, for its `--run-*` hue. Run only. */
  run: StudioRunState | null;
  /** The wire value, where the registry carries no verb for it. */
  missing: string | null;
  working: boolean;
};

/**
 * A Run or Job whose state has not been read. A Studio carries a reference to
 * the run or the Job and never its state, so until one is read there is nothing
 * to say — and saying nothing beats guessing.
 */
const UNREAD: StudioNodeReading = { words: null, status: null, run: null, missing: null, working: false };

export function studioNodeReading(node: StudioNodeOf): StudioNodeReading {
  if (node.kind === "job") {
    if (node.state === undefined) return UNREAD;
    const rendering = JOB_STATUS[node.state] ?? null;
    const usable = rendering?.verb != null && rendering.badgeStatus != null && rendering.icon != null;
    return {
      words: usable ? rendering.verb : node.state,
      status: usable ? rendering : null,
      run: null,
      missing: usable ? null : node.state,
      working: node.state === "running",
    };
  }
  if (node.kind === "run") {
    if (node.state === undefined) return UNREAD;
    return { words: node.state, status: null, run: node.state, missing: null, working: node.state === "running" };
  }
  if (!("state" in node)) return { words: null, status: null, run: null, missing: null, working: false };
  return {
    words: NEUTRAL_STATE[node.state] ?? node.state,
    status: null,
    run: null,
    missing: null,
    working: node.kind === "finding" && node.state === "gathering",
  };
}

/** The node's accessible name: kind, title, then state. */
export function studioNodeLabel(node: StudioNodeOf & { title: string }): string {
  const { words } = studioNodeReading(node);
  const head = `${STUDIO_NODE_KIND[node.kind]}: ${node.title}`;
  return words === null ? head : `${head}, ${words}`;
}

/** What a frame is called where it is read aloud. The Note's own words are already above it. */
export const STUDIO_FRAME_LABEL = "The screen this Note was captured from";

/**
 * The picture on the card: what was on screen when the Note was made.
 *
 * **A figure and never a control.** The board is a drag surface, so a button
 * inside a node would be a press fighting a drag; opening the frame is an act
 * on the selected node, where every other act on a node already is.
 *
 * **The box is a screen's shape and is drawn before the bytes land**, so a
 * board does not jump as the pictures arrive — `FramesShown`'s rule, and the
 * same reason: a reflow is how a person loses the node they were reading.
 */
function Frame({ frame }: { frame: StudioNodeFrame }) {
  if (frame.src === undefined) {
    return (
      <span className="armada-studio-node__frame" data-empty>
        {/* Nothing to draw and no reason yet is a read in flight, and it says
            so rather than drawing a blank that reads as a photograph of a
            blank screen. */}
        <span className="armada-studio-node__why">{frame.why ?? "reading…"}</span>
      </span>
    );
  }
  return (
    <span className="armada-studio-node__frame">
      <img className="armada-studio-node__image" src={frame.src} alt={STUDIO_FRAME_LABEL} />
    </span>
  );
}

export function StudioNode(props: StudioNodeProps) {
  const { title, facts = [], selected = false } = props;
  const reading = studioNodeReading(props);
  const { status } = reading;
  // **Clipped to the card, with the whole of it in the title** — `FactChip`'s
  // rule for a value longer than its column, and the defect `#1378` was raised
  // on: an address wrapped over three lines is what the node was before.
  const address = props.kind === "link" ? props.address : null;
  const untitled = address !== null && address === title;
  return (
    <Card
      // A card on the canvas is glass. `docs/contracts/design-system.md`, Depth.
      className="armada-studio-node armada-glass"
      data-kind={props.kind}
      data-selected={selected || undefined}
      aria-busy={reading.working || undefined}
    >
      <div className="armada-studio-node__head">
        <span className="armada-studio-node__kind">{STUDIO_NODE_KIND[props.kind]}</span>
        {status?.badgeStatus != null && status.icon != null ? (
          <Badge status={status.badgeStatus} icon={status.icon} pulsing={reading.working}>
            {status.verb}
          </Badge>
        ) : reading.words === null ? null : (
          <span
            className="armada-studio-node__state"
            data-run={reading.run ?? undefined}
            data-working={reading.working || undefined}
            title={reading.missing === null ? undefined : `No verb in the registry for ${reading.missing}`}
          >
            {reading.words}
          </span>
        )}
      </div>
      {/* One box for what the card says, sized by the kind rather than by the
          words — see `StudioNode.css`. */}
      <div className="armada-studio-node__said">
        <p
          className="armada-studio-node__title"
          data-clipped={untitled || undefined}
          title={untitled ? title : undefined}
        >
          {title}
        </p>
        {address === null || untitled ? null : (
          <p className="armada-studio-node__address" title={address}>
            {address}
          </p>
        )}
      </div>
      {props.kind !== "note" || props.frame === undefined ? null : <Frame frame={props.frame} />}
      {facts.length === 0 ? null : (
        <ul className="armada-studio-node__facts">
          {facts.map((fact) => (
            <li key={fact}>
              <FactChip title={fact}>{fact}</FactChip>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
