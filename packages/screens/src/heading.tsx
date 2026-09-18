// The Job header, built — and the Job that has no header, so cannot be drawn.
//
// The badge, the headline, the Job's own facts, and the acts that end or
// replace it. **Four things about the Job rather than about any step**, which
// is the line between this file and `step.tsx`: what is here changes when the
// Job does, and what is there changes when the selection does.
//
// **This is where the next thing lands.** Pilot's slot is left of the kill
// group — `#250` — and the pull request link arrived here in `#422`. Both were
// written into `JobDetail.tsx` because that was where the header was assembled;
// the header has a file now, and the screen has one line.
//
// **A Job the registry has no glyph or verb for has no header at all**, so
// `headingOf` answers `null` rather than a half-built one, and `Unrenderable`
// is what the screen draws instead. The two are here together because they are
// one decision: the badge is the header, so there is no partial render to fall
// back to.

import { JOB_LIFECYCLE, JOB_STATUS, type JobDetailHeading } from "@armada/components";
import type { FileReport, JobDetail as JobWhole, JobSummary, Outcome } from "@armada/protocol";
import { Acts, type ConfirmableAct, type HeldAct } from "./Acts";
import { factsOf } from "./facts";
import type { ActAnswer, ActingAct } from "./pending";
import { settingsButtonOf } from "./settings";
import { openPullRequest, type OpenPullRequest } from "./opening";
import { leading, readingOf } from "./reading";
import type { Render } from "./render";

/** What the header is built from. The Job's, never a step's. */
export type Heading = {
  job: JobSummary;
  /** `GET /jobs/:job_id`, or `null` while it has not arrived. */
  whole: JobWhole | null;
  /** Now, injected. A whole-Job elapsed is read, so it has to move. */
  now: number;
  render: Render;
  stale: boolean;
  acting: boolean;
  /** Which act `acting` is, so the control that sent it is the one that waits. #1117. */
  actingAct?: ActingAct | undefined;
  /** What Fleet said to the last act on this Job. The header's face answers its own. */
  answered?: ActAnswer | undefined;
  approving: boolean;
  /** Whether the report dialog is up. Held by the screen; `b` opens it too. */
  reporting: boolean;
  onReporting: (reporting: boolean) => void;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  /** A kill held for `--duration-hold`. It sends, with no dialog. */
  onActHeld: (act: HeldAct, jobId: string) => void;
  onApprove: (jobId: string) => void;
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  /** Give this job a higher cost ceiling, in millionths of a dollar. */
  onRaiseCap: (jobId: string, costCapMicros: number) => void;
  /** Whether the cost-cap dialog is up. Held by the screen; `B` opens it too. */
  raising: boolean;
  onRaising: (raising: boolean) => void;
  /** Let this job take more turns. A turn count, and no conversion. */
  onRaiseTurnCap: (jobId: string, turnCap: number) => void;
  /** Whether the turn-cap dialog is up. Held by the screen; `T` opens it too. */
  raisingTurns: boolean;
  onRaisingTurns: (raising: boolean) => void;
  /** Open the Job settings panel. The sheet is the screen's, so the screen opens it. */
  onOpenSettings: () => void;
  onOpenPullRequest: OpenPullRequest;
  /** Land on another Job. The `Redispatched from` fact is the header's one. */
  onOpenJob?: ((jobId: string) => void) | undefined;
  onCopied: (value: string) => void;
  /** Say a sentence to the person. Only ever a failure — see `opening.ts`. */
  onSaid: (sentence: string) => void;
};

/**
 * The Job header. **`null` is a Job that cannot be drawn**, not a header that
 * is missing a field: the badge is the header, and the registry carries no
 * sanctioned glyph or verb for this status.
 */
export function headingOf({
  job,
  whole,
  now,
  render,
  stale,
  acting,
  actingAct,
  answered,
  approving,
  reporting,
  onReporting,
  onAct,
  onActHeld,
  onApprove,
  onReport,
  onRaiseCap,
  raising,
  onRaising,
  onRaiseTurnCap,
  raisingTurns,
  onRaisingTurns,
  onOpenSettings,
  onOpenPullRequest,
  onOpenJob,
  onCopied,
  onSaid,
}: Heading): JobDetailHeading | null {
  const reading = readingOf(job);
  if (reading.as !== "badge") return null;
  return {
    status: reading.status,
    statusIcon: reading.icon,
    // The registry's own verb, opening a line. `enum-verbs.toml` spells it
    // lowercase because most of its readings are mid-sentence; here it is the
    // first word in the badge, and the badge is the header.
    statusLabel: leading(reading.verb),
    headline: job.title,
    // **The handle, not the ULID.** This is the mono value a person copies out
    // of the header, and what they copy is what they have to type back — into
    // a branch name, a worktree path, or a sentence to somebody else. The id
    // still names every request the screen makes; it is just not the thing a
    // person is asked to read.
    //
    // **Behind the word `Job`, since #1484.** It was the first value on the
    // line and it was bare, so it read as one more unexplained string in a run
    // of them — the owner's own question about this header was which of them
    // was the id.
    jobId: job.handle,
    jobIdLabel: "Job",
    fields: factsOf(job, whole, now),
    // The acts that end or replace the Job. **Pilot's slot is this one**, left
    // of the kill group — #250, and it lands without this line changing.
    //
    // **The way into a running Job's settings goes first**, left of the acts:
    // it ends nothing, so it sits before the group that does. It opens rather
    // than sends, so stale and in flight leave it on — the panel is where its
    // controls go off.
    actions: (
      <>
        {settingsButtonOf(job, whole, onOpenSettings)}
        <Acts
          job={job}
          whole={whole}
          render={render}
          acting={acting}
          actingAct={actingAct}
          answered={answered}
          approving={approving}
          stale={stale}
          onAct={onAct}
          onActHeld={onActHeld}
          onApprove={onApprove}
          onReport={onReport}
          reporting={reporting}
          onReporting={onReporting}
          onRaiseCap={onRaiseCap}
          raising={raising}
          onRaising={onRaising}
          onRaiseTurnCap={onRaiseTurnCap}
          raisingTurns={raisingTurns}
          onRaisingTurns={onRaisingTurns}
          onCopied={onCopied}
        />
      </>
    ),
    // The pull request fact, followed. The address the link carries is what the
    // fact drew from; what is sent is the Job id, so the string never decides
    // what opens. `opening.ts` says why.
    onFollowed: () => {
      void openPullRequest(onOpenPullRequest, job.id).then((because) => {
        if (because !== null) onSaid(because);
      });
    },
    // The `Redispatched from` fact, pressed. The same act the callout on the
    // job this one replaced already performs, from the other end — `#1474`.
    onOpenJob,
  };
}

/**
 * A Job neither registry `renderFor` reads has a row for. The badge is the
 * header, so there is no partial render to fall back to — and no glyph is
 * invented for it here any more than in the list.
 *
 * **Names which registry is short, rather than saying "variant" always.**
 * That used to be the only word this printed, including once for a Job whose
 * `readingOf` was a complete badge — `escalated` with no reason this build
 * could name, which has its own verb and glyph and was never missing a
 * variant at all. `renderFor` no longer reaches here for that Job; this
 * still says the true thing for the case that remains, a wire spelling
 * `JOB_STATUS` has no row for at all, or one it has a row for while
 * `JOB_LIFECYCLE` — decided separately, per `render.ts`'s own comment —
 * does not.
 */
export function Unrenderable({ job }: { job: JobSummary }) {
  const missing = [
    ...(JOB_STATUS[job.status] === undefined ? ["variant"] : []),
    ...(JOB_STATUS[job.status] !== undefined && JOB_LIFECYCLE[job.status] === undefined
      ? ["lifecycle"]
      : []),
  ];
  return (
    <p className="text-fg-muted">
      {`${job.title} — `}
      <span className="mono">{job.status}</span>
      {`. The registry carries no ${missing.join(" and no ")} for it, so this Job has no detail to draw.`}
    </p>
  );
}
