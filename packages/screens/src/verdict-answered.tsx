// The verdict sheet's fifth arrangement — a Job that is done, after a person
// answered it somewhere along the way: approved, merged, or overruled a
// verdict that had stopped a step.
//
// **This is the one arrangement that reads the whole Job.** `verdict.tsx`'s
// own header says why the other four read one `StepDetail`: the panel's
// `open` step is "the step this Job is about" everywhere else on the screen.
// It is not, here — a verdict a person set aside is the reason this
// arrangement exists, and the step that carries it is very often not the one
// a fresh reader lands on. A record that only showed what the open step
// happened to be would hide the exact thing it exists to surface. So this
// file reads `whole.steps` whole, groups the Checks the way a person reads a
// finished Job — which steps passed, and on what — and gives every judged
// criterion, on every step, its own row.
//
// **`verdictOf` is still not this file's.** It is the four-block, one-step
// reading `verdictSlotAtGate` and `verdictSlotFinished` use, and duplicating
// its plumbing here for a different shape would be the second copy that made
// this split worth making. The four block labels are the only thing shared —
// "What you asked for", "What came back", "What proves it", "What it left
// alone" — because the sheet is one document with one layout; what fills each
// block is this file's own read.

import type { ReactNode } from "react";
import {
  CheckRuns,
  VerdictSheet,
  CHECK_OUTCOME,
  CRITERION_VERDICT_JUDGE,
  type CheckRun as CheckRunRow,
  type VerdictFigure,
  type VerdictSheetProps,
} from "@armada/components";
import type {
  Diff,
  Evidence,
  JobDetail as JobWhole,
  JobSummary,
  Noted,
  Remarks,
  Settled,
  StepDetail,
} from "@armada/protocol";

import { money, pullRequestNumber } from "./facts";
import { absoluteOf, span } from "./duration";
import { checkRow, saidOf, iconOf } from "./checks";
import { checksOf, didNotPass, mechanicalRunsOf, panelsOf } from "./gates";
import { basename, keptOf, type Opens } from "./phases";
import { cameBackOf, criteriaOf, leftAloneOf, neverDelivers } from "./verdict";
import { openPullRequest, type OpenPullRequest } from "./opening";
import { LANDED } from "./Row";

/** The steps a Job froze, ordered. `facts.ts`'s own read, kept local rather than adding a dependency for one line. */
function orderedSteps(whole: JobWhole | null): StepDetail[] {
  return whole === null ? [] : [...whole.steps].sort((a, b) => a.ordinal - b.ordinal);
}

/**
 * The step a person answers at — `advance_gate: human_always` — where this
 * Job's frozen workflow carries one.
 */
export function humanGateStepOf(steps: readonly StepDetail[]): StepDetail | undefined {
  return steps.find((one) => one.advance_gate === "human_always");
}

/**
 * The step "What came back", "What it left alone" and the deliverable figure
 * read — the delivering step, or the last one. **Never the open step**: a
 * reader who has not navigated anywhere still gets the Job's own summary.
 */
function chosenStepOf(steps: readonly StepDetail[]): StepDetail | undefined {
  return steps.find((one) => one.delivers === true) ?? steps[steps.length - 1];
}

/**
 * What this Job's Drones claimed, read as one sentence for the whole Job.
 * **The delivering or last step's claim, falling back to the last step that
 * claimed anything.** A workflow's closing step often only confirms the
 * handoff — `evidence_type: "facts_note"`, nothing of its own to claim — and a
 * reader asking what this Job did wants the step that actually did it.
 */
function chosenClaimOf(steps: readonly StepDetail[], evidence: Evidence) {
  const submitted = evidence.state === "read" ? evidence.steps : [];
  const primary = chosenStepOf(steps);
  const primaryClaim = submitted.find((one) => one.step_id === primary?.step_id);
  if (primaryClaim !== undefined) return primaryClaim;
  for (let at = steps.length - 1; at >= 0; at -= 1) {
    const claim = submitted.find((one) => one.step_id === steps[at]?.step_id);
    if (claim !== undefined) return claim;
  }
  return undefined;
}

/** The one line Fleet writes when a person overrules a gate. `crate::overruling`'s own. */
const OVERRULE_NOTE = "a person overruled the gate and the step advanced";

/**
 * The reason a person gave for overruling the named step, where the Job's own
 * log kept one. **`undefined` is a real answer, never a placeholder** — an
 * overrule with nothing typed into it writes no `said` field at all
 * (`Overruling::saying`), and a step overruled before Fleet logged the note
 * carries none either.
 *
 * **Scoped to one step and the last note about it.** A step retried after an
 * overrule can be overruled again, and the most recent note is the one that
 * answers "why does this step's record say what it says now".
 */
export function overruleReasonOf(notes: readonly Noted[], stepId: string): string | undefined {
  const note = [...notes].reverse().find((one) => one.step === stepId && one.msg === OVERRULE_NOTE);
  return note?.fields?.find((field) => field.name === "said")?.value;
}

/** `["scope", "implement", "tests"]` as `"Scope, implement and tests"`. */
function sentenceListOf(items: readonly string[]): string {
  const said =
    items.length <= 1 ? (items[0] ?? "") : `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
  return said.charAt(0).toUpperCase() + said.slice(1);
}

/**
 * The step, with its own `attempts` read back to the last one that actually
 * produced Check or Judge data.
 *
 * **A step Fleet re-asked the gate on, without re-running anything, records a
 * further `attempts` entry carrying none of its own.** This real Job's own
 * `tests` step is exactly that: its first attempt stopped `gate_undecided`,
 * a person asked the gate again, and the second attempt answered
 * `gate_failure` off the same Checks and the same Judge call — writing no
 * `check_runs` and no `judged` row tagged with its own number.
 * `onlyCurrentAttempt` (`facts.ts`) reads `attempts.at(-1)` and finds nothing
 * for a step shaped like that, which is a defect in that shared read and not
 * this one — scoped here, on the one arrangement whose whole argument is that
 * an overruled verdict is never hidden, rather than changed for every other
 * caller of a read this task was not asked to touch.
 */
function withDataAttempt(step: StepDetail): StepDetail {
  const last = step.attempts.at(-1);
  if (last === undefined) return step;
  const hasDataAt = (attempt: number) =>
    step.check_runs.some((run) => run.attempt === attempt) || step.judged.some((one) => one.attempt === attempt);
  if (hasDataAt(last.attempt)) return step;
  for (let at = last.attempt - 1; at >= 1; at -= 1) {
    if (hasDataAt(at)) return { ...step, attempts: [...step.attempts.slice(0, -1), { ...last, attempt: at }] };
  }
  return step;
}

/**
 * Every Check any step ran, across the whole Job, as `provesIt` draws Checks.
 * **Passing steps fold into one row** — `"Scope, implement and tests passed
 * their Checks"`, naming which Checks — because a reader scanning a finished
 * Job for what proved it wants to know which steps held, not to re-read every
 * Check on every one of them one at a time. **A Check that did not pass keeps
 * its own row, per step**: that is the one thing this whole-Job read cannot
 * fold away without hiding it.
 */
function checksAcrossJobOf(steps: readonly StepDetail[], now: number): CheckRunRow[] {
  const passedSteps: string[] = [];
  const passedNames: string[] = [];
  const rows: CheckRunRow[] = [];
  for (const raw of steps) {
    const step = withDataAttempt(raw);
    const reads = checksOf(step);
    const mechanical = mechanicalRunsOf(step);
    if (reads.length === 0 && mechanical.length === 0) continue;
    // **`skipped` still advances — `CHECK_ADVANCES.skipped` — so it never
    // blocks a step from joining the passed group.** It never ran, though, so
    // it is left out of the group's own Check names: naming a Check that did
    // not run beside the ones that did would claim it for something it never
    // did.
    const named: { name: string; row: CheckRunRow; ran: boolean }[] = [
      ...reads.map((read) => ({ name: read.name, row: checkRow(read, now), ran: read.run?.outcome !== "skipped" })),
      ...mechanical.map((run) => ({
        name: run.name,
        ran: run.outcome !== "skipped",
        row: {
          id: run.name,
          says: saidOf(run),
          identifier: run.name,
          named: (didNotPass(run) ? "failed" : "passed") as CheckRunRow["named"],
          icon: iconOf(run),
          ...(run.produced === undefined ? {} : { result: run.produced }),
        },
      })),
    ];
    if (named.length > 0 && named.every(({ row }) => row.named === "passed")) {
      passedSteps.push(step.step_id);
      for (const { name, ran } of named) if (ran && !passedNames.includes(name)) passedNames.push(name);
    } else {
      for (const { name, row } of named) {
        if (row.named !== "passed") rows.push({ ...row, id: `${step.step_id}:${name}` });
      }
    }
  }
  if (passedSteps.length > 0) {
    rows.unshift({
      id: "checks-passed",
      says: `${sentenceListOf(passedSteps)} passed their Checks`,
      identifier: passedNames.join(" · "),
      named: "passed",
      icon: CHECK_OUTCOME.passed?.icon ?? undefined,
    });
  }
  return rows;
}

/**
 * Every criterion any step's Judge answered, across the whole Job — one row
 * each, in plain words. **The criterion itself, then what the Judge said,
 * never a count** — `"1 of 2 criteria refused"` is a fact about the panel,
 * not about the criterion, and this reads one criterion at a time. **A
 * criterion whose step was overruled always carries the Judge's own grounds
 * and, where the log kept one, the person's own reason** — labelled apart,
 * because they answer different questions: why the Judge refused, and why a
 * person let the work stand anyway.
 */
function judgeRowsAcrossJobOf(
  steps: readonly StepDetail[],
  criteria: JobWhole["acceptance_criteria"],
  notes: readonly Noted[],
): CheckRunRow[] {
  const rows: CheckRunRow[] = [];
  for (const raw of steps) {
    if ((raw.judge_checks?.length ?? 0) === 0) continue;
    const step = withDataAttempt(raw);
    for (const panel of panelsOf(step, criteria)) {
      const text = panel.criterion?.text ?? panel.criterionId;
      if (panel.verdict === "met") {
        rows.push({
          id: `${step.step_id}:${panel.criterionId}`,
          says: text,
          identifier: "Judge: met",
          identifierIsAName: true,
          named: "passed",
          icon: CRITERION_VERDICT_JUDGE.met?.icon ?? undefined,
        });
        continue;
      }
      const overruled = step.overridden === true;
      const finding = panel.refused[0];
      const grounds = finding?.expected ?? finding?.produced ?? finding?.consequence;
      const reason = overruled ? overruleReasonOf(notes, step.step_id) : undefined;
      rows.push({
        id: `${step.step_id}:${panel.criterionId}`,
        says: text,
        identifier: overruled ? "Judge: not met · overruled by you" : "Judge: not met",
        identifierIsAName: true,
        named: overruled ? "overruled" : "refused",
        icon: CRITERION_VERDICT_JUDGE.not_met?.icon ?? undefined,
        ...(grounds === undefined && reason === undefined
          ? {}
          : {
              detail: (
                <>
                  {grounds === undefined ? null : (
                    <span className="armada-check-runs__detail-line">{`Judge’s reason: ${grounds}`}</span>
                  )}
                  {reason === undefined ? null : (
                    <span className="armada-check-runs__detail-line">{`Your reason: “${reason}”`}</span>
                  )}
                </>
              ),
            }),
      });
    }
  }
  return rows;
}

/** What stands in for the checklist where no step recorded anything to prove it. */
const NOTHING_CHECKED_WHAT_IT_SAYS = "Nothing checked what it says";

/** What proves it, across the whole Job. */
function provesItAcrossJobOf(
  steps: readonly StepDetail[],
  criteria: JobWhole["acceptance_criteria"],
  notes: readonly Noted[],
  now: number,
): CheckRunRow[] {
  const rows = [...checksAcrossJobOf(steps, now), ...judgeRowsAcrossJobOf(steps, criteria, notes)];
  if (rows.length > 0) return rows;
  return [{ id: "nothing-checked", says: NOTHING_CHECKED_WHAT_IT_SAYS, identifier: "no Judge · no Checks" }];
}

/** How many files this Job's worktree holds against the branch it was cut from. */
function filesCountOf(diff: Diff, jobId: string): number | undefined {
  const mine = diff.state !== "none" && diff.jobId === jobId ? diff : null;
  if (mine === null || mine.state !== "read" || mine.work === undefined) return undefined;
  return mine.work.files.length;
}

/**
 * How long the Job ran, and what it cost — off the human gate step's own
 * `updated_at`, the instant that closed the Job, rather than the live clock
 * every other render reads against.
 */
function tookOf(job: JobSummary, whole: JobWhole | null, gate: StepDetail | undefined): string | undefined {
  const elapsed = gate === undefined ? null : span(job.created_at, gate.updated_at);
  if (elapsed === null) return undefined;
  const spend = whole?.spend;
  return spend === undefined ? elapsed : `${elapsed} · ${money(spend.cost_micros)}`;
}

/** The figures, across the whole Job. */
function figuresAcrossJobOf({
  job,
  whole,
  chosen,
  gate,
  diff,
  opens,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  chosen: StepDetail | undefined;
  gate: StepDetail | undefined;
  diff: Diff;
  opens: Opens;
}): VerdictFigure[] {
  const never = neverDelivers(whole?.steps ?? []);
  const figures: VerdictFigure[] = [];
  if (never === true && chosen !== undefined) {
    const kept = keptOf(chosen, opens);
    if (kept.length > 0) figures.push({ label: "Document", value: basename(kept[0]?.path ?? ""), mono: true });
  } else {
    figures.push(
      job.branch === undefined
        ? { label: "Branch", absent: "No branch yet" }
        : { label: "Branch", value: job.branch, mono: true },
    );
    const files = filesCountOf(diff, job.id);
    if (files !== undefined) figures.push({ label: "Files", value: String(files), mono: true });
  }
  const took = tookOf(job, whole, gate);
  if (took !== undefined) figures.push({ label: "Took", value: took, mono: true });
  if (whole?.spend?.drones !== undefined) {
    figures.push({ label: "Drones", value: String(whole.spend.drones), mono: true });
  }
  const steps = whole?.steps ?? [];
  if (steps.length > 0) {
    const advanced = steps.filter((one) => one.state === "advanced").length;
    figures.push({ label: "Steps", value: `${advanced} of ${steps.length} advanced`, mono: true });
  }
  if (never === true) figures.push({ label: "Pull request", value: "never, for this workflow", mono: true });
  return figures;
}

/**
 * The pull request block for a Job that is over — case 5 of `why-b.md`.
 * **One line, and it opens on the forge.** `pullRequestBlockOf`'s open-review
 * block reads `mergeable` and open reviews, which a settled pull request no
 * longer carries; this reads `landed` instead, and the number follows the
 * same link every other pull request fact on this screen already offers.
 */
function settledPullRequestBlockOf(
  address: string | undefined,
  landed: Settled | undefined,
  onFollow: () => void,
): ReactNode | undefined {
  if (address === undefined || landed === undefined) return undefined;
  const said = LANDED[landed];
  if (said === undefined) return undefined;
  const number = pullRequestNumber(address) ?? "Pull request";
  return (
    <p className="text-xs text-fg-muted">
      <a
        href={address}
        title={address}
        className="mono armada-verdict__pr-link"
        onClick={(event) => {
          event.preventDefault();
          onFollow();
        }}
      >
        {number}
      </a>
      {" · "}
      {said}
    </p>
  );
}

/** What "Done" stands beside, in the header. */
const DONE = "Done";

function headerOf(gate: StepDetail | undefined): VerdictSheetProps["header"] {
  if (gate === undefined) return undefined;
  const when = absoluteOf(gate.updated_at) ?? gate.updated_at;
  return { done: DONE, when: gate.overridden ? `overruled ${when}` : `approved ${when}` };
}

/** The caps label over the record that replaces the buttons. `Block`'s own class. */
const YOUR_ANSWER = "Your answer";

function yourAnswerOf(gate: StepDetail | undefined, landed: Settled | undefined): ReactNode {
  const when = gate === undefined ? undefined : absoluteOf(gate.updated_at);
  const said =
    gate === undefined || when === undefined
      ? "You answered this Job. Nothing is asked of anyone now."
      : gate.overridden
        ? `You overruled the verdict and let the work stand, on ${when}. Nothing is asked of anyone now.`
        : landed === undefined
          ? `You approved the work on ${when}. Nothing is asked of anyone now.`
          : `You approved the work on ${when}, and the pull request ${LANDED[landed] ?? "settled"}. ` +
            "Nothing is asked of anyone now.";
  return (
    <>
      <span className="armada-verdict__label">{YOUR_ANSWER}</span>
      <p className="armada-verdict__said">{said}</p>
    </>
  );
}

export type VerdictSlotAfterAnswerArgs = {
  job: JobSummary;
  whole: JobWhole | null;
  recorded: { diff: Diff; evidence: Evidence; remarks: Remarks };
  opensRecords: Opens;
  /** Now, injected. `checkRow`'s own reason: a Check's row can read a live elapsed time. */
  now: number;
  /** The Job's own log, for every overruled step's reason. */
  notes: readonly Noted[];
  /** Open this Job's pull request on the forge. */
  onOpenPullRequest: OpenPullRequest;
};

/**
 * The verdict sheet once a Job is done and a person answered it along the
 * way — the fifth arrangement. **Reads the whole Job, not the open step**: a
 * verdict a person set aside is the reason this arrangement exists, and it
 * has to be found whichever step a reader lands on, including the default —
 * usually the delivering step, which frequently carries no Judge of its own.
 */
export function verdictSlotAfterAnswer({
  job,
  whole,
  recorded,
  opensRecords,
  now,
  notes,
  onOpenPullRequest,
}: VerdictSlotAfterAnswerArgs): ReactNode {
  const steps = orderedSteps(whole);
  const chosen = chosenStepOf(steps);
  const claim = chosenClaimOf(steps, recorded.evidence);
  const address = whole?.delivery?.pull_request;
  const landed = whole?.delivery?.landed;
  const gate = humanGateStepOf(steps);
  const never = neverDelivers(steps);
  const criteria = whole?.acceptance_criteria ?? [];
  return (
    <VerdictSheet
      header={headerOf(gate)}
      title={job.title}
      criteria={criteriaOf(whole)}
      criteriaAbsent="This Job's frozen workflow named no acceptance criteria."
      cameBack={cameBackOf(claim)}
      {...(never === true && chosen !== undefined
        ? { deliverable: keptOf(chosen, opensRecords)[0]?.opening }
        : {})}
      {...(address === undefined || landed === undefined
        ? {}
        : {
            pullRequest: settledPullRequestBlockOf(address, landed, () => {
              void openPullRequest(onOpenPullRequest, job.id).then((because) => {
                if (because !== null) opensRecords.onSaid(because);
              });
            }),
          })}
      provesIt={<CheckRuns rows={provesItAcrossJobOf(steps, criteria, notes, now)} />}
      leftAlone={leftAloneOf(claim)}
      figures={figuresAcrossJobOf({ job, whole, chosen, gate, diff: recorded.diff, opens: opensRecords })}
      recordNote={yourAnswerOf(gate, landed)}
    />
  );
}
