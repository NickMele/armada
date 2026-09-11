import type { Meta, StoryObj } from "@storybook/react-vite";
import { boardJobs, boardWorkflows } from "../../../fixtures/build/board";
import { BoardFrom } from "./Board";

/**
 * The Board, drawn by the app's own `Shell`, `headOf` and `Jobs` from rows in
 * the shape `list_jobs` sends. Change the screen and this changes with it;
 * only the data is made up. `Screens/Job detail` is built the same way.
 */
const meta = {
  title: "Screens/Board",
  component: BoardFrom,
  parameters: { layout: "fullscreen" },
  args: { jobs: boardJobs(), workflows: boardWorkflows() },
} satisfies Meta<typeof BoardFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** One Job in every state the Board lists. */
export const EveryState: Story = { name: "Every state" };

/** Fleet holds no Jobs for this repository. */
export const Empty: Story = { name: "Empty", args: { jobs: [] } };

/** Fleet is not running, so the Board has nothing it can trust. */
export const FleetNotRunning: Story = {
  name: "Fleet not running",
  args: {
    jobs: [],
    connection: {
      state: "not_running",
      absence: {
        why: "no_runtime_file",
        path: "/Users/user/Library/Application Support/Armada/fleet.runtime.json",
      },
    },
  },
};
