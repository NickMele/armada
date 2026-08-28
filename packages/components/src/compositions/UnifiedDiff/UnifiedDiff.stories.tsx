import type { Meta, StoryObj } from "@storybook/react-vite";
import { UnifiedDiff } from "./UnifiedDiff";

/**
 * What moved inside the files, as the repository rendered it.
 *
 * The one place the patch bytes are spent. A file list answers *is it doing
 * what I asked* from names alone; this answers *do I take this work*, and
 * nothing short of the bytes answers that.
 */
const meta: Meta<typeof UnifiedDiff> = {
  title: "Compositions/Unified diff",
  component: UnifiedDiff,
};
export default meta;

type Story = StoryObj<typeof UnifiedDiff>;

const READ = "Read from this job's worktree against the branch it was cut from.";

/** The ordinary case: two files, adds and removes, in the order git wrote them. */
export const APatchToDecideOn: Story = {
  args: {
    emptyNote: "",
    note: `${READ} Every path is inside the plan this step declared.`,
    files: [
      {
        path: "crates/fleet/src/gate.rs",
        lines: [
          { kind: "hunk", text: "@@ -41,7 +41,11 @@ impl Gate {" },
          { kind: "context", text: "     fn decide(&self, step: &Step) -> Advance {" },
          { kind: "context", text: "         let carried = step.advance_gate();" },
          { kind: "removed", text: "-        if carried == AdvanceGate::Auto {" },
          { kind: "removed", text: "-            return Advance::Now;" },
          { kind: "added", text: "+        match carried {" },
          { kind: "added", text: "+            AdvanceGate::Auto => Advance::Now," },
          { kind: "added", text: "+            AdvanceGate::HumanAlways => Advance::Wait," },
          { kind: "added", text: "+            AdvanceGate::AutoIfJudgePasses => self.ask_the_judge(step)," },
          { kind: "context", text: "         }" },
          { kind: "context", text: "     }" },
        ],
      },
      {
        path: "crates/fleet/src/reviewing.rs",
        meta: "new file mode 100644",
        lines: [
          { kind: "hunk", text: "@@ -0,0 +1,4 @@" },
          { kind: "added", text: "+//! The three acts a person takes on finished work." },
          { kind: "added", text: "+//!" },
          { kind: "added", text: "+//! All three refuse anywhere but `awaiting_review`." },
          { kind: "added", text: "+" },
        ],
      },
    ],
  },
};

/**
 * A file the step's declared plan does not cover. **A mark, not a judgement** —
 * drift does not fail a step, and the wording is the one the changed-file list
 * already uses so the two cannot disagree about one fact.
 */
export const AFileOutsideTheDeclaredPlan: Story = {
  args: {
    emptyNote: "",
    note: `${READ} 1 of 2 paths are outside the plan this step declared.`,
    files: [
      {
        path: "crates/fleet/src/gate.rs",
        lines: [
          { kind: "hunk", text: "@@ -41,3 +41,3 @@" },
          { kind: "context", text: "     fn decide(&self, step: &Step) -> Advance {" },
          { kind: "added", text: "+        // The gate the workflow carried, honoured." },
          { kind: "removed", text: "-        // TODO: read the gate." },
        ],
      },
      {
        path: "crates/config/settings.toml",
        outsidePlan: true,
        lines: [
          { kind: "hunk", text: "@@ -12,2 +12,3 @@ [judge]" },
          { kind: "context", text: ' model = "haiku"' },
          { kind: "added", text: " timeout_seconds = 90" },
        ],
      },
    ],
  },
};

/**
 * The patch was longer than the bound and the rest is not on screen.
 *
 * **The loudest thing this component draws.** No virtualization library is
 * chosen, so a long patch is cut — and a decision taken on part of a diff is
 * the failure this whole surface exists to prevent, so the notice names the
 * worktree rather than trailing off.
 */
export const APatchTooLongToDraw: Story = {
  args: {
    emptyNote: "",
    note: READ,
    cut:
      "This is the first 2,000 lines of a 14,318-line patch. The rest is not on screen. " +
      "Read the whole diff in the worktree named under Where the work is before deciding.",
    files: [
      {
        path: "crates/store/src/schema.rs",
        lines: [
          { kind: "hunk", text: "@@ -199,4 +199,5 @@ const MIGRATIONS: &[Migration] = &[" },
          { kind: "context", text: "    Migration { id: 11, sql: include_str!(\"sql/011_steps.sql\") }," },
          { kind: "added", text: "+    Migration { id: 12, sql: include_str!(\"sql/012_review.sql\") }," },
          { kind: "context", text: "];" },
        ],
      },
    ],
  },
};

/**
 * A worktree that opened and holds no change. **Ordinary, and never an error**
 * — it is what fails a `diff_nonempty` check, and it is a different sentence
 * from a job that never had a worktree at all.
 */
export const ADroneThatChangedNothing: Story = {
  args: {
    files: [],
    emptyNote:
      "This job's worktree opened and holds no change against the branch it was cut from. " +
      "That is what a diff_nonempty check refuses.",
  },
};

/**
 * A job with no worktree to read. The other silence, and it says so in its own
 * words — absent is not empty, and one sentence for both would tell somebody a
 * drone wrote nothing when what is true is that it never had anywhere to write.
 */
export const AJobWithNoWorktree: Story = {
  args: {
    files: [],
    emptyNote:
      "This job has no worktree, so there is nothing to read. A job at the approval gate has " +
      "not been given one.",
  },
};
