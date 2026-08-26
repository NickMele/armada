import type { Meta, StoryObj } from "@storybook/react-vite";
import { BoardEmptyState } from "../../compositions/BoardEmptyState/BoardEmptyState";
import { Button } from "../../primitives/Button/Button";

/**
 * Bridge connects to a Fleet that has been running without it, and the list is
 * empty. Two readings, side by side, because a Fleet that is up with no work
 * and a Fleet that is not there are different states with different things to
 * do about them.
 *
 * **Not running and unreachable differ on the runtime file.** Fleet writes
 * port, pid and protocol version on startup and removes them on a clean exit,
 * so a missing file is a Fleet that is not there — which is why the second
 * state can name the command rather than reporting a timeout.
 */
const meta: Meta = {
  title: "Screens/First launch",
};
export default meta;

type Story = StoryObj;

export const FirstLaunch: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__row">
        <div className="armada-screen__card" data-width="half">
          <span className="armada-screen__caption">Fleet running, no jobs</span>
          <BoardEmptyState quiet action={<Button variant="primary">New job</Button>}>
            No jobs. Fleet has been up 6 days.
          </BoardEmptyState>
        </div>

        <div className="armada-screen__card" data-width="half">
          <span className="armada-screen__caption">Fleet is not running</span>
          <BoardEmptyState
            command="armada-fleet start"
            note="Run that in a terminal. Bridge connects on its own once the runtime file appears."
          >
            Fleet is not running. Bridge has nothing to read.
          </BoardEmptyState>
        </div>
      </div>
    </div>
  ),
};
