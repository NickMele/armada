import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ComponentProps } from "react";
import { ClipboardList, HardDrive, Megaphone } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { ActiveJobsList } from "../../compositions/ActiveJobsList/ActiveJobsList";
import { JobRowStacked } from "../../compositions/JobRowStacked/JobRowStacked";
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

/**
 * **A screen taller than the window, mounted the way Bridge mounts one.**
 *
 * This is the story that was missing, and its absence is why the layout kept
 * reading right here and wrong in the app. Every other screen story mounts a
 * screen on its own, into a box the story sized. Bridge mounts it into the
 * shell — under a head, over a status bar, inside a rail — and the chain from
 * the window down to the screen is exactly the part that was broken.
 *
 * **Nothing outside a pane scrolls.** The rail holds, the status bar holds, and
 * the tall content moves inside its own box. If this story ever scrolls the
 * whole shell — if the status bar leaves the bottom of the frame — the chain is
 * broken again, and it is broken in Bridge with it.
 *
 * No head, which is the shape a Job read whole takes: the screen's own header
 * is the top of the window.
 */
export const AScreenTallerThanTheWindow: Story = {
  args: {
    ...shell,
    title: undefined,
    summary: undefined,
    actions: undefined,
    children: (
      <div className="armada-screen__mounted">
      <div className="armada-screen__pane">
        {Array.from({ length: 40 }, (_, at) => (
          <div key={at} style={{ flex: "none" }}>
            {`Row ${at + 1} of 40 — the pane scrolls, the frame does not`}
          </div>
        ))}
      </div>
      </div>
    ),
  },
  render: Shell.render,
};

/**
 * **One Job on the Board, in a window with room for twenty.**
 *
 * A row is a row whether there is one of them or forty. The frame fills the
 * pane — that is what lets it scroll — and a grid stretches its rows into the
 * space it is given unless told not to, so a Board holding one Job drew that
 * Job a window tall with its content floating in the middle of it.
 *
 * What the pane does with the room left over is nothing, which is the correct
 * answer and the one a person expects.
 */
export const OneRowInATallWindow: Story = {
  args: {
    ...shell,
    // No summary on the head: Bridge puts the count on the list, beside the
    // control that changes it. The head keeps the surface's name, and the two
    // have to line up — which is what this story is now also for.
    summary: undefined,
    children: (
      <div className="armada-screen__mounted">
        <div className="armada-screen__stack">
          <ActiveJobsList summary="1 job needs you. 1 on the Board." selectable label="Job Board">
            <JobRowStacked
              status="escalated"
              statusIcon={Megaphone}
              statusLabel="went quiet"
              headline="Preserve job metadata during resource cleanup"
              jobId="01M21BKVPW002DC0ATD1X9T0VF"
              fields={[
                { label: "Workflow", value: "feature, 4 steps" },
                { label: "Progress", value: "scope" },
                { label: "Run time", value: "5h 40m", mono: true },
              ]}
              action={<Button variant="secondary" size="sm">Open</Button>}
            />
          </ActiveJobsList>
        </div>
      </div>
    ),
  },
  render: Shell.render,
};
