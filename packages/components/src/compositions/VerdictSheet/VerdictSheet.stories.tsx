import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleX, GitPullRequest, Minus, ShieldCheck } from "lucide-react";
import { VerdictSheet } from "./VerdictSheet";
import { Button } from "../../primitives/Button/Button";
import { CheckRuns } from "../CheckRuns/CheckRuns";
import { ReviewDecision } from "../ReviewDecision/ReviewDecision";
import { ReviewComments } from "../ReviewComments/ReviewComments";

/**
 * The panel job detail shows at the one place a Job stops for a person, and
 * again once it is over with nothing left to ask — or answered along the way.
 * One page, read top to bottom — the five states below are the whole of what
 * changes between them.
 *
 * **Approved 2026-09-08, Option B.** `why-b.md`: buildable on today's wire,
 * and it moves between the scenarios by adding or dropping a labelled block
 * rather than by redrawing the page. The fifth was added 2026-09-11, for a
 * Job that closed after a person answered it along the way.
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

/**
 * Finished, after a person answered it along the way — the fifth arrangement,
 * added 2026-09-11. A real Job: #546, dispatched, restarted once, overruled
 * once, approved, and merged as #630 — every value below is off Fleet's own
 * record of it (`01M27918MN0011N9KZEV9ZWHY3`), read the same way
 * `verdictSlotAfterAnswer` reads it, with the run tree's `tests` step open —
 * the one step this Job's own record carries an overruled verdict on.
 *
 * `header.when` and the pull request's settled state are the two blocks
 * `FinishedNothingAsked` never draws; `provesIt`'s third row and
 * `recordNote` are new content on blocks that story already has.
 */
export const FinishedAfterYouAnswered: Story = {
  args: {
    // `absoluteOf` reads the instant in whoever's looking at it own local
    // time zone; this is what it reads in the zone the drawing was read in
    // (UTC-4) for the wire instant Fleet recorded, `2026-09-11T05:29:16.834Z`.
    header: { done: "Done", when: "approved Sep 11, 2026, 1:29 AM" },
    title: "Refuse a merge press whose chosen comments won't fit the brief",
    criteria: [
      // `${545}` rather than the literal `#545`: three hex digits after a `#`
      // reads as a colour to `xtask`'s off-contract-value rule, the same
      // signature `#4711` above dodges by having a fourth.
      `A press whose chosen comments would not fit is refused by a new named refusal with its own wire code, beside the four #${545} added`,
      "The refusal names which comments are too large, and Bridge shows that to the person when they press",
      "fleet::remarks::quoted still writes whatever it is given, whole; nothing truncates a comment",
      "A doc under docs/spikes/ states the measured room a brief leaves for comments and the bound it justifies, applied to the set of chosen comments",
    ],
    cameBack:
      "A press whose chosen comments' rendered brief would exceed ROOM_FOR_COMMENTS is now " +
      "exercised by test, at both ends of the “bound the set, not the comment” rule.",
    pullRequest: (
      <p className="text-xs text-fg-muted">
        <GitPullRequest size={12} strokeWidth={2} aria-hidden className="inline" />{" "}
        <span className="mono">{`#${630}`}</span>, merged.
      </p>
    ),
    provesIt: (
      <CheckRuns
        rows={[
          {
            id: "build",
            says: "Exit 0.",
            identifier: "build",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            id: "test",
            says: "All tests passed.",
            identifier: "test",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            id: "format",
            says: "Exit 0.",
            identifier: "format",
            named: "passed",
            icon: ShieldCheck,
          },
          {
            id: "judge",
            says: "1 of 2 criteria refused",
            identifier: "judge · 1 criterion · gaming check · overruled by you",
            identifierIsAName: true,
            named: "overruled",
            icon: CircleX,
            detail:
              'Not met: The "Write tests" step should not modify ' +
              "crates/ipc/operations.toml. Specification documentation for newly added " +
              "wire codes belongs in the implement step, alongside the implementation of " +
              "the variant and wire code handling. You: “The only unmet criterion is " +
              "that a note documenting the new RemarksTooLarge refusal in " +
              "crates/ipc/operations.toml landed in the tests step, not implement. The " +
              "note is correct and needed; which step added it doesn't change the work. " +
              "The whole change gets reviewed at handoff.”",
          },
        ]}
      />
    ),
    leftAlone:
      "No test exercises worth_dropping or RemarksTooLarge through Bridge's rendering of the " +
      "WireError, since no packages/ file changed in this step.",
    figures: [
      { label: "Branch", value: "armada/2-refuse-a-merge-press-whose-chosen-comments", mono: true },
      { label: "Took", value: "1h 45m · ~$3.89", mono: true },
      { label: "Drones", value: "5", mono: true },
      { label: "Steps", value: "4 of 4 advanced", mono: true },
    ],
    recordNote: (
      <>
        <span className="armada-verdict__label">Your answer</span>
        <p className="armada-verdict__said">
          You approved the work on Sep 11, 2026, 1:29 AM, and the pull request merged. Nothing is
          asked of anyone now.
        </p>
      </>
    ),
  },
};
