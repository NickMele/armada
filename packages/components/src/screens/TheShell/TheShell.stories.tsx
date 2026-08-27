import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ComponentProps } from "react";
import { Activity } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { TheShell } from "./TheShell";

/**
 * Rail, panel, status bar — with one surface in the rail, because one surface
 * exists.
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
  // The drawing's rail row carries a label and a count and no glyph. Sidebar
  // requires one, and `activity` is what the registry assigns to Active jobs,
  // so that is the glyph. Reported.
  surfaces: [{ id: "active", label: "Active jobs", icon: Activity, count: 6 }],
  activeId: "active",
  title: "Active jobs",
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
    surfaces: [{ id: "active", label: "Active jobs", icon: Activity, count: 0 }],
    status: {
      fleet: "not-running",
      fleetLabel: "Fleet is not running",
      detail: "no runtime file at ~/Library/Application Support/Armada/fleet.json",
      advice: "Start Fleet. Bridge reconnects on its own.",
    },
  },
  render: Shell.render,
};
