import type { Meta, StoryObj } from "@storybook/react-vite";
import { GitPullRequest, Minus, ShieldCheck } from "lucide-react";
import { VerdictSheet } from "./VerdictSheet";
import { Button } from "../../primitives/Button/Button";
import { CheckRuns } from "../CheckRuns/CheckRuns";
import { ReviewDecision } from "../ReviewDecision/ReviewDecision";
import { ReviewComments } from "../ReviewComments/ReviewComments";

/**
 * The panel job detail shows at the one place a Job stops for a person, and
 * again once it is over with nothing left to ask. One page, read top to
 * bottom — the four states below are the whole of what changes between them.
 *
 * **Approved 2026-09-08, Option B.** `why-b.md`: buildable on today's wire,
 * and it moves between the four scenarios by adding or dropping a labelled
 * block rather than by redrawing the page.
 */
const meta: Meta<typeof VerdictSheet> = {
  title: "Compositions/Verdict sheet",
  component: VerdictSheet,
};
export default meta;

type Story = StoryObj<typeof VerdictSheet>;

const PROVES_IT_ROWS = [
  {
    id: "test",
    says: "The seven cases run and pass",
    identifier: "check:test",
    named: "passed" as const,
    icon: ShieldCheck,
  },
  {
    id: "rule",
    says: "The new rule catches the original bug",
    identifier: "5 unit tests",
    named: "passed" as const,
    icon: ShieldCheck,
  },
];

/**
 * A gate, before any pull request. Approve, request changes or reject — the
 * ordinary three, offered exactly as `Decide` offers them today.
 */
export const AtAGate: Story = {
  args: {
    title: "Declare undeclared test files and add a gate rule to prevent recurrence",
    criteria: [
      "Every test file under src/tests/ is declared and its cases run as part of the suite.",
      "A gate rule fails the next undeclared file.",
    ],
    cameBack:
      "Every test file under a src/tests/ submodule is now declared, the seven cases run and " +
      "pass as part of the workspace suite, and a new gate rule fails on the next undeclared file.",
    provesIt: <CheckRuns rows={PROVES_IT_ROWS} />,
    leftAlone:
      "Two pre-existing lint warnings in proving.rs and clean.rs. capacity.rs was not restructured.",
    figures: [
      { label: "Branch", value: "armada/01K20E8JS4…", mono: true },
      { label: "Files", value: "5", mono: true },
      { label: "Took", value: "29m · ~$4.03", mono: true },
      { label: "Steps", value: "3 of 3 passed", mono: true },
    ],
    note: "The run tree on the left is where each step's own evidence is. This reads the Job.",
    actions: (
      <ReviewDecision
        note=""
        onNote={() => {}}
        onApprove={() => {}}
        onRequestChanges={() => {}}
        onReject={() => {}}
      />
    ),
  },
};

/**
 * A pull request is open. The pull request block sits above the checklist,
 * and `Decide` offers merge as the primary act — the same component, the same
 * `onMerge` prop, only its presence changed.
 */
export const PullRequestOpen: Story = {
  args: {
    ...AtAGate.args,
    pullRequest: (
      <div className="flex flex-col gap-2">
        <p className="text-xs text-fg-muted">
          <GitPullRequest size={12} strokeWidth={2} aria-hidden className="inline" />{" "}
          <span className="mono">#4711</span> · Declare capacity.rs and gate the next undeclared
          test file — open, mergeable, no reviews yet.
        </p>
        <p className="text-2xs text-fg-subtle">
          Read it there — this page says what Armada knows that the pull request's own page does
          not.
        </p>
      </div>
    ),
    note:
      "This repository sets auto_merge: never. Merging here is Fleet acting on your press, and " +
      "it runs the after-merge Checks. Merging on the forge does not.",
    actions: (
      <>
        <ReviewDecision
          note=""
          onNote={() => {}}
          onMerge={() => {}}
          onApprove={() => {}}
          onRequestChanges={() => {}}
          onReject={() => {}}
        />
        <ReviewComments comments={[]} onTakeUp={() => {}} />
      </>
    ),
  },
};

/**
 * A gate on a workflow that never delivers — `design-plan`, `epic`. No pull
 * request row, ever, and approving ends the Job here. What proves it says how
 * little does: no Judge, no Checks, only a file that has to exist.
 */
export const GateWithoutAPullRequest: Story = {
  args: {
    title: "Decide how Fleet should handle a review comment too long for a Drone's brief",
    criteria: ["Propose a rule, and say what it should be measured against."],
    cameBack:
      "A two-page draft. Fleet refuses a merge press whose chosen comments would not fit in " +
      "what the brief leaves free, and names the refusal so Bridge can say which comments to drop.",
    // The real screen's `deliverable` is `phases.tsx`'s `Opening` — a ghost
    // button reading the basename, the whole path on `title`. No icon: that
    // component draws none, and a story inventing one here would show a
    // reader something the real screen never does.
    deliverable: (
      <Button variant="ghost" size="sm" title=".armada/artifacts/draft.md">
        draft.md
      </Button>
    ),
    provesIt: (
      <CheckRuns
        rows={[
          {
            id: "exists",
            says: "The draft exists",
            identifier: "artifact_exists",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            // No `named`: this row reports an absence of verification rather
            // than a Check result, and the dash glyph carries that — never
            // `shield-*`, which means a Check ran, and never `x`, reserved to
            // a system failure.
            id: "nothing",
            says: "Nothing checked what it says",
            identifier: "no Judge · no Checks",
            icon: Minus,
          },
        ]}
      />
    ),
    provesItNote: "Reading the draft is the review. Your answer is the only verdict this step gets.",
    leftAlone: "It proposes no number for the limit. The draft says one should be measured first.",
    figures: [
      { label: "Document", value: "draft.md", mono: true },
      { label: "Took", value: "14m · ~$0.62", mono: true },
      { label: "Steps", value: "1 of 2 passed", mono: true },
      { label: "Pull request", value: "never, for this workflow", mono: true },
    ],
    note: "Approving ends the Job here. Nothing is merged, and nothing waits on a forge.",
    actions: (
      <ReviewDecision
        note=""
        onNote={() => {}}
        onApprove={() => {}}
        onRequestChanges={() => {}}
        onReject={() => {}}
      />
    ),
  },
};

/**
 * Finished, and nothing was ever asked — `code-review`, `prototype`. No
 * action row at all: the dashed note is the only thing where the buttons
 * would be, because this is a record rather than a decision.
 */
export const FinishedNothingAsked: Story = {
  args: {
    title: "Review the change that lets a Drone run Checks against only the files it touched",
    criteria: ["Say whether a narrowed run can pass work that a full run would fail."],
    cameBack:
      "One finding and one note. A narrowed run can miss a Check whose inputs sit outside the " +
      "touched files; the review asks for the full run before a step advances.",
    deliverable: (
      <Button variant="ghost" size="sm" title=".armada/artifacts/assess.md">
        assess.md
      </Button>
    ),
    provesIt: (
      <CheckRuns
        rows={[
          {
            id: "read",
            says: "Its reading notes exist",
            identifier: "read.md · artifact_exists",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            id: "assess",
            says: "Its assessment exists",
            identifier: "assess.md · artifact_exists",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            // The dash glyph: an absence of verification rather than a
            // failed one, so never `x`, which is reserved to a system
            // failure, and never `shield-*`, which means a Check ran.
            id: "nothing",
            says: "Nothing checked what it says",
            identifier: "no Judge · no Checks",
            icon: Minus,
          },
        ]}
      />
    ),
    provesItNote: "Every step advanced on its own. No person was asked, and no gate read the review.",
    leftAlone: "This step's submission drew no boundary around what it did not change.",
    figures: [
      { label: "Took", value: "21m · ~$1.10", mono: true },
      { label: "Steps", value: "3 of 3 advanced", mono: true },
      { label: "Pull request", value: "never, for this workflow", mono: true },
    ],
  },
};
