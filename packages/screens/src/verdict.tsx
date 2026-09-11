// The verdict sheet's own data: what the four blocks and the figures are built
// from, for the one step a Job is waiting at or finished on.
//
// **Everything here reads one `StepDetail`** — the panel's `open` step, the
// same one `Decide` and `phasesOf` already read. A finished Job whose record
// spans several steps is a real reading this file does not attempt: the wire
// fields the verdict sheet is built from name a single step's `check_runs`
// and `judged`, and the open step is the one every other region on this panel
// already agrees is "the step this Job is about".
//
// **`checksOf` alone is not the whole gate.** It joins from `step.checks`, the
// declaration, so a mechanical check with no declared counterpart —
// `artifact_exists` on a workflow with no other tier — has a run and no row.
// `mechanicalRunsOf` is the other half of that read, and both are folded into
// one list here so nothing a gate actually ran goes missing from the record.
//
// **The fifth arrangement is `verdict-answered.tsx`, not here.**

import { openPullRequest, type OpenPullRequest } from "./opening";
import type { ReactNode } from "react";
import { GitPullRequest, Minus } from "lucide-react";
import {
  CheckRuns,
  VerdictSheet,
  type CheckRun as CheckRunRow,
  type VerdictFigure,
  type VerdictSheetProps,
} from "@armada/components";
import type {
  Diff,
  Evidence,
  JobDetail as JobWhole,
  JobSummary,
  PullRequestDetail,
  Remarks,
  StepDetail,
  Submitted,
} from "@armada/protocol";

import { hostLabel, money, pullRequestNumber } from "./facts";
import { span } from "./duration";
import { checkRow, judgeRow, saidOf, iconOf } from "./checks";
import { Decide } from "./Decide";
import { checksOf, didNotPass, mechanicalRunsOf, panelsOf } from "./gates";
import { basename, keptOf, type Opens } from "./phases";
import type { Render } from "./render";

/**
 * Whether this Job's frozen workflow ever opens a pull request.
 *
 * **Three answers, because two facts do not imply each other.** `true` where
 * any step says `delivers`, `false` where every step says it does not, and
 * `undefined` where at least one step cannot say and none has said `true` —
 * the one case a client must not round to a claim either way.
 */
export function neverDelivers(steps: readonly StepDetail[]): boolean | undefined {
  if (steps.length === 0) return undefined;
  if (steps.some((step) => step.delivers === true)) return false;
  if (steps.every((step) => step.delivers === false)) return true;
  return undefined;
}

/**
 * Whether this Job's frozen workflow ever stops for a person.
 *
 * Same three-answer shape as [`neverDelivers`], read off `advance_gate`
 * instead of `delivers`. `render.ts`'s `reviewing` already tells a live Job
 * that is stopped for one apart from a Job that is not; this is what a
 * **finished** Job needs, because the render alone cannot say whether the
 * ending it reached ever asked anybody anything.
 */
export function neverAsksAPerson(steps: readonly StepDetail[]): boolean | undefined {
  if (steps.length === 0) return undefined;
  if (steps.some((step) => step.advance_gate === "human_always")) return false;
  if (steps.every((step) => step.advance_gate !== undefined)) return true;
  return undefined;
}

/**
 * `crates/core-model/src/job/workflow.rs::ARTIFACT_EXISTS` — the one
 * mechanical check that is not really a gate: it confirms the deliverable
 * exists and looks at nothing about what it says.
 */
const ARTIFACT_EXISTS = "artifact_exists";

/**
 * Whether nothing beyond the deliverable's existence looked at this step.
 *
 * **No Judge, and no Check but `artifact_exists`.** A step declaring a real
 * Check — `diff_nonempty`, a Manifest Check — has something else gating it
 * and this reads `false`; a step Fleet cannot describe at all has no checks
 * to test this against either, which is `provesItNoteOf`'s "cannot say" case
 * and not this one — so this is `false` there too, on an empty list rather
 * than a true one.
 */
function nothingButExistence(step: StepDetail): boolean {
  if ((step.judge_checks?.length ?? 0) > 0) return false;
  const named = [
    ...checksOf(step).map((read) => read.name),
    ...mechanicalRunsOf(step).map((run) => run.name),
  ];
  return named.length > 0 && named.every((name) => name === ARTIFACT_EXISTS);
}

/** What the checklist's own row says where [`nothingButExistence`] is true. */
const NOTHING_CHECKED_WHAT_IT_SAYS = "Nothing checked what it says";

/** The rows for "what proves it" — every Check this attempt ran, and the Judge's. */
export function provesItOf(
  step: StepDetail,
  criteria: JobWhole["acceptance_criteria"],
  now: number,
  /** Fleet's own reason the gate could not decide. `checksChapter`'s own. */
  undecided?: string,
  /** A person's own words for overruling this step, where the log kept one. */ reason?: string,
): CheckRunRow[] {
  const rows = checksOf(step).map((read) => checkRow(read, now));
  for (const run of mechanicalRunsOf(step)) {
    rows.push({
      id: run.name,
      says: saidOf(run),
      identifier: run.name,
      named: didNotPass(run) ? "failed" : "passed",
      icon: iconOf(run),
      ...(run.produced === undefined ? {} : { result: run.produced }),
    });
  }
  const panels = panelsOf(step, criteria);
  const judge = judgeRow(step, panels, undecided, reason);
  if (judge !== undefined) rows.push(judge);
  // **The row that says nothing else looked.** Drawn from the same data that
  // decides `provesItNoteOf`'s closing sentence, on a workflow whose only
  // mechanical tier is confirming the deliverable exists — never hard-coded
  // to a render or a workflow name, so a workflow that later adds a real
  // Check or a Judge loses the row on its own.
  if (nothingButExistence(step)) {
    rows.push({
      id: "nothing-checked",
      says: NOTHING_CHECKED_WHAT_IT_SAYS,
      identifier: "no Judge · no Checks",
      icon: Minus,
    });
  }
  return rows;
}

/**
 * The line under the checklist. **Only where nothing semantic looked at the
 * work** — a step with a Judge declared says nothing extra here, because the
 * checklist above it already carries the panel's own row.
 */
export function provesItNoteOf(step: StepDetail, render: Render): string | undefined {
  if ((step.judge_checks?.length ?? 0) > 0) return undefined;
  if (checksOf(step).length === 0 && mechanicalRunsOf(step).length === 0) {
    return "Fleet cannot say what gates this step, because it does not hold the workflow this Job named.";
  }
  return render === "finished"
    ? "Every step advanced on its own. No person was asked, and no gate read the review."
    : "Reading the document is the review. Your answer is the only verdict this step gets.";
}

/** What the Drone says it did — its own claim, or why there is nothing to read yet. */
export function cameBackOf(claim: Submitted | undefined): string {
  return claim?.claimed ?? "This step has not submitted its evidence yet.";
}

/** What the Drone says it left alone — `not_claimed`, or the named absence of a boundary. */
export function leftAloneOf(claim: Submitted | undefined): string {
  if (claim === undefined) return "This step has not submitted its evidence yet.";
  return claim.not_claimed ?? "This step's submission drew no boundary around what it did not change.";
}

/** The acceptance criteria, as the sentences a person asked for. */
export function criteriaOf(whole: JobWhole | null): string[] {
  return (whole?.acceptance_criteria ?? []).map((one) => one.text);
}

/**
 * Fleet's `**bold**` and `` `code` `` spans, read as plain text.
 *
 * **`review`'s Markdown is written for the pull request's own renderer.** The
 * words are one builder's — that is the whole point of `#665` — but the
 * marks around them are not; a sheet with no Markdown renderer would draw the
 * asterisks and backticks themselves, and that is not the same words.
 */
function plainTextOf(markdown: string): string {
  return markdown.replaceAll("**", "").replaceAll("`", "");
}

/**
 * The brief — Fleet's own `why` section, the same words the pull request's
 * "Why was the change needed?" carries. Absent where Fleet has composed no
 * review yet: a Job still running, or read off a Fleet older than 10.10.
 */
export function briefOf(whole: JobWhole | null): string | undefined {
  const why = whole?.review?.why;
  return why === undefined || why.length === 0 ? undefined : plainTextOf(why);
}

/**
 * What nothing checked, and what the base carries that this Job did not
 * write — Fleet's own `risks` section, the same words the pull request's
 * "Risks" carries. Absent for `briefOf`'s reason.
 */
export function risksOf(whole: JobWhole | null): string | undefined {
  const risks = whole?.review?.risks;
  return risks === undefined || risks.trim().length === 0 ? undefined : plainTextOf(risks.trim());
}

/** The figures, in the order the drawing runs them. */
export function figuresOf({
  job,
  whole,
  step,
  render,
  diff,
  opens,
  now,
  drones = false,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  step: StepDetail;
  render: Render;
  diff: Diff;
  opens: Opens;
  now: number;
  /** Whether to add how many Drones ran. Off by default — the fifth arrangement's own ask. */
  drones?: boolean;
}): VerdictFigure[] {
  const never = neverDelivers(whole?.steps ?? []);
  const figures: VerdictFigure[] = [];

  if (never === true) {
    const kept = keptOf(step, opens);
    if (kept.length > 0) {
      figures.push({ label: "Document", value: basename(kept[0]?.path ?? ""), mono: true });
    }
  } else {
    figures.push(
      job.branch === undefined
        ? { label: "Branch", absent: "No branch yet" }
        : { label: "Branch", value: job.branch, mono: true },
    );
    const files = filesCountOf(diff, job.id);
    if (files !== undefined) figures.push({ label: "Files", value: String(files), mono: true });
  }

  const took = tookOf(job, whole, now);
  if (took !== undefined) figures.push({ label: "Took", value: took, mono: true });

  if (drones && whole?.spend?.drones !== undefined)
    figures.push({ label: "Drones", value: String(whole.spend.drones), mono: true });
  const steps = whole?.steps ?? [];
  if (steps.length > 0) {
    const advanced = steps.filter((one) => one.state === "advanced").length;
    figures.push({
      label: "Steps",
      value: `${advanced} of ${steps.length} ${render === "finished" ? "advanced" : "passed"}`,
      mono: true,
    });
  }

  if (never === true) {
    figures.push({ label: "Pull request", value: "never, for this workflow", mono: true });
  }

  return figures;
}

/** How many files this Job's worktree holds against the branch it was cut from. */
function filesCountOf(diff: Diff, jobId: string): number | undefined {
  const mine = diff.state !== "none" && diff.jobId === jobId ? diff : null;
  if (mine === null || mine.state !== "read" || mine.work === undefined) return undefined;
  return mine.work.files.length;
}

/** How long the Job has run, and what it has spent, as one figure. */
function tookOf(job: JobSummary, whole: JobWhole | null, now: number): string | undefined {
  const elapsed = span(job.created_at, now);
  const spend = whole?.spend;
  if (elapsed === null) return undefined;
  return spend === undefined ? elapsed : `${elapsed} · ${money(spend.cost_micros)}`;
}

/**
 * The pull request block, where this Job has one: its number as a link to it,
 * and what Fleet's rotation last read of it.
 *
 * **The link draws on the address alone.** Before the rotation has read the
 * pull request there is no detail to show, and the owner asked on 11 Sep 2026
 * for the review to reach its pull request without going back up to the
 * header. It opens through `openPullRequest`, the header's own path, so the
 * address a click carries never decides what opens.
 *
 * **`mergeable` absent is "the forge would not say", never "conflicting".**
 * Drawing it as a refusal would be inventing a verdict the forge did not give.
 */
export function pullRequestBlockOf(
  address: string | undefined,
  detail: PullRequestDetail | undefined,
  onOpen?: () => void,
): ReactNode | undefined {
  if (address === undefined) return undefined;
  const number =
    detail?.number === undefined ? (pullRequestNumber(address) ?? "Pull request") : `#${detail.number}`;
  return (
    <div className="armada-verdict__pr">
      <p className="text-xs text-fg-muted">
        <span className="armada-verdict__pr-ref">
          <GitPullRequest size={12} strokeWidth={2} aria-hidden />
          <a
            className="armada-verdict__pr-link mono"
            href={address}
            title={address}
            onClick={(event) => {
              event.preventDefault();
              onOpen?.();
            }}
          >
            {number}
          </a>
        </span>
        {detail === undefined ? null : pullRequestReadOf(detail)}
      </p>
    </div>
  );
}

/** What the rotation last read of a pull request: its title, whether it merges, its reviews. */
function pullRequestReadOf(detail: PullRequestDetail): string {
  const mergeable =
    detail.mergeable === true ? "mergeable" : detail.mergeable === false ? "not mergeable" : "mergeable unknown";
  const approved = detail.reviews.filter((one) => one.verdict === "approved").length;
  const changes = detail.reviews.filter((one) => one.verdict === "changes_requested").length;
  const reviewed =
    detail.reviews.length === 0
      ? "no reviews yet"
      : [
          approved > 0 ? `${approved} approved` : undefined,
          changes > 0 ? `${changes} requested changes` : undefined,
        ]
          .filter((part): part is string => part !== undefined)
          .join(" · ") || `${detail.reviews.length} reviewed, unresolved`;
  const title = detail.title === undefined ? "" : ` · ${detail.title}`;
  return `${title}, open, ${mergeable}, ${reviewed}.`;
}

/** What `verdictOf` is built from — the panel's own reading of one Job and its open step. */
export type VerdictArgs = {
  job: JobSummary;
  whole: JobWhole | null;
  step: StepDetail;
  render: Render;
  diff: Diff;
  opens: Opens;
  now: number;
  /** This step's own submission, where the Drone has made one. */
  claim: Submitted | undefined;
  /** The pull request address and its live detail, off `JobDetail.delivery`, and how to open it. */
  pullRequest?: {
    address: string | undefined;
    detail: PullRequestDetail | undefined;
    onOpen?: () => void;
  };
  /** Fleet's own reason the gate could not decide, scoped to this step. */
  undecided?: string;
  /** A person's own words for overruling the open step, where the log kept one. */
  reason?: string;
  /** Whether the figures add how many Drones ran. */
  drones?: boolean;
};

/**
 * The whole of the verdict sheet's data, save `actions` — the acts a person
 * may take, which are `Decide`'s own region and never this file's to build.
 */
export function verdictOf({
  job,
  whole,
  step,
  render,
  diff,
  opens,
  now,
  claim,
  pullRequest,
  undecided,
  reason,
  drones,
}: VerdictArgs): Omit<VerdictSheetProps, "actions" | "note" | "recordNote"> {
  const kept = keptOf(step, opens);
  const never = neverDelivers(whole?.steps ?? []);
  return {
    title: job.title,
    ...(briefOf(whole) === undefined ? {} : { brief: briefOf(whole) }),
    criteria: criteriaOf(whole),
    criteriaAbsent: "This Job's frozen workflow named no acceptance criteria.",
    cameBack: cameBackOf(claim),
    ...(never === true && kept.length > 0 ? { deliverable: kept[0]?.opening } : {}),
    ...(pullRequest === undefined
      ? {}
      : { pullRequest: pullRequestBlockOf(pullRequest.address, pullRequest.detail, pullRequest.onOpen) }),
    provesIt: <CheckRuns rows={provesItOf(step, whole?.acceptance_criteria ?? [], now, undecided, reason)} />,
    ...(provesItNoteOf(step, render) === undefined
      ? {}
      : { provesItNote: provesItNoteOf(step, render) }),
    ...(risksOf(whole) === undefined ? {} : { risks: risksOf(whole) }),
    leftAlone: leftAloneOf(claim),
    figures: figuresOf({ job, whole, step, render, diff, opens, now, drones }),
  };
}

/** The three readings the verdict sheet's slots all need from the panel. */
export type VerdictReads = { diff: Diff; evidence: Evidence; remarks: Remarks };

export type VerdictSlotAtGateArgs = {
  job: JobSummary;
  whole: JobWhole | null;
  open: StepDetail;
  render: Render;
  recorded: VerdictReads;
  opensRecords: Opens;
  now: number;
  claimed: Submitted | undefined;
  /** Fleet's own reason the gate could not decide, scoped to this step. */
  undecided?: string;
  onNeedMaterial: (jobId: string | null) => void;
  onNeedRemarks: (jobId: string | null) => void;
  stale: boolean;
  deciding: boolean;
  onMergePullRequest: (jobId: string) => void;
  onApproveReview: (jobId: string) => void;
  onRequestChanges: (jobId: string, note: string) => void;
  onReject: (jobId: string) => void;
  onTakeUpRemarks: (jobId: string, remarks: string[]) => void;
  onOpenRemarkLink: (jobId: string, remarkId: string) => void;
  onOpenPullRequest: OpenPullRequest;
  onSaid: (sentence: string) => void;
};

/**
 * The verdict sheet at a gate — cases 1 through 3 of `why-b.md`. `Decide`'s
 * acts and confirmations are unchanged; they sit at `actions`, and everything
 * around them is the record built from the same Job.
 */
export function verdictSlotAtGate({
  job,
  whole,
  open,
  render,
  recorded,
  opensRecords,
  now,
  claimed,
  undecided,
  onNeedMaterial,
  onNeedRemarks,
  stale,
  deciding,
  onMergePullRequest,
  onApproveReview,
  onRequestChanges,
  onReject,
  onTakeUpRemarks,
  onOpenRemarkLink,
  onOpenPullRequest,
  onSaid,
}: VerdictSlotAtGateArgs): ReactNode {
  const address = whole?.delivery?.pull_request;
  const detail = whole?.delivery?.pull_request_detail;
  const never = neverDelivers(whole?.steps ?? []);
  // Never `auto_merge`: approving here never merges regardless of that
  // policy, which holds a later, separate gate (`fleet::gate`, `reviewing`).
  const note: ReactNode =
    never === true ? (
      "Approving ends the Job here. Nothing is merged, and no pull request is waiting on it."
    ) : address !== undefined ? (
      <>
        <strong>Merge and take the work</strong> merges this pull request on{" "}
        {hostLabel(address)}, then runs the repository&rsquo;s after-merge Checks on what landed.{" "}
        <strong>Approve the work</strong> takes it without merging — the pull request stays open.{" "}
        <strong>Request changes</strong> sends your note to the Drone, which keeps working on this
        same branch.
      </>
    ) : (
      "The run tree on the left is where each step's own evidence is. This reads the Job."
    );
  return (
    <VerdictSheet
      {...verdictOf({
        job,
        whole,
        step: open,
        render,
        diff: recorded.diff,
        opens: opensRecords,
        now,
        claim: claimed,
        pullRequest: {
          address,
          detail,
          onOpen: () =>
            void openPullRequest(onOpenPullRequest, job.id).then((because) => {
              if (because !== null) onSaid(because);
            }),
        },
        undecided,
      })}
      note={note}
      actions={
        <Decide
          job={job}
          onNeedMaterial={onNeedMaterial}
          onNeedRemarks={onNeedRemarks}
          evidence={recorded.evidence}
          remarks={recorded.remarks}
          stale={stale}
          deciding={deciding}
          {...(address === undefined ? {} : { pullRequest: address })}
          onMerge={onMergePullRequest}
          onApprove={onApproveReview}
          onRequestChanges={onRequestChanges}
          onReject={onReject}
          onTakeUpRemarks={onTakeUpRemarks}
          onOpenRemarkLink={onOpenRemarkLink}
        />
      }
    />
  );
}

export type VerdictSlotFinishedArgs = {
  job: JobSummary;
  whole: JobWhole | null;
  open: StepDetail;
  render: Render;
  recorded: VerdictReads;
  opensRecords: Opens;
  now: number;
  claimed: Submitted | undefined;
  /** Fleet's own reason the gate could not decide, scoped to this step. */
  undecided?: string;
};

/**
 * The verdict sheet at a finish nothing asked — case 4 of `why-b.md`. No
 * `actions`, so the sheet draws its own dashed record note in their place.
 */
export function verdictSlotFinished({
  job,
  whole,
  open,
  render,
  recorded,
  opensRecords,
  now,
  claimed,
  undecided,
}: VerdictSlotFinishedArgs): ReactNode {
  return (
    <VerdictSheet
      {...verdictOf({
        job,
        whole,
        step: open,
        render,
        diff: recorded.diff,
        opens: opensRecords,
        now,
        claim: claimed,
        undecided,
      })}
    />
  );
}
