// The acts that change a step rather than the Job, in the step panel's own
// header.
//
// Split out of `Acts.tsx` when that file crossed the gate's 500-line warning,
// the same way `Acts.tsx` itself came out of `JobDetail.tsx`. **The line is
// where the two subjects already were**: that file is what may be done to the
// Job — ended, replaced, its disk given back — and this is what may be done to
// the step a person is reading, which leaves the Job exactly where it is.
//
// The two readings behind it stay where they are. `recovery.ts` answers for a
// Job that stopped and `steering.ts` for one that has not, and this file
// chooses between neither: it draws whichever one applies.

import { Button, Tooltip } from "@armada/components";

import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";
import type { ConfirmableAct } from "./Acts";
import { ACT_LABEL } from "./copy";
import { OverruleControl } from "./Overrule";
import type { ActingAct } from "./pending";
import type { Opens } from "./phases";
import { recourseOf } from "./recovery";
import { RedirectControl } from "./Redirect";
import type { Render } from "./render";
import { steeringOf } from "./steering";

/**
 * The five acts that change a step rather than the Job, in the panel header
 * beside the step they act on.
 *
 * **They were rendered at Job level and five of the eleven do not act on the
 * Job.** Redirecting a Drone, restarting a step, overruling the verdict on a
 * step, asking a step's gate again and running its Checks again all leave the
 * Job exactly where it is — only the step moves. Drawn in the Job header they
 * read as peers of the two kills, which is the reading job detail was
 * redrawn to end.
 *
 * **The accent goes with them.** The object of attention on this screen is the
 * open step, and the Job header keeps only the acts that end or replace the
 * Job. Which one takes the fill is the caller's, because it depends on which
 * act the state calls for — nothing here decides emphasis for a state it cannot
 * see.
 *
 * **Which of the five is offered is `recovery.ts`'s reading, unchanged.** It is
 * the side that can see the worktree, and a restart offered without that answer
 * was refused on the press every time the worktree had been reclaimed. Moving
 * the controls did not move that decision.
 *
 * **On a Job that is working, one of the five is still offered, and by a second
 * reading.** `stuck` is absent on a Job that has not stopped — it is the record
 * of a stop — so `steering.ts` answers for that Job instead, off the pointer
 * this header already draws `kill_drone` from. A redirect is legal on a healthy
 * Drone since #145 and reached no control anywhere in Bridge, which is #383.
 * The two readings never both apply: a Job has stopped or it has not.
 *
 * **What each one does is on its own tooltip, with its binding.** The journey
 * says it plainly — a step's help text is a tooltip carrying its binding, shown
 * on hover and on focus — and the four sentences were being concatenated into
 * one paragraph above the buttons, which described the menu rather than the
 * step and put every sentence out of reach of the control it was about. What a
 * destructive act costs is still stated again in its confirmation.
 */
export function StepActs({
  job,
  whole,
  opens,
  render,
  acting,
  actingAct,
  rerunningChecks,
  stale,
  onAct,
  onRedirect,
  onOverrule,
  onRerun,
  onRerunChecks,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  /**
   * How a kept record opens. **Carried through rather than used here** —
   * nothing in this header opens a file, and the override dialog draws a
   * gaming flag's brief, which is one of the three records the panel already
   * opens beside the verdict it argues with.
   */
  opens: Opens;
  render: Render;
  acting: boolean;
  /** Which act `acting` is, so the control that sent it is the one that waits. #1117. */
  actingAct?: ActingAct | undefined;
  /**
   * This Job's stopped step is having its Checks run again. **A wait worth
   * saying**: the press can run for minutes, and the button stays legible
   * about what it is doing rather than only going grey the way a redirect or
   * a restart's instant round trip does.
   */
  rerunningChecks: boolean;
  stale: boolean;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Overrule the verdict, with the reason. Straight through like a redirect —
   * the dialog that collected the reason is the confirmation.
   */
  onOverrule: (jobId: string, reason: string) => void;
  /**
   * Ask the gate again on a step it could not decide. **Straight through like a
   * redirect, and for a different reason** — that one already confirmed in its
   * own dialog, and this one has nothing to confirm.
   */
  onRerun: (jobId: string) => void;
  /** Run a stopped step's Checks again. `onRerun`'s shape, for the other trigger. */
  onRerunChecks: (jobId: string) => void;
}) {
  // Fleet's answer, on the render that carries one. A Job nothing is wrong with
  // has no `stuck` and so offers none of these — which is the wire's reading
  // and not a rule written here.
  const recourse = render === "stopped" ? recourseOf(job, whole) : undefined;
  // The other reading, on the render `stuck` says nothing about. **Never both**
  // — a Job has stopped or it has not — and the two are separate calls rather
  // than one widened answer, which is the whole of #383: `stuck` is the record
  // of a stop, and a Job that has not stopped has no stop to describe.
  const steering = render === "working" ? steeringOf(job, whole) : undefined;
  // One control, two states that reach it. Since #145 a redirect is legal on a
  // healthy Drone as well as one holding at a step that stopped, and what
  // differs is which sentence it is offered in — `docs/concepts/drone.md` had
  // said so all along, and nothing on this screen drew it.
  const redirect: Redirect | undefined =
    recourse?.act === "redirect"
      ? { drone: "holding", says: recourse.says.redirect }
      : steering?.act === "redirect"
        ? { drone: "working", says: steering.says.redirect }
        : undefined;
  const canRestart = recourse?.act === "restart_step";
  // Beside the two rather than instead of one: which trigger stopped the step
  // decides these, and whether a Drone is there decides those. **Never both**,
  // because the two triggers partition — `recovery.ts` says so.
  const overrule = recourse?.overrule;
  const reread = recourse?.reread;
  const rerunChecks = recourse?.rerunChecks;

  return (
    <>
      {/* First of the acts that resume, because it is the one that takes
          nothing away — the refused step's own work is kept. Secondary and not
          primary: an override that looked like an approval would be claiming
          the work was right rather than that the Judge was wrong. */}
      {overrule === undefined ? null : (
        // No binding on the tooltip: `actions.toml` registers none for the
        // override, and a tooltip promising a key the map does not hold is
        // worse than one with no key at all.
        <Tooltip label={recourse?.says.override_verdict ?? ACT_LABEL.override_verdict}>
          <OverruleControl
            jobId={job.id}
            overrule={overrule}
            opens={opens}
            disabled={acting || stale}
            pending={acting && actingAct === "override_verdict"}
            onOverrule={onOverrule}
          />
        </Tooltip>
      )}
      {/* Where nothing ruled, in the place the override would be: the two are
          mutually exclusive, and both keep the step's work. **No dialog and no
          confirmation** — a re-run destroys nothing, overrules nothing and
          commits nothing, so stopping to ask would claim a cost Fleet does not
          charge. */}
      {reread === undefined ? null : (
        <Tooltip label={recourse?.says.rerun_gate ?? ACT_LABEL.rerun_gate}>
          <Button
            variant="secondary"
            pending={acting && actingAct === "rerun_gate"}
            disabled={acting || stale}
            onClick={() => onRerun(job.id)}
          >
            {acting && actingAct === "rerun_gate" ? ASKING_AGAIN : ACT_LABEL.rerun_gate}
          </Button>
        </Tooltip>
      )}
      {/* Neither ends the Job, so neither is a plain-red act. The dialog a
          redirect opens is itself the confirmation — a person who cancels it
          has sent nothing. */}
      {redirect === undefined ? null : (
        <Tooltip label={redirect.says ?? ACT_LABEL.redirect} shortcut={REDIRECT_KEY}>
          <RedirectControl
            jobId={job.id}
            drone={redirect.drone}
            disabled={acting || stale}
            pending={acting && actingAct === "redirect"}
            onRedirect={onRedirect}
          />
        </Tooltip>
      )}
      {/* Ahead of restart, because it takes nothing away — no Drone spawns and
          no retry is spent, where a restart spends both. While it runs, the
          button says so rather than reading as a dead end: this can take
          minutes, `CHECKS_MS`'s reason. **`pending` covers both waits**: the
          initial send, where `actingAct` names it, and the run itself, where
          `rerunningChecks` does once Fleet has taken it up — one bar for a
          wait that changes which flag is carrying it partway through. */}
      {rerunChecks === undefined ? null : (
        <Tooltip label={rerunningChecks ? CHECKS_RUNNING : (recourse?.says.rerun_checks ?? ACT_LABEL.rerun_checks)}>
          <Button
            variant="secondary"
            pending={(acting && actingAct === "rerun_checks") || rerunningChecks}
            disabled={acting || stale}
            onClick={() => onRerunChecks(job.id)}
          >
            {rerunningChecks ? RUNNING_CHECKS : ACT_LABEL.rerun_checks}
          </Button>
        </Tooltip>
      )}
      {canRestart ? (
        <Tooltip label={recourse?.says.restart_step ?? ACT_LABEL.restart_step} shortcut={RESTART_KEY}>
          <Button
            variant="secondary"
            pending={acting && actingAct === "restart_step"}
            disabled={acting || stale}
            onClick={() => onAct("restart_step", job.id)}
          >
            {acting && actingAct === "restart_step" ? RESTARTING : ACT_LABEL.restart_step}
          </Button>
        </Tooltip>
      ) : null}
    </>
  );
}

/**
 * The bindings the two step acts carry, from `actions.toml`'s contextual tier —
 * `d` redirect, `s` restart step. Written beside the tooltip that displays
 * them, because a tooltip promising a key the map does not hold is worse than
 * one with no key at all.
 */
const REDIRECT_KEY = "d";
const RESTART_KEY = "s";

/** The button's face while its own press is out. `ACT_LABEL.rerun_checks`'s verb, in flight. */
const RUNNING_CHECKS = "Running Checks";

/** The tooltip while its own press is out — said rather than only greyed, `CHECKS_MS`'s reason. */
const CHECKS_RUNNING = "The Checks are running now. This can take a few minutes.";

/** `ACT_LABEL.rerun_gate`'s verb, in flight. #1117. */
const ASKING_AGAIN = "Asking the gate again…";

/** `ACT_LABEL.restart_step`'s verb, in flight. #1117. */
const RESTARTING = "Restarting the step…";

/**
 * The redirect on offer, and which job it is about. **Which reading produced it
 * is not carried** — the control takes one act and one wait, and the two
 * readings that reach it have already agreed on both by the time they get here.
 */
type Redirect = {
  drone: "holding" | "working";
  /** The act's own sentence, where the reading had one. */
  says: string | undefined;
};
