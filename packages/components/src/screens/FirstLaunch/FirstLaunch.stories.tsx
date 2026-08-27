import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "../../primitives/Button/Button";
import { FirstLaunch as FirstLaunchScreen } from "./FirstLaunch";

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
const meta: Meta<typeof FirstLaunchScreen> = {
  title: "Screens/First launch",
  component: FirstLaunchScreen,
};
export default meta;

type Story = StoryObj<typeof FirstLaunchScreen>;

export const FirstLaunch: Story = {
  render: () => (
    <div className="armada-screen">
      <FirstLaunchScreen
        running={{
          caption: "Fleet running, no jobs",
          quiet: true,
          action: <Button variant="primary">New job</Button>,
          children: "No jobs. Fleet has been up 6 days.",
        }}
        notRunning={{
          caption: "Fleet is not running",
          command: "armada-fleet start",
          note: "Run that in a terminal. Bridge connects on its own once the runtime file appears.",
          children: "Fleet is not running. Bridge has nothing to read.",
        }}
      />
    </div>
  ),
};
