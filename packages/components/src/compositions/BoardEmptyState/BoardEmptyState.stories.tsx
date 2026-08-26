import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "../../primitives/Button/Button";
import { BoardEmptyState } from "./BoardEmptyState";

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
