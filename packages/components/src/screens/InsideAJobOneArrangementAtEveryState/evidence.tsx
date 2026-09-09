import type { ReactNode } from "react";
import { CircleCheck, CircleX, ShieldCheck } from "lucide-react";
import { CheckRuns, type CheckRun } from "../../compositions/CheckRuns/CheckRuns";
import { JudgeRefusal } from "../../compositions/JudgeRefusal/JudgeRefusal";
import { JudgeVerdicts } from "../../compositions/JudgeVerdicts/JudgeVerdicts";
import type { JudgeVerdictRow } from "../../compositions/JudgeVerdicts/JudgeVerdicts";
import type { PhaseStripProps } from "../../compositions/PhaseStrip/PhaseStrip";
import type { StepChapter } from "../../compositions/StepStory/StepStory";
import type { JobDetailField } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { InsideAJob, type StepPanel } from "./InsideAJobOneArrangementAtEveryState";
import { BRIEF, CHAPTERS, ESCALATED_HEADING, RUN_STOPPED, WHERE } from "./fixtures";
import { Button } from "../../primitives/Button/Button";
import { chapterAct } from "./fixtures";

/**
 * The evidence half of a step's story — the Checks and the panel that read
 * them, as chapters four and five.
 *
 * **They are chapters and not a new region.** The story is the order things
 * happened: the Drone was instructed, it worked, it produced, the Checks ran,
 * the panel read them. A Checks region drawn anywhere else on the screen would
 * be the sixth place a reader has to learn.
 *
 * **The real screen draws these, and it is the source of truth.**
 * `screens/checks.tsx` and `screens/verdicts.tsx` build both chapters from
 * `StepDetail`, `chaptersOf` returns them, and `screens/evidence.test.tsx`
 * mounts that builder's output. What is here is the same arrangement with
 * fixture content, kept so the drawing can be read at a glance — and held to
 * the same facts, because a Storybook arrangement the app cannot draw is what
 * hid the missing chapters for a week in the first place.
 *
 * **Half of this file was that arrangement, and it has been deleted.** An
 * artifact viewer, an evidence strip, an assertion set, a console renderer, the
 * panel's citations and the digest of what it was handed were all drawn here
 * against invented data, and nothing on the wire serves any of them — every one
 * is a record of what a judge or a Check *read* rather than what it answered.
 * The six components went with the fixtures; the argument for them is in the
 * pull request that introduced them, and the code is in git history.
 *
 * **What the wire serves, and what it does not.** Checked against
 * `packages/protocol` on 2026-09-09.
 *
 * | Drawn here | On the wire |
 * |---|---|
 * | Each declared Check and what it came to | `StepDetail.checks`, `StepDetail.check_runs` |
 * | A Check's output, opened | `CheckRun.output_path` |
 * | The `j1`/`j2`/`j3` columns and the split | `Judged.member`, since protocol 7.7 |
 * | A refusal's finding | `Judged.expected`, `produced`, `consequence` |
 * | The brief a verdict answers | `Judged.brief_path` |
 * | The criterion's own words and its position | `JobDetail.acceptance_criteria` |
 * | The hand-back edge | `StepDetail.attempts[].outcome` |
 */

/** The four Checks the refactor Job ran, and what each came to. */
export const CHECK_RUNS: CheckRun[] = [
  {
    id: "chk-suite",
    says: "Passed",
    identifier: "check:test_suite",
    // The basename of `CheckRun.output_path`, which is what the wire carries
    // and what the real screen draws. It said `output · 2,180 lines` until
    // 2026-09-09; nothing serves a line count, so nothing showed one.
    output: "test_suite.log",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "chk-api",
    says: "Passed",
    identifier: "check:public_api",
    output: "public_api.log",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "chk-bench",
    says: "Passed",
    identifier: "check:bench",
    output: "bench.log",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "judge",
    says: "3 of 4 criteria refused",
    identifier: "judge · 4 criteria · panel of 3",
    identifierIsAName: true,
    named: "refused",
    icon: CircleX,
  },
];

/** The panel, and the marks it rules with. One spelling wherever a grid is drawn. */
export const JUDGES = ["j1", "j2", "j3"];
export const GLYPHS = { met: CircleCheck, not_met: CircleX };

/**
 * The refusal of criterion 02, drawn inside criterion 02's own row.
 *
 * **No heading and no acts.** The row above it names the criterion, so a
 * `Refused — 02 Behaviour unchanged` line inside it is the screen saying one
 * thing twice; and the acts belong to the step's decision, where one set
 * serves however many criteria were refused.
 *
 * **The finding and the brief, and nothing else.** It carried the lines the
 * judges quoted, the citation sets that said where they differed, and
 * selectors into a viewer — `Judged` records none of those, so none of them is
 * drawn. What is left is the three fields a refusal owes and the path to the
 * brief it answers, which is what the app draws.
 */
const BEHAVIOUR_REFUSED = (
  <JudgeRefusal
    split="2 of 3 judges refused, on the same grounds"
    otherwise="j2 had no objection"
    finding={{
      expected: "Suite red when a loose-format date with trailing spaces is parsed",
      produced: "Suite green, with that case deleted from tests/loose.rs",
      consequence: "A parser regression ships as verified",
    }}
    citedLabel="What this verdict answers"
    cited={[{ id: "brief-02", says: "The whole brief the Judge was given", where: "c2.md" }]}
  />
);

/** A second refusal, on the same panel — the state one refusal cannot show. */
const ENTRY_POINT_REFUSED = (
  <JudgeRefusal
    split="1 of 3 judges refused"
    otherwise="j1 and j3 had no objection"
    finding={{
      expected: "parse_loose unreachable from outside the crate after the refactor",
      produced: "parse_loose still exported by the re-export at src/lib.rs:9",
      consequence: "Callers keep the second entry point the refactor was meant to remove",
    }}
    citedLabel="What this verdict answers"
    cited={[{ id: "brief-04", says: "The whole brief the Judge was given", where: "c4.md" }]}
  />
);

/**
 * The panel's four criteria, and what it made of each.
 *
 * **Every row carries marks, because every row is a criterion the panel
 * answered.** The `measured` band — a criterion a Check settled, drawn as one
 * span across the judge columns — is gone: nothing on the wire joins a
 * criterion to the Check that settled it, so the app cannot draw one, and a
 * drawing that could was the whole hazard this file is being held to.
 */
export const VERDICT_ROWS: JudgeVerdictRow[] = [
  {
    ordinal: 1,
    criterionId: "c1",
    name: "The public API is byte-identical.",
    marks: ["met", "met", "met"],
  },
  {
    ordinal: 2,
    criterionId: "c2",
    name: "Behaviour is unchanged for every input the previous implementation accepted.",
    marks: ["not_met", "met", "not_met"],
    split: "refused by 2 of 3",
    refusal: BEHAVIOUR_REFUSED,
  },
  {
    ordinal: 3,
    criterionId: "c3",
    name: "The benchmark does not regress beyond 1.5µs per parse.",
    marks: ["met", "met", "met"],
  },
  {
    ordinal: 4,
    criterionId: "c4",
    name: "Exactly one parsing entry point remains.",
    marks: ["met", "met", "met"],
  },
];

/**
 * The same panel with a second criterion refused.
 *
 * **This is the state the arrangement was changed for.** Two refusals in
 * blocks below the grid was two `Refused — 0N` headings a reader had to map
 * back to rows they had scrolled past; in the rows, each one is where its
 * criterion is and the closed one still says `refused by 1 of 3`.
 */
export const TWO_REFUSALS: JudgeVerdictRow[] = VERDICT_ROWS.map((row) =>
  row.criterionId !== "c4"
    ? row
    : {
        ...row,
        marks: ["met", "not_met", "met"],
        split: "refused by 1 of 3",
        refusal: ENTRY_POINT_REFUSED,
      },
);

/**
 * Chapters four and five. **Drawn only where the step has them** — a step that
 * declares no Check draws no Checks chapter and one that asks no Judge draws no
 * Verdicts chapter, because an empty labelled region reads as a value that
 * failed to load. That rule is the app's, in `screens/evidence.tsx`; these
 * fixtures are one step that has both.
 */
export const EVIDENCE_CHAPTERS: StepChapter[] = [
  {
    id: "checks",
    ordinal: 4,
    title: "Checks",
    summary: "3 of 3 passed",
    // The preview is the whole list — four rows is not a reading. What has no
    // end is what is behind each row, and that is what the act opens.
    preview: <CheckRuns rows={CHECK_RUNS} openSaid="Click to open this output in your editor" />,
    act: chapterAct("Open the output", "o"),
  },
  {
    id: "verdicts",
    ordinal: 5,
    title: "Verdicts",
    summary: "1 of 4 criteria refused",
    // The grid is its own disclosure, so the chapter has no second one: a
    // refusal opens under the row it refuses.
    preview: <JudgeVerdicts judges={JUDGES} glyphs={GLYPHS} rows={VERDICT_ROWS} />,
  },
];

/** The same two chapters on a Job where the panel refused two criteria. */
export const TWO_REFUSAL_CHAPTERS: StepChapter[] = [
  {
    id: "checks",
    ordinal: 4,
    title: "Checks",
    summary: "3 of 3 passed",
    // The judge row is the Checks chapter's own, so it has to agree with the
    // grid below it: what the panel refused is two criteria rather than one.
    preview: (
      <CheckRuns
        openSaid="Click to open this output in your editor"
        rows={CHECK_RUNS.map((run) =>
          run.id !== "judge" ? run : { ...run, says: "2 of 4 criteria refused" },
        )}
      />
    ),
    act: chapterAct("Open the output", "o"),
  },
  {
    id: "verdicts",
    ordinal: 5,
    title: "Verdicts",
    summary: "2 of 4 criteria refused",
    preview: <JudgeVerdicts judges={JUDGES} glyphs={GLYPHS} rows={TWO_REFUSALS} />,
  },
];

/**
 * The decision on a refused step. **One set, however many criteria were
 * refused** — a Job is killed once, and the acts sat inside the refusal block
 * until 2026-09-08, which on a Job with two refusals offered to kill it twice.
 *
 * The note names no criterion for the same reason: it is true of the step.
 */
/** The acts that end or replace the Job, on the header of a refused one. */
const REFUSED_JOB_ACTS = <Button variant="ghost">Kill</Button>;

export const REFUSED_DECISION = (
  <div className="armada-screen__actions">
    <Button variant="primary">Kill &amp; redispatch</Button>
    <Button variant="secondary">Overrule the verdict</Button>
    <Button variant="ghost">Kill</Button>
  </div>
);

/** The same two chapters on a step nothing has gated yet. */
export const QUEUED_EVIDENCE_CHAPTERS: StepChapter[] = [
  {
    id: "checks",
    ordinal: 4,
    title: "Checks",
    summary: "1 of 2 passed",
    preview: (
      <CheckRuns
        openSaid="Click to open this output in your editor"
        rows={[
          {
            id: "chk-suite",
            says: "Passed",
            identifier: "check:test_suite",
            output: "test_suite.log",
            result: "exit 0",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            id: "chk-budget",
            says: "Nothing has run this Check yet.",
            identifier: "check:query_budget",
            named: "queued",
          },
          {
            id: "judge",
            says: "Waiting for every Check to finish.",
            identifier: "judge · 4 criteria · panel of 3",
            identifierIsAName: true,
            named: "queued",
          },
        ]}
      />
    ),
    act: chapterAct("Open the output", "o"),
  },
  {
    id: "verdicts",
    ordinal: 5,
    title: "Verdicts",
    summary: "not reached",
    // A sentence rather than an empty grid, which is the app's own copy: what
    // is coming is knowable, because the criteria were frozen at dispatch.
    preview:
      "The panel has not been asked anything on this step yet. The 4 criteria it will be " +
      "asked were frozen when the Job was dispatched, and are in the brief above.",
  },
];

/**
 * Where the refused step stands — every Check cleared, the panel refused.
 *
 * **The Judge tier is the only failed one, and the Checks tier beside it reads
 * `3 of 3 passed` at the same time.** That pair is the whole state: nothing
 * mechanical is wrong, and the work is still not what was asked for.
 */
export const REFUSED_PHASES: PhaseStripProps = {
  note: "Every Check passed. The panel refused criterion 02, which is what stopped the step — a refusal escalates rather than failing the Job.",
  stages: [
    { id: "instructed", label: "Instructed", state: "cleared" },
    { id: "working", label: "Working", state: "cleared" },
    { id: "submitted", label: "Submitted", state: "cleared" },
    {
      id: "checks",
      label: "test, api, bench",
      kind: "checks",
      state: "cleared",
      stands: "3 of 3 passed",
      rows: [
        { label: "check:test_suite", mono: true, result: "exit 0 · 2.41s", named: "passed" },
        { label: "check:public_api", mono: true, result: "exit 0", named: "passed" },
        { label: "check:bench", mono: true, result: "exit 0 · 1.19µs", named: "passed" },
      ],
    },
    {
      id: "judge",
      label: "Judge · 1 of 4 refused",
      kind: "judge",
      state: "failed",
      stands: "refused by 2 of 3",
      rows: [
        { label: "Behaviour unchanged", result: "refused by 2 of 3", named: "not_met" },
        { label: "One entry point", result: "no objection · 3 judges", named: "met" },
      ],
    },
    { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting on you" },
  ],
};

/** Where the running step stands — one Check in flight, the panel not reached. */
export const RUNNING_PHASES: PhaseStripProps = {
  note: "The suite is still running. Nothing behind it has been asked anything yet.",
  stages: [
    { id: "instructed", label: "Instructed", state: "cleared" },
    { id: "working", label: "Working", state: "current" },
    { id: "submitted", label: "Submitted", state: "cleared" },
    {
      id: "checks",
      label: "test, query budget",
      kind: "checks",
      state: "current",
      stands: "1 of 2 passed",
      rows: [
        { label: "check:test_suite", mono: true, result: "exit 0", named: "passed" },
        { label: "check:query_budget", mono: true, result: "not reached" },
      ],
    },
    { id: "judge", label: "Judge · 4 criteria", kind: "judge", state: "ahead", stands: "not reached" },
    { id: "you", label: "You", kind: "human", state: "ahead" },
  ],
};

/**
 * The refused Job, drawn once.
 *
 * **Three stories are the same Job at three moments**, which is the screen's
 * own claim — that the arrangement does not move between states — so repeating
 * the whole scaffolding each time says the opposite of what the stories are for
 * and buries the one line each of them actually differs by. What varies is
 * named here and nothing else is.
 */
export function aRefusedJob(over: {
  notice?: StepPanel["notice"];
  chapters?: StepChapter[];
  after?: ReactNode;
  fields?: JobDetailField[];
  acts?: ReactNode;
}) {
  return (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...ESCALATED_HEADING, actions: REFUSED_JOB_ACTS }}
        run={RUN_STOPPED}
        runElapsed="52m 09s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Verify",
          fields: over.fields ?? [
            { label: "Took", value: "8m 41s", mono: true },
            { label: "Attempt", value: "3 of 3", mono: true },
          ],
          acts: over.acts,
          notice: over.notice,
          phases: REFUSED_PHASES,
          chapters: over.chapters ?? [...CHAPTERS, ...EVIDENCE_CHAPTERS],
          after: over.after,
        }}
      />
    </div>
  );
}
