import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Eye } from "lucide-react";
import { expect, fn, userEvent, within } from "storybook/test";

import { JobMembers } from "./JobMembers";
import type { JobMemberRow } from "./JobMembers";

/**
 * A Job whose members are Jobs — several pull requests landing in a set order.
 *
 * The screen never names a shape (#1530): it says what it is, and the word for
 * a Job inside it is "member".
 */
const meta: Meta<typeof JobMembers> = {
  title: "Compositions/Job members",
  component: JobMembers,
};
export default meta;

type Story = StoryObj<typeof JobMembers>;

const COMPLETE = "Done when every member has landed. One of three pull requests is in.";

const QUESTION = {
  question: "Does every selector answer for a store nothing has written to yet?",
  expected: "A case for the empty store beside each selector",
  produced: "Two of the six selectors are exercised only against a filled store",
  consequence: "A first launch reads undefined through those two, before anything is saved",
  moves: "Agreeing stops this pull request, and the one stacked on it waits where it is.",
};

/** The first member: its pull request merged, so nothing is before or behind it. */
const LANDED: JobMemberRow = {
  id: "a",
  ordinal: 1,
  title: "Give the store one shape",
  state: { as: "badge", status: "completed-success", icon: Check, label: "done" },
  landed: true,
  targets: "main",
  pullRequest: "https://git.example/armada/pull/1591",
  pullRequestLabel: "1591",
  branch: "armada/22-give-the-store-one-shape",
  scope: ["packages/settings/src/store.ts"],
  tasks: "4 of 4",
};

const AWAITING: JobMemberRow = {
  id: "b",
  ordinal: 2,
  title: "Read the store through selectors",
  state: { as: "badge", status: "awaiting-review", icon: Eye, label: "awaiting review" },
  link: "published",
  targets: "main",
  pullRequest: "https://git.example/armada/pull/1598",
  pullRequestLabel: "1598",
  branch: "armada/23-read-the-store-through-selectors",
  scope: ["packages/settings/src/read.ts", "apps/desktop/src/renderer/"],
  tasks: "3 of 3",
};

const STACKED: JobMemberRow = {
  id: "c",
  ordinal: 3,
  title: "Drop the store singleton",
  state: { as: "badge", status: "running", icon: CircleDot, label: "running" },
  link: "stacked",
  targets: "armada/23-read-the-store-through-selectors",
  branch: "armada/24-drop-the-store-singleton",
  scope: ["packages/settings/src/", "crates/config/src/"],
  tasks: "1 of 5",
};

/** Three pull requests: one merged, one waiting on a person, one stacked on it. */
export const ThreeLandingInOrder: Story = {
  args: { completeWhen: COMPLETE, members: [LANDED, AWAITING, STACKED] },
};

/** The third parked as a draft instead: it targets main and waits for the second. */
export const TheThirdParked: Story = {
  args: {
    completeWhen: COMPLETE,
    members: [
      LANDED,
      AWAITING,
      { ...STACKED, link: "merged", targets: "main", pullRequestLabel: "1601", pullRequest: "https://git.example/armada/pull/1601" },
    ],
  },
};

/** A member holding a Judge refusal, answered where it is read. */
export const AMemberAsksSomething: Story = {
  args: {
    completeWhen: COMPLETE,
    members: [LANDED, { ...AWAITING, question: { ...QUESTION, onAnswer: fn() } }, STACKED],
  },
};

/** One member dropped: its pull request is closed and its branch is kept. */
export const OneDropped: Story = {
  args: {
    completeWhen: "Done when every member has landed. One of two pull requests is in.",
    members: [
      LANDED,
      { ...AWAITING, dropped: "The selectors moved into the third landing.", landed: false },
      { ...STACKED, ordinal: 3, link: "stacked", targets: "armada/22-give-the-store-one-shape" },
    ],
  },
};

/** Nothing has a worktree yet: every part says why rather than going blank. */
export const NothingServedYet: Story = {
  args: {
    completeWhen: "Done when every member has landed. No pull request is in.",
    members: [
      { id: "a", ordinal: 1, title: "Give the store one shape", state: LANDED.state },
      {
        id: "b",
        ordinal: 2,
        title: "Read the store through selectors",
        state: AWAITING.state,
        link: "merged",
      },
    ],
  },
};

/** A status this build's registry has no verb for: the wire spelling, never a blank. */
export const AStateTheRegistryCannotDraw: Story = {
  args: {
    completeWhen: COMPLETE,
    members: [
      LANDED,
      {
        id: "b",
        ordinal: 2,
        title: "Read the store through selectors",
        state: { as: "text", wire: "reconciling", missing: "No verb in the registry for reconciling" },
        link: "merged",
      },
    ],
  },
};

/** No Job lands under this one, which is what an ordinary Job looks like here. */
export const NoMembers: Story = {
  args: { completeWhen: COMPLETE, members: [] },
};

/**
 * A drop asks for a reason and refuses an empty one.
 *
 * **The rule this asserts is what does not happen**: pressing Drop with a
 * blank reason sends nothing and leaves the form open. A rendering cannot show
 * that, which is what earns it a `play`.
 */
export const DroppingNeedsAReason: Story = {
  args: {
    completeWhen: COMPLETE,
    members: [LANDED, AWAITING, { ...STACKED, onDrop: fn() }],
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Drop…" }));

    // Enter on a blank reason sends nothing, and the hint that was advice
    // becomes the error it now is. The button beside it is off for the same
    // reason, which is why the keyboard is where the rule is asserted.
    const reason = canvas.getByLabelText("Reason");
    await userEvent.type(reason, "{Enter}");
    await expect(args.members[2]?.onDrop).not.toHaveBeenCalled();
    await expect(canvas.getByRole("button", { name: "Drop" })).toBeDisabled();

    await userEvent.type(reason, "It folded into the second landing");
    await userEvent.click(canvas.getByRole("button", { name: "Drop" }));
    await expect(args.members[2]?.onDrop).toHaveBeenCalledWith("It folded into the second landing");
    // The form closes on an accepted drop: nothing is left to press twice.
    await expect(canvas.queryByLabelText("Reason")).toBeNull();
  },
};
