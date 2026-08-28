import type { Meta, StoryObj } from "@storybook/react-vite";
import { Eye, FileCheck, Folder, GitBranch } from "lucide-react";
import { AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop } from "./AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop";

/**
 * The last of the seven steps in `docs/scope.md` — *when the work is complete
 * he has a set of work he can review*. Until this screen existed every job
 * ended by sending its owner to a terminal.
 *
 * The header is `Job detail header actions`, the same component the other three
 * job renders use. The badge is `awaiting review`, whose glyph is `eye` —
 * reserved to review and to nothing else.
 *
 * **The claims sit above the diff and the decision sits below it.** A
 * submission is a signal and never the source of truth, so the account is read
 * first and the bytes either bear it out or do not; and the reply is on the same
 * page as the diff because reviewing and replying is one loop.
 */
const meta: Meta<typeof AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop> = {
  title: "Screens/A job awaiting review — the diff and the reply are one loop",
  component: AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop,
};
export default meta;

type Story = StoryObj<typeof AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop>;

const heading = {
  status: "awaiting-review",
  statusIcon: Eye,
  statusLabel: "Awaiting review",
  headline: "Honour human_always on a workflow's advance gate",
  jobId: "job_7c22",
  fields: [
    { label: "At", value: "4 of 4", mono: true, suffix: "steps, waiting on you" },
    { label: "Ran", value: "22m 06s", mono: true },
    { label: "Drone", value: "drn_01M4K", mono: true, suffix: "held" },
    { label: "Dispatched by you" },
  ],
};

const brief = {
  criteria: [
    { text: "A workflow declaring human_always loads rather than being refused.", source: "check" },
    { text: "A job on such a step stops at awaiting_review.", source: "check" },
    { text: "All three review acts refuse anywhere else.", source: "judge" },
  ],
};

const claims = {
  entries: [
    {
      step: "Widen the gate",
      provenance: "09:41:02 · code_change · manifest_check: cargo fmt, cargo clippy",
      icon: FileCheck,
      iconLabel: "Evidence",
      claimed: "AdvanceGate carries a HumanAlways variant and gate.rs returns Wait on it.",
      shownBy: "crates/core-model/src/workflow/gate.rs, crates/fleet/src/gate.rs",
      notClaimed: "Nothing loads a workflow declaring it yet — that is the next step.",
    },
    {
      step: "Carry it through config",
      provenance: "10:03:19 · code_change · manifest_check: cargo test",
      icon: FileCheck,
      iconLabel: "Evidence",
      claimed:
        "A workflow declaring advance_gate: human_always loads instead of raising Fault::OutsideM1.",
      shownBy: "cargo test -p armada-config gate_ -- 6 passed",
      notClaimed: "",
    },
  ],
};

const diff = {
  emptyNote: "",
  note: "Read from this job's worktree against the branch it was cut from. Every path is inside the plan this step declared.",
  files: [
    {
      path: "crates/config/src/workflow.rs",
      lines: [
        { kind: "hunk" as const, text: "@@ -441,7 +441,7 @@ fn gate_of(named: &str) -> Result<AdvanceGate, Fault> {" },
        { kind: "context" as const, text: '         "auto" => Ok(AdvanceGate::Auto),' },
        { kind: "removed" as const, text: '-        "human_always" => Err(Fault::OutsideM1("human_always")),' },
        { kind: "added" as const, text: '+        "human_always" => Ok(AdvanceGate::HumanAlways),' },
        { kind: "context" as const, text: "         other => Err(Fault::UnknownGate(other.into()))," },
      ],
    },
    {
      path: "crates/fleet/src/gate.rs",
      lines: [
        { kind: "hunk" as const, text: "@@ -41,4 +41,6 @@ impl Gate {" },
        { kind: "context" as const, text: "         match step.advance_gate() {" },
        { kind: "context" as const, text: "             AdvanceGate::Auto => Advance::Now," },
        { kind: "added" as const, text: "+            AdvanceGate::HumanAlways => Advance::Wait," },
        { kind: "context" as const, text: "         }" },
      ],
    },
  ],
};

const work = {
  rows: [
    {
      icon: Folder,
      iconLabel: "Worktree",
      value: "/repos/armada/.armada/worktrees/job_7c22",
      copyValue: "/repos/armada/.armada/worktrees/job_7c22",
    },
    {
      icon: GitBranch,
      iconLabel: "Branch",
      value: "armada/job_7c22",
      copyValue: "armada/job_7c22",
    },
  ],
  note: "The worktree follows from this job's id and the repository its manifest was read from. The branch is served.",
};

const decision = {
  note: "",
  onNote: () => {},
  onApprove: () => {},
  onRequestChanges: () => {},
  onReject: () => {},
};

/** The screen as it is opened: the account, the bytes, and three answers. */
export const WorkWaitingOnADecision: Story = {
  args: { heading, brief, claims, diff, decision, work },
};

/**
 * A note written, so the reply is live. **The field never moves and never opens
 * a second surface** — it was on the page before anything was typed into it.
 */
export const ChangesBeingWritten: Story = {
  args: {
    heading,
    brief,
    claims,
    diff,
    work,
    decision: {
      ...decision,
      note:
        "The gate arm is right, but nothing refuses a workflow that declares human_always on a " +
        "step with no checks. Add that and a test that loads one.",
    },
  },
};

/**
 * A patch longer than the bound. **The loudest thing on the screen**, because a
 * decision taken on part of a diff is the failure this surface exists to
 * prevent — and the notice names the worktree that the region at the bottom of
 * the page then gives the path to.
 */
export const APatchTooLongToDraw: Story = {
  args: {
    heading,
    brief,
    claims,
    work,
    decision,
    diff: {
      ...diff,
      cut:
        "This is the first 2,000 lines of a 14,318-line patch. The rest is not on screen. " +
        "Read the whole diff in the worktree named under Where the work is before deciding.",
    },
  },
};

/**
 * A drone that changed nothing. The evidence still reads as work done, which is
 * exactly the disagreement this screen exists to make visible — and it is what
 * a `diff_nonempty` check refuses.
 */
export const AClaimWithNothingBehindIt: Story = {
  args: {
    heading,
    brief,
    claims,
    work,
    decision,
    diff: {
      files: [],
      emptyNote:
        "This job's worktree opened and holds no change against the branch it was cut from. " +
        "That is what a diff_nonempty check refuses.",
    },
  },
};

/**
 * The diff has not arrived. Every other region draws, because they are separate
 * reads — a screen that blanked while the expensive one was in flight would be
 * a screen that looks broken every time it is opened.
 */
export const TheDiffNotReadYet: Story = {
  args: {
    heading,
    brief,
    claims,
    work,
    decision,
    diff: undefined,
    diffAbsent: "Reading this job's diff.",
  },
};
