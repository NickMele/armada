import type { Meta, StoryObj } from "@storybook/react-vite";
import { SplitButton } from "./SplitButton";

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
