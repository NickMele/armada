import { useState } from "react";
import type { ReactElement, ReactNode } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { HelmComposer, type HelmRepositoryOption } from "./HelmComposer";

const repositories: HelmRepositoryOption[] = [
  { id: "01M2ARMADA", label: "armada" },
  { id: "01M2SHOP", label: "shop-01" },
];

/** The switch's own entry while Helm is pointed at nothing — the word the repository asks elsewhere use. */
const UNPOINTED = "Choose a repository";

/**
 * Picking an entry the way a person's mouse does, which is not what
 * `selectOptions` does on its own: **a native `<select>` fires `change` only
 * when the displayed value actually moves**, and `user-event` dispatches one
 * regardless. That difference is this whole defect — the switch displayed
 * *armada* while holding no value, so choosing *armada* changed nothing and
 * Chromium stayed silent — and a play that let `user-event` fire anyway would
 * report the handler called when the app's first repository could not be
 * chosen at all.
 */
async function pick(
  select: HTMLElement,
  value: string,
  events: { selectOptions: (target: HTMLElement, value: string) => Promise<void> },
): Promise<void> {
  if ((select as HTMLSelectElement).value === value) return;
  await events.selectOptions(select, value);
}

/** The dock's own width and glass — every story draws inside it, so a story that
 *  types is measured at the width the composer really has. */
function InTheDock({ children }: { children: ReactNode }): ReactElement {
  return (
    <div
      className="armada-glass"
      style={{ width: "var(--w-dock)", borderRadius: "var(--radius-lg)", padding: "var(--space-4)" }}
    >
      {children}
    </div>
  );
}

/** The composer under Helm's thread, drawn at the dock's own width, on the dock's own glass. */
const meta: Meta<typeof HelmComposer> = {
  title: "Compositions/Helm composer",
  component: HelmComposer,
  args: { value: "", onChange: fn(), onSend: fn(), location: "Job Board" },
  render: (args) => (
    <InTheDock>
      <HelmComposer {...args} />
    </InTheDock>
  ),
};
export default meta;

type Story = StoryObj<typeof HelmComposer>;

/** A single repository: no switch to draw, nothing to switch to. */
export const AtRest: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!], onStartFresh: fn() },
};

/** On All repositories, the dock's own switch — the rail's pick never moves for it. */
export const SwitchOnAll: Story = {
  args: { current: repositories[0]!.id, repositories, onSwitch: fn(), onStartFresh: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const switcher = canvas.getByRole("combobox");
    // Pointed at one, the switch stands on it and carries nothing else — the
    // unpointed entry is not an option a pointed switch offers.
    await expect(switcher).toHaveDisplayValue(repositories[0]!.label);
    await expect(canvas.queryByRole("option", { name: UNPOINTED })).not.toBeInTheDocument();
    await userEvent.selectOptions(switcher, repositories[1]!.id);
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[1]!.id);
  },
};

/**
 * Pointed at nothing, with repositories to point at: the switch stands at its
 * own entry, not at whichever repository happens to be listed first.
 *
 * **Both halves of this are the defect.** A `<select>` whose `value` matches no
 * `<option>` falls back to displaying the first one, so the switch read
 * *armada* while the chip beside it read *No repository to ask yet* — and
 * because *armada* was already the displayed value, picking it fired no
 * `change` at all, which left the first repository in the list unchoosable. A
 * play that only picks the second repository passes either way.
 */
export const UnpointedStandsAtItsOwnEntry: Story = {
  args: { repositories, onSwitch: fn(), onStartFresh: fn(), disabled: true },
  play: async ({ args, canvas, userEvent }) => {
    const switcher = canvas.getByRole("combobox", { name: "Point Helm at a different repository" });
    await expect(switcher).toHaveDisplayValue(UNPOINTED);
    await expect(switcher).not.toHaveDisplayValue(repositories[0]!.label);
    // Said once: the entry says it and says what to do, so the line does not
    // repeat it — at the dock's width the two cut each other down to "No repo…".
    await expect(canvas.queryByText("No repository to ask yet")).not.toBeInTheDocument();

    // The first repository in the list, which is the one that could not be chosen.
    await pick(switcher, repositories[0]!.id, userEvent);
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[0]!.id);
  },
};

/** A reply is being written — Fleet refuses Start fresh until it finishes. */
export const StartFreshRefusedWhileReplying: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    onStartFresh: fn(),
    startFreshDisabled: true,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Start fresh" })).toBeDisabled();
  },
};

/** Nothing is servable yet — no repository has a Manifest for Helm to answer for. */
export const NothingToAskYet: Story = {
  args: { disabled: true },
  play: async ({ args, canvas }) => {
    // No switch is drawn with nothing to point at, so the line is the only
    // place the state can be said, and it says it.
    await expect(canvas.queryByRole("combobox")).not.toBeInTheDocument();
    await expect(canvas.getByText("No repository to ask yet")).toBeInTheDocument();
    await expect(canvas.getByRole("textbox")).toBeDisabled();
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    await userEvent.click(send, { pointerEventsCheck: 0 });
    await expect(args.onSend).not.toHaveBeenCalled();
    // A disabled field takes no focus, so the press lands on the document —
    // and nothing else in the tree answers ⌘Enter.
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSend).not.toHaveBeenCalled();
  },
};

/** A Job's detail is open: the chip names it, above the message box. #1075. */
export const ChipOnAJob: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    chip: { jobHandle: "12", title: "Fix the poke loop" },
    onRemoveChip: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Remove Job 12 · Fix the poke loop" })).toBeInTheDocument();
  },
};

/** The chip's own `×` — dropped from the next ask's context, without closing the Job. #1075. */
export const ChipRemovedWithX: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    chip: { jobHandle: "12", title: "Fix the poke loop" },
    onRemoveChip: fn(),
  },
  play: async ({ args, canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Remove Job 12 · Fix the poke loop" }));
    await expect(args.onRemoveChip).toHaveBeenCalled();
  },
};

/** Off a Job, nothing is chipped — the composer draws no differently than any other ask. #1075. */
export const NoChipOffAJob: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByText(/^Job \d/)).not.toBeInTheDocument();
  },
};

/** The footer names the screen and the cursor row — no lead-in. #1094. */
export const FooterNamesTheCursorRow: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    location: "Overview · cursor on Job 16",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Overview · cursor on Job 16")).toBeInTheDocument();
    await expect(canvas.queryByText(/Helm reads/i)).not.toBeInTheDocument();
  },
};

/** A Job's detail, chipped: the footer agrees with the chip above it. #1094. */
export const FooterNamesAJobsDetail: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    chip: { jobHandle: "16", title: "Preserve job metadata during resource cleanup" },
    onRemoveChip: fn(),
    location: "Job 16's detail (in context)",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Job 16's detail (in context)")).toBeInTheDocument();
  },
};

function Typed({ onSend = fn() }: { onSend?: () => void }): ReactElement {
  const [value, setValue] = useState("");
  return (
    <InTheDock>
      <HelmComposer
        current={repositories[0]!.id}
        repositories={[repositories[0]!]}
        location="Job Board"
        value={value}
        onChange={setValue}
        onSend={onSend}
      />
    </InTheDock>
  );
}

/**
 * Where Send is drawn and where a typed line may go — neither of which a
 * rendering can state. The field's text band is its content box: everything
 * inside the border and the padding, which is exactly the room a line of
 * typing can occupy.
 */
function drawn(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole("textbox");
  const frame = field.getBoundingClientRect();
  const send = canvas.getByRole("button", { name: "Send" }).getBoundingClientRect();
  const style = getComputedStyle(field);
  const row = Number.parseFloat(style.lineHeight);
  const top = frame.top + Number.parseFloat(style.borderTopWidth) + Number.parseFloat(style.paddingTop);
  const bottom =
    frame.bottom - Number.parseFloat(style.borderBottomWidth) - Number.parseFloat(style.paddingBottom);
  return { frame, send, row, text: { top, bottom, height: bottom - top } };
}

/**
 * Send is inside the field here too, and the dock's narrow width is the stated
 * cost of one treatment: the button's box is within the field's on all four
 * edges, the band a typed line may occupy ends above it rather than under it,
 * and the field is more than one row tall before anything is asked.
 */
export const SendSitsInsideTheField: Story = {
  render: () => <Typed />,
  play: async ({ canvasElement }) => {
    const rest = drawn(canvasElement);
    await expect(rest.text.height).toBeGreaterThan(rest.row * 1.5);

    await userEvent.type(
      within(canvasElement).getByRole("textbox"),
      "Why did job 12 stall?{Enter}It was running an hour ago",
    );
    const { frame, send, text } = drawn(canvasElement);
    await expect(send.left).toBeGreaterThanOrEqual(frame.left);
    await expect(send.right).toBeLessThanOrEqual(frame.right);
    await expect(send.top).toBeGreaterThanOrEqual(frame.top);
    await expect(send.bottom).toBeLessThanOrEqual(frame.bottom);
    await expect(text.bottom).toBeLessThanOrEqual(send.top);
  },
};

/**
 * `⌘Enter` asks, and plain `Enter` writes a second line — the drone message
 * box's contract, honoured here off the same registry row.
 */
export const CmdEnterSends: Story = {
  render: (args) => <Typed onSend={args.onSend} />,
  // The play's own `userEvent`, not the imported one: a hold that spans two
  // calls needs the instance that remembers `⌘` is down between them.
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByRole("textbox");
    await userEvent.type(field, "Why did job 12 stall?");

    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).not.toHaveBeenCalled();
    await expect(field).toHaveValue("Why did job 12 stall?\n");

    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSend).toHaveBeenCalledTimes(1);
  },
};

/**
 * Held `⌘` puts `⌘Enter` on Helm's own Send, and releasing takes it away. The
 * tint and the wording are untouched, and the keycap is `aria-hidden`, so the
 * button is still named `Send`.
 */
export const RevealedOnHold: Story = {
  render: (args) => (
    <ShortcutRevealProvider>
      <Typed onSend={args.onSend} />
    </ShortcutRevealProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    const badge = () =>
      canvas.queryByText((_, el) => el?.tagName === "KBD" && el.textContent === "⌘Enter");
    await userEvent.type(canvas.getByRole("textbox"), "Why did job 12 stall?");
    await expect(badge()).not.toBeInTheDocument();

    await userEvent.keyboard("{Meta>}");
    await expect(badge()).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Send" })).toBeInTheDocument();

    await userEvent.keyboard("{/Meta}");
    await expect(badge()).not.toBeInTheDocument();
  },
};

/** Blank never sends — the button stays off until there is something to say. */
export const BlankDoesNotSend: Story = {
  render: (args) => <Typed onSend={args.onSend} />,
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    await userEvent.type(canvas.getByRole("textbox"), "   ");
    await expect(send).toBeDisabled();
    await userEvent.click(send, { pointerEventsCheck: 0 });
    await expect(args.onSend).not.toHaveBeenCalled();
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSend).not.toHaveBeenCalled();
    await userEvent.type(canvas.getByRole("textbox"), "Why did job 12 stall?");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
  },
};
