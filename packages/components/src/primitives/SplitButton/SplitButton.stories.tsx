import type { Meta, StoryObj } from "@storybook/react-vite";
import { Plus } from "lucide-react";
import { expect, fn } from "storybook/test";
import { SplitButton, type SplitButtonProps } from "./SplitButton";
import { holdDurationOf } from "../HoldButton/useHold";

const meta: Meta<typeof SplitButton> = {
  title: "Primitives/Split button",
  component: SplitButton,
};
export default meta;

type Story = StoryObj<typeof SplitButton>;

/** A sunken row — the ground a list's split button actually sits on. The
 *  wrapper takes no overflow, so an open menu is never clipped by it. */
function Row({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: "var(--space-2)",
        padding: "var(--space-3)",
        borderRadius: "var(--radius-sm)",
        background: "var(--bg-sunken)",
      }}
    >
      {children}
    </div>
  );
}

const reviewActions = [
  { label: "Reject" },
  { label: "Redispatch with notes" },
  { label: "Open diff", shortcut: "d" },
];

/** Closed — the default. Two segments in one control: the label commits, the
 *  caret offers the alternatives. */
export const Closed: Story = {
  render: () => (
    <Row>
      <SplitButton items={reviewActions} ground="sunken">
        Approve
      </SplitButton>
    </Row>
  ),
};

/** Open. The menu holds what the row could also do — never a repeat of the
 *  label, and never an item that says Open, since clicking the row does that. */
export const Open: Story = {
  render: () => (
    <Row>
      <SplitButton items={reviewActions} ground="sunken" defaultOpen>
        Approve
      </SplitButton>
    </Row>
  ),
};

/** The label is the act the state calls for, so it changes with the Job. The
 *  menu carries the rest in the header's order — destructive last. */
export const EscalatedRow: Story = {
  render: () => (
    <Row>
      <SplitButton
        ground="sunken"
        defaultOpen
        items={[{ label: "Kill & Redispatch" }, { label: "Kill", danger: true, shortcut: "x" }]}
      >
        Pilot
      </SplitButton>
    </Row>
  ),
};

/** The ring goes round whichever segment holds focus — the whole segment, not
 *  the seam between the two. It rounds on the outer corners and stays square on
 *  the joined edge, sits at `--focus-ring-offset` so it differs from the resting
 *  edge in position as well as colour and width, and draws over its neighbour
 *  rather than under it. Neither segment changes size, so the group stays one
 *  height. `data-preview-focus` selects the same declarations as
 *  `:focus-visible`, which a static story cannot reach. */
export const Focused: Story = {
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-4)" }}>
      <div data-preview-focus="action">
        <Row>
          <SplitButton items={reviewActions} ground="sunken">
            Approve
          </SplitButton>
        </Row>
      </div>
      <div data-preview-focus="caret">
        <Row>
          <SplitButton items={reviewActions} ground="sunken">
            Approve
          </SplitButton>
        </Row>
      </div>
    </div>
  ),
};

/** `--fg-subtle` on both segments, hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Row>
      <SplitButton items={reviewActions} ground="sunken" disabled>
        Approve
      </SplitButton>
    </Row>
  ),
};

/**
 * The act this control opened a dialog for is out, and Fleet has not
 * answered. The face sweeps a bar and stays focusable — `aria-busy`, not
 * `disabled` — and swallows a second press; the caret goes off and offers
 * no menu, since there is nothing else to disclose while one press is
 * already on its way. #1117.
 */
export const Pending: Story = {
  args: {
    items: reviewActions,
    ground: "sunken",
    pending: true,
    pendingLabel: "Killing job…",
    onAction: fn(),
    children: "Kill job",
  } as Story["args"],
  render: (args) => (
    <Row>
      <SplitButton {...args} />
    </Row>
  ),
  play: async ({ args, canvas, userEvent }) => {
    const face = canvas.getByRole("button", { name: "Killing job…" });
    await expect(face).toHaveAttribute("aria-busy", "true");
    await expect(face).not.toBeDisabled();
    await expect(canvas.getByRole("button", { name: "More actions" })).toBeDisabled();

    // `[aria-disabled="true"]` carries `pointer-events: none` — the global
    // rule refuses the pointer before a click ever reaches this control's own
    // guard. Keyboard activation does not go through that rule, so `Enter` on
    // the focused, still-focusable face is what proves the second press is
    // swallowed rather than sent twice.
    face.focus();
    await userEvent.keyboard("{Enter}");
    await expect(args.onAction).not.toHaveBeenCalled();
  },
};

/** Every variant, each on the ground it is drawn against in the stories above. */
const EVERY: SplitButtonProps["variant"][] = ["secondary", "primary", "destructive", "tonal"];

/** One row holding a split button per variant, wrapped for a preview attribute. */
function EveryVariant({
  preview,
  ...props
}: Omit<SplitButtonProps, "children" | "items"> & { preview?: Record<string, string> }) {
  return (
    <div {...preview}>
      <Row>
        {EVERY.map((variant) => (
          <SplitButton key={variant} {...props} variant={variant} items={reviewActions}>
            {variant === "destructive" ? "Kill job" : "Approve"}
          </SplitButton>
        ))}
      </Row>
    </div>
  );
}

/**
 * Disabled, one control per variant. Each carries its own act and its own menu
 * name so the `play` below can name the segment it reads rather than count
 * them.
 */
const DISABLED_VARIANTS: {
  variant: SplitButtonProps["variant"];
  label: string;
  menuLabel: string;
}[] = [
  { variant: "secondary", label: "Approve", menuLabel: "More for approve" },
  { variant: "primary", label: "Approve all", menuLabel: "More for approve all" },
  { variant: "destructive", label: "Kill job", menuLabel: "More for kill job" },
  { variant: "tonal", label: "Dispatch", menuLabel: "More for dispatch" },
];

/**
 * The colour a token resolves to inside this canvas, read rather than typed.
 * The assertion below compares two computed values, so no hex is spelled in a
 * story and `packages/tokens` stays the one place the value lives.
 */
function tokenColour(within: HTMLElement, token: string) {
  const probe = document.createElement("span");
  probe.style.color = `var(${token})`;
  within.append(probe);
  const colour = getComputedStyle(probe).color;
  probe.remove();
  return colour;
}

/**
 * Disabled, every variant. A disabled control has no variant left to express:
 * all four collapse onto `--fg-subtle` on `--bg-sunken`, as `Button` does, and
 * the caret's chevron dims with the label beside it.
 *
 * **This one colour earns a `play` because the rule was there and losing.**
 * `primary`, `destructive` and `tonal` each paint their segments through a
 * descendant selector, which outweighed the disabled rule on the segment
 * itself — so the ground went dead while the text stayed bright, and every
 * story that drew it looked plausible next to the secondary one that worked.
 * Reading the computed colour is what tells the two apart.
 */
export const DisabledEveryVariant: Story = {
  render: () => (
    <Row>
      {DISABLED_VARIANTS.map(({ variant, label, menuLabel }) => (
        <SplitButton
          key={variant}
          variant={variant}
          ground="sunken"
          items={reviewActions}
          menuLabel={menuLabel}
          disabled
        >
          {label}
        </SplitButton>
      ))}
    </Row>
  ),
  play: async ({ canvas, canvasElement }) => {
    const subtle = tokenColour(canvasElement, "--fg-subtle");

    const drawn: Record<string, { label: string; chevron: string }> = {};
    const disabled: Record<string, { face: boolean; caret: boolean }> = {};
    for (const { variant, label, menuLabel } of DISABLED_VARIANTS) {
      const face = canvas.getByRole("button", { name: label });
      const caret = canvas.getByRole("button", { name: menuLabel });
      // The chevron is drawn in `currentColor`, so its own computed colour is
      // what a person sees, not the colour the button was asked for.
      const chevron = caret.querySelector("svg");
      await expect(chevron).not.toBeNull();
      disabled[variant as string] = {
        face: (face as HTMLButtonElement).disabled,
        caret: (caret as HTMLButtonElement).disabled,
      };
      drawn[variant as string] = {
        label: getComputedStyle(face).color,
        chevron: getComputedStyle(chevron as SVGElement).color,
      };
    }

    // One assertion over all four, so a failure names every variant that kept
    // its live colour rather than stopping at the first.
    await expect(drawn).toEqual(
      Object.fromEntries(
        DISABLED_VARIANTS.map(({ variant }) => [variant, { label: subtle, chevron: subtle }]),
      ),
    );
    await expect(disabled).toEqual(
      Object.fromEntries(
        DISABLED_VARIANTS.map(({ variant }) => [variant, { face: true, caret: true }]),
      ),
    );
  },
};

/**
 * Pressed, every variant — the face on the first row, the caret on the second.
 * A segment walks one step down its ground at `--duration-press` and the face's
 * line opens from the centre; the caret carries no line, since it sends
 * nothing. `data-preview-press` selects the same declarations as `:active`.
 */
export const Pressed: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
      <EveryVariant preview={{ "data-preview-press": "action" }} />
      <EveryVariant preview={{ "data-preview-press": "caret" }} />
    </div>
  ),
};

/** Pending, every variant: one rendering, with the line travelling the face. */
export const PendingEveryVariant: Story = {
  render: () => <EveryVariant pending />,
};

/**
 * Fleet accepted, every variant. The face's line fills the edge in
 * `--status-completed-success` while the control keeps the pending rendering,
 * held for `--duration-answer` in the app and on a loop here.
 */
export const Accepted: Story = {
  render: () => <EveryVariant answer="accepted" preview={{ "data-preview-held": "" }} />,
};

/** Fleet refused, every variant. The line retracts in `--status-escalated`; nothing else moves. */
export const Refused: Story = {
  render: () => <EveryVariant answer="refused" preview={{ "data-preview-held": "" }} />,
};

/**
 * Answered is not waiting. Where `Pending` swallows a press and closes the
 * caret, a refused face takes the press again and the menu opens again.
 */
export const RefusedTakesAPressAgain: Story = {
  args: {
    items: reviewActions,
    ground: "sunken",
    answer: "refused",
    onAction: fn(),
    children: "Kill job",
  } as Story["args"],
  render: (args) => (
    <Row>
      <SplitButton {...args} />
    </Row>
  ),
  play: async ({ args, canvas, userEvent }) => {
    const face = canvas.getByRole("button", { name: "Kill job" });
    await expect(face).not.toHaveAttribute("aria-busy");
    await userEvent.click(face);
    await expect(args.onAction).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "More actions" }));
    await expect(canvas.getByRole("menu")).toBeVisible();
  },
};

/**
 * On job detail there is one Job and one primary, so the control may take the
 * accent. A list row never does.
 */
export const PrimaryOnJobDetail: Story = {
  render: () => (
    <div
      style={{
        display: "flex",
        gap: "var(--space-3)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)",
      }}
    >
      <SplitButton items={reviewActions} variant="primary">
        Approve
      </SplitButton>
    </div>
  ),
};

/**
 * Dark is primary and a light story is the secondary case. No light theme
 * exists in `packages/tokens`, so this renders dark — written so the gap is
 * visible rather than absent.
 */
export const Light: Story = {
  render: () => (
    <div data-theme="light">
      <Row>
        <SplitButton items={reviewActions} ground="sunken">
          Approve
        </SplitButton>
      </Row>
    </div>
  ),
};

/**
 * Focus on the accent fill. The ring clears the control rather than sitting on
 * it, which is the only thing separating it from the resting edge here — on
 * `primary` both are `--accent`, so colour and width say nothing and position
 * carries the reading alone. See the report.
 */
export const FocusedOnPrimary: Story = {
  render: () => (
    <div className="armada-split-button-focus-row">
      <div data-preview-focus="action">
        <SplitButton items={reviewActions} variant="primary">
          Approve
        </SplitButton>
      </div>
      <div data-preview-focus="caret">
        <SplitButton items={reviewActions} variant="primary">
          Approve
        </SplitButton>
      </div>
    </div>
  ),
};

/**
 * Tonal — chrome, not a list row's act. `--accent-muted` fill and
 * `--accent-hover` text, no outer border, only the inner divider. Drawn on
 * `--bg-sunken`, the title row's own ground, since tonal reads against that
 * surface and nowhere else yet.
 */
export const Tonal: Story = {
  render: () => (
    <Row>
      <SplitButton items={reviewActions} variant="tonal">
        Dispatch
      </SplitButton>
    </Row>
  ),
};

/**
 * Nothing behind the caret yet. `items={[]}` is a real mode, not an empty
 * menu: the caret calls `onAction` directly rather than popping a floating
 * box with nothing in it. The title row's Dispatch is the one caller — both
 * segments read as the same control because they are. The leading `plus` is
 * that same caller's icon (#1107, `packages/icons/icons.toml`) — `aria-hidden`,
 * so it never joins the accessible name the `play` below asserts on.
 */
export const TonalNoMenu: Story = {
  args: {
    items: [],
    variant: "tonal",
    menuLabel: "Dispatch a job",
    onAction: fn(),
  },
  render: (args) => (
    <Row>
      <SplitButton {...args} icon={<Plus size={16} strokeWidth={2} aria-hidden />}>
        Dispatch
      </SplitButton>
    </Row>
  ),
  /** What a rendering cannot show: the caret does not open a menu, it fires
   *  the same handler the label does. Asserted by call count and by the
   *  menu's absence, never by reading `noMenu` off the markup. */
  play: async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Dispatch a job" }));
    await expect(args.onAction).toHaveBeenCalledTimes(1);
    expect(canvas.queryByRole("menu")).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "Dispatch" }));
    await expect(args.onAction).toHaveBeenCalledTimes(2);
  },
};

const killActions = [{ label: "Kill job, it ends here", danger: true }];

const holdArgs = {
  items: killActions,
  menuLabel: "Everything else this job can do",
  onAction: fn(),
  hold: {
    label: "Hold to kill drone",
    description: "Kills the drone once held until it fills. Letting go sooner kills nothing. The job stays open.",
    onCommit: fn(),
  },
  children: "Kill drone",
} as Story["args"];

/**
 * A face that confirms in place — `Kill drone` on a running Job's header. The
 * face fills while held and commits once held for `--duration-hold`; a click is
 * a press let go at once, so it kills nothing and asks nothing. **The caret is
 * still only a menu trigger**: pressing and holding it starts no hold.
 */
export const HoldFace: Story = {
  args: holdArgs,
  render: (args) => (
    <Row>
      <SplitButton {...args} />
    </Row>
  ),
  play: async ({ args, canvas, userEvent }) => {
    const face = canvas.getByRole("button", { name: "Hold to kill drone" });
    await expect(face).toHaveAccessibleDescription(args.hold?.description ?? "");
    const hold = holdDurationOf(face) ?? 0;
    await expect(hold).toBeGreaterThan(0);

    await userEvent.click(face);
    const caret = canvas.getByRole("button", { name: "Everything else this job can do" });
    // Pressed and held past the hold's length: a caret that armed would commit here.
    caret.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, isPrimary: true }));
    await new Promise((resolve) => setTimeout(resolve, hold + 100));
    caret.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, button: 0, isPrimary: true }));
    await userEvent.click(caret);
    await expect(args.hold?.onCommit).not.toHaveBeenCalled();
    await expect(args.onAction).not.toHaveBeenCalled();
    await expect(canvas.getByRole("menu")).toBeVisible();
  },
};

/** Held halfway, seeded so the fill does not depend on when the screenshot lands. No line opens under it. */
export const HoldFaceArming: Story = {
  args: holdArgs,
  render: (args) => (
    <div data-preview-held="" data-preview-press="action">
      <Row>
        <SplitButton {...args} />
      </Row>
    </div>
  ),
};

/**
 * Under `prefers-reduced-motion` the hold is not offered: the face reads as the
 * act and a press calls `onAction`, which is the dialog, because the fill is
 * the only thing that says how long is left.
 */
export const HoldFaceReducedMotion: Story = {
  args: holdArgs,
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
  render: (args) => (
    <Row>
      <SplitButton {...args} />
    </Row>
  ),
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Kill drone" }));
    await expect(args.onAction).toHaveBeenCalledTimes(1);
    await expect(args.hold?.onCommit).not.toHaveBeenCalled();
  },
};
