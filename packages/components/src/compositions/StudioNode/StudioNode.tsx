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

/** Each kind, with the states `studio.md` gives it. A kind with none takes no `state`. */
export type StudioNodeOf =
  | { kind: "run"; state: StudioRunState }
  | { kind: "note" }
  | { kind: "cluster" }
  | { kind: "finding"; state: StudioFindingState }
  | { kind: "contradiction"; state: StudioContradictionState }
  | { kind: "sketch"; state: "frozen" }
  | { kind: "link" }
  | { kind: "deferral"; state: StudioDeferralState }
  | { kind: "outline"; state: StudioOutlineState }
  | { kind: "issue_draft"; state: "draft" }
  /** `state` is the `job_status` wire value. */
  | { kind: "job"; state: string };

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

export function studioNodeReading(node: StudioNodeOf): StudioNodeReading {
  if (node.kind === "job") {
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

export function StudioNode(props: StudioNodeProps) {
  const { title, facts = [], selected = false } = props;
  const reading = studioNodeReading(props);
  const { status } = reading;
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
      <p className="armada-studio-node__title">{title}</p>
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
