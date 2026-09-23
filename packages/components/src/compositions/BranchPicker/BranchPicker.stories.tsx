import { useState } from "react";
import type { ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { BranchPicker } from "./BranchPicker";
import type { BranchOption } from "./BranchPicker";

/**
 * Where the work starts and where it lands, as a field that offers what the
 * repository has and still takes a name typed by hand.
 *
 * The complaint this answers, in the owner's words: *"These should be
 * dropdowns that prefill with the branches. The repo base being the default.
 * For the 'lands in', I should be able to select one or create a new branch
 * just by typing the name."*
 *
 * **Every state here is one a person meets.** Nothing listed, the list open,
 * narrowed by typing, a name that is not a branch yet, and the same name where
 * making one is not on offer.
 */
const meta: Meta<typeof BranchPicker> = {
  title: "Compositions/Branch picker",
  component: BranchPicker,
  args: {
    label: "Lands in",
    value: "main",
    onValue: fn(),
    branches: BRANCHES(),
    offerNew: true,
  },
};
export default meta;

type Story = StoryObj<typeof BranchPicker>;

/** The repository's branches as the draft derives them: base first, then Jobs. */
function BRANCHES(): BranchOption[] {
  return [
    { name: "main", base: true },
    { name: "armada/18-fold-the-capacity-read", job: "Fold the capacity read into one query" },
    { name: "armada/19-give-the-rail-its-own-scroll", job: "Give the rail its own scroll" },
  ];
}

/** A field whose value feeds back, which is what a picked name has to survive. */
function Picking(props: ComponentProps<typeof BranchPicker>) {
  const [value, setValue] = useState(props.value);
  return (
    <BranchPicker
      {...props}
      value={value}
      onValue={(name) => {
        setValue(name);
        props.onValue(name);
      }}
    />
  );
}

/**
 * Nothing listed them, so the field is the plain one it was.
 *
 * **This is Bridge on a real Fleet.** No operation lists a repository's refs,
 * so the picker draws nothing to pick from rather than an empty list — which
 * would say the repository has no branches, and nothing has asked it.
 */
export const NothingListed: Story = {
  args: { branches: null, value: "" },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("combobox")).toBeNull();
    await expect(canvas.getByRole("textbox", { name: "Lands in" })).toBeVisible();
  },
};

/**
 * The list, with what there is to say about each branch: the base it opens on,
 * and the Job sitting on every other.
 */
export const Offering: Story = {
  render: Picking,
  /**
   * The keyboard contract, which no rendering shows: down opens and moves,
   * Enter takes the row, and the field holds what was taken.
   */
  play: async ({ args, canvas, userEvent, step }) => {
    const field = canvas.getByRole("combobox", { name: "Lands in" });

    await step("the list opens under the field and says what each branch is", async () => {
      await userEvent.click(field);
      await expect(canvas.getByRole("option", { name: /^main base$/ })).toBeVisible();
      await expect(
        canvas.getByRole("option", {
          name: "armada/18-fold-the-capacity-read Fold the capacity read into one query",
        }),
      ).toBeVisible();
    });

    await step("down moves and Enter takes the row the cursor is on", async () => {
      await userEvent.keyboard("{ArrowDown}{Enter}");
      await expect(args.onValue).toHaveBeenLastCalledWith("armada/18-fold-the-capacity-read");
      await expect(field).toHaveValue("armada/18-fold-the-capacity-read");
    });

    await step("and taking one closes the list", async () => {
      await expect(canvas.queryByRole("option")).toBeNull();
    });
  },
};

/**
 * Typed past the branch it was resting on, the list narrows to what matches.
 * A value resting on a branch narrows to nothing, so the field opens on all of
 * them rather than on the one row a person already has.
 */
export const Narrowed: Story = {
  args: { value: "rail" },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("combobox", { name: "Lands in" }));
    await expect(canvas.getAllByRole("option")).toHaveLength(2);
    await expect(
      canvas.getByRole("option", { name: /armada\/19-give-the-rail-its-own-scroll/ }),
    ).toBeVisible();
  },
};

/**
 * A name no branch carries, on the field that may make one.
 *
 * **Typing the name is the whole of creating it.** There is no second control
 * and no dialog: the row says the branch is new, and taking it is agreeing to
 * that.
 */
export const ANameThatIsNotThereYet: Story = {
  args: { value: "release/16" },
  render: Picking,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("combobox", { name: "Lands in" }));
    const made = canvas.getByRole("option", { name: "release/16 new branch" });
    await expect(made).toBeVisible();

    await userEvent.click(made);
    await expect(args.onValue).toHaveBeenLastCalledWith("release/16");
  },
};

/**
 * The same name on the field the work starts from, where making one is not on
 * offer — you cannot begin on a branch that does not exist.
 *
 * **The typed name still stands.** The list is what Armada has met rather than
 * the repository's own, so a branch it has never seen is not a branch that is
 * not there, and the field says which of the two this is.
 */
export const StartingFromSomethingUnlisted: Story = {
  args: { label: "From", value: "release/16", offerNew: false },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("combobox", { name: "From" });
    await userEvent.click(field);

    await expect(canvas.queryByRole("option")).toBeNull();
    await expect(canvas.getByText(/No branch Armada has met matches/)).toBeVisible();
    // Nothing takes the value away: the field is still what was typed.
    await expect(field).toHaveValue("release/16");
  },
};

/**
 * Escape closes the list and stops there.
 *
 * **A `play`, because what must not happen is the point.** The composer this
 * field sits in leaves on Escape, and it reads `defaultPrevented` to know a
 * layer answered first. A picker that swallowed the key with the list already
 * shut would trap a person on the surface; one that never prevented it would
 * close the whole composer on the press meant for the list.
 */
export const EscapeClosesTheListAndNothingElse: Story = {
  play: async ({ canvas, userEvent }) => {
    const seen: boolean[] = [];
    const watch = (event: KeyboardEvent): void => {
      if (event.key === "Escape") seen.push(event.defaultPrevented);
    };
    window.addEventListener("keydown", watch);
    try {
      await userEvent.click(canvas.getByRole("combobox", { name: "Lands in" }));
      await expect(canvas.getAllByRole("option")).toHaveLength(3);

      await userEvent.keyboard("{Escape}");
      await expect(canvas.queryByRole("option")).toBeNull();

      await userEvent.keyboard("{Escape}");
      await expect(seen).toEqual([true, false]);
    } finally {
      window.removeEventListener("keydown", watch);
    }
  },
};
