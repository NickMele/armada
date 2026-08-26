import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  Activity,
  Bell,
  ClipboardList,
  Eye,
  FileCog,
  MessageSquare,
  ScrollText,
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
 * `Navigation` group. `file-cog` is the Manifest surface, added after the
 * `components.toml` row for Sidebar recorded its glyph list; that row is now
 * short one entry. Reported.
 */
const meta: Meta<typeof Sidebar> = {
  title: "Compositions/Sidebar",
  component: Sidebar,
};
export default meta;

type Story = StoryObj<typeof Sidebar>;

const surfaces: SidebarItem[] = [
  { id: "board", label: "Job Board", icon: ClipboardList },
  { id: "active", label: "Active jobs", icon: Activity, count: 6 },
  { id: "alerts", label: "Alerts", icon: Bell },
  { id: "reviews", label: "Reviews", icon: Eye },
  { id: "feed", label: "Activity feed", icon: ScrollText },
  { id: "doctor", label: "Doctor", icon: Stethoscope },
  { id: "manifest", label: "Manifest", icon: FileCog },
];

const helm: SidebarItem = { id: "helm", label: "Helm", icon: MessageSquare };

/** 200px, the resting width. Bridge above the rule, Helm beneath it. */
export const Expanded: Story = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada" },
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
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada", collapsed: true },
};

/** The narrow end of the drag range. */
export const AtMinimumWidth: Story = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada", width: "var(--sidebar-min)" },
};

/** The wide end of the drag range. */
export const AtMaximumWidth: Story = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada", width: "var(--sidebar-max)" },
};

/**
 * M1's shell: one surface in the rail, because one surface exists. The count
 * is a job count — never an escalation or approval count, which the status bar
 * carries on every surface.
 */
export const M1OneSurface: Story = {
  args: {
    surfaces: [{ id: "active", label: "Active jobs", icon: Activity, count: 6 }],
    activeId: "active",
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
  args: { surfaces, activeId: "active", sectionLabel: undefined, appName: "Armada" },
};
