import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  Bell,
  ClipboardList,
  FileCog,
  HardDrive,
  Stethoscope,
} from "lucide-react";
import { useState } from "react";
import { expect } from "storybook/test";
import { Select } from "../../primitives/Select/Select";
import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { Sidebar, type SidebarItem } from "./Sidebar";

/**
 * One story per width and state the layout model names: the 200px default, the
 * 160px and 320px ends of the drag range, and the 48px rail. Navigation is one
 * level: Helm left it for the dock (#948), so no story draws a tier beneath
 * the surfaces.
 *
 * The roster is Bridge's — it is passed in, and the glyphs are the registry's
 * `Navigation` group. It is the rail `docs/concepts/bridge.md` fixes: Active
 * jobs, Reviews and the Activity feed were drawn here until 2026-09-03, three
 * surfaces retired when the Board became every Job with state as a filter, and
 * `Held worktrees` is the one that joined. `file-cog` is the Manifest surface
 * and `hard-drive` the held worktrees, both added after the `components.toml`
 * row for Sidebar recorded its glyph list; that row is now short two entries.
 * Reported.
 *
 * **Five in the group is a different composition from four**, which is the
 * reason to look at this story rather than read the roster: 48px of column is
 * what the collapsed state has to stay legible in.
 */
const meta: Meta<typeof Sidebar> = {
  title: "Compositions/Sidebar",
  component: Sidebar,
};
export default meta;

type Story = StoryObj<typeof Sidebar>;

const surfaces: SidebarItem[] = [
  { id: "board", label: "Job Board", icon: ClipboardList, count: 6, shortcut: "⌘1" },
  { id: "alerts", label: "Alerts", icon: Bell, shortcut: "⌘2" },
  { id: "doctor", label: "Doctor", icon: Stethoscope, shortcut: "⌘3" },
  { id: "manifest", label: "Manifest", icon: FileCog, shortcut: "⌘4" },
  { id: "worktrees", label: "Cleanup", icon: HardDrive, shortcut: "⌘5" },
];

/**
 * 200px, the resting width, with Bridge's label above the surfaces. Every
 * glyph sits in the one accent-tinted chip, and the active row's chip is solid
 * `--accent` on the row's own `--accent-muted` fill.
 */
export const Expanded: Story = {
  args: { surfaces, activeId: "board", appName: "Armada" },
};

/**
 * The 48px rail. Labels go and the chips centre. It is more
 * usable than it looks: ⌘-digit reaches every surface without labels.
 *
 * With no label, the chip is the whole affordance: every one shares the soft
 * accent tint, and the current section's is solid. The row around it, not the
 * 28px chip, is still what a click lands on (#1265).
 */
export const CollapsedRail: Story = {
  args: { surfaces, activeId: "board", appName: "Armada", collapsed: true },
};

/**
 * The collapse control, wired — #1591. The same button in both states, at the
 * column's trailing edge open and centred at the rail, drawing
 * `panel-left-close` and `panel-left-open` so it says which state it is in
 * without being hovered.
 *
 * What a rendering cannot show, and what the `play` reads: the press reaches
 * the same 48px rail the breakpoint does rather than a second narrow width,
 * and the control comes back with it. A control that collapsed a column and
 * then went with it is a person stuck at 48px.
 */
export const CollapseControl: Story = {
  args: { surfaces, activeId: "board", appName: "Armada", collapseBinding: "⌘\\" },
  render: (args) => {
    const [collapsed, setCollapsed] = useState(false);
    return <Sidebar {...args} collapsed={collapsed} onCollapsedChange={setCollapsed} />;
  },
  play: async ({ canvas, userEvent }) => {
    const nav = canvas.getByRole("navigation");
    const open = nav.getBoundingClientRect().width;

    await userEvent.click(canvas.getByRole("button", { name: "Collapse the left column" }));
    const rail = nav.getBoundingClientRect().width;
    await expect(rail).toBeLessThan(open);
    await expect(canvas.queryByText("Job Board")).toBeNull();

    const back = canvas.getByRole("button", { name: "Expand the left column" });
    await expect(back).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(back);
    await expect(nav.getBoundingClientRect().width).toBe(open);
    await expect(canvas.getByText("Job Board")).toBeVisible();
  },
};

/**
 * The rail a person asked for, rather than the one the window imposed. It is
 * the same 48px `CollapsedRail` draws — the only difference is that the
 * control is there, saying which way out is.
 */
export const CollapsedByChoice: Story = {
  args: {
    surfaces,
    activeId: "board",
    appName: "Armada",
    collapsed: true,
    collapseBinding: "⌘\\",
    onCollapsedChange: () => {},
  },
};

/** The narrow end of the drag range. */
export const AtMinimumWidth: Story = {
  args: { surfaces, activeId: "board", appName: "Armada", width: "var(--sidebar-min)" },
};

/** The wide end of the drag range. */
export const AtMaximumWidth: Story = {
  args: { surfaces, activeId: "board", appName: "Armada", width: "var(--sidebar-max)" },
};

/**
 * What Bridge actually draws: the surfaces that are built, which is two of the
 * five. The rest hold their place in the order and their digit and draw
 * nothing — a disabled row would be a promise Armada does not keep.
 *
 * The count is a job count, and it is on the Board alone — never an escalation
 * or approval count, which the status bar carries on every surface, and never
 * on the held worktrees, which are read only while that screen is open.
 */
export const WhatIsBuilt: Story = {
  args: {
    surfaces: [
      { id: "board", label: "Job Board", icon: ClipboardList, count: 6 },
      { id: "worktrees", label: "Cleanup", icon: HardDrive },
    ],
    activeId: "board",
    appName: "Armada",
    header: (
      <Select aria-label="Project">
        <option>armada</option>
      </Select>
    ),
  },
};

/**
 * No section label: the surfaces alone, which is what `TheShell` draws — it
 * passes `sectionLabel={null}`. Beside `Expanded`, the difference is the label
 * and nothing else.
 */
export const FlatForContrast: Story = {
  args: { surfaces, activeId: "board", sectionLabel: undefined, appName: "Armada" },
};

/**
 * Holding Cmd grows every row's own `⌘`-digit without moving the label beside
 * it, and releasing it takes every badge away at once. `useShortcutReveal()`
 * needs a provider above it for the state to reach anywhere at all, which
 * `TheShell` is in the running app and this story stands in for here.
 */
export const RevealedShortcuts: Story = {
  args: { surfaces, activeId: "board", appName: "Armada" },
  render: (args) => (
    <ShortcutRevealProvider>
      <Sidebar {...args} />
    </ShortcutRevealProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    const badge = canvas.getByText((_, el) => el?.tagName === "KBD" && el.textContent === "⌘1");
    await expect(badge).not.toBeVisible();

    await userEvent.keyboard("{Meta>}");
    await expect(badge).toBeVisible();

    await userEvent.keyboard("{/Meta}");
    await expect(badge).not.toBeVisible();
  },
};
