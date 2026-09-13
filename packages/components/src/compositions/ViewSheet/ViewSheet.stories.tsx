import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { ViewSheet, type ViewSheetStep } from "./ViewSheet";

/**
 * The code a finding is about, as a chain in the order one change forces the next. #904.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so the story draws one.
 */
const meta: Meta<typeof ViewSheet> = {
  title: "Compositions/View sheet",
  component: ViewSheet,
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

type Story = StoryObj<typeof ViewSheet>;

const STEPS: ViewSheetStep[] = [
  {
    file: "crates/fleet/src/headroom.rs",
    summary: "Removes CPU as a way to be short of room.",
    tieToNext: "admitting.rs turned a CPU shortage into a hold, and nothing produces one now.",
    lines: [
      { kind: "hunk", text: "@@ -40,6 +40,4 @@ pub enum Short {" },
      { kind: "context", text: "     Memory," },
      { kind: "removed", text: "-    Cpu," },
    ],
  },
  {
    file: "crates/fleet/src/admitting.rs",
    summary: "The arm that turned a CPU shortage into a CPU hold goes.",
    tieToNext: "fields.rs defines that hold, and nothing produces it now.",
    lines: [
      { kind: "hunk", text: "@@ -212,7 +212,6 @@" },
      { kind: "context", text: "     Room::Bound => Some(AdmissionHold::ConcurrencyBound)," },
      { kind: "removed", text: "-    Room::Machine(Short::Cpu) => Some(AdmissionHold::Cpu)," },
      { kind: "context", text: "     Room::Machine(Short::Memory) => Some(AdmissionHold::Memory)," },
    ],
  },
  {
    file: "crates/core-model/src/job/fields.rs",
    summary: "The CPU hold is deleted, along with the word it sent to Bridge.",
    lines: null,
  },
];

/** Three steps, folded, with the last one's hunk gone from the patch since the review. */
export const AChain: Story = {
  args: {
    open: true,
    title: "A busy CPU no longer delays a Job",
    steps: STEPS,
    onOpenFile: fn(),
    onClose: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const first = canvas.getByRole("button", { name: /Removes CPU/ });
    await expect(first).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.getByText(/^-\s+Cpu,$/)).not.toBeVisible();

    await userEvent.click(first);
    await expect(first).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText(/^-\s+Cpu,$/)).toBeVisible();
    await expect(canvas.getByText(/^Next: admitting\.rs/)).toBeVisible();

    const [openFile] = canvas.getAllByRole("button", { name: "Open the whole file" });
    await userEvent.click(openFile!);
    await expect(args.onOpenFile).toHaveBeenCalledWith("crates/fleet/src/headroom.rs");

    await userEvent.click(canvas.getByRole("button", { name: "Open all" }));
    await expect(canvas.getByText(/This hunk is not in the patch any more/)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Fold all" })).toBeVisible();
  },
};

/** A note written under the chain goes onto What should change, and the field clears. #907. */
export const ANoteForTheDrone: Story = {
  args: {
    open: true,
    title: "A busy CPU no longer delays a Job",
    steps: STEPS,
    onAddNote: fn(),
    onClose: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const add = canvas.getByRole("button", { name: "Add to What should change" });
    await expect(add).toBeDisabled();
    const field = canvas.getByLabelText("Note for the drone");
    await userEvent.type(field, "Say in the status bar that CPU never holds a Job.");
    await userEvent.click(add);
    await expect(args.onAddNote).toHaveBeenCalledWith(
      "Say in the status bar that CPU never holds a Job.",
    );
    await expect(field).toHaveValue("");
    await expect(canvas.getByRole("status")).toHaveTextContent("1 note added to What should change.");
  },
};
