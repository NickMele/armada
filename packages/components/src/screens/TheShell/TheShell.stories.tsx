import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ComponentProps } from "react";
import { ClipboardList, HardDrive } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { TheShell } from "./TheShell";

/**
 * Rail, panel, status bar — with the surfaces in the rail that are built,
 * which is two of the five the concept page fixes. The rest hold their place
 * in the order and their digit and draw nothing.
 *
 * The values are the drawing's own: pid 4417, port 7411, six jobs, one of them
 * waiting on you. **Two of the drawing's are not here.** `today ~$4.80` has
 * nothing behind it — nothing measures spend — and `1 drone` has nothing
 * behind it either, because `assigned_drone` has no event that sets it. Both
 * are left out rather than drawn as a labelled blank.
 */
const meta: Meta<typeof TheShell> = {
  title: "Screens/The shell",
  component: TheShell,
};
export default meta;

type Story = StoryObj<typeof TheShell>;

const shell: ComponentProps<typeof TheShell> = {
  railHeader: (
    <Select aria-label="Project">
      <option>armada</option>
    </Select>
  ),
  // The drawing's rail row carried a label and a count and no glyph, and drew
  // Active jobs — a surface retired when the Board became every Job with state
  // as a filter. This is the roster now, with the registry's own glyph for
  // each. Only the Board carries a count.
  surfaces: [
    { id: "board", label: "Job Board", icon: ClipboardList, count: 6 },
    { id: "worktrees", label: "Held worktrees", icon: HardDrive },
  ],
  activeId: "board",
  title: "Job Board",
  summary: "6 jobs. 1 awaiting approval.",
  actions: <Button variant="primary">New job</Button>,
  children: <div className="armada-screen__mount">The list mounts here — 1d</div>,
  status: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411",
    items: ["6 jobs"],
    approvals: 1,
  },
};

export const Shell: Story = {
  args: shell,
  render: (args) => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <TheShell {...args} />
      </div>
    </div>
  ),
};

/**
 * The 48px icon rail, below the layout breakpoint. The rail never disappears:
 * losing navigation entirely is worse than losing 48px at any width.
 */
export const CollapsedRail: Story = {
  args: { ...shell, collapsed: true },
  render: Shell.render,
};

/**
 * Fleet down. The bar is present when Fleet is, and says which of the two
 * failures this is — a missing runtime file and a live pid that does not
 * answer call for different things.
 */
export const FleetIsNotRunning: Story = {
  args: {
    ...shell,
    summary: "No jobs.",
    surfaces: [
      { id: "board", label: "Job Board", icon: ClipboardList, count: 0 },
      { id: "worktrees", label: "Held worktrees", icon: HardDrive },
    ],
    status: {
      fleet: "not-running",
      fleetLabel: "Fleet is not running",
      detail: "no runtime file at ~/Library/Application Support/Armada/fleet.json",
      advice: "Start Fleet. Bridge reconnects on its own.",
    },
  },
  render: Shell.render,
};
