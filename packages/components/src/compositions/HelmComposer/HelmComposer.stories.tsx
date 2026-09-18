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

/** What the switch is called with Helm pointed at nothing: there is no *different* repository yet. */
const POINT = "Point Helm at a repository";

/** And once it is pointed at one. */
const REPOINT = "Point Helm at a different repository";

/** The caret's own name on the record's split button — what is behind it, in words. */
const MORE = "More ways to report this answer";

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

/**
 * A single repository, and Helm pointed at it: no switch to draw, nothing to
 * switch to. **The switch offered here would be a switch to where Helm already
 * is**, so the line names the repository instead and the handler goes unused.
 */
export const AtRest: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    onSwitch: fn(),
    onCopyRecord: fn(),
    onOpenRecord: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("combobox")).not.toBeInTheDocument();
    await expect(canvas.getByText(repositories[0]!.label)).toBeInTheDocument();
  },
};

/**
 * How a bad answer is carried to somebody who could fix it — `#1367`, as one
 * control since 18 Sep 2026. **The banner form, split**: *Copy debug info*
 * acts on one press from the face, and *Details* opens the same artifact to
 * read from behind the caret. Each act still calls its own handler and neither
 * calls the other's.
 */
export const RecordCopiedAndRead: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    onCopyRecord: fn(),
    onOpenRecord: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Copy debug info" }));
    await expect(args.onCopyRecord).toHaveBeenCalled();
    await expect(args.onOpenRecord).not.toHaveBeenCalled();

    // The reading is behind the caret, which is the whole of what the caret is
    // for: nothing else in this row discloses anything.
    await userEvent.click(canvas.getByRole("button", { name: MORE }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "Details" }));
    await expect(args.onOpenRecord).toHaveBeenCalled();
    await expect(args.onCopyRecord).toHaveBeenCalledTimes(1);
  },
};

/**
 * One handler, and there is nothing to split. **A caret over an empty menu is
 * a control that does not answer**, so the act is drawn as the plain button it
 * already was — whichever of the two the dock could offer.
 */
export const OnlyOneActToReportWith: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!], onCopyRecord: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: MORE })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Copy debug info" }));
    await expect(args.onCopyRecord).toHaveBeenCalled();
  },
};

/**
 * The same, with the reading as the only act: the plain button carries
 * *Details* rather than a face with a dead caret beside it.
 */
export const OnlyTheReadingToReportWith: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!], onOpenRecord: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: MORE })).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "Copy debug info" })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Details" }));
    await expect(args.onOpenRecord).toHaveBeenCalled();
  },
};

/**
 * **The row holds one line at the dock's own width** — the owner's note of
 * 18 Sep 2026, where three controls beside the repository wrapped it onto a
 * second line 72px tall. Every control this row can hold is present: the
 * switch, and the record's split button with both acts.
 *
 * A rendering cannot state this, and neither can a role: the fact is that
 * every control shares one line and none of them is pushed past the row's
 * trailing edge.
 */
export const HeadHoldsOneLineAtDockWidth: Story = {
  args: {
    current: repositories[0]!.id,
    repositories,
    onSwitch: fn(),
    onCopyRecord: fn(),
    onOpenRecord: fn(),
  },
  play: async ({ canvas, canvasElement }) => {
    async function onOneLine(): Promise<void> {
      const head = canvasElement.querySelector(".armada-helm-composer__head");
      if (!(head instanceof HTMLElement)) throw new Error("the composer drew no head row");
      const row = head.getBoundingClientRect();
      const controls = [
        canvas.getByRole("combobox", { name: REPOINT }),
        canvas.getByRole("button", { name: "Copy debug info" }),
        canvas.getByRole("button", { name: MORE }),
      ].map((one) => one.getBoundingClientRect());

      // One line: the row is no taller than the tallest control standing in it.
      const tallest = Math.max(...controls.map((one) => one.height));
      await expect(row.height).toBeLessThanOrEqual(tallest);
      // And every control is on it, within its edges — a row that does not wrap
      // is only an improvement if nothing has been pushed out of sight instead.
      for (const control of controls) {
        await expect(control.top).toBeGreaterThanOrEqual(row.top - 1);
        await expect(control.bottom).toBeLessThanOrEqual(row.bottom + 1);
        await expect(control.left).toBeGreaterThanOrEqual(row.left - 1);
        await expect(control.right).toBeLessThanOrEqual(row.right + 1);
      }
    }

    await onOneLine();

    // And again at the floor of the dock's drag range, which is where the
    // owner's own dock was standing when he wrote the note: 286px of head is
    // --w-dock-min, not --w-dock. A row that only holds at rest holds nowhere
    // a person actually drags to.
    const dock = canvasElement.querySelector(".armada-glass");
    if (!(dock instanceof HTMLElement)) throw new Error("the story drew no dock around the composer");
    dock.style.width = "var(--w-dock-min)";
    await onOneLine();
  },
};

/**
 * Pointed at nothing with exactly one repository set up — the moment the dock
 * reads *Helm is not pointed at a repository. Pick armada to ask about it.*
 * and, until now, gave nobody anything to pick with: the count that governs
 * the control said one repository is nothing to switch between, which is true
 * only while Helm is pointed at it.
 *
 * **The switch is the act that sentence names**, so it is drawn here, standing
 * at its own entry with that one repository under it to choose.
 */
export const UnpointedWithOneRepository: Story = {
  args: { repositories: [repositories[0]!], onSwitch: fn(), disabled: true },
  play: async ({ args, canvas, userEvent }) => {
    const switcher = canvas.getByRole("combobox", { name: POINT });
    // Its own entry, not the one repository: Helm is not pointed at that yet,
    // and a switch displaying it would say it was.
    await expect(switcher).toHaveDisplayValue(UNPOINTED);
    await expect(switcher).not.toHaveDisplayValue(repositories[0]!.label);
    // Said once, as with two or more: the entry says it and says what to do.
    await expect(canvas.queryByText("No repository to ask yet")).not.toBeInTheDocument();

    // And the one repository is pickable — which is the whole of why the
    // control is drawn. `pick` because the browser fires no `change` for an
    // entry already displayed, and `user-event` fires one regardless.
    await pick(switcher, repositories[0]!.id, userEvent);
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[0]!.id);
  },
};

/** On All repositories, the dock's own switch — the rail's pick never moves for it. */
export const SwitchOnAll: Story = {
  args: { current: repositories[0]!.id, repositories, onSwitch: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const switcher = canvas.getByRole("combobox", { name: REPOINT });
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
  args: { repositories, onSwitch: fn(), disabled: true },
  play: async ({ args, canvas, userEvent }) => {
    const switcher = canvas.getByRole("combobox", { name: POINT });
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

/**
 * Nothing is servable yet — no repository has a Manifest for Helm to answer
 * for. **The handler is given and the switch is still not drawn**: pointing
 * Helm at nothing is a reason to offer a repository, not to offer none.
 */
export const NothingToAskYet: Story = {
  args: { repositories: [], onSwitch: fn(), disabled: true },
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
