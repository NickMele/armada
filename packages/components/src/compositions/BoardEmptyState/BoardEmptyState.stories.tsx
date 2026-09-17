import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { actionOf } from "../../actions";
import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { BoardEmptyState } from "./BoardEmptyState";

const DISPATCH = actionOf("new_job");

/**
 * The two states first launch draws, side by side in the drawing because they
 * are the two readings of an empty list: Fleet is up and there is no work, or
 * Fleet is not there at all.
 *
 * **Not running and unreachable differ on the runtime file**, which is why the
 * second state can say "not running" rather than "no connection". Fleet writes
 * port, pid and protocol version on startup and removes them on a clean exit,
 * so a missing file is a Fleet that is not there.
 */
const meta: Meta<typeof BoardEmptyState> = {
  title: "Compositions/Board empty state",
  component: BoardEmptyState,
};
export default meta;

type Story = StoryObj<typeof BoardEmptyState>;

/**
 * Fleet running, no jobs. One line and the action — the uptime is in the line
 * because it is the fact that turns "nothing here" into "nothing here, and
 * that is not a fault".
 *
 * The `New job` button is the one control, and it is a primary because this
 * region is the whole view: an empty board has one act available.
 */
export const FleetRunningNoJobs: Story = {
  args: {
    quiet: true,
    children: "No jobs. Fleet has been up 6 days.",
    action: <Button variant="primary">New job</Button>,
  },
};

/**
 * Fleet is not running — the state a person will actually meet in M1, since
 * Fleet is started by hand.
 *
 * **The command is a value to copy, not a button.** Bridge does not start
 * Fleet at this milestone, so it names the command and says what happens once
 * the runtime file appears rather than offering a control that cannot act.
 */
export const FleetIsNotRunning: Story = {
  args: {
    children: "Fleet is not running. Bridge has nothing to read.",
    command: "armada-fleet start",
    note: "Run that in a terminal. Bridge connects on its own once the runtime file appears.",
  },
};

const compose = fn();

/**
 * Overview with no Jobs — `design-system.md`, Overview empty state (#1262).
 * A glass card rather than a well, because Overview's middle is a column of
 * cards; the fact over what to do about it, and Dispatch beneath as the view's
 * one Primary. The title row's Dispatch stays Tonal.
 *
 * **`n` is drawn on the button, and it is not part of its name.** The key is
 * reference material beside the label, so the button still reads "Dispatch" —
 * `Dialog`'s own rule for the key drawn where it fires. Verb and key both come
 * from the registry's `new_job`.
 */
export const OverviewNoJobs: Story = {
  args: {
    card: true,
    quiet: true,
    lead: "No jobs.",
    children: "Propose one.",
    action: (
      <Button variant="primary" onClick={compose}>
        {DISPATCH.verb}
        <Kbd aria-hidden>{DISPATCH.shortcut}</Kbd>
      </Button>
    ),
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText("No jobs.")).toBeVisible();
    await expect(canvas.getByText("Propose one.")).toBeVisible();
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(dispatch).toHaveTextContent("n");
    compose.mockClear();
    await userEvent.click(dispatch);
    await expect(compose).toHaveBeenCalledOnce();
  },
};
