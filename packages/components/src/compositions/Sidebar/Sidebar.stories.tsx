import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  Bell,
  ClipboardList,
  FileCog,
  HardDrive,
  MessageSquare,
  Stethoscope,
} from "lucide-react";
import { Select } from "../../primitives/Select/Select";
import { Sidebar, type SidebarItem } from "./Sidebar";

/**
 * One story per width and state the layout model names: the 200px default, the
 * 160px and 320px ends of the drag range, the 48px rail, and the two-level
 * structure with Helm active.
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
 * **Five in the group and one beneath the rule is a different composition from
 * four and one**, which is the reason to look at this story rather than read
 * the roster: 48px of column is what the collapsed state has to stay legible
 * in, and the rule that separates Helm from the group is doing more work with
 * a longer group above it.
 */
const meta: Meta<typeof Sidebar> = {
  title: "Compositions/Sidebar",
  component: Sidebar,
};
export default meta;

type Story = StoryObj<typeof Sidebar>;

const surfaces: SidebarItem[] = [
  { id: "board", label: "Job Board", icon: ClipboardList, count: 6 },
  { id: "alerts", label: "Alerts", icon: Bell },
  { id: "doctor", label: "Doctor", icon: Stethoscope },
  { id: "manifest", label: "Manifest", icon: FileCog },
  { id: "worktrees", label: "Held worktrees", icon: HardDrive },
];

const helm: SidebarItem = { id: "helm", label: "Helm", icon: MessageSquare };

/** 200px, the resting width. Bridge above the rule, Helm beneath it. */
export const Expanded: Story = {
  args: { surfaces, sibling: helm, activeId: "board", appName: "Armada" },
};

/**
 * Helm is a sibling of the whole group, not one more peer inside it, and its
 * active state reads the same as any surface's — the hierarchy is structural
 * rather than a second treatment.
 */
export const HelmActive: Story = {
  args: { surfaces, sibling: helm, activeId: "helm", appName: "Armada" },
};

/**
 * The 48px rail. Labels go, the glyphs centre, and the rule stays. It is more
 * usable than it looks: ⌘-digit reaches every surface without labels.
 */
export const CollapsedRail: Story = {
  args: { surfaces, sibling: helm, activeId: "board", appName: "Armada", collapsed: true },
};

/** The narrow end of the drag range. */
export const AtMinimumWidth: Story = {
  args: { surfaces, sibling: helm, activeId: "board", appName: "Armada", width: "var(--sidebar-min)" },
};

/** The wide end of the drag range. */
export const AtMaximumWidth: Story = {
  args: { surfaces, sibling: helm, activeId: "board", appName: "Armada", width: "var(--sidebar-max)" },
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
      { id: "worktrees", label: "Held worktrees", icon: HardDrive },
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
 * No section label and no sibling: the sidebar reduced to a flat list, which is
 * what the two-level rule exists to prevent. Drawn so the difference is visible
 * beside `Expanded` rather than argued for in prose.
 */
export const FlatForContrast: Story = {
  args: { surfaces, activeId: "board", sectionLabel: undefined, appName: "Armada" },
};
