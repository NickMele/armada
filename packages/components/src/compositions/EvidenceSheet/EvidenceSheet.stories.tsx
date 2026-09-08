import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleCheck, CircleX, FileDiff } from "lucide-react";
import { expect } from "storybook/test";
import { AssertionSet } from "../AssertionSet/AssertionSet";
import { ConsoleOutput, type ConsoleRow } from "../ConsoleOutput/ConsoleOutput";
import { EvidenceStrip, type EvidenceChip } from "../EvidenceStrip/EvidenceStrip";
import { JudgeCitations } from "../JudgeCitations/JudgeCitations";
import { JudgeInputs } from "../JudgeInputs/JudgeInputs";
import { JudgeRefusal } from "../JudgeRefusal/JudgeRefusal";
import { JudgeVerdicts } from "../JudgeVerdicts/JudgeVerdicts";
import { EvidenceSheet } from "./EvidenceSheet";

/**
 * One viewer, and the stories are the artifacts it has to be able to hold: a
 * check's console output, the assertion set behind the same check, and a
 * panel's judgment with its three renderings.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof EvidenceSheet> = {
  title: "Compositions/Evidence sheet",
  component: EvidenceSheet,
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          height: "var(--palette-max-height)",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof EvidenceSheet>;

const chips: EvidenceChip[] = [
  { id: "assertions", says: "assertion set", kind: "the default view", named: "not_met" },
  { id: "sym", says: "public symbols 41 → 41", kind: "measurement" },
  { id: "bench", says: "bench 1.42 → 1.19µs", kind: "measurement" },
  { id: "diff", says: "−318 +94 · 5 files", kind: "code diff", icon: FileDiff },
];

const transcript: ConsoleRow[] = [
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
  { row: "fold", at: 2011, says: "309 further tests, all ok — 309 lines" },
  { row: "line", at: 2321, text: "test result: ok. 315 passed; 0 failed; finished in 2.41s" },
  { row: "line", at: 2322, text: "at HEAD~1: 316 passed; 0 failed; finished in 2.38s" },
];

/**
 * A check's console output, opened from its row.
 *
 * **The strip is a band on the layer**, so every other artifact this step
 * produced stays one press away — a viewer you have to close to change what
 * you are looking at is a modal wearing a sheet's clothes.
 */
export const AChecksConsoleOutput: Story = {
  args: {
    open: true,
    kind: "Console output",
    name: "check:test_suite — output",
    step: "Implement",
    jobId: "job_733",
    extent: "2,180 lines · 4.1MB",
    views: [
      { id: "output", label: "Output" },
      { id: "assertions", label: "Assertions" },
      { id: "timing", label: "Timing" },
    ],
    view: "output",
    strip: <EvidenceStrip chips={chips} openId="chk-suite" label="also produced" />,
    onOpenFull: () => {},
    children: (
      <ConsoleOutput
        rows={transcript}
        region={{
          says: "lines 1,940–2,322 of 2,180",
          path: ".armada/jobs/733/checks/test_suite.log",
          size: "4.1MB",
          why: "jumped to first difference",
        }}
        tools={{
          find: { term: "trailing", found: "3 in 2 folds" },
          differences: { says: "1 of 1 difference" },
        }}
      />
    ),
  },
};

/**
 * The same check, rendered as its assertions.
 *
 * **A different view, not a different artifact** — which is why it is a tab
 * and the strip is unchanged beneath it. Choosing a different artifact is what
 * the strip is for.
 */
export const TheSameCheckAsAssertions: Story = {
  args: {
    open: true,
    kind: "Test results",
    name: "check:test_suite — assertions",
    step: "Implement",
    jobId: "job_733",
    extent: "315 assertions",
    views: [
      { id: "output", label: "Output" },
      { id: "assertions", label: "Assertions" },
      { id: "timing", label: "Timing" },
    ],
    view: "assertions",
    strip: <EvidenceStrip chips={chips} openId="assertions" label="also produced" />,
    children: (
      <AssertionSet
        rows={[
          {
            says: "An RFC 3339 timestamp with a numeric offset parses to the right instant",
            identifier: "parses_rfc3339_offset",
            named: "passed",
            against: "identical at HEAD and HEAD~1",
          },
          {
            says: "A loose-format date with spaces after it parses instead of erroring",
            identifier: "parses_loose_trailing_whitespace",
            named: "absent",
            against: "passed at HEAD~1 · absent at HEAD",
          },
          { identifier: "rejects_empty_string", named: "passed", against: "identical at HEAD and HEAD~1" },
        ]}
        rest={{ says: "312 further assertions", against: "identical at HEAD and HEAD~1" }}
      />
    ),
  },
};

/**
 * A panel's judgment — the one artifact Armada authors rather than passes
 * through from the repository.
 *
 * **No `Open full page`.** A judgment has a size the sheet can hold whole, and
 * a control that leaves the screen for something that fits is a control that
 * loses the criteria behind the layer for nothing.
 */
export const APanelsJudgment: Story = {
  args: {
    open: true,
    kind: "Judge verdicts",
    name: "The Judge — a panel of 3, independent, on identical inputs",
    step: "Review",
    jobId: "job_733",
    views: [
      { id: "verdicts", label: "Verdicts" },
      { id: "citations", label: "Citations" },
      { id: "inputs", label: "Inputs" },
    ],
    view: "verdicts",
    strip: <EvidenceStrip chips={chips} label="also produced" />,
    children: (
      <>
        <JudgeVerdicts
          judges={["j1", "j2", "j3"]}
          glyphs={{ met: CircleCheck, not_met: CircleX }}
          rows={[
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
            },
            {
              ordinal: 4,
              criterionId: "c4",
              name: "One entry point",
              text: "Exactly one parsing entry point remains.",
              marks: ["met", "met", "met"],
            },
          ]}
        />
        <JudgeRefusal
          heading="Refused — 02 Behaviour unchanged"
          split="2 of 3 judges refused, on the same grounds"
          otherwise="j2 had no objection"
          finding={{
            expected: "Suite red when a loose-format date with trailing spaces is parsed",
            produced: "Suite green, with that case deleted from tests/loose.rs",
            consequence: "A parser regression ships as verified",
          }}
          quoteLead="The assertion they mean, quoted from check:test_suite"
          quoted={`2007  — 1 test present at HEAD~1 is absent here —
2008  test parses_loose_trailing_whitespace ... ok   ran at HEAD~1 only`}
          cited={[
            { id: "chk-suite", says: "the missing assertion", where: "check:test_suite · 2007–2008" },
            { id: "diff-loose", says: "the deleted test file lines", where: "tests/loose.rs · −14" },
          ]}
          silence="j2 had no objection and wrote nothing — a Judge writes only when it refuses."
        />
      </>
    ),
  },
};

/**
 * The judgment's citations. **The densest navigation surface on the screen** —
 * every row points into another artifact, and pressing one changes what the
 * viewer is showing without leaving it.
 */
export const TheJudgmentsCitations: Story = {
  args: {
    open: true,
    kind: "Judge verdicts",
    name: "The Judge — what the panel cited",
    step: "Review",
    jobId: "job_733",
    views: [
      { id: "verdicts", label: "Verdicts" },
      { id: "citations", label: "Citations" },
      { id: "inputs", label: "Inputs" },
    ],
    view: "citations",
    children: (
      <JudgeCitations
        rows={[
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
            id: "diff-mod",
            who: "j2 · 04",
            criterion: "Exactly one parsing entry point remains",
            where: "src/parse/mod.rs diff +94",
            named: "met",
            verdict: "met",
          },
        ]}
      />
    ),
  },
};

/**
 * The judgment's inputs. **The view a log file could never give you** — a
 * panel is only a panel if the judges ran independently on identical inputs,
 * and the digest is the evidence for that.
 */
export const TheJudgmentsInputs: Story = {
  args: {
    open: true,
    kind: "Judge verdicts",
    name: "The Judge — what the panel was shown",
    step: "Review",
    jobId: "job_733",
    views: [
      { id: "verdicts", label: "Verdicts" },
      { id: "citations", label: "Citations" },
      { id: "inputs", label: "Inputs" },
    ],
    view: "inputs",
    children: (
      <JudgeInputs
        identical="All 3 judges received this identical object"
        rows={[
          { name: "scope digest", value: "sha256:9f31c2…a70b" },
          { name: "context_paths", value: "src/parse/mod.rs\nsrc/parse/loose.rs\ntests/loose.rs" },
          { name: "work product", value: "diff, delivered — 5 files, 412 lines" },
          { name: "yardstick", value: "acceptance_criteria[] frozen at dispatch" },
          { name: "size", value: "38.2k of 40k max_context_size" },
        ]}
      />
    ),
  },
};

/**
 * At the floor. **The views drop into the strip band** and the Job leaves the
 * subtitle, for the reason the activity log's filters do: a title, a subtitle
 * and a close in 768px is already the line that breaks.
 */
export const AtTheFloor: Story = {
  args: {
    ...AChecksConsoleOutput.args,
    floor: true,
  } as Story["args"],
};

/**
 * The viewer says which artifact it is holding, and the way back is the strip.
 *
 * **What earns the assertion is that the kind alone is not an answer.** A sheet
 * whose header named only what it was would say `Console output` above four
 * different checks, and a reader three selections in could not tell which one
 * they were reading.
 *
 * **There is no restore control, and that is the design.** It read `Back to the
 * default view`, sat beside `Close`, and could not be told from it. The
 * artifact the page composed with is a chip in the strip like any other, marked
 * as the one it opens on — so pressing it is the way back, and the strip is the
 * only vocabulary for choosing what to look at.
 */
export const ItNamesTheArtifactItIsHolding: Story = {
  args: AChecksConsoleOutput.args,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("check:test_suite — output")).toBeVisible();

    // The close carries its binding, so the accessible name is both words.
    await expect(canvas.getByRole("button", { name: /Close/ })).toBeVisible();
  },
};
