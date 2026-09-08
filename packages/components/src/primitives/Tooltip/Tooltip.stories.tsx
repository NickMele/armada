import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";
import { ACTION } from "../../actions";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../Table/Table";
import { Tooltip } from "./Tooltip";

/**
 * Approve's binding, read rather than typed. It was the literal `a` in both
 * stories below until `open_log` moved off `Enter` and `open_output` took `o`
 * on the same day — a map that moves is a map no story may hold a copy of.
 */
const APPROVE = ACTION.approve?.shortcut ?? undefined;

const meta: Meta<typeof Tooltip> = {
  title: "Primitives/Tooltip",
  component: Tooltip,
};
export default meta;

type Story = StoryObj<typeof Tooltip>;

/**
 * The state the contract names first: a secondary value truncates in a row and
 * does not vanish, and the tooltip carries the full string. No field is
 * dropped at any width.
 */
export const TruncatedValue: Story = {
  args: {
    defaultOpen: true,
    label: "crates/api/src/session/refresh_coalescing.rs",
    children: (
      <span className="armada-tooltip__truncated">
        crates/api/src/session/refresh_coalescing.rs
      </span>
    ),
  },
};

/**
 * The consequence the keyboard section records: a tooltip gains a trailing kbd
 * where the action has a binding. The delay is unchanged by it.
 */
export const WithShortcut: Story = {
  args: {
    defaultOpen: true,
    label: "Approve dispatch",
    shortcut: APPROVE,
    children: (
      <button type="button" className="armada-tooltip__action">
        Approve
      </button>
    ),
  },
};

/**
 * Closed. The 400ms delay is `--tooltip-delay` and the component reads it from
 * the token rather than carrying its own number — hover the value to see it.
 */
export const Resting: Story = {
  args: {
    label: "crates/api/src/session/refresh_coalescing.rs",
    children: (
      <span className="armada-tooltip__truncated">
        crates/api/src/session/refresh_coalescing.rs
      </span>
    ),
  },
};

const longPath = "crates/api/src/session/refresh_coalescing.rs";

/**
 * A frame that puts the trigger against an edge, so the flip is visible at a
 * size that fits on the page. `contain: layout` makes the frame the bubble's
 * containing block, which is the window in the app.
 */
function Frame({ edge, children }: { edge: "left" | "right" | "bottom"; children: React.ReactNode }) {
  return (
    <div className={`armada-tooltip-frame armada-tooltip-frame--${edge}`}>{children}</div>
  );
}

/**
 * On the leading edge, where the bubble's own alignment already fits. Nothing
 * flips — the placement is the preference and only the edge overrides it.
 */
export const AtTheLeftEdge: Story = {
  render: () => (
    <Frame edge="left">
      <Tooltip defaultOpen label={longPath}>
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </Frame>
  ),
};

/**
 * On the trailing edge. A path is long and the bubble is wide, so a
 * leading-aligned bubble would run past the window; the alignment flips.
 */
export const AtTheRightEdge: Story = {
  render: () => (
    <Frame edge="right">
      <Tooltip defaultOpen label={longPath}>
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </Frame>
  ),
};

/** No room below — the bubble opens above the value it describes. */
export const WithNoRoomBelow: Story = {
  render: () => (
    <Frame edge="bottom">
      <Tooltip defaultOpen label={longPath}>
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </Frame>
  ),
};

/**
 * **The first hover waits; the next one does not.** Crossing a row of annotated
 * chips used to mean one 400ms wait per chip, because the delay was held per
 * instance — so a reader scanning a run tree's facts either stopped on each one
 * or saw nothing. The group is warm for `--tooltip-grace` after any tooltip
 * closes, and inside that window the next opens on arrival.
 *
 * **What earns the assertion is that a rendering cannot show either half.** A
 * screenshot of an open bubble is the same picture whether it waited or not, so
 * the story hovers the first chip and reads nothing, waits it out, then hovers
 * the second and reads it with no wait at all.
 */
export const TheSecondHoverIsInstant: Story = {
  render: () => (
    <div className="armada-tooltip-row">
      <Tooltip label="Started at 14:22:07">
        <span className="armada-tooltip__truncated">6m 40s</span>
      </Tooltip>
      <Tooltip label="Started at 14:31:58">
        <span className="armada-tooltip__truncated">2m 04s</span>
      </Tooltip>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    const [first, second] = canvas.getAllByText(/^(6m 40s|2m 04s)$/);

    await userEvent.hover(first as HTMLElement);
    // Nothing yet. The delay is the whole reason a tooltip does not fire at
    // every value a pointer crosses on its way somewhere else.
    await expect(canvas.getByText("Started at 14:22:07")).not.toBeVisible();
    await waitFor(() => expect(canvas.getByText("Started at 14:22:07")).toBeVisible());

    await userEvent.unhover(first as HTMLElement);
    await userEvent.hover(second as HTMLElement);
    // No `waitFor`: the assertion is that it is already open.
    await expect(canvas.getByText("Started at 14:31:58")).toBeVisible();
  },
};

/**
 * **A tooltip nobody can reach describes nothing.** The bubble carried
 * `role="tooltip"` and no association, so it was announced by no assistive
 * technology at all, and a value that is not a control had no tab stop to open
 * it from.
 *
 * Both are fixed at the element a keyboard actually lands on: where the tooltip
 * wraps a control, the control takes the description and keeps its own single
 * stop; where it wraps a value, the wrapper takes the stop, because one is
 * owed and there was none.
 */
export const ReachedByKeyboardAndAnnounced: Story = {
  render: () => (
    <div className="armada-tooltip-row">
      <Tooltip label="Approve dispatch" shortcut={APPROVE}>
        <button type="button" className="armada-tooltip__action">
          Approve
        </button>
      </Tooltip>
      <Tooltip label="Click to copy the worktree name">
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    const approve = canvas.getByRole("button", { name: "Approve" });
    // The description is on the button, not on a wrapper around it: on an
    // ancestor it is announced by nothing.
    await expect(approve).toHaveAccessibleDescription(/Approve dispatch/);

    await userEvent.tab();
    await expect(approve).toHaveFocus();
    await waitFor(() => expect(canvas.getByText("Approve dispatch")).toBeVisible());

    // The value is not a control and still has to be reachable, so the wrapper
    // takes the stop. One stop per tooltip either way.
    await userEvent.tab();
    await waitFor(() =>
      expect(canvas.getByText("Click to copy the worktree name")).toBeVisible(),
    );
  },
};

/**
 * **A table cell cannot be wrapped, so `asChild` annotates it in place.** A
 * `span` around a `td` is not markup the DOM has; the header cell takes the
 * handlers and the description itself, and the bubble is appended inside it.
 * `JudgeVerdicts` is a table and every check list is one, so without this the
 * densest surfaces on job detail are the ones a hover cannot reach.
 */
export const OnATableCell: Story = {
  render: () => (
    <Table>
      <TableHead>
        <TableRow>
          <Tooltip
            asChild
            label="The criterion's frozen position in the brief. A citation names this, never the row's place on screen."
          >
            <TableHeaderCell scope="col">#</TableHeaderCell>
          </Tooltip>
          <TableHeaderCell scope="col">Criterion</TableHeaderCell>
        </TableRow>
      </TableHead>
      <TableBody>
        <TableRow>
          <TableCell variant="mono">01</TableCell>
          <TableCell>Selectors import without the store</TableCell>
        </TableRow>
      </TableBody>
    </Table>
  ),
  play: async ({ canvas, userEvent }) => {
    const header = canvas.getByRole("columnheader", { name: /#/ });
    // Still a cell of the table. A wrapper here would have put a span inside
    // the row and taken the column out of the grid.
    await expect(header.tagName).toBe("TH");
    await expect(header).toHaveAccessibleDescription(/frozen position/);

    await userEvent.hover(header);
    await waitFor(() => expect(canvas.getByText(/frozen position/)).toBeVisible());
  },
};
