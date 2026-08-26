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
          <span className="armada-screen__caption">
            One line and the action. No centred glyph, no illustration — the empty state
            points at the work available and nothing else.
          </span>
        </div>

        <div className="armada-screen__card" data-width="half">
          <span className="armada-screen__caption">Fleet is not running</span>
          <BoardEmptyState
            command="armada-fleet start"
            note="Run that in a terminal. Bridge connects on its own once the runtime file appears."
          >
            Fleet is not running. Bridge has nothing to read.
          </BoardEmptyState>
          <span className="armada-screen__caption">
            The state a person will actually meet in M1, since Fleet is started by hand. The
            command is machine-derived, so it is mono, and it is a value to copy rather than
            a button — Bridge does not start Fleet at this milestone.
          </span>
        </div>
      </div>
    </div>
  ),
};
