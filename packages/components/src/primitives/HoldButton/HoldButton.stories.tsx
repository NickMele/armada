import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { HoldButton, holdDurationOf } from "./HoldButton";

const meta: Meta<typeof HoldButton> = {
  title: "Primitives/Hold button",
  component: HoldButton,
  args: {
    children: "Hold to kill job",
    askLabel: "Kill job",
    description: "Kills the job once held until it fills. Letting go sooner kills nothing.",
    onCommit: fn(),
    onAsk: fn(),
  },
  render: (args) => (
    <div style={{ padding: "var(--pad-card)", background: "var(--bg-raised)" }}>
      <HoldButton {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof HoldButton>;

/** At rest. A click is a press let go at once, so it kills nothing and asks nothing. */
export const Rest: Story = {
  play: async ({ args, canvas, userEvent }) => {
    const button = canvas.getByRole("button", { name: "Hold to kill job" });
    await expect(button).toHaveAccessibleDescription(args.description);
    await userEvent.click(button);
    const hold = holdDurationOf(button) ?? 0;
    await new Promise((resolve) => setTimeout(resolve, hold + 100));
    await expect(args.onCommit).not.toHaveBeenCalled();
    await expect(args.onAsk).not.toHaveBeenCalled();
  },
};

/** Held halfway, seeded so the fill does not depend on when the screenshot lands. */
export const Arming: Story = {
  render: (args) => (
    <div style={{ padding: "var(--pad-card)", background: "var(--bg-raised)" }}>
      <HoldButton {...args} data-preview-held="" />
    </div>
  ),
};

/** Held long enough, and Fleet has not answered. `Button`'s own pending rendering. */
export const Pending: Story = {
  args: { children: "Killing job…", askLabel: "Killing job…", pending: true },
};

/**
 * Under `prefers-reduced-motion` the hold is not offered: the label is the act
 * and a press asks, because the fill is the only thing that says how long is left.
 */
export const ReducedMotion: Story = {
  beforeEach: () => {
    const real = window.matchMedia;
    window.matchMedia = (query: string) => {
      if (!query.includes("prefers-reduced-motion")) return real.call(window, query);
      // A preference that never changes, so nothing is ever dispatched to a listener.
      const reduced = new EventTarget() as MediaQueryList;
      return Object.assign(reduced, { matches: true, media: query, onchange: null });
    };
    return () => {
      window.matchMedia = real;
    };
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Kill job" }));
    await expect(args.onAsk).toHaveBeenCalledTimes(1);
    await expect(args.onCommit).not.toHaveBeenCalled();
  },
};
