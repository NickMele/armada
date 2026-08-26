import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  Activity,
  Bell,
  CircleDot,
  ClipboardList,
  CornerUpRight,
  Eye,
  MessageSquare,
  Power,
  Send,
  Settings,
  Stethoscope,
} from "lucide-react";
import { CommandPalette, type CommandPaletteEntry } from "./CommandPalette";
import { Dialog } from "../Dialog/Dialog";

const meta: Meta<typeof CommandPalette> = {
  title: "Primitives/CommandPalette",
  component: CommandPalette,
};
export default meta;

type Story = StoryObj<typeof CommandPalette>;

const glyph = { size: 12, strokeWidth: 2, "aria-hidden": true } as const;

/**
 * Contents in the order the contract fixes: actions available on the current
 * context, navigation, jobs by id or name, settings. Every entry displays its
 * binding, and Kill is `x` rather than `k` — a destructive key is never
 * adjacent to a navigation key.
 */
const entries: CommandPaletteEntry[] = [
  {
    id: "dispatch",
    section: "Actions",
    label: "Dispatch a job",
    aliases: ["start", "launch", "new"],
    shortcut: "⌘D",
    icon: <Send {...glyph} />,
  },
  {
    id: "approve",
    section: "Actions",
    label: "Approve dispatch",
    aliases: ["accept", "ok"],
    shortcut: "a",
    icon: <Eye {...glyph} />,
  },
  {
    id: "redirect",
    section: "Actions",
    label: "Redirect the drone",
    aliases: ["steer", "correct"],
    shortcut: "r",
    icon: <CornerUpRight {...glyph} />,
  },
  {
    id: "kill",
    section: "Actions",
    label: "Kill the drone",
    aliases: ["terminate", "stop", "abort"],
    shortcut: "x",
    icon: <Power {...glyph} />,
    destructive: true,
  },
  {
    id: "board",
    section: "Navigation",
    label: "Job Board",
    shortcut: "⌘1",
    icon: <ClipboardList {...glyph} />,
  },
  {
    id: "active",
    section: "Navigation",
    label: "Active jobs",
    shortcut: "⌘2",
    icon: <Activity {...glyph} />,
  },
  {
    id: "alerts",
    section: "Navigation",
    label: "Alerts",
    shortcut: "⌘3",
    icon: <Bell {...glyph} />,
  },
  {
    id: "doctor",
    section: "Navigation",
    label: "Doctor",
    shortcut: "⌘6",
    icon: <Stethoscope {...glyph} />,
  },
  {
    id: "helm",
    section: "Navigation",
    label: "Helm",
    shortcut: "⌘8",
    icon: <MessageSquare {...glyph} />,
  },
  {
    id: "job-8f2a1c",
    section: "Jobs",
    label: "job_8f2a1c — split the settings reducer",
    shortcut: "↵",
    icon: <CircleDot {...glyph} />,
  },
  {
    id: "job-2d90bb",
    section: "Jobs",
    label: "job_2d90bb — coalesce the session refresh",
    shortcut: "↵",
    icon: <Eye {...glyph} />,
  },
  {
    id: "kit",
    section: "Settings",
    label: "Kit",
    aliases: ["skills", "allowlist", "mcp"],
    shortcut: "⌘,",
    icon: <Settings {...glyph} />,
  },
];

/** Resting: no query, every section in the contract's order. */
export const Resting: Story = {
  args: { open: true, entries },
};

/**
 * The lexicon holding. The query is "terminate", the alias that finds Kill —
 * and the row still reads Kill, because the alias never renders.
 */
export const AliasFindsTheLexiconTerm: Story = {
  args: { open: true, entries, defaultQuery: "terminate" },
};

/**
 * No match. The palette is the discovery surface, so the empty line says where
 * every action is rather than apologising.
 */
export const NoMatch: Story = {
  args: { open: true, entries, defaultQuery: "zzz" },
};

/**
 * The safety rule, rendered. Selecting Kill from the palette does not kill —
 * every destructive action confirms, even from the keyboard, and in the
 * confirmation Cancel holds initial focus.
 *
 * Type, arrow to Kill the drone, press Enter.
 */
export const DestructiveEntryConfirms: Story = {
  render: () => {
    const [pending, setPending] = useState<CommandPaletteEntry | undefined>(undefined);
    return (
      <>
        <CommandPalette
          open
          entries={entries}
          defaultQuery="kill"
          onConfirm={(entry) => setPending(entry)}
        />
        <Dialog
          open={pending !== undefined}
          tone="destructive"
          title="Kill the drone on job 12?"
          confirmLabel="Kill job"
          onCancel={() => setPending(undefined)}
          onConfirm={() => setPending(undefined)}
        >
          Step 3 of 5, 18 minutes in. The worktree is left in place and evidence carries forward
          if you redispatch.
        </Dialog>
      </>
    );
  },
};
