import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { ConfidenceSheet } from "./ConfidenceSheet";

/**
 * Armada's review, drawn above the verdict sheet at the gate. #903. The verdict and Needs you
 * are always open; the rest folds, and Tests in the change opens itself on a removed test.
 */
const meta: Meta<typeof ConfidenceSheet> = {
  title: "Compositions/Confidence sheet",
  component: ConfidenceSheet,
};
export default meta;

type Story = StoryObj<typeof ConfidenceSheet>;

const REMOVED = "a_loaded_machine_holds_a_job_back_too";

/** A confident review, with one change in behaviour for the person and a test removed without a reason. */
export const ARemovedTest: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: [
        "Every Check passed and the Judge met every criterion.",
        "One change in behaviour needs your yes before it lands.",
      ],
      areas: [
        {
          name: "Admission",
          what: "CPU no longer holds a Job",
          files: ["crates/fleet/src/headroom.rs", "crates/fleet/src/admitting.rs"],
        },
        {
          name: "Tests",
          what: "The CPU hold's test asserts the opposite",
          files: ["crates/fleet/src/tests/headroom.rs"],
        },
      ],
      tests: {
        opened_because: { kind: "test_removed", name: REMOVED },
        proves: [{ area: "Admission", what: "A saturated CPU holds nothing back", tests: 2 }],
        changed: [
          {
            name: REMOVED,
            change: "removed",
            replaced_by: "a_machine_whose_cpu_is_saturated_still_admits",
            flagged: true,
          },
        ],
        untested: [
          { code: "Opening Fleet settings from the status bar", why: "No test opens the sheet from it" },
        ],
      },
      needs_you: [
        {
          finding: `\`${REMOVED}\` was removed with no reason given`,
          why: "A test taken out or weakened is the reviewer's to explain",
        },
        { finding: "A busy CPU no longer delays a Job", why: "People may rely on the old behaviour" },
      ],
      small_fixes: [],
      for_context: [
        { finding: "The pull request asks for a check of the lock order when saving", why: "The author flagged it" },
      ],
    },
  },
  play: async ({ canvas, userEvent }) => {
    const callout = canvas.getByRole("note");
    await expect(callout).toHaveTextContent("A test was removed");
    await expect(callout).toHaveTextContent(REMOVED);
    await expect(canvas.getByRole("button", { name: /Tests in the change/ })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    const shape = canvas.getByRole("button", { name: /Shape of the change/ });
    await expect(shape).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(shape);
    await expect(shape).toHaveAttribute("aria-expanded", "true");
  },
};

/** Confident, with nothing for the person: Needs you says so, and the empty sections are left out. */
export const NothingNeedsYou: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: ["It only adds code: one new route and one new message."],
      areas: [{ name: "New code", what: "The repository scan", files: ["crates/fleet/src/scanning.rs"] }],
      needs_you: [],
      small_fixes: [],
      for_context: [],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Nothing needs you.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Small fixes for a Drone/ })).toBeNull();
    await expect(canvas.queryByRole("note")).toBeNull();
  },
};

/** Not confident, with a small fix a Drone can make. */
export const NotConfident: Story = {
  args: {
    confidence: {
      says: "not_confident",
      reasons: ["The change reaches every repository that uses the workflows, and it changes a database trigger."],
      areas: [
        { name: "Engine", what: "Walking back through a finished step", files: ["crates/fleet/src/dispatch.rs"] },
      ],
      needs_you: [
        { finding: "Undoing the trigger change needs a second migration", why: "Reverting the code does not undo it" },
      ],
      small_fixes: [
        { finding: "The store module is over 500 lines", why: "In scope. Move the migration into its own file." },
      ],
      for_context: [],
    },
  },
};
