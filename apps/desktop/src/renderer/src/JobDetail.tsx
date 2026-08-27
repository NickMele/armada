// One Job, read whole. The three renders the design draws, chosen by the Job's
// status and fed from `GET /jobs/:job_id` rather than from the row beside it.
//
// # What is served is drawn, and what is not is named
//
// The rail is built in `rail.ts`, which says what of it the wire carries. What
// the Judge answered is on it too, beneath the step it judged: a refusal is not
// a failed Check and does not render as one, and `CriterionVerdicts` carries
// that difference — see its own header for the three ways.
//
// Where the work is is the one region built from both: the branch is served,
// and the worktree, log and transcript paths are derived in `work.ts` from the
// job id and the repository. A step's Checks and what each one did are served
// too, since protocol 3. Evidence and spend are not, and every region that
// wants one says so where it would have gone.
//
// # Absent is not empty, and the two get different sentences
//
// Every optional field on the detail is omitted rather than sent null, which
// makes the distinction readable: `write_targets` absent is scope undetermined
// and present-and-empty is determined to write nothing. Collapsing them would
// tell somebody a Job has no scope when what is true is that nobody set one.

import {
  AFailedJobADeadEndReadAsOne,
  AFinishedJobABranchAndAnEvidenceTrail,
  ARunningJob,
  Button,
  SplitButton,
  type JobDetailHeading,
  type SplitButtonItem,
} from "@armada/components";

import type { Watched } from "../../shared/bridge";
import {
  ESCALATION_REASON,
  JOB_LIFECYCLE,
  JOB_STATUS,
} from "../../shared/generated/vocabulary";
import type {
  JobDetail as JobWhole,
  JobSummary,
  ManifestSummary,
  WorkflowSummary,
} from "../../shared/protocol";
import { factsOf } from "./facts";
import { railOf, stoppedAt } from "./rail";
import { readingOf } from "./reading";
import { whyNoWork, workOf } from "./work";

/**
 * Which render a Job takes. Three, because the design draws three.
 *
 * **The choice reads the registries, not a list of statuses typed here.**
 * `job-statuses.toml` says whether a Job is over and what it is doing, and
 * `enum-verbs.toml` says which token a status carries — both arrive through
 * the generated module. A Job that stopped and asked takes the dead-end render
 * whatever its status says, because that screen is the one built to state why
 * something stopped and where the work was left.
 */
export type Render = "working" | "finished" | "stopped" | "unrenderable";

/**
 * The token the one successful terminal status carries. Named once: rename it
 * in `enum-verbs.toml` and a finished Job falls to the dead-end render, which
 * is visible rather than silent.
 */
const SUCCEEDED = "--status-completed-success";

export function renderFor(job: JobSummary): Render {
  const base = JOB_STATUS[job.status];
  const life = JOB_LIFECYCLE[job.status];
  if (base === undefined || life === undefined) return "unrenderable";
  if (escalation(job) !== undefined) return "stopped";
  if (!life.terminal) return "working";
  return base.statusToken === SUCCEEDED ? "finished" : "stopped";
}

/** The escalation reason a Job carries, where the registry has that spelling. */
function escalation(job: JobSummary) {
  const named = job.reason?.named;
  if (named === undefined || job.status !== "escalated") return undefined;
  return ESCALATION_REASON[named];
}

/** What the two kills and the redispatch are called, and what each one does. */
export type JobAct = "kill_drone" | "kill_job" | "redispatch";

/**
 * The statuses a redispatch is offered on. Three, and **`rejected` is not one**:
 * a rejected Job never ran, so it has no Facts and no Evidence to carry
 * forward, and redispatching it would only be proposing a new Job — which the
 * composer already does.
 *
 * Written here rather than read from the generated vocabulary because no
 * registry file carries the set: `job-fields.toml` still asks it as an open
 * question on `redispatched_from`. Fleet's route is the authority and refuses
 * anything else; this only keeps a button off the screen that would be.
 */
const REDISPATCHABLE: ReadonlySet<string> = new Set([
  "escalated",
  "completed_failed",
  "killed",
]);

export type JobDetailProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id` for this Job, as main published it. */
  watched: Watched;
  workflows: readonly WorkflowSummary[];
  manifests: readonly ManifestSummary[];
  /** True while what is shown is not live. Every control is refused. */
  stale: boolean;
  /** Now, injected. A whole-Job elapsed is read, so it has to move. */
  now: number;
  /** In flight. A second press does not send a second command. */
  acting: boolean;
  /** An approval already sent for this Job. */
  approving: boolean;
  /** Ask for a confirmation. Nothing destructive is one press from here. */
  onAct: (act: JobAct, jobId: string) => void;
  /**
   * Let this Job run. **Sent on the press, with no confirmation** — approving
   * is the ordinary path, it is reversible by killing, and a gate that costs
   * two clicks for the common case is a gate in the wrong place.
   */
  onApprove: (jobId: string) => void;
  /**
   * Open this Job's turns. **Not an act on the Drone** — it opens a read-only
   * view and takes nothing over, which is why it takes no confirmation and does
   * not go through `onAct` with the three that end something.
   */
  onObserve: () => void;
  onCopied: (value: string) => void;
};

export function JobDetail({
  job,
  watched,
  workflows,
  manifests,
  stale,
  now,
  acting,
  approving,
  onAct,
  onApprove,
  onObserve,
  onCopied,
}: JobDetailProps) {
  const reading = readingOf(job);
  const render = renderFor(job);
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);
  // The detail is only this Job's while it names this Job. A stale one from the
  // Job that was open a moment ago would draw another Job's steps under this
  // Job's title.
  const whole = watched.state === "read" && watched.jobId === job.id ? watched.detail : null;

  // The badge is the header, so a Job the registry has no glyph or verb for
  // cannot be drawn at all. Named rather than half-drawn — the same answer the
  // list gives for the same Job.
  if (reading.as !== "badge" || render === "unrenderable") {
    return <Unrenderable job={job} />;
  }

  const heading: JobDetailHeading = {
    status: reading.status,
    statusIcon: reading.icon,
    statusLabel: reading.verb,
    headline: job.title,
    jobId: job.id,
    fields: factsOf(job, whole, workflow, manifest, now),
    actions: (
      <Acts
        job={job}
        render={render}
        acting={acting}
        approving={approving}
        stale={stale}
        onAct={onAct}
        onApprove={onApprove}
        onObserve={onObserve}
      />
    ),
  };

  const rail = whole === null ? [] : railOf(whole, now);
  const stepsAbsent = whyNoSteps(watched, job.id);
  // The brief and the paths, on every render. The finished one takes the
  // branch out: its handover names it, and one value drawn twice is two
  // places to keep in step.
  const workAbsent = whyNoWork(watched, job.id);

  if (render === "finished") {
    return (
      <AFinishedJobABranchAndAnEvidenceTrail
        heading={heading}
        // Read off the row, which carries the branch too and cannot disagree
        // with the detail: this region says the branch is missing only when the
        // Job genuinely has no worktree, rather than while the detail is still
        // being read. No diff stat and no log beside it — neither is on the
        // wire, and a branch alone is the handover the screen exists to make.
        handover={
          job.branch === undefined ? undefined : { branch: job.branch, note: HANDOVER_NOTE }
        }
        handoverAbsent={NOT_SERVED.branch}
        work={workOf(job, whole, manifest, false)}
        workAbsent={workAbsent}
        trailAbsent={NOT_SERVED.evidence}
        onCopied={onCopied}
      />
    );
  }

  if (render === "stopped") {
    return (
      <AFailedJobADeadEndReadAsOne
        heading={heading}
        why={whyOf(job, whole)}
        steps={rail}
        stepsAbsent={stepsAbsent}
        work={workOf(job, whole, manifest, true)}
        outputAbsent={NOT_SERVED.output}
        workAbsent={workAbsent}
        onCopied={onCopied}
      />
    );
  }

  return (
    <ARunningJob
      heading={heading}
      steps={rail}
      stepsAbsent={stepsAbsent}
      evidenceAbsent={NOT_SERVED.evidence}
      log={workOf(job, whole, manifest, true)}
      logAbsent={workAbsent}
      onCopied={onCopied}
    />
  );
}

/**
 * What the wire does not carry, said in the place the design puts it. One
 * sentence each, naming the operation that would have to serve it — a hole
 * that names its cause is a finding, one that reads "coming soon" is not.
 */
const NOT_SERVED = {
  evidence: "No operation serves a work submission, so there is nothing to draw here.",
  branch: "This Job has no worktree yet, so it has no branch.",
  // The path is served per Check run and is drawn on the gate row that owns
  // it — a step with three Checks wrote three files, and one region can only
  // hold one. The contents are not served, and Bridge does not read the
  // filesystem, so naming the file is the whole of what it can do.
  output: "Each check names its output file on its own row. Nothing serves the contents.",
} as const;

/**
 * What is still owed after a Job finishes. **Armada does not push and does not
 * merge**, so the screen that hands over a branch says what is left to do
 * rather than implying the work has landed.
 */
const HANDOVER_NOTE = "Armada does not push and does not merge. The branch is yours to take.";

/** Why the rail has no rows, which is never the same sentence twice. */
function whyNoSteps(watched: Watched, jobId: string): string | undefined {
  if (watched.state === "read" && watched.jobId === jobId) {
    return watched.detail.steps.length === 0
      ? "This Job's frozen workflow has no steps."
      : undefined;
  }
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this Job, so its steps are unknown.";
  }
  return "Reading this Job.";
}


/**
 * What can be done to this Job from here.
 *
 * **Four acts, and none of them collapses into another.** Killing the Drone
 * ends a process and leaves the Job open with its worktree held; killing the
 * Job ends the Job at `killed`, terminal; redispatch does the second and mints
 * a replacement; approving lets a Job at the gate run.
 *
 * | Act | Drawn on | Confirms |
 * |---|---|---|
 * | `approve` | `awaiting_approval` | no — see `onApprove` |
 * | `redispatch` | `escalated`, `completed_failed`, `killed` | yes |
 * | `kill_drone` | a Job holding an `assigned_drone` | yes |
 * | `kill_job` | every non-terminal status | yes |
 *
 * **The three that end something are one split button, not a row of red.** Two
 * outlined reds side by side read as one control with two labels, which is the
 * thing they are least like. What is on the face is the act that state calls
 * for; the rest sit in the menu and each one's label says what survives it, so
 * the caret never turns a terminal act into a variant of a milder one.
 */
function Acts({
  job,
  render,
  acting,
  approving,
  stale,
  onAct,
  onApprove,
  onObserve,
}: {
  job: JobSummary;
  render: Render;
  acting: boolean;
  approving: boolean;
  stale: boolean;
  onAct: (act: JobAct, jobId: string) => void;
  onApprove: (jobId: string) => void;
  onObserve: () => void;
}) {
  const life = JOB_LIFECYCLE[job.status];
  const over = life?.terminal ?? true;
  // Menu order, mildest first — the split button puts destructive last.
  const acts: JobAct[] = [
    // Only where Fleet accepts one; anything else is its 409, and this does not
    // offer a button that is refused on press.
    ...(render === "stopped" && REDISPATCHABLE.has(job.status)
      ? (["redispatch"] as JobAct[])
      : []),
    // `assigned_drone` is presence rather than state: there is nothing to kill
    // without one.
    ...(job.assigned_drone === undefined ? [] : (["kill_drone"] as JobAct[])),
    ...(over ? [] : (["kill_job"] as JobAct[])),
  ];
  // What the state calls for goes on the face: replacing a Job that stopped, and
  // otherwise the kill that ends it. Never the milder kill — the act with the
  // larger consequence does not hide behind a caret.
  const face = FACE.find((act) => acts.includes(act)) ?? acts[0];
  const menu: SplitButtonItem[] = acts
    .filter((act) => act !== face)
    .map((act) => ({
      label: MENU_LABEL[act],
      danger: act === "kill_job",
      onSelect: () => onAct(act, job.id),
    }));

  return (
    <>
      {/* Ghost, and first: watching is not one of the acts. It ends nothing,
          confirms nothing and is offered on every Job, because the transcript
          is the Job's history across every Drone it has had — a Job that never
          had one says so in the pane rather than by having no control. */}
      <Button variant="ghost" disabled={stale} onClick={onObserve}>
        Watch the turns
      </Button>
      {face === undefined ? null : menu.length === 0 ? (
        // A split button with nothing in its menu is a button. Outlined, because
        // a solid red control reads as an error state rather than as an act.
        <Button
          variant="destructive"
          disabled={acting || stale}
          onClick={() => onAct(face, job.id)}
        >
          {ACT_LABEL[face]}
        </Button>
      ) : (
        <SplitButton
          variant="destructive"
          disabled={acting || stale}
          menuLabel="What else ends this job"
          items={menu}
          onAction={() => onAct(face, job.id)}
        >
          {ACT_LABEL[face]}
        </SplitButton>
      )}
      {/* The one primary this header ever carries, and the only forward act in
          the set. Last, where the shell head puts its own primary — the accent
          fill and the distance are what keep it from reading as a peer of the
          red group. Approving a Job you opened in order to read it is the whole
          point of the gate; going back to the list to say yes is not. */}
      {job.status === "awaiting_approval" ? (
        <Button
          variant="primary"
          disabled={approving || stale}
          onClick={() => onApprove(job.id)}
        >
          {approving ? "Approving" : "Approve dispatch"}
        </Button>
      ) : null}
    </>
  );
}

/**
 * What each act is called on its button. **Redispatch does not say "retry" or
 * "run again"** — nothing resumes, and a label implying the same Job continues
 * would describe an act Fleet does not perform. The confirmation states the
 * rest; the button names the act.
 */
export const ACT_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone",
  kill_job: "Kill job",
  redispatch: "Redispatch as a new job",
};

/**
 * The same acts inside the menu, where each says what survives it. A caret hides
 * the consequence that a button's own position states, so the label has to carry
 * it — `Kill drone` and `Kill job` differ by everything and by three characters.
 */
const MENU_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone, the job stays open",
  kill_job: "Kill job, it ends here",
  redispatch: "Redispatch as a new job",
};

/** Which act takes the split button's face, in preference order. */
const FACE: readonly JobAct[] = ["redispatch", "kill_job"];

/**
 * Why a Job stopped: the reason's own verb, the criteria it still owes, and
 * the step it stopped at with what the gate found there. The label above it
 * supplies the grammar, so no sentence is composed around a word the registry
 * chose.
 *
 * **Where it stopped is stated even where no reason was stored.** Four of the
 * five statuses this screen draws store none — a failed Job, a killed one, a
 * rejected one and a superseded one all arrive with `reason` absent — and
 * without the step they say only that something ended. `stoppedAt` reads the
 * step and its Check runs, which are served; nothing here is inferred and
 * nothing is composed beyond the separators the rail already uses.
 */
function whyOf(job: JobSummary, whole: JobWhole | null) {
  const reason = escalation(job);
  const owed = job.reason?.criteria_owed ?? [];
  const at = whole === null ? undefined : stoppedAt(whole);
  if (reason?.verb == null && at === undefined) return undefined;
  return (
    <>
      {reason?.verb == null ? null : (
        <>
          {reason.verb}
          {owed.length === 0 ? null : (
            <>
              {" · owes "}
              <span className="mono">{owed.join(", ")}</span>
            </>
          )}
          {at === undefined ? null : " · "}
        </>
      )}
      {at === undefined ? null : (
        <>
          {"stopped at "}
          {at.labelIsAnIdentifier ? <span className="mono">{at.label}</span> : at.label}
          {at.check === undefined ? null : (
            <>
              {" · "}
              <span className="mono">{at.check}</span>
            </>
          )}
          {at.outputPath === undefined ? null : (
            <>
              {" · "}
              <span className="mono">{at.outputPath}</span>
            </>
          )}
        </>
      )}
    </>
  );
}

/**
 * A Job the registry has no sanctioned glyph, verb or hue for. The badge is
 * the header, so there is no partial render to fall back to — and no glyph is
 * invented for it here any more than in the list.
 */
function Unrenderable({ job }: { job: JobSummary }) {
  const reading = readingOf(job);
  const missing = reading.as === "badge" ? ["variant"] : reading.missing;
  return (
    <p className="text-fg-muted">
      {`${job.title} — `}
      <span className="mono">{job.status}</span>
      {`. The registry carries no ${missing.join(" and no ")} for it, so this Job has no detail to draw.`}
    </p>
  );
}
