import type { Meta, StoryObj } from "@storybook/react-vite";
import { RotateCw } from "lucide-react";
import { useState } from "react";
import { expect, fn, waitFor } from "storybook/test";
import { Button, type ButtonAnswer, type ButtonProps, type ButtonVariant } from "./Button";

const meta: Meta<typeof Button> = {
  title: "Primitives/Button",
  component: Button,
};
export default meta;

type Story = StoryObj<typeof Button>;

/** A card — the ground a secondary is filled one step down from. */
function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--space-3)",
        flexWrap: "wrap",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)",
      }}
    >
      {children}
    </div>
  );
}

/* The four variants at rest, one story each — the rows of the contract's
   Button table — plus `Tonal`, chrome for the one caller outside it. */

export const Primary: Story = {
  args: { variant: "primary", children: "Dispatch job" },
  render: (args) => (
    <Card>
      <Button {...args} />
    </Card>
  ),
};

export const Secondary: Story = {
  args: { variant: "secondary", children: "Cancel" },
  render: (args) => (
    <Card>
      <Button {...args} />
    </Card>
  ),
};

export const Ghost: Story = {
  args: { variant: "ghost", children: "Ghost" },
  render: (args) => (
    <Card>
      <Button {...args} />
    </Card>
  ),
};

export const Destructive: Story = {
  args: { variant: "destructive", children: "Kill job" },
  render: (args) => (
    <Card>
      <Button {...args} />
    </Card>
  ),
};

/**
 * Tonal — chrome, not a list row's act. Same tokens as `SplitButton`'s own
 * `tonal`, drawn on `--bg-sunken`, the title row's own ground (#1156).
 */
export const Tonal: Story = {
  args: { variant: "tonal", children: "Dispatch" },
  render: (args) => (
    <div style={{ padding: "var(--pad-card)", background: "var(--bg-sunken)" }}>
      <Button {...args} />
    </div>
  ),
};

/** Hover, tabulated for every variant. `data-preview-hover` selects the same
 *  declarations as `:hover`; a static story cannot hold a pointer. */
export const Hover: Story = {
  render: () => (
    <Card>
      <Button variant="primary" data-preview-hover="">
        Dispatch job
      </Button>
      <Button variant="secondary" data-preview-hover="">
        Cancel
      </Button>
      <Button variant="ghost" data-preview-hover="">
        Ghost
      </Button>
      <Button variant="destructive" data-preview-hover="">
        Kill job
      </Button>
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, no glow. It differs from every
 *  resting edge in colour, width and position. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Button variant="secondary" data-preview-focus="">
        Focused
      </Button>
    </Card>
  ),
};

/** `--fg-subtle` text with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Button variant="primary" disabled>
        Dispatch job
      </Button>
      <Button variant="secondary" disabled>
        Cancel
      </Button>
      <Button variant="destructive" disabled>
        Kill job
      </Button>
    </Card>
  ),
};

/**
 * Pressed, and Fleet has not answered. The pressed control is the one in its
 * group that is not greyed out, and a second press sends nothing. #1117.
 */
export const Pending: Story = {
  args: { onClick: fn() },
  render: (args) => (
    <Card>
      <Button variant="primary" disabled>
        Approve the work
      </Button>
      <Button variant="secondary" pending onClick={args.onClick}>
        Requesting changes…
      </Button>
      <Button variant="destructive" disabled>
        Reject the work
      </Button>
    </Card>
  ),
  play: async ({ args, canvas, userEvent }) => {
    const pressed = canvas.getByRole("button", { name: "Requesting changes…" });
    await expect(pressed).toHaveAttribute("aria-busy", "true");
    await expect(pressed).toHaveAttribute("aria-disabled", "true");
    // By keyboard: it stays focusable, and Enter on it sends nothing.
    pressed.focus();
    await expect(pressed).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    await expect(args.onClick).not.toHaveBeenCalled();
    await expect(canvas.getByRole("button", { name: "Approve the work" })).toBeDisabled();
  },
};

/** Every variant, with the label each story above gives it. */
const EVERY: { variant: ButtonVariant; label: string }[] = [
  { variant: "primary", label: "Dispatch job" },
  { variant: "secondary", label: "Cancel" },
  { variant: "ghost", label: "Ghost" },
  { variant: "destructive", label: "Kill job" },
  { variant: "tonal", label: "Dispatch" },
];

/**
 * Pressed, every variant. A press walks one step down the button's ground at
 * `--duration-press` — `--accent` to `--accent-muted`, a secondary or ghost to
 * `--bg-sunken`, tonal back to its `--accent-muted` rest, destructive's fill
 * one step deeper — and the line
 * opens from the centre along the bottom edge. Nothing transforms and nothing
 * reflows. `data-preview-press` selects the same declarations as `:active`,
 * which a static story cannot hold.
 */
export const Pressed: Story = {
  render: () => (
    <Card>
      {EVERY.map(({ variant, label }) => (
        <Button key={variant} variant={variant} data-preview-press="">
          {label}
        </Button>
      ))}
      <Button variant="secondary" ground="sunken" data-preview-press="">
        On a sunken row
      </Button>
    </Card>
  ),
};

/**
 * A toggle that is on, beside the same control off. `--accent-muted` is the
 * contract's selected fill and `--accent` its edge; the two read apart at a
 * glance, which `aria-pressed` used to say to a screen reader alone.
 */
export const On: Story = {
  render: () => (
    <Card>
      <Button aria-pressed={false}>Draw</Button>
      <Button aria-pressed>Draw</Button>
      <Button size="sm" aria-pressed>
        Stay on the running step
      </Button>
      <Button aria-pressed disabled>
        Draw
      </Button>
    </Card>
  ),
};

/** Pending, every variant: one rendering, `--bg-sunken`, with the line travelling. */
export const PendingEveryVariant: Story = {
  render: () => (
    <Card>
      {EVERY.map(({ variant, label }) => (
        <Button key={variant} variant={variant} pending>
          {label}
        </Button>
      ))}
    </Card>
  ),
};

/**
 * Fleet accepted, every variant. The line fills the edge in
 * `--status-completed-success` and the control keeps the pending rendering
 * while it holds, so the mark is never on an accent fill. In the app it holds
 * for `--duration-answer` and rests; `data-preview-held` keeps it here.
 */
export const Accepted: Story = {
  render: () => (
    <Card>
      {EVERY.map(({ variant, label }) => (
        <Button key={variant} variant={variant} answer="accepted" data-preview-held="">
          {label}
        </Button>
      ))}
    </Card>
  ),
};

/**
 * Fleet refused, every variant. The line retracts to the centre in
 * `--status-escalated`, and nothing else moves. Held on a loop here so the
 * retract can be watched; under reduced motion it holds full instead, and
 * colour carries it.
 */
export const Refused: Story = {
  render: () => (
    <Card>
      {EVERY.map(({ variant, label }) => (
        <Button key={variant} variant={variant} answer="refused" data-preview-held="">
          {label}
        </Button>
      ))}
    </Card>
  ),
};

/** A caller's half: pending while the act is out, then the answer until the next press. */
function Sending({
  label,
  waiting,
  answer,
  variant,
  onClick,
}: {
  label: string;
  waiting: string;
  answer: ButtonAnswer;
  variant: ButtonVariant;
  onClick?: ButtonProps["onClick"];
}) {
  const [pending, setPending] = useState(false);
  const [answered, setAnswered] = useState<ButtonAnswer>();
  return (
    <Button
      variant={variant}
      pending={pending}
      answer={answered}
      onClick={(event) => {
        onClick?.(event);
        setAnswered(undefined);
        setPending(true);
        setTimeout(() => {
          setPending(false);
          setAnswered(answer);
        }, 1200);
      }}
    >
      {pending ? waiting : label}
    </Button>
  );
}

/**
 * The whole line, pressed for real: press, pending, then Fleet's answer. One
 * control is accepted and one refused. What a rendering cannot show is that an
 * answered control is no longer waiting — it is not busy, and a refused act can
 * be pressed again and is sent again.
 */
export const Answering: Story = {
  args: { onClick: fn() },
  render: (args) => (
    <Card>
      <Sending
        variant="primary"
        label="Approve"
        waiting="Approving…"
        answer="accepted"
      />
      <Sending
        variant="secondary"
        label="Request changes"
        waiting="Requesting changes…"
        answer="refused"
        onClick={args.onClick}
      />
    </Card>
  ),
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Request changes" }));
    await expect(args.onClick).toHaveBeenCalledTimes(1);
    const waiting = canvas.getByRole("button", { name: "Requesting changes…" });
    await expect(waiting).toHaveAttribute("aria-busy", "true");

    const refused = await waitFor(() => canvas.getByRole("button", { name: "Request changes" }), {
      timeout: 3000,
    });
    await expect(refused).not.toHaveAttribute("aria-busy");
    await expect(refused).not.toHaveAttribute("aria-disabled");
    await userEvent.click(refused);
    await expect(args.onClick).toHaveBeenCalledTimes(2);
  },
};

/** Pending wins over a stale answer: the control waits, and a press sends nothing. */
export const PendingOverAnAnswer: Story = {
  args: { onClick: fn() },
  render: (args) => (
    <Card>
      <Button variant="secondary" pending answer="refused" onClick={args.onClick}>
        Requesting changes…
      </Button>
    </Card>
  ),
  play: async ({ args, canvas, userEvent }) => {
    const pressed = canvas.getByRole("button", { name: "Requesting changes…" });
    await expect(pressed).toHaveAttribute("aria-busy", "true");
    pressed.focus();
    await userEvent.keyboard("{Enter}");
    await expect(args.onClick).not.toHaveBeenCalled();
  },
};

/** sm — 32px, for use inside table rows. */
export const Small: Story = {
  render: () => (
    <Card>
      <Button size="sm">Review</Button>
      <Button size="sm">Open diff</Button>
      <Button size="sm" variant="ghost" iconOnly aria-label="Retry">
        <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
      </Button>
    </Card>
  ),
};

/** Every button in a group is the same height. A ghost recedes by losing its
 *  fill and dropping to `--fg-muted`, never by shrinking. */
export const Group: Story = {
  render: () => (
    <Card>
      <Button variant="primary">Approve</Button>
      <Button variant="secondary">Open diff</Button>
      <Button variant="ghost">Redirect</Button>
      <Button variant="destructive">Kill</Button>
    </Card>
  ),
};

/** A secondary is filled one surface step from its ground. On the sunken well
 *  below, `--bg-sunken` would make the button disappear into it. */
export const SecondaryOnASunkenGround: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
      <Card>
        <Button variant="secondary" ground="card">
          On a card
        </Button>
      </Card>
      <div
        style={{
          display: "flex",
          gap: "var(--space-3)",
          padding: "var(--pad-card)",
          borderRadius: "var(--radius-md)",
          background: "var(--bg-sunken)",
        }}
      >
        <Button variant="secondary" ground="sunken">
          On a sunken row
        </Button>
        <Button variant="secondary" ground="card">
          Wrong ground
        </Button>
      </div>
    </div>
  ),
};

/**
 * Dark is primary and a light story is the secondary case. No light theme
 * exists: `packages/tokens` declares one `:root` block and nothing keyed to a
 * theme, so this story renders dark. It is written so the gap is visible
 * rather than absent.
 */
export const Light: Story = {
  render: () => (
    <div data-theme="light">
      <Card>
        <Button variant="primary">Dispatch job</Button>
        <Button variant="secondary">Cancel</Button>
      </Card>
    </div>
  ),
};
