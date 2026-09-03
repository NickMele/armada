import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect } from "storybook/test";
import type { WorktreeHeld } from "@armada/protocol";

import { HeldWorktree } from "./HeldWorktree";

const meta: Meta<typeof HeldWorktree> = {
  title: "Compositions/Held worktree",
  component: HeldWorktree,
};
export default meta;

type Story = StoryObj<typeof HeldWorktree>;

/** The shape fleet answers with. One place, so a story cannot drift from it. */
function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "01JOBHELD0001",
    job_title: "Port the settings selectors",
    status: "completed_success",
    last_moved_at: "2026-08-30T09:14:00Z",
    path: "/Users/user/armada/.armada/worktrees/01JOBHELD0001",
    branch: "armada/01JOBHELD0001",
    held: [],
    ...over,
  };
}

/**
 * **The other half of the rule, said on the row.** Every safety test passed, so
 * fleet gives this one back on its own sweep — the row carries no control,
 * because there is nothing here for anybody to decide.
 *
 * It is drawn rather than filtered out. A person who came looking for a
 * worktree and does not find it cannot tell "already given back" from "held and
 * not said".
 */
export const NothingIsHoldingIt: Story = {
  args: { held: held() },
};

/**
 * **The reason a person acts on most freely.** There is no force on this seam,
 * so the branch is kept and every commit stays on it — the checkout goes and
 * nothing is lost. The count says how much, the base says what cannot reach it,
 * and the tip is what the work is reachable from afterwards.
 */
export const ABranchTheBaseCannotReach: Story = {
  args: {
    held: held({
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 4,
          tip: "9f1c2ab84d5e6710b3c4d5e6f708192a3b4c5d6e",
        },
      ],
    }),
    selected: false,
    onSelect: () => {},
  },
};

/**
 * **The one reason where reclaiming destroys something.** No branch carries
 * these files, so the checkout is the only copy — which is why they are named
 * one by one rather than counted, and why this is the row a person opens the
 * directory before answering.
 *
 * **And the only row that carries an age.** Work abandoned twenty minutes ago
 * and work abandoned four days ago are answered differently, and without the
 * second number the row asks somebody to guess at the moment guessing costs
 * work. It is said as what it is — when armada last moved the job — because the
 * dirty reading answers names and not times, so nothing knows when a file was
 * written.
 */
export const FilesCommittedNowhere: Story = {
  args: {
    held: held({
      job_title: "Trial the new judge prompt",
      status: "killed",
      held: [
        {
          why: "uncommitted",
          files: ["crates/config/src/judge.rs", "notes/what-the-prompt-missed.md"],
        },
      ],
    }),
    selected: false,
    onSelect: () => {},
    sitting: "4 days",
  },
};

/**
 * **Held, drawn, and not offered.** Fleet refuses to reclaim a job that has not
 * ended, so a checkbox here would be a control whose only outcome is a refusal.
 * The row exists so that an absence is not mistaken for a worktree already
 * gone — issue 385's own table says what a person decides about one: leave it
 * alone.
 */
export const AJobThatHasNotEnded: Story = {
  args: {
    held: held({
      job_title: "Rewrite the dispatch brief",
      status: "running",
      held: [{ why: "not_terminal", status: "running" }],
    }),
  },
};

/**
 * Two tests failed and both are drawn. A list that stopped at the first would
 * tell somebody to commit their changes and then find the job still held.
 *
 * This is also the pair that reads in opposite directions: the branch survives
 * the reclaim and the two loose files do not.
 *
 * **And it is the status with no badge.** `enum-verbs.toml` carries no verb and
 * no glyph for `escalated`, so there is nothing to draw one from — the wire's
 * own spelling renders in mono instead, which is what `ChangedFiles` does with
 * an unworded change kind. A blank there would leave a person unable to tell an
 * escalated job from a finished one. The fix is a registry row, not a component.
 */
export const TwoReasonsOnOneWorktree: Story = {
  args: {
    held: held({
      job_title: "Split the overlap check",
      status: "escalated",
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 1,
          tip: "3ac10de99b7f4e21c0d5a6b7c8d9e0f1a2b3c4d5",
        },
        { why: "uncommitted", files: ["crates/fleet/src/overlap.rs"] },
      ],
    }),
    selected: false,
    onSelect: () => {},
  },
};

/** A job waiting behind this one has not run, so what it wrote may still be needed. */
export const SomethingElseIsWaitingOnIt: Story = {
  args: {
    held: held({
      job_title: "Emit the token stylesheet",
      held: [{ why: "depended_on", by: ["01JOBNEXT0002"] }],
    }),
  },
};

/**
 * A lock is a person saying not yet, and an unreadable checkout is nothing
 * having said at all. **Unanswered and clean must never read alike**, because
 * only one of them can be taken back.
 */
export const AskedAndNotAnswered: Story = {
  args: {
    held: held({
      job_title: "Bisect the flaky delivery test",
      held: [
        { why: "locked", reason: "mid-bisect, do not touch" },
        { why: "unreadable", detail: "fatal: detected dubious ownership" },
      ],
    }),
  },
};

/**
 * **Selected, which is the state the confirmation is built from.** The fill is
 * the shell's own selected-row token and the edge is added on top: what follows
 * a selection here is destructive, and a tint alone is a state a person can miss
 * in the moment they most need to see it.
 */
export const ChosenToBeReclaimed: Story = {
  args: {
    held: held({
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 4,
          tip: "9f1c2ab84d5e6710b3c4d5e6f708192a3b4c5d6e",
        },
      ],
    }),
    selected: true,
    onSelect: () => {},
  },
};

/**
 * **The answer when one half happened and the other did not**, which is the
 * ordinary outcome rather than a partial failure: the checkout is gone and the
 * branch was kept on purpose, because deleting it would have destroyed commits
 * nobody has taken.
 *
 * The `play` is here and not on the variants above because this is the claim a
 * rendering cannot make for itself — a single flag would have to say the reclaim
 * failed or that everything went, and both are untrue.
 */
export const OneHalfHappened: Story = {
  args: {
    held: held({
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 4,
          tip: "9f1c2ab84d5e6710b3c4d5e6f708192a3b4c5d6e",
        },
      ],
    }),
    reclaimed: {
      job_id: "01JOBHELD0001",
      worktree: {
        path: "/Users/user/armada/.armada/worktrees/01JOBHELD0001",
        removed: true,
      },
      branch: {
        branch: "armada/01JOBHELD0001",
        deleted: false,
        tip: "9f1c2ab84d5e6710b3c4d5e6f708192a3b4c5d6e",
        base: "main",
        unmerged_commits: 4,
      },
    },
  },
  play: async ({ canvas }) => {
    const checkout = canvas.getByText("The checkout").nextElementSibling;
    const branch = canvas.getByText("The branch").nextElementSibling;
    await expect(checkout).toHaveTextContent("Gone from disk.");
    await expect(branch).toHaveTextContent("Kept, with 4 commits still on it.");
  },
};

/**
 * The reclaim was sent and the checkout would not go, so the branch could not go
 * either — git refuses to delete a branch a registration still has checked out.
 *
 * **Both halves say what happened to them.** A person reads a lock message and
 * goes and looks; a person reading "reclaim failed" goes and asks.
 */
export const NeitherHalfHappened: Story = {
  args: {
    held: held({ held: [{ why: "locked", reason: "mid-bisect, do not touch" }] }),
    reclaimed: {
      job_id: "01JOBHELD0001",
      worktree: {
        path: "/Users/user/armada/.armada/worktrees/01JOBHELD0001",
        removed: false,
        why: "mid-bisect, do not touch",
      },
      branch: {
        branch: "armada/01JOBHELD0001",
        deleted: false,
        why: "the checkout is still registered",
      },
    },
  },
};

/**
 * **A checkbox is what the whole surface is for**, so the selection is asserted
 * rather than looked at: the row hands back the job id it was drawn from, which
 * is what the caller reclaims by. A row that carried the selection itself would
 * make a bulk act unrepresentable.
 */
export const ChoosingOne: Story = {
  render: () => {
    const [chosen, setChosen] = useState<string | null>(null);
    const row = held({
      held: [{ why: "unmerged", base: "main", commits: 2, tip: "3ac10de99b7f4e21c0d5" }],
    });
    return (
      <ul style={{ margin: 0, padding: 0 }}>
        <HeldWorktree
          held={row}
          selected={chosen === row.job_id}
          onSelect={(jobId, selected) => setChosen(selected ? jobId : null)}
        />
      </ul>
    );
  },
  play: async ({ canvas, userEvent }) => {
    const box = canvas.getByRole("checkbox", { name: "Port the settings selectors" });
    await expect(box).not.toBeChecked();
    // The row does not hold its own selection — it hands back the job id it was
    // drawn from, and the caller decides. A row that kept the state would make
    // the bulk act the surface exists for unrepresentable, and this is the only
    // way to see that the id, and not merely a boolean, went out.
    await userEvent.click(box);
    await expect(box).toBeChecked();
  },
};
