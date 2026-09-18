import { useState } from "react";
import type { ReactElement, ReactNode } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { DroneMessageBox } from "./DroneMessageBox";

/** The width a step's chapter or the log sheet gives the box — every story
 *  draws inside it, so a story that types is measured at the real width. */
function InTheChapter({ children }: { children: ReactNode }): ReactElement {
  return (
    <div style={{ width: "var(--w-sheet)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      {children}
    </div>
  );
}

/** The box, drawn at the width a step's chapter or the log sheet gives it. */
const meta: Meta<typeof DroneMessageBox> = {
  title: "Compositions/Drone message box",
  component: DroneMessageBox,
  args: { value: "", onChange: fn(), onSend: fn() },
  render: (args) => (
    <InTheChapter>
      <DroneMessageBox {...args} />
    </InTheChapter>
  ),
};
export default meta;

type Story = StoryObj<typeof DroneMessageBox>;

function Typed({ onSend = fn() }: { onSend?: () => void }): ReactElement {
  const [value, setValue] = useState("");
  return (
    <InTheChapter>
      <DroneMessageBox value={value} onChange={setValue} onSend={onSend} />
    </InTheChapter>
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

/** A drone is working the step: the field takes a message and Send lights up once there is one. */
export const EnabledWhileADroneWorks: Story = {
  render: () => <Typed />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await userEvent.type(canvas.getByRole("textbox"), "Check the second failing test too");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
  },
};

/**
 * Send is inside the field, and the text stops above it — the owner's note of
 * 17 Sep 2026. Three things a screenshot cannot state: the button's box is
 * within the field's on all four edges, the band a typed line may occupy ends
 * above the button rather than under it, and the field is more than one row
 * tall before anything is typed.
 */
export const SendSitsInsideTheField: Story = {
  render: () => <Typed />,
  play: async ({ canvasElement }) => {
    const rest = drawn(canvasElement);
    await expect(rest.text.height).toBeGreaterThan(rest.row * 1.5);

    await userEvent.type(
      within(canvasElement).getByRole("textbox"),
      "Check the second failing test too{Enter}and say what it asserts",
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
 * `⌘Enter` sends and plain `Enter` does not — a keyboard contract no rendering
 * carries. The field keeps the paragraph break `Enter` made, which is the
 * reason the send takes a modifier at all.
 */
export const CmdEnterSends: Story = {
  render: (args) => <Typed onSend={args.onSend} />,
  // The play's own `userEvent`, not the imported one: a hold that spans two
  // calls needs the instance that remembers `⌘` is down between them.
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByRole("textbox");
    await userEvent.type(field, "Check the second failing test too");

    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).not.toHaveBeenCalled();
    await expect(field).toHaveValue("Check the second failing test too\n");

    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSend).toHaveBeenCalledTimes(1);
  },
};

/**
 * Held `⌘` puts `⌘Enter` on Send and releasing takes it away, through the one
 * `ShortcutRevealProvider` `TheShell` mounts in the running app. Send is still
 * named `Send` with the badge up: the keycap is `aria-hidden`.
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
    await userEvent.type(canvas.getByRole("textbox"), "Check the second failing test too");
    await expect(badge()).not.toBeInTheDocument();

    await userEvent.keyboard("{Meta>}");
    await expect(badge()).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Send" })).toBeInTheDocument();

    await userEvent.keyboard("{/Meta}");
    await expect(badge()).not.toBeInTheDocument();
  },
};

/** Whitespace is not a message: Send stays off, and a press sends nothing. */
export const BlankNeverSends: Story = {
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
  },
};

/** Queued, between steps, or finished — no drone is on the step to send to. */
export const DisabledWithNoDrone: Story = {
  args: { disabled: true, disabledReason: "No drone is on this step, so there's nothing to send this to." },
  play: async ({ args, canvas }) => {
    await expect(canvas.getByRole("textbox")).toBeDisabled();
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    await userEvent.click(send, { pointerEventsCheck: 0 });
    await expect(args.onSend).not.toHaveBeenCalled();
    // A disabled field takes no focus, so the press lands on the document —
    // which is the point: nothing else in the tree answers ⌘Enter.
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSend).not.toHaveBeenCalled();
    await expect(
      canvas.getByText("No drone is on this step, so there's nothing to send this to."),
    ).toBeInTheDocument();
  },
};

/** A redirect is already out. The field stays open — sending again replaces it, as it does today. */
export const RedirectAlreadyWaiting: Story = {
  args: {
    waiting:
      "Sent, waiting for the drone. The instruction went into its session at 2:41pm, and nothing " +
      "here moves when it lands — this job was never held, and it goes on working either way. " +
      "Redirecting again replaces what is outstanding.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("textbox")).toBeEnabled();
    await expect(canvas.getByText(/Sent, waiting for the drone/)).toBeInTheDocument();
  },
};
