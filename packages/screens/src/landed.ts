// What a finished Job shows: how it was answered, what it left, what it cost
// and what was run against it. The Land board's whole reading. #1542.
//
// Every figure is derived from what the Job carries, retries included — a
// retried group ran its tasks and its Checks again, and a count that ignores
// that contradicts the retry drawn beside it.
//
// The draft half (#1532) fills what the wire cannot: groups, cases, runs and
// the landing rule. Absent, each falls back to its own `…Of(detail)`, so this
// board draws against a real Fleet as well as the mock.

import { CRITERION_VERDICT_JUDGE, STEP_STATE, sized } from "@armada/components";
import type {
  JobDetail as JobWhole,
  JobResources,
  JobSummary,
  ManifestSummary,
  StepDetail,
} from "@armada/protocol";
import { artifactPath, recordsOf, repoOf } from "@armada/protocol";

import type { BoardDraft } from "./boards";
import { costOf, groupOf, summaryOf, GROUPS_NOTE, type LandedCost, type LandedGroup } from "./landed-cost";
import { clock, span } from "./duration";
import {
  caseViewsOf,
  criterionViewsOf,
  groupsTimedBy,
  landingRuleOf,
  taskGroupsOf,
  CASE_RUN_OUTCOME_WORDS,
  CRITERION_NO_VERDICT_WORD,
  type CaseRunView,
  type CaseView,
  type CompleteWhen,
  type CriterionView,
  type LandingRule,
} from "./draft";
import { LANDED } from "./Row";

/** The one Job status this board is drawn for: work that finished. */
const FINISHED = "completed_success";

/**
 * Whether the Land board is what this Job's Overview draws.
 *
 * **Read by *Where things are*, which drops the branch and the worktree where
 * this is true** (owner, 22 Sep 2026): the board carries both, and one
 * derivation drawn twice on one screen is two places to disagree.
 */
export function landBoardDraws(job: JobSummary): boolean {
  return job.status === FINISHED;
}

/** One row of a section — a part of what the Job produced or left behind. */
export type LandedPart = {
  name: string;
  /** Which glyph the row takes, by the registry's own name. */
  mark?: "branch" | "commit" | "pull-request" | "worktree" | "log";
  value?: string;
  meta?: string;
  /** Why there is no value. Said in words, never left as a blank. */
  absent?: string;
  /** Whether a press opens it. Only the pull request has anywhere to go. */
  opens?: "pull_request";
};

export type LandedSection = {
  name: string;
  meta?: string;
  parts: LandedPart[];
  note?: string;
};

/** One run of one case, with who ran it and what became of the run. */
export type LandedRun = {
  spec: string;
  who: string;
  outcome: string;
  status?: string;
  meta?: string;
  when?: string;
};

export type LandedRuns = {
  name: string;
  meta?: string;
  runs: LandedRun[];
  absent?: string;
  note?: string;
};

/** One step of the run, with what it came to. */
export type LandedStep = {
  label: string;
  verdict: string;
  status?: string;
  took?: string;
  meta?: string;
};

export type { LandedCost, LandedGroup };

/** What a landed Job shows, in the order it is read. */
export type LandedRead = {
  verb: string;
  count: string;
  says: string;
  criteria: { text: string; verdict: string; status: string }[];
  completes: string;
  sections: LandedSection[];
  steps: { name: string; meta: string; steps: LandedStep[]; absent: string };
  cost: LandedCost;
  runs: LandedRuns[];
  groups: LandedGroup[];
  groupsSummary: string;
  groupsNote: string;
  groupsAbsent: string;
  followUp: string;
};

export type LandedInput = {
  job: JobSummary;
  whole: JobWhole | null;
  draft: BoardDraft;
  manifest: ManifestSummary | undefined;
  /** What the Job holds on the machine, where anybody has read it. */
  holding: JobResources | null;
};

/**
 * The board, or nothing where this Job is not one that finished.
 *
 * A Job whose detail has not arrived draws nothing either: every figure here
 * is the whole record's, and half a board is worse than none.
 */
export function landedOf({ job, whole, draft, manifest, holding }: LandedInput): LandedRead | undefined {
  if (!landBoardDraws(job) || whole === null) return undefined;
  const rule = draft.landing ?? landingRuleOf();
  const groups = groupsTimedBy(draft.groups ?? taskGroupsOf(whole), draft.record ?? []);
  const cases = draft.cases ?? caseViewsOf(whole);
  const runs = draft.runs ?? runsOfCases(cases);
  const criteria = answered(draft.criteria ?? criterionViewsOf(whole), whole);

  return {
    verb: verbOf(whole),
    count: countOf(criteria),
    says: saysOf(criteria),
    criteria,
    completes: COMPLETES[rule.complete_when],
    sections: [producedOf(whole, rule), leftBehindOf(job, whole, manifest, holding)],
    steps: {
      name: "The run",
      meta: whole.job.workflow_id,
      steps: whole.steps.map(stepOf),
      absent: "Fleet answered with no steps for this Job.",
    },
    cost: costOf(whole, groups),
    runs: runSetsOf(cases, runs),
    groups: groups.map(groupOf),
    groupsSummary: summaryOf(groups),
    groupsNote: GROUPS_NOTE,
    groupsAbsent: "This Job recorded no plan, so it ran as one piece.",
    followUp: "Dispatch a follow-up",
  };
}

/** Where the work got to, in one word. */
function verbOf(whole: JobWhole): string {
  const delivery = whole.delivery;
  if (delivery?.landed !== undefined) {
    const said = LANDED[delivery.landed] ?? delivery.landed;
    return said === "merged" ? "Landed" : `Closed — ${said}`;
  }
  return delivery?.pull_request === undefined ? "Finished" : "Delivered";
}

/** What completing this Job means. The two the boards say, and two more. */
const COMPLETES: Record<CompleteWhen, string> = {
  pr_merged: "Completes when its pull request lands.",
  all_members_landed: "Completes when every member has landed.",
  pr_opened: "Completes when its pull request is opened.",
  delivered: "Completes when its delivering step has delivered.",
};

/** The verdicts a criterion can read, and the hue each takes. */
const MET = { verdict: CRITERION_VERDICT_JUDGE.met?.verb ?? "met", status: "completed-success" };
const REFUSED = {
  verdict: CRITERION_VERDICT_JUDGE.not_met?.verb ?? "not_met",
  status: "completed-failed",
};
/**
 * Nothing wrote a verdict down for it. **Never green, and never "not covered"**
 * — that is a case with no spec, and one sentence for two facts teaches a
 * reader to distrust both (owner, 22 Sep 2026). A Check-verified criterion
 * lands here because nothing on the wire links a Check to what it answers.
 */
const NO_VERDICT = {
  verdict: CRITERION_NO_VERDICT_WORD.verb ?? "no verdict recorded",
  status: CRITERION_NO_VERDICT_WORD.badgeStatus ?? "not-started",
};

/** Each criterion with the last verdict any step returned for it. */
function answered(criteria: CriterionView[], whole: JobWhole): LandedRead["criteria"] {
  return criteria.map((one) => {
    const said = lastVerdictOf(whole, one.criterion_id);
    const read = said === undefined ? NO_VERDICT : said === "met" ? MET : REFUSED;
    return { text: one.text, ...read };
  });
}

// Steps are in run order and `judged` is in attempt order, so the last match
// down both is the one that stands.
function lastVerdictOf(whole: JobWhole, criterionId: string | undefined): string | undefined {
  if (criterionId === undefined) return undefined;
  let said: string | undefined;
  for (const step of whole.steps) {
    for (const judged of step.judged) {
      if (judged.criterion_id === criterionId) said = judged.verdict;
    }
  }
  return said;
}

/**
 * The headline's figure. **It counts what it says it counts**: where every
 * criterion carries a verdict the figure is what was met, and where any does
 * not it is how many were answered at all — a Job that merged reading
 * `0 of 2 met` is a screen claiming a refusal nobody made.
 */
function countOf(criteria: LandedRead["criteria"]): string {
  const ruled = criteria.filter((one) => one.status !== NO_VERDICT.status).length;
  if (ruled < criteria.length) return `${ruled} of ${criteria.length} with a verdict recorded`;
  const met = criteria.filter((one) => one.status === MET.status).length;
  return `${met} of ${criteria.length} met`;
}

function saysOf(criteria: LandedRead["criteria"]): string {
  const missing = criteria.filter((one) => one.status === NO_VERDICT.status).length;
  const refused = criteria.filter((one) => one.status === REFUSED.status).length;
  if (criteria.length === 0) return "Nothing was written down for this Job to be held to.";
  if (missing === 0 && refused === 0) return "Everything this Job was held to was met.";
  if (refused > 0) return refused === 1 ? "One was refused." : `${refused} were refused.`;
  return missing === criteria.length
    ? "No verdict was recorded for any of them."
    : `No verdict was recorded for ${missing} of them.`;
}

/** What reached the repository: the pull request, and the commit under it. */
function producedOf(whole: JobWhole, rule: LandingRule): LandedSection {
  const delivery = whole.delivery;
  const target = rule.target;
  const settled = delivery?.landed === undefined ? undefined : LANDED[delivery.landed];
  const parts: LandedPart[] = [
    {
      name: "Pull request",
      mark: "pull-request",
      ...(delivery?.pull_request === undefined
        ? { absent: "No pull request was opened for this Job." }
        : { value: delivery.pull_request, opens: "pull_request" as const }),
      ...(settled === undefined
        ? target === null
          ? {}
          : { meta: `into ${target}` }
        : { meta: target === null ? settled : `${settled} into ${target}` }),
    },
    {
      name: "Commit",
      mark: "commit",
      ...(delivery?.commit === undefined
        ? { absent: "Nothing was committed over this Job's work." }
        : { value: delivery.commit }),
      ...(delivery?.pushed === undefined ? {} : { meta: delivery.pushed }),
    },
  ];
  // "Delivered" and not "Produced": the Produced panel under this region draws
  // the groups, and one screen cannot have two regions of the same name.
  return { name: "Delivered", parts };
}

/** What is still on the machine once it is over. */
function leftBehindOf(
  job: JobSummary,
  whole: JobWhole,
  manifest: ManifestSummary | undefined,
  holding: JobResources | null,
): LandedSection {
  const branch = whole.branch ?? job.branch;
  const worktree = holding?.worktree;
  const repo = repoOf(manifest);
  // The record is under the Manifest's own records root, so an unread Manifest
  // is a row with nothing to name rather than a path rooted at `/`.
  const records = recordsOf(manifest);
  // `folder` means workspace in the registry and a worktree has no row of its
  // own — `work.tsx` names the same gap on the same row rather than inventing
  // a glyph. `file` is the log row's, which is what a Job's record is.
  const parts: LandedPart[] = [
    {
      name: "Branch",
      mark: "branch",
      ...(branch === undefined ? { absent: "This Job has no worktree, so it has no branch." } : { value: branch }),
    },
    {
      name: "Worktree",
      mark: "worktree",
      ...(worktree === undefined
        ? { absent: "Nothing has read what this Job holds, so its worktree is unknown." }
        : {
            value: worktree.path,
            ...(worktree.bytes === undefined ? {} : { meta: `${sized(worktree.bytes)} on disk` }),
          }),
    },
    {
      name: "Record",
      mark: "log",
      ...(records === null || repo === null
        ? { absent: "No Manifest was read for this Job, so its record has no home to name." }
        : { value: artifactPath("log", repo, records, job.id, job.assigned_drone) }),
    },
  ];
  return {
    name: "Left behind",
    parts,
    note: "Reclaiming the worktree takes the checkout back and leaves the branch and the record.",
  };
}

/** The hue a step's last verdict takes. Anything else stays neutral. */
const VERDICT_STATUS: Record<string, string> = {
  passed: "completed-success",
  failed: "completed-failed",
};

/**
 * One step: what it came to and how long it took. The verdict is the last
 * attempt's, which is what stands — `run.ts` reads the same field.
 */
function stepOf(step: StepDetail): LandedStep {
  const ruled = step.last_verdict ?? step.verdicts[step.verdicts.length - 1];
  const took = span(step.entered_at, step.updated_at);
  const attempts = step.attempts.length;
  const row: LandedStep = {
    label: step.label,
    verdict: ruled?.named ?? STEP_STATE[step.state]?.verb ?? step.state,
  };
  const status = ruled === undefined ? undefined : VERDICT_STATUS[ruled.named];
  if (status !== undefined) row.status = status;
  if (took !== null) row.took = took;
  if (attempts > 1) row.meta = `${attempts} runs`;
  return row;
}

/** Who ran a case, in the words the board says it in. */
const RAN_BY: Record<string, string> = {
  fleet: "Fleet",
  drone: "the Drone",
  person: "you",
  contributor: "a contributor",
};

function whoRan(run: CaseRunView): string {
  const said = RAN_BY[run.actor] ?? run.actor;
  if (run.actor === "person") return run.who ?? said;
  if (run.actor === "contributor") return `${run.who ?? said}, on their machine`;
  return said;
}

/** The two sets: what Fleet ran at handoff, and what a person ran themselves. */
function runSetsOf(cases: CaseView[], runs: CaseRunView[]): LandedRuns[] {
  const byId = new Map(cases.map((one) => [one.id, one]));
  const handoff = runs.filter((run) => run.purpose === "handoff");
  const byHand = runs.filter((run) => run.actor === "person" || run.actor === "contributor");
  return [
    {
      name: "The test set, run again at handoff",
      meta: `${cases.length} cases · before the pull request was offered`,
      runs: handoff.map((run) => rowOf(run, byId)),
      absent: "This Job's set was not run again before it was offered.",
      note: "There is no before-run: the baseline capture is off, so each of these stands alone.",
    },
    {
      name: "Run by hand",
      runs: byHand.map((run) => rowOf(run, byId)),
      absent: "Nobody has run one of these themselves, before or since it landed.",
      note: "A run from another contributor's machine needs a store between Armada instances.",
    },
  ];
}

/** One run's row. A case with no spec reads `not covered`, never as passing. */
function rowOf(run: CaseRunView, cases: Map<string, CaseView>): LandedRun {
  const one = cases.get(run.case);
  const word = CASE_RUN_OUTCOME_WORDS[run.outcome];
  const frames = run.frames === 1 ? "1 frame" : `${run.frames} frames`;
  return {
    spec: one?.spec ?? run.case,
    who: whoRan(run),
    outcome: word.verb ?? run.outcome,
    ...(word.badgeStatus === null ? {} : { status: word.badgeStatus }),
    meta: run.not_run_reason ?? frames,
    when: clock(run.ran_at),
  };
}

/** Every case's last run, where the caller handed no run list of its own. */
function runsOfCases(cases: CaseView[]): CaseRunView[] {
  return cases.flatMap((one) => (one.last_run === undefined ? [] : [one.last_run]));
}
