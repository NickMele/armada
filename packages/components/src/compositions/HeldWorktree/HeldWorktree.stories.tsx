import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect } from "storybook/test";
import type { WorktreeHeld } from "@armada/protocol";

import { HeldWorktree } from "./HeldWorktree";
import type { RowChoice } from "./HeldWorktree";

const meta: Meta<typeof HeldWorktree> = {
  title: "Compositions/Held worktree",
  component: HeldWorktree,
};
export default meta;

type Story = StoryObj<typeof HeldWorktree>;

const NO_CHOICE: RowChoice = { removeCheckout: false, deleteBranch: false, forget: false };

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
    on_disk: true,
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
 * **The reason a person acts on most freely.** Removing the checkout is still
 * never a force — the branch is offered on its own line, its own checkbox,
 * unchecked until somebody chooses it.
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
    choice: NO_CHOICE,
    offered: { removeCheckout: true, deleteBranch: true, forget: false },
    onChoose: () => {},
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
    choice: NO_CHOICE,
    offered: { removeCheckout: true, deleteBranch: false, forget: false },
    onChoose: () => {},
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
 * **And it is the status this surface exists to have exposed.** `escalated`
 * carried no verb and no glyph, so the badge slot drew empty here and the row
 * fell back to the wire spelling in mono. It reads `needs you` behind
 * `megaphone` since #400 — this list is served a job's status and no escalation
 * reason, so a status that renders only its reason had nothing to draw.
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
    choice: NO_CHOICE,
    offered: { removeCheckout: true, deleteBranch: true, forget: false },
    onChoose: () => {},
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
 * **The checkout is already gone, so removing it is not offered.** `on_disk`
 * is the fact this reads — a terminal Job whose worktree fleet already took
 * back, but whose branch still holds commits the base cannot reach, offers only
 * the branch and the record. This is the row #931's bug left stuck forever.
 */
export const TheCheckoutIsAlreadyGone: Story = {
  args: {
    held: held({
      job_title: "Rework the retry ceiling",
      on_disk: false,
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 2,
          tip: "5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e",
        },
      ],
    }),
    choice: NO_CHOICE,
    offered: { removeCheckout: false, deleteBranch: true, forget: false },
    onChoose: () => {},
  },
};

/**
 * **Forget is offered only once the other two read as settled.** The checkout
 * is already gone and the branch is chosen for deletion in this same act — the
 * moment both are true, the third checkbox appears rather than being drawn
 * disabled: a control that cannot yet be pressed is not on the row until it can.
 */
export const ForgetJoinsOnceTheOtherTwoAreSettled: Story = {
  args: {
    held: held({
      job_title: "Rework the retry ceiling",
      on_disk: false,
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 2,
          tip: "5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e",
        },
      ],
    }),
    choice: { removeCheckout: false, deleteBranch: true, forget: false },
    offered: { removeCheckout: false, deleteBranch: true, forget: true },
    onChoose: () => {},
  },
};

/**
 * **The answer when a checkout is given back and its branch survives**, which
 * is the ordinary outcome rather than a partial failure: the checkout is gone
 * and the branch was kept on purpose, because deleting it would have destroyed
 * commits nobody has taken.
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
 * **The explicit delete has its own receipt.** `reclaimed.branch` says what the
 * safe reclaim did to the branch, ordinarily nothing; `branchDeleted` is the
 * forcing act's own answer, and the two never both apply to one commit.
 */
export const TheBranchWasDeletedOnPurpose: Story = {
  args: {
    held: held({
      on_disk: false,
      held: [
        {
          why: "unmerged",
          base: "main",
          commits: 2,
          tip: "5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e",
        },
      ],
    }),
    branchDeleted: {
      job_id: "01JOBHELD0001",
      branch: "armada/01JOBHELD0001",
      tip: "5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e",
    },
  },
  play: async ({ canvas }) => {
    const branch = canvas.getByText("The branch").nextElementSibling;
    await expect(branch).toHaveTextContent("Deleted, at 5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e.");
  },
};

/**
 * **Three checkboxes is what the whole surface is for**, so choosing one is
 * asserted rather than looked at: the row hands back the job id it was drawn
 * from and the whole `RowChoice`, which is what the caller acts on. A row that
 * carried its own choice would make the confirmation this act builds
 * unrepresentable.
 */
export const ChoosingOne: Story = {
  render: () => {
    const [choice, setChoice] = useState<RowChoice>(NO_CHOICE);
    const row = held({
      held: [{ why: "unmerged", base: "main", commits: 2, tip: "3ac10de99b7f4e21c0d5" }],
    });
    return (
      <ul style={{ margin: 0, padding: 0 }}>
        <HeldWorktree
          held={row}
          choice={choice}
          offered={{ removeCheckout: true, deleteBranch: true, forget: false }}
          onChoose={(_jobId, next) => setChoice(next)}
        />
      </ul>
    );
  },
  play: async ({ canvas, userEvent }) => {
    const box = canvas.getByRole("checkbox", { name: /^Remove the checkout/ });
    await expect(box).not.toBeChecked();
    // The row does not hold its own choice — it hands back the job id it was
    // drawn from and the whole `RowChoice`, and the caller decides. A row that
    // kept the state would make the confirmation the surface exists to build
    // unrepresentable, and this is the only way to see that the whole choice,
    // and not merely a boolean, went out.
    await userEvent.click(box);
    await expect(box).toBeChecked();
  },
};
