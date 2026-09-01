// Job detail, whole, at the states that draw it differently — rendered from the
// screen the app actually mounts.
//
// **This renders `JobDetail`, not a header or a control.** An earlier pass here
// captured `Acts` on its own and produced five pictures of one button: true,
// and useless for the question the tool exists to answer, which is whether the
// screen is right. Everything on these shots — the badge, the field run, the
// run tree, the panel, the acts — is derived by the app's own code from one
// fixture, the way it is derived from one wire read at runtime.
//
// # What a fixture is
//
// The values `GET /jobs/:job_id` would carry, written down. Nothing is mocked
// and nothing is stubbed: `JobDetail` takes data and callbacks, so a fixture is
// data and the callbacks are inert. No Fleet, no socket, no daemon — a screen
// that needed one is a screen nobody captures.
//
// **The reads are `{ state: "none" }` deliberately.** The diff, the footprint
// and the evidence arrive on their own sockets after the screen is drawn, so a
// fixture that pre-filled them would be drawing a state the first paint never
// has. What the panel says when a read has not landed is itself worth a shot.
//
// # Marks
//
// `inside-a-job-<state>`, which is what the gallery's screen stories already
// derive for the same states. Same mark, different side: `.shots/app/` is the
// component library's arrangement and `.shots/bridge/` is this one, and the day
// a drawing carries these marks either can be paired against it.
//
// # What is not here
//
// Hover, focus, and open menus — `shoot` captures resting states, so the split
// button in the header is captured closed and what its menu holds is not on the
// shot. Reading that means reading `Acts.tsx`.

import type { ReactElement } from "react";

import type { Observed, Watched } from "../../shared/bridge";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "../../shared/protocol";
import { JobDetail } from "./JobDetail";

/** One capture: the mark it pairs by, what to call it, and what to draw. */
export type Screen = { mark: string; name: string; element: ReactElement };

/**
 * Fixed, because a shot has to be the same shot twice. Every elapsed on the
 * screen is `now` minus a recorded instant, so a clock would make every capture
 * differ from the last one by the time between them.
 */
const NOW = Date.parse("2026-09-01T11:20:00Z");
const STARTED = "2026-09-01T09:16:40Z";

/**
 * An escalated Job carries the reason it escalated, and the screen depends on
 * it: `escalated` has `verb: null, icon: null` in the registry, and both come
 * from the reason. A fixture without one refuses to draw at all — which this
 * file learned by capturing an `Unrenderable` and looking at it. That is the
 * tool working, and it is why the fixture is written against the app rather
 * than against a picture of it.
 */
const DISPUTED = { named: "evidence_suspect" };

const JOB: JobSummary = {
  id: "01M1D80SFQ0042MT27VBCXXWHX",
  title: "Clear finished jobs should remove the worktree it left behind",
  status: "escalated",
  reason: DISPUTED,
  workflow_id: "bug",
  owner_manifest_id: "armada",
  origin: "person",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  current_step_id: "regression_verify",
  created_at: STARTED,
};

/**
 * A run of three, stopped on the third. The shape a bug workflow has.
 *
 * **Every step carries the wire's required fields and nothing optional.** The
 * compiler enforces that, which is the point of building the fixture out of the
 * app's own types: a field the protocol adds turns into an error here rather
 * than into a screen quietly drawing without it.
 */
const step = (over: Pick<StepDetail, "step_id" | "label" | "ordinal" | "state">): StepDetail => ({
  ...over,
  check_runs: [],
  overridden: false,
  judged: [],
  flagged: [],
  attempts: [],
  entered_at: STARTED,
  updated_at: STARTED,
});

const STEPS: StepDetail[] = [
  step({ step_id: "reproduce", label: "Reproduce", ordinal: 1, state: "completed" }),
  step({ step_id: "fix", label: "Fix", ordinal: 2, state: "completed" }),
  step({ step_id: "regression_verify", label: "Regression verify", ordinal: 3, state: "stopped" }),
];

/**
 * What Fleet says it will do to this Job now.
 *
 * `worktree_on_disk` is the field that makes a restart offerable, and it is
 * `true` here: these Jobs stopped, and nothing has reclaimed what their Drone
 * was working in.
 */
const stuck = (recourse: string[]) => ({ recourse, worktree_on_disk: true });

function whole(over: Partial<JobWhole> = {}): JobWhole {
  return {
    job: JOB,
    created_at: STARTED,
    branch: "armada/clear-finished-jobs",
    steps: STEPS,
    acceptance_criteria: [
      {
        criterion_id: "worktree_removed",
        text: "The worktree the job left behind is gone after clear.",
        source: "person",
      },
    ],
    dependencies: [],
    ...over,
  };
}

/**
 * The same Job at a status that is not an escalation, so the escalation reason
 * goes with it. A killed Job carrying `evidence_suspect` is a Job whose badge
 * would read the reason it stopped being about — which is a shot of a state the
 * wire never sends.
 */
const ended = (status: string): JobSummary => {
  const { reason: _escalation, ...rest } = JOB;
  return { ...rest, status };
};

/** The read that has landed, for the Job the screen is about. */
const read = (detail: JobWhole): Watched => ({ state: "read", jobId: JOB.id, detail });

/** Nothing is being observed: these screens are a first paint, not a session. */
const UNOBSERVED: Observed = { state: "none" };

/**
 * Every callback the screen takes, inert, and every read unlanded.
 *
 * Spread rather than restated per screen — none of them differs between states,
 * and a reader comparing two screens should see only what actually differs.
 */
const INERT = {
  workflows: [],
  manifests: [],
  stale: false,
  now: NOW,
  acting: false,
  approving: false,
  deciding: false,
  observed: UNOBSERVED,
  recorded: {
    footprint: { state: "none" },
    evidence: { state: "none" },
    diff: { state: "none" },
  },
  onAct: () => {},
  onRedirect: () => {},
  onAnswer: () => {},
  onOverrule: () => {},
  onRerun: () => {},
  onReport: async () => ({ ok: true }) as never,
  onApprove: () => {},
  onApproveReview: () => {},
  onRequestChanges: () => {},
  onReject: () => {},
  onCopied: () => {},
} as const;

function screen(job: JobSummary, detail: JobWhole): ReactElement {
  return <JobDetail job={job} watched={read(detail)} {...INERT} />;
}

export const title = "Inside a job";

export const screens: Screen[] = [
  {
    mark: "inside-a-job-evidence-disputed",
    name: "Evidence disputed",
    // Waiting on a person, so the header's control takes the accent, and Fleet
    // offers a replacement — the lead the drawing names for this state.
    element: screen(
      JOB,
      whole({ stuck: stuck(["redispatch_job", "override_verdict"]) }),
    ),
  },
  {
    mark: "inside-a-job-out-of-attempts",
    name: "Out of attempts, drone holding",
    // A live Drone, so killing it is a second act and both kills sit behind the
    // caret. The panel offers the redirect; the header offers neither.
    element: screen(
      { ...JOB, assigned_drone: "drone_2d90bb" },
      whole({ stuck: stuck(["redispatch_job", "redirect_drone"]) }),
    ),
  },
  {
    mark: "inside-a-job-killed",
    name: "Killed",
    // Terminal: nobody is waiting, so the same control is quiet. The fill is
    // the state and the lead has not moved.
    element: screen(ended("killed"), whole({ job: ended("killed"), stuck: stuck(["redispatch_job"]) })),
  },
  {
    mark: "inside-a-job-failed",
    name: "Failed, no replacement offered",
    // Nothing Fleet will do, so the report is the only act left and a split
    // button with an empty menu is a button.
    element: screen(ended("completed_failed"), whole({ job: ended("completed_failed"), stuck: stuck([]) })),
  },
  {
    mark: "inside-a-job-running",
    name: "Running",
    // The render the split button has no legal lead on: the drawing leads this
    // one with Pilot, which is #250. Captured so the shot before it is on
    // record.
    element: screen(
      { ...ended("running"), assigned_drone: "drone_2d90bb" },
      whole({ job: ended("running") }),
    ),
  },
  {
    mark: "inside-a-job-waiting-on-you",
    name: "Awaiting review",
    // The decision block under the story, and a header that carries no split
    // button — `Review` is not a header act on this screen, which is the other
    // half of why the drawing's lead table is not all built.
    element: screen(
      ended("awaiting_review"),
      whole({ job: ended("awaiting_review") }),
    ),
  },
];
