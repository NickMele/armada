import type { ReactNode } from "react";
import { CircleCheck, CircleX, FileDiff, ShieldCheck } from "lucide-react";
import { AssertionSet } from "../../compositions/AssertionSet/AssertionSet";
import { CheckRuns, type CheckRun } from "../../compositions/CheckRuns/CheckRuns";
import { ConsoleOutput, type ConsoleRow } from "../../compositions/ConsoleOutput/ConsoleOutput";
import { EvidenceSheet } from "../../compositions/EvidenceSheet/EvidenceSheet";
import { EvidenceStrip, type EvidenceChip } from "../../compositions/EvidenceStrip/EvidenceStrip";
import { JudgeCitations, type JudgeCitation } from "../../compositions/JudgeCitations/JudgeCitations";
import { JudgeInputs } from "../../compositions/JudgeInputs/JudgeInputs";
import { JudgeRefusal } from "../../compositions/JudgeRefusal/JudgeRefusal";
import { JudgeVerdicts } from "../../compositions/JudgeVerdicts/JudgeVerdicts";
import { UnifiedDiff, type DiffFile } from "../../compositions/UnifiedDiff/UnifiedDiff";
import { Prose } from "../../primitives/Prose/Prose";
import type { JudgeVerdictRow } from "../../compositions/JudgeVerdicts/JudgeVerdicts";
import type { PhaseStripProps } from "../../compositions/PhaseStrip/PhaseStrip";
import type { StepChapter } from "../../compositions/StepStory/StepStory";
import type { JobDetailField } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { InsideAJob, type StepPanel } from "./InsideAJobOneArrangementAtEveryState";
import { BRIEF, CHAPTERS, ESCALATED_HEADING, JOB, RUN_STOPPED, WHERE } from "./fixtures";
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
 * **The output opens as a sheet, exactly as the log and the diff do.** 2,180
 * lines is not a longer preview, so the Checks chapter carries a preview and an
 * act that leaves — the shape `StepChapter` already names for a chapter whose
 * content has no end.
 *
 * **What the wire serves, and what it does not.** Checked against
 * `packages/protocol` on 2026-09-08, because a fixture file that says nothing
 * is served is worse than one that says nothing at all: it stops anyone
 * looking.
 *
 * | Drawn here | On the wire |
 * |---|---|
 * | A Check's output | `CheckRun.output_path`. Served, and `phases.tsx` has been opening it for a while |
 * | A refusal's finding | `Judged.expected`, `produced`, `consequence` — the three `agent-copy.md` specifies |
 * | The brief a verdict answers | `Judged.brief_path` |
 * | The hand-back edge | `StepDetail.attempts[].outcome`, read by `phasesOf` |
 * | A panel of judges — the `j1`/`j2`/`j3` columns, per-judge citations, the input digest | **Nothing.** `Judged` is one row per criterion with a single `verdict`; there is no panel on the wire at all |
 * | The assertion set, and any comparison against `HEAD~1` | **Nothing** |
 * | An artifact `kind` | **Nothing.** `output_path` is enough for console output and not for the rest |
 *
 * **The unserved half is one piece of work, not five.** Everything in it treats
 * the Judge as a panel rather than as one verdict, and that is a change to
 * `Judged` before it is a change to any screen.
 *
 * These fixtures are here so the arrangement can be argued before the field
 * exists, which is the order the rest of this screen was built in — and none of
 * them is wired into `JobDetail`, so nothing on a real screen claims to have
 * what the wire has not got.
 */

/** The four Checks the refactor Job ran, and what each came to. */
export const CHECK_RUNS: CheckRun[] = [
  {
    id: "chk-suite",
    says: "All 315 tests passed",
    identifier: "check:test_suite",
    output: "output · 2,180 lines",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "chk-api",
    says: "No exported symbol added or removed",
    identifier: "check:public_api",
    output: "output · 44 lines",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "chk-bench",
    says: "1.19µs per parse — under the 1.5µs cap",
    identifier: "check:bench",
    output: "output · 96 lines",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "judge",
    says: "“Behaviour unchanged” refused by 2 of 3 judges",
    identifier: "The Judge — a panel of 3",
    identifierIsAName: true,
    output: "verdicts",
    result: "refused",
    named: "refused",
    icon: CircleX,
  },
];

/**
 * Everything the step produced. Every chip opens the same viewer.
 *
 * **The suite's output is a chip, and it is the way back.** `EvidenceSheet` has
 * no separate restore control: the artifact a page composed with is a chip like
 * any other, marked as the one it opens on, and pressing it restores. A strip
 * that left the composed artifact out would leave a reader three selections in
 * with no way back to the opinion the page was built with.
 */
export const PRODUCED_CHIPS: EvidenceChip[] = [
  { id: "chk-suite", says: "check:test_suite", kind: "every line the test run printed" },
  { id: "assertions", says: "315 assertions, 1 absent", kind: "each case the suite checked", named: "not_met" },
  { id: "sym", says: "41 → 41", kind: "symbols the crate exports" },
  { id: "bench", says: "1.42 → 1.19µs", kind: "time to parse one date" },
  { id: "diff", says: "−318 +94 · 5 files", kind: "the patch this step wrote", icon: FileDiff },
  // Free text a Drone wrote, beside four values a machine produced. The strip
  // holds both because both are artifacts, which is the claim it makes.
  { id: "migration", says: "MIGRATION.md", kind: "how callers move off the old function" },
];

/** What the list is. Past tense: this step is over. */
export const PRODUCED_LABEL = "Everything this step produced";

/** The region of the run log a selector lands on: the difference, in context. */
export const TRANSCRIPT: ConsoleRow[] = [
  { row: "fold", at: 1940, says: "compiling chrono-lite v0.4.2 (18.4s) — 62 lines" },
  { row: "fold", at: 2003, says: "running 315 tests", open: true },
  { row: "line", at: 2004, text: "test parses_rfc3339_offset ... ok" },
  { row: "line", at: 2005, text: "test parses_rfc2822_obsolete_zone ... ok" },
  { row: "line", at: 2006, text: "test parses_loose_no_separator ... ok" },
  { row: "line", at: 2007, against: "marker", text: "— 1 test present at HEAD~1 is absent here —" },
  {
    row: "line",
    at: 2008,
    against: "absent",
    landed: true,
    text: "test parses_loose_trailing_whitespace ... ok",
    note: "ran and passed at HEAD~1 · never ran here",
  },
  { row: "line", at: 2009, text: "test rejects_empty_string ... ok" },
  { row: "line", at: 2010, text: "test rejects_year_zero ... ok" },
  { row: "fold", at: 2011, says: "309 further tests, all ok — 309 lines" },
  { row: "line", at: 2321, text: "test result: ok. 315 passed; 0 failed; finished in 2.41s" },
  { row: "line", at: 2322, text: "at HEAD~1: 316 passed; 0 failed; finished in 2.38s" },
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
 * **No silence line either.** The grid one row up draws j2's green mark and
 * `otherwise` says *j2 had no objection* — a third statement of it was the
 * surface explaining its own contract to somebody who did not ask.
 */
function behaviourRefused(onOpen?: (citedId: string) => void) {
  return (
  <JudgeRefusal
    split="2 of 3 judges refused, on the same grounds"
    otherwise="j2 had no objection"
    finding={{
      expected: "Suite red when a loose-format date with trailing spaces is parsed",
      produced: "Suite green, with that case deleted from tests/loose.rs",
      consequence: "A parser regression ships as verified",
    }}
    quoteLead="The referenced assertion"
    quoted={`2007  — 1 test present at HEAD~1 is absent here —
2008  test parses_loose_trailing_whitespace ... ok   ran at HEAD~1 only`}
    cited={[
      { id: "chk-suite", says: "the missing assertion", where: "check:test_suite · 2007–2008", onOpen },
      { id: "diff-loose", says: "the deleted test file lines", where: "tests/loose.rs · −14", onOpen },
    ]}
    reading="Both judges found the same fault in the same place, so the criterion is not in doubt — the work is. Send it back with the brief unchanged."
  />
  );
}

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
    quoteLead="The referenced line"
    quoted={`src/lib.rs:9
  pub use crate::parse::loose::parse_loose;   // still public`}
    cited={[{ id: "diff-lib", says: "the surviving re-export", where: "src/lib.rs · +1" }]}
    overlap="j1 and j3 read only src/parse/mod.rs, where there is one entry point. j2 is the only judge that read src/lib.rs."
    reading="Only j2 read the file the refusal rests on, so this is one judge finding something rather than three disagreeing. Read what it cited before deciding whether to overrule it."
  />
);

/**
 * The panel's four criteria, and what it made of each.
 *
 * **A builder rather than a constant, because the citations have a
 * destination.** A refusal points into artifacts, and which viewer they land in
 * is the surface's to say — so the rows are made once per surface with that
 * answer supplied, rather than made once here with nowhere to go.
 */
export function verdictRows(onOpen?: (citedId: string) => void): JudgeVerdictRow[] {
  return [
  {
    ordinal: 1,
    criterionId: "c1",
    name: "API unchanged",
    text: "The public API is byte-identical.",
    measured: "measured — check:public_api",
  },
  {
    ordinal: 2,
    criterionId: "c2",
    name: "Behaviour unchanged",
    text: "Behaviour is unchanged for every input the previous implementation accepted.",
    marks: ["not_met", "met", "not_met"],
    split: "2 of 3 refused",
    refusal: behaviourRefused(onOpen),
  },
  {
    ordinal: 3,
    criterionId: "c3",
    name: "No slowdown",
    text: "The benchmark does not regress beyond 1.5µs per parse.",
    measured: "measured — check:bench",
  },
  {
    ordinal: 4,
    criterionId: "c4",
    name: "One entry point",
    text: "Exactly one parsing entry point remains.",
    marks: ["met", "met", "met"],
  },
  ];
}

/**
 * The same panel with a second criterion refused.
 *
 * **This is the state the arrangement was changed for.** Two refusals in
 * blocks below the grid was two `Refused — 0N` headings a reader had to map
 * back to rows they had scrolled past; in the rows, each one is where its
 * criterion is and the closed one still says `1 of 3 refused`.
 */
export const TWO_REFUSALS: JudgeVerdictRow[] = verdictRows().map((row) =>
  row.criterionId !== "c4"
    ? row
    : {
        ...row,
        marks: ["met", "not_met", "met"],
        split: "1 of 3 refused",
        refusal: ENTRY_POINT_REFUSED,
      },
);

/**
 * Chapters four and five. **Present at every state**, which is the screen's own
 * rule: a step that has not been gated draws its Checks queued rather than
 * drawing nothing, because the shape of what is coming is part of reading a
 * running Job.
 */
export function evidenceChapters(wired?: Wired): StepChapter[] {
  return [
    {
      id: "checks",
      ordinal: 4,
      title: "Checks",
      summary: "3 of 4 passed · 1 refused",
      // The preview is the whole list — four rows is not a reading. What has no
      // end is what is behind each row, and that is what the act opens.
      preview: (
        <CheckRuns
          rows={CHECK_RUNS}
          note="tailed from the run log"
          openId={wired?.openId ?? null}
          onOpen={wired?.onOpen}
        />
      ),
      act: chapterAct("Open the output", "o", wired && (() => wired.onOpen("chk-suite"))),
    },
    {
      id: "verdicts",
      ordinal: 5,
      title: "Verdicts",
      summary: "3 of 4 criteria met · 1 refused",
      // The grid is its own disclosure, so the chapter has no second one: a
      // refusal opens under the row it refuses, and the citations and the inputs
      // are renderings of the artifact that live in the viewer with it.
      preview: <JudgeVerdicts judges={JUDGES} glyphs={GLYPHS} rows={verdictRows(wired?.onOpen)} />,
    },
  ];
}

/**
 * What a surface holding the viewer tells the fixtures below it: which artifact
 * is showing, and where a selector sends one. Absent leaves every selector
 * inert, which is what the stories that are a record of a state want.
 */
export type Wired = { openId: string | null; onOpen: (artifactId: string) => void };

export const EVIDENCE_CHAPTERS: StepChapter[] = evidenceChapters();

/** The same two chapters on a Job where the panel refused two criteria. */
export const TWO_REFUSAL_CHAPTERS: StepChapter[] = [
  {
    id: "checks",
    ordinal: 4,
    title: "Checks",
    summary: "3 of 4 passed · 1 refused",
    // The judge row is the Checks chapter's own, so it has to agree with the
    // grid below it: one judge_check refused, and what it refused is two
    // criteria rather than one.
    preview: (
      <CheckRuns
        note="tailed from the run log"
        rows={CHECK_RUNS.map((run) =>
          run.id !== "judge"
            ? run
            : { ...run, says: "2 of 4 criteria refused by the panel", result: "refused" },
        )}
      />
    ),
    act: chapterAct("Open the output", "o"),
  },
  {
    id: "verdicts",
    ordinal: 5,
    title: "Verdicts",
    summary: "2 of 4 criteria met · 2 refused",
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
    summary: "1 running · 2 queued",
    preview: (
      <CheckRuns
        note="tailed from the run log"
        openId="chk-suite"
        rows={[
          {
            id: "chk-suite",
            says: "1,124 of 1,204 tests run",
            identifier: "check:test_suite",
            output: "output · 1,125 lines",
            result: "running",
            named: "running",
          },
          {
            id: "chk-budget",
            says: "Waiting for the suite to finish",
            identifier: "check:query_budget",
            result: "queued",
            named: "queued",
          },
          {
            id: "judge",
            says: "Waiting for every check to finish",
            identifier: "The Judge — a panel of 3",
    identifierIsAName: true,
            result: "queued",
            named: "queued",
          },
        ]}
      />
    ),
    act: chapterAct("Follow the output", "o"),
  },
  {
    id: "verdicts",
    ordinal: 5,
    title: "Verdicts",
    summary: "not reached",
    preview:
      "No Check has finished, so the panel has not been asked anything. The criteria it will be " +
      "asked are frozen and are in the brief above.",
  },
];

/**
 * The suite's output, as the viewer draws it. **Lifted out of the sheet** so
 * one rendering serves both the drawn-open story and the surface that resolves
 * an artifact id to it.
 */
export const SUITE_OUTPUT = (
  <ConsoleOutput
    rows={TRANSCRIPT}
    region={{
      says: "lines 1,940–2,322 of 2,180",
      path: `.armada/jobs/${JOB}/checks/test_suite.log`,
      size: "4.1MB",
      why: "jumped to first difference",
    }}
    tools={{
      find: { term: "trailing", found: "3 in 2 folds" },
      differences: { says: "1 of 1 difference" },
    }}
  />
);

/** The viewer, holding a Check's console output. */
export const CONSOLE_SHEET = (
  <EvidenceSheet
    open
    kind="Console output"
    name="check:test_suite — output"
    step="Verify"
    jobId={JOB}
    extent="2,180 lines · 4.1MB"
    views={[
      { id: "output", label: "Output" },
      { id: "assertions", label: "Assertions" },
      { id: "timing", label: "Timing" },
    ]}
    view="output"
    strip={<EvidenceStrip chips={PRODUCED_CHIPS} openId="chk-suite" label={PRODUCED_LABEL} />}
    onOpenFull={() => {}}
  >
    {SUITE_OUTPUT}
  </EvidenceSheet>
);

/** The same Check drawn as what it asserted, rather than as what it printed. */
export const SUITE_ASSERTIONS = (
  <AssertionSet
    rows={[
      {
        says: "An RFC 3339 timestamp with a numeric offset parses to the right instant",
        identifier: "parses_rfc3339_offset",
        named: "passed",
        against: "identical at HEAD and HEAD~1",
      },
      {
        says: "An RFC 2822 date using an obsolete zone name (EST, GMT) still parses",
        identifier: "parses_rfc2822_obsolete_zone",
        named: "passed",
        against: "identical at HEAD and HEAD~1",
      },
      {
        says: "A loose-format date with spaces after it parses instead of erroring",
        identifier: "parses_loose_trailing_whitespace",
        named: "absent",
        against: "passed at HEAD~1 · absent at HEAD",
      },
      {
        identifier: "rejects_empty_string",
        named: "passed",
        against: "identical at HEAD and HEAD~1",
      },
    ]}
    rest={{ says: "311 further assertions", against: "identical at HEAD and HEAD~1" }}
  />
);

/** The viewer, holding the same Check rendered as its assertions. */
export const ASSERTIONS_SHEET = (
  <EvidenceSheet
    open
    kind="Test results"
    name="check:test_suite — assertions"
    step="Verify"
    jobId={JOB}
    extent="315 assertions"
    views={[
      { id: "output", label: "Output" },
      { id: "assertions", label: "Assertions" },
      { id: "timing", label: "Timing" },
    ]}
    view="assertions"
    strip={<EvidenceStrip chips={PRODUCED_CHIPS} openId="assertions" label={PRODUCED_LABEL} />}
  >
    {SUITE_ASSERTIONS}
  </EvidenceSheet>
);

/**
 * Every line the panel pointed at, whichever way it ruled. **Rows rather than a
 * drawn list**, so the surface holding the viewer says where each one opens.
 */
export const PANEL_CITATIONS: JudgeCitation[] = [
  {
    id: "chk-suite",
    who: "j1 · 02",
    criterion: "Behaviour is unchanged for every input the old code accepted",
    where: "check:test_suite lines 2007–2008",
    named: "not_met",
    verdict: "refusal",
  },
  {
    id: "chk-suite",
    who: "j3 · 02",
    criterion: "Behaviour is unchanged for every input the old code accepted",
    where: "check:test_suite lines 2007–2008",
    named: "not_met",
    verdict: "refusal",
  },
  {
    id: "diff-loose",
    who: "j3 · 02",
    criterion: "Behaviour is unchanged for every input the old code accepted",
    where: "tests/loose.rs diff −14",
    named: "not_met",
    verdict: "refusal",
  },
  {
    id: "diff-mod",
    who: "j1 · 04",
    criterion: "Exactly one parsing entry point remains",
    where: "src/parse/mod.rs diff +94",
    named: "met",
    verdict: "met",
  },
  {
    id: "diff-mod",
    who: "j2 · 04",
    criterion: "Exactly one parsing entry point remains",
    where: "src/parse/mod.rs diff +94",
    named: "met",
    verdict: "met",
  },
];

/** The viewer, holding the panel's judgment — its citations view. */
export const CITATIONS_SHEET = (
  <EvidenceSheet
    open
    kind="Judge verdicts"
    name="The Judge — what the panel cited"
    step="Verify"
    jobId={JOB}
    views={[
      { id: "verdicts", label: "Verdicts" },
      { id: "citations", label: "Citations" },
      { id: "inputs", label: "Inputs" },
    ]}
    view="citations"
    strip={<EvidenceStrip chips={PRODUCED_CHIPS} label={PRODUCED_LABEL} />}
  >
    <JudgeCitations rows={PANEL_CITATIONS} />
  </EvidenceSheet>
);

/** What Fleet handed every judge on this Job. One object, read back. */
const PANEL_INPUTS = [
  { name: "scope digest", value: "sha256:9f31c2…a70b" },
  { name: "context_paths", value: "src/parse/mod.rs\nsrc/parse/loose.rs\ntests/loose.rs" },
  { name: "work product", value: "diff, delivered — 5 files, 412 lines" },
  { name: "yardstick", value: "acceptance_criteria[] frozen at dispatch" },
  { name: "facts", value: "3 check results, pre-loaded" },
  { name: "size", value: "38.2k of 40k max_context_size" },
];

/** The inputs view, drawn once — the sheet below and the registry both use it. */
export const PANEL_INPUTS_VIEW = (
  <JudgeInputs
    identical="All 3 judges received this identical object"
    rows={PANEL_INPUTS}
    // Every judge's own object behind a segment, so the guarantee is
    // checkable rather than taken on trust. They are identical here, and
    // being able to see that they are is the claim.
    each={["j1", "j2", "j3"].map((judge) => ({ id: judge, judge, rows: PANEL_INPUTS }))}
  />
);

/** The viewer, holding the evidence that the panel was one panel. */
export const INPUTS_SHEET = (
  <EvidenceSheet
    open
    kind="Judge verdicts"
    name="The Judge — what the panel was shown"
    step="Verify"
    jobId={JOB}
    views={[
      { id: "verdicts", label: "Verdicts" },
      { id: "citations", label: "Citations" },
      { id: "inputs", label: "Inputs" },
    ]}
    view="inputs"
    strip={<EvidenceStrip chips={PRODUCED_CHIPS} label={PRODUCED_LABEL} />}
  >
    {PANEL_INPUTS_VIEW}
  </EvidenceSheet>
);

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
        { label: "check:test_suite", mono: true, result: "exit 0 · 2.41s", named: "passed", opens: "chk-suite" },
        { label: "check:public_api", mono: true, result: "exit 0", named: "passed", opens: "chk-api" },
        { label: "check:bench", mono: true, result: "exit 0 · 1.19µs", named: "passed", opens: "chk-bench" },
      ],
    },
    {
      id: "judge",
      label: "Judge \u00b7 1 of 4 refused",
      kind: "judge",
      state: "failed",
      stands: "refused by 2 of 3 judges",
      rows: [
        { label: "Behaviour unchanged", result: "refused", named: "not_met", opens: "judge" },
        { label: "One entry point", result: "met", named: "met", opens: "judge" },
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
      stands: "1 running \u00b7 1 queued",
      rows: [
        { label: "check:test_suite", mono: true, result: "1,124 of 1,204", named: "waiting", opens: "chk-suite" },
        { label: "check:query_budget", mono: true, result: "queued" },
      ],
    },
    { id: "judge", label: "Judge \u00b7 4 criteria", kind: "judge", state: "ahead", stands: "not reached" },
    { id: "you", label: "You", kind: "human", state: "ahead" },
  ],
};

/**
 * The refused Job, drawn once.
 *
 * **Five stories are the same Job at five moments**, which is the screen's own
 * claim — that the arrangement does not move between states — so repeating the
 * whole scaffolding five times says the opposite of what the stories are for
 * and buried the one line each of them actually differs by. What varies is
 * named here and nothing else is.
 */
export function aRefusedJob(over: {
  notice?: StepPanel["notice"];
  chapters?: StepChapter[];
  sheet?: ReactNode;
  after?: ReactNode;
  fields?: JobDetailField[];
  acts?: ReactNode;
  /** Absent leaves the tree's and the strip's selectors inert, as a drawing. */
  onOpenArtifact?: (artifactId: string) => void;
}) {
  return (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...ESCALATED_HEADING, actions: REFUSED_JOB_ACTS }}
        run={RUN_STOPPED}
        runElapsed="52m 09s"
        onOpenArtifact={over.onOpenArtifact ?? (() => {})}
        where={WHERE}
        brief={BRIEF}
        sheet={over.sheet}
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

/** What `check:public_api` printed, and the symbol table it wrote out. */
export const API_OUTPUT: ConsoleRow[] = [
  { row: "fold", at: 1, says: "reading the symbol table at HEAD — 18 lines" },
  { row: "fold", at: 20, says: "reading the symbol table at HEAD~1 — 18 lines" },
  { row: "line", at: 39, text: "comparing 41 exported symbols against 41 at HEAD~1" },
  { row: "line", at: 42, text: "no symbol added, removed or re-signed" },
  { row: "line", at: 44, text: "check:public_api: exit 0" },
];

export const SYMBOLS: ConsoleRow[] = [
  { row: "line", at: 1, text: "pub fn parse(&str) -> Result<Timestamp, ParseError>" },
  { row: "line", at: 2, text: "pub fn parse_rfc3339(&str) -> Result<Timestamp, ParseError>" },
  { row: "line", at: 3, text: "pub fn parse_rfc2822(&str) -> Result<Timestamp, ParseError>" },
  { row: "line", at: 4, text: "pub struct Timestamp" },
  { row: "line", at: 5, text: "pub enum ParseError" },
  { row: "fold", at: 6, says: "36 further symbols, identical at HEAD~1 — 36 lines" },
];

/** What `check:bench` printed, and the number the Judge was handed. */
export const BENCH_OUTPUT: ConsoleRow[] = [
  { row: "fold", at: 1, says: "compiling the benchmark harness (22.8s) — 71 lines" },
  { row: "line", at: 72, text: "Benchmarking parse_rfc3339: collecting 100 samples" },
  { row: "line", at: 88, text: "parse_rfc3339   time:   [1.1874 µs 1.1903 µs 1.1938 µs]" },
  { row: "line", at: 89, text: "                change: [-16.31% -16.12% -15.94%] (p = 0.00 < 0.05)" },
  { row: "line", at: 90, text: "                Performance has improved." },
  { row: "line", at: 96, text: "check:bench: 1.1903 µs, under the 1.5 µs cap — exit 0" },
];

export const BENCH_MEASURED: ConsoleRow[] = [
  { row: "line", at: 1, text: "parse_rfc3339   HEAD~1    1.4197 µs" },
  { row: "line", at: 2, text: "parse_rfc3339   HEAD      1.1903 µs" },
  { row: "line", at: 3, text: "cap                       1.5000 µs" },
  {
    row: "line",
    at: 4,
    against: "marker",
    text: "— 16.1% faster, with 0.31 µs of headroom under the cap —",
  },
];

/**
 * The patch, as the citations point into it.
 *
 * **Three files of the five.** `UnifiedDiff` says what it left undrawn rather
 * than trailing off, and a cut patch is a state the viewer has to be able to
 * hold — a decision taken on a diff that quietly stopped is what this whole
 * surface exists to prevent.
 */
export const PATCH: DiffFile[] = [
  {
    path: "tests/loose.rs",
    lines: [
      { kind: "hunk", text: "@@ -11,14 +11,0 @@" },
      { kind: "removed", text: "-#[test]" },
      { kind: "removed", text: "-fn parses_loose_trailing_whitespace() {" },
      { kind: "removed", text: '-    assert!(parse_loose("2026-09-08  ").is_ok());' },
      { kind: "removed", text: "-}" },
    ],
  },
  {
    path: "src/parse/mod.rs",
    lines: [
      { kind: "hunk", text: "@@ -1,6 +1,9 @@" },
      { kind: "context", text: " mod loose;" },
      { kind: "added", text: "+/// The one entry point. Every accepted format is decided here." },
      { kind: "added", text: "+pub fn parse(text: &str) -> Result<Timestamp, ParseError> {" },
      { kind: "added", text: "+    dispatch(text)" },
      { kind: "added", text: "+}" },
    ],
  },
  {
    path: "src/lib.rs",
    outsidePlan: true,
    lines: [
      { kind: "hunk", text: "@@ -7,2 +7,3 @@" },
      { kind: "context", text: " pub use crate::parse::parse;" },
      { kind: "added", text: "+pub use crate::parse::loose::parse_loose;" },
    ],
  },
];

/** The patch, cut to the file a citation named. */
export const inFile = (path: string) => PATCH.filter((file) => file.path === path);

export const CUT = `2 further files, 96 lines, not drawn. The whole patch is in the worktree at .armada/worktrees/${JOB}.`;

/** A file's diff as the viewer draws it, with the same cut sentence every time. */
export function patchOf(files: DiffFile[], note: ReactNode) {
  return (
    <UnifiedDiff
      files={files}
      emptyNote="This Drone changed nothing."
      cut={files.length === PATCH.length ? CUT : undefined}
      note={note}
    />
  );
}

/** The document the step produced beside its code. Free text, so `Prose` draws it. */
export const MIGRATION = `# Migrating off parse_loose

\`parse_loose\` is no longer the way in. Call \`parse\`, which decides the format
itself and returns the same \`Timestamp\`.

- \`parse_loose(s)\` becomes \`parse(s)\`
- \`parse_rfc3339(s)\` and \`parse_rfc2822(s)\` are unchanged
- Trailing whitespace is **no longer accepted**. Trim before calling.

The last point is a behaviour change and this document is the only place it is
written down.`;

/** The document as the viewer draws it. `Prose` is the renderer for free text. */
export const MIGRATION_DOC = <Prose text={MIGRATION} />;

/**
 * One attempt's run log. **Three attempts kept three logs**, and the tree's
 * claim that attempts 2 and 3 hit `the same failure again` is only checkable
 * because each fact opens its own.
 */
export const ATTEMPT: ConsoleRow[] = [
  { row: "fold", at: 1196, says: "1,195 earlier lines — the build and 314 passing tests" },
  { row: "line", at: 1201, against: "marker", text: "test settings_selectors_memoise ... FAILED" },
  { row: "line", at: 1204, text: "failures:" },
  { row: "line", at: 1205, text: "    settings_selectors_memoise" },
  { row: "line", at: 1206, text: "test result: FAILED. 314 passed; 1 failed; finished in 2.38s" },
  { row: "line", at: 1208, text: "error: test failed — exit 101" },
];
