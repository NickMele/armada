import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState, type ComponentProps } from "react";
import { expect } from "storybook/test";
import { ClipboardList, HardDrive, Megaphone } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { ActiveJobsList } from "../ActiveJobsList/ActiveJobsList";
import { DockQuestions } from "../DockQuestions/DockQuestions";
import { JobRowStacked } from "../JobRowStacked/JobRowStacked";
import { Select } from "../../primitives/Select/Select";
import { TheShell } from "./TheShell";

/**
 * The left column, panel and dock — with the surfaces in Navigation that are
 * built, which is two of the five the concept page fixes.
 *
 * The values are the drawing's own: pid 4417, port 7411, six jobs, one of them
 * waiting on you. `today ~$4.80` is left out — nothing measures spend.
 */
const meta: Meta<typeof TheShell> = {
  title: "Compositions/The shell",
  component: TheShell,
};
export default meta;

type Story = StoryObj<typeof TheShell>;

const shell: ComponentProps<typeof TheShell> = {
  // The picker moved out of the rail and into the title row with #1087.
  repositoryPicker: (
    <Select aria-label="Project">
      <option>armada</option>
    </Select>
  ),
  onSearch: () => {},
  onDispatch: () => {},
  // The drawing's rail row carried a label and a count and no glyph, and drew
  // Active jobs — a surface retired when the Board became every Job with state
  // as a filter. This is the roster now, with the registry's own glyph for
  // each. Only the Board carries a count.
  surfaces: [
    { id: "board", label: "Job Board", icon: ClipboardList, count: 6 },
    { id: "worktrees", label: "Cleanup", icon: HardDrive },
  ],
  activeId: "board",
  children: <div className="armada-screen__mount">The list mounts here — 1d</div>,
  stats: {
    rows: [
      { id: "approval", label: "Awaiting approval", value: 1, tone: "warn", hue: "status-awaiting-review", idle: false },
      { id: "review", label: "Needs review", value: 0, hue: "status-awaiting-review", idle: true },
      { id: "escalated", label: "Escalated", value: 0, hue: "status-escalated", idle: true },
      { id: "jobs", label: "Jobs", value: 6, hue: "status-not-started", idle: false },
      { id: "drones", label: "Drones", value: "1 of 2", hue: "stat-drones", idle: false },
      { id: "manifest", label: "Manifest", value: "Current", hue: "stat-manifest-current", idle: true },
    ],
    open: true,
    onOpenChange: () => {},
  },
  fleet: {
    state: "running",
    label: "Running",
    detail: "pid 4417 · port 7411",
    doctor: { outcome: "pass", checked: "Fleet, SQLite, Manifest, System stats" },
    open: true,
    onOpenChange: () => {},
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
 * Fleet down. The Fleet panel says which of the two failures this is — a
 * missing runtime file and a live pid that does not answer call for
 * different things.
 */
export const FleetIsNotRunning: Story = {
  args: {
    ...shell,
    surfaces: [
      { id: "board", label: "Job Board", icon: ClipboardList, count: 0 },
      { id: "worktrees", label: "Cleanup", icon: HardDrive },
    ],
    fleet: {
      state: "not-running",
      label: "Not running",
      detail: "no runtime file at ~/Library/Application Support/Armada/fleet.json",
      open: true,
      onOpenChange: () => {},
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
 * shell, beside the left column, and the chain from the window down to the
 * screen is exactly the part that was broken.
 *
 * **Nothing outside a pane scrolls.** The left column holds, and the tall
 * content moves inside its own box. If this story ever scrolls the whole
 * shell, the chain is broken again, and it is broken in Bridge with it.
 */
export const AScreenTallerThanTheWindow: Story = {
  args: {
    ...shell,
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
              handle="9-preserve-job-metadata-cleanup"
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

/** The left column's own width, so a story can drag and nudge what the app drags and nudges. */
function WithResizableLeftColumn(args: ComponentProps<typeof TheShell>) {
  const [leftWidth, setLeftWidth] = useState(args.leftWidth ?? 200);
  return <TheShell {...args} leftWidth={leftWidth} onResizeLeft={setLeftWidth} />;
}

/**
 * The left column's trailing edge, grabbable and keyboard-operable — one
 * handle for Navigation, Stats and Fleet together, dragged wider, dragged to
 * `--sidebar-min`, or nudged by the arrow keys the ARIA separator pattern
 * names. `onResizeLeft` absent draws no handle at all: `Shell` above is that
 * case.
 */
export const LeftColumnResizable: Story = {
  args: { ...shell, leftWidth: 200, onResizeLeft: () => {} },
  render: (args) => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <WithResizableLeftColumn {...args} />
      </div>
    </div>
  ),
  play: async ({ canvas, canvasElement, userEvent }) => {
    // Navigation, Stats and Fleet are one column, so each is as wide as the others.
    const panelWidths = () =>
      [...(canvasElement.querySelector(".armada-shell__left")?.children ?? [])].map(
        (panel) => panel.getBoundingClientRect().width,
      );
    const handle = canvas.getByRole("separator", { name: "Resize the left column" });
    await expect(handle).toHaveAttribute("aria-valuenow", "200");
    await userEvent.click(handle);
    await expect(handle).toHaveFocus();
    await userEvent.keyboard("{ArrowRight}");
    await expect(handle).toHaveAttribute("aria-valuenow", "216");
    await expect(new Set(panelWidths()).size).toBe(1);
    await userEvent.keyboard("{ArrowLeft}{ArrowLeft}");
    await expect(handle).toHaveAttribute("aria-valuenow", "184");
    await expect(new Set(panelWidths()).size).toBe(1);
  },
};

/**
 * Below the breakpoint the rail collapses to 48px and there is nothing left
 * to drag — the handle draws at all, the same way the dock's own disappears
 * while folded.
 */
export const LeftColumnCollapsedHasNoHandle: Story = {
  args: { ...shell, collapsed: true, leftWidth: 200, onResizeLeft: () => {} },
  render: LeftColumnResizable.render,
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("separator", { name: "Resize the left column" })).not.toBeInTheDocument();
  },
};

/** The dock's own open state, so a story can press what the app presses. */
function WithDock(args: ComponentProps<typeof TheShell>) {
  const [open, setOpen] = useState(args.dock?.open ?? false);
  const dock = args.dock === undefined ? undefined : { ...args.dock, open, onOpen: setOpen };
  return <TheShell {...args} dock={dock} />;
}

/** The dock's own width, so a story can drag and nudge what the app drags and nudges. */
function WithResizableDock(args: ComponentProps<typeof TheShell>) {
  const [open, setOpen] = useState(args.dock?.open ?? false);
  const [width, setWidth] = useState(args.dock?.width ?? 380);
  const dock = args.dock === undefined ? undefined : { ...args.dock, open, width, onOpen: setOpen, onResize: setWidth };
  return <TheShell {...args} dock={dock} />;
}

/** At `--layout-breakpoint` and wider: Helm's dock beside the content, on every surface. */
export const DockBesideTheContent: Story = {
  args: { ...shell, dock: { open: true, binding: "⌘J", onOpen: () => {} } },
  render: (args) => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <WithDock {...args} />
      </div>
    </div>
  ),
};

/**
 * At `--layout-breakpoint` and wider, closed: no strip, no residue — #1094.
 * The title row's own Helm button is the only way back, and pressing it
 * reopens the dock beside the content.
 */
export const DockClosedAtWidth: Story = {
  args: { ...shell, dock: { open: false, binding: "⌘J", onOpen: () => {} } },
  render: DockBesideTheContent.render,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByLabelText("Helm")).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: /Open Helm/ })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Helm" }));
    await expect(canvas.getByLabelText("Helm")).toBeVisible();
  },
};

/** Questions from two repositories in the dock's questions zone, above where Helm's conversation will sit. */
export const DockWithQuestions: Story = {
  args: {
    ...shell,
    dock: {
      open: true,
      binding: "⌘J",
      questions: 2,
      onOpen: () => {},
      children: (
        <DockQuestions
          questions={[
            {
              id: "b:judge",
              repository: "shop-01",
              job: "3",
              title: "Checkout total ignores the discount code",
              label: "Judge refused a criterion and is asking you",
              asked: "Does the fix address the cause the note names?",
              detail: "A customer with a valid code is still charged the full price.",
              waiting: "22m",
              answers: [
                { id: "agree", label: "Agree with the refusal" },
                { id: "disagree_once", label: "Disagree, just this step" },
                { id: "disagree_always", label: "Always disagree" },
              ],
              note: "Open job 3 to answer.",
            },
            {
              id: "a:command",
              repository: "armada",
              job: "12",
              title: "The drone count is wrong after a restart",
              label: "The drone wants to run a command it was not given",
              asked: <span className="mono">cargo nextest run -p store</span>,
              waiting: "1m",
              answers: [
                { id: "allow_for_job", label: "Allow for this job" },
                { id: "always_allow", label: "Always allow in this repository" },
                { id: "reject", label: "Reject" },
              ],
              note: "Open job 12 to answer.",
            },
          ]}
        />
      ),
    },
  },
  render: DockBesideTheContent.render,
};

/**
 * The dock's leading edge, grabbable and keyboard-operable — dragged wider,
 * dragged to `--w-dock-min`, or nudged by the arrow keys the ARIA separator
 * pattern names. `onResize` absent draws no handle at all: `DockBesideTheContent`
 * above is that case.
 */
export const DockResizable: Story = {
  args: { ...shell, dock: { open: true, binding: "⌘J", width: 380, onOpen: () => {}, onResize: () => {} } },
  render: (args) => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <WithResizableDock {...args} />
      </div>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    const handle = canvas.getByRole("separator", { name: "Resize Helm" });
    await expect(handle).toHaveAttribute("aria-valuenow", "380");
    await userEvent.click(handle);
    await expect(handle).toHaveFocus();
    await userEvent.keyboard("{ArrowLeft}");
    await expect(handle).toHaveAttribute("aria-valuenow", "396");
    await userEvent.keyboard("{ArrowRight}{ArrowRight}");
    await expect(handle).toHaveAttribute("aria-valuenow", "364");
  },
};

/** Below the breakpoint: the dock folds to an edge strip carrying the questions waiting, and opens as a sheet. */
export const DockFoldedToAStrip: Story = {
  args: {
    ...shell,
    collapsed: true,
    dock: { open: false, folded: true, questions: 3, binding: "⌘J", onOpen: () => {} },
  },
  render: (args) => (
    <div className="armada-screen" style={{ width: "var(--layout-breakpoint-narrow)" }}>
      <div className="armada-screen__window">
        <WithDock {...args} />
      </div>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog", { name: "Helm" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open Helm, 3 questions waiting" }));
    await expect(canvas.getByRole("dialog", { name: "Helm" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: /^Close/ }));
    await expect(canvas.queryByRole("dialog", { name: "Helm" })).toBeNull();
  },
};
