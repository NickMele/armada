import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Bell, ClipboardList, FileCog, HardDrive, Settings, Stethoscope } from "lucide-react";
import { expect } from "storybook/test";

import { actsIn, ALIASES, globalActs, type Action } from "../../actions";
import { Dialog } from "../Dialog/Dialog";
import {
  CommandPalette,
  type PaletteEntry,
  type PaletteSection,
} from "./CommandPalette";

const meta: Meta<typeof CommandPalette> = {
  title: "Primitives/CommandPalette",
  component: CommandPalette,
};
export default meta;

type Story = StoryObj<typeof CommandPalette>;

/**
 * The one Job the board's cursor is on. **The context block is titled with the
 * job it acts on**, so a palette that will Kill something says which something
 * before the row is read.
 */
const CONTEXT = "job_2d90bb — coalesce the session refresh";

/**
 * Where a person can go: the rail's four destinations, and one that is not in
 * the rail. Each glyph is the icon registry's own assignment for that surface;
 * the registry's `bridge_surfaces` row carries none by design, because a fifth
 * glyph standing for all four is not a thing — which is also why the rail
 * expands into rows here rather than that one row being drawn. Helm is not
 * among them: it is a sibling surface with a registry row of its own, and it
 * arrives through `globalActs`.
 *
 * **`Held worktrees` carries no digit, and this is where that is visible.**
 * `⌘1–⌘4` is bound to Bridge surfaces *in rail order*, so a digit is a place
 * in the rail; the held worktrees are reached from the Board's head and have
 * no such place, and the two digits going spare belong to Alerts and to Helm.
 * A blank key column beside four filled ones is the honest drawing of that,
 * and it is what a person looking for the missing binding should be shown.
 *
 * The Manifest drew `clipboard-list` here until this row was added — the Job
 * Board's glyph, on a second destination, in the one place the set is read as
 * a set. `file-cog` is the registry's assignment and its own row reserves it
 * to the Manifest surface and the file.
 */
const RAIL: PaletteEntry[] = [
  { id: "nav-board", section: "navigation", label: "Job Board", shortcut: "⌘1", icon: ClipboardList },
  { id: "nav-alerts", section: "navigation", label: "Alerts", shortcut: "⌘2", icon: Bell },
  { id: "nav-doctor", section: "navigation", label: "Doctor", shortcut: "⌘3", icon: Stethoscope },
  { id: "nav-manifest", section: "navigation", label: "Manifest", shortcut: "⌘4", icon: FileCog },
  {
    id: "nav-worktrees",
    section: "navigation",
    label: "Held worktrees",
    aliases: ["disk", "held disk"],
    icon: HardDrive,
  },
];

/** Jobs carry no key. Opening a specific job is a search result, not an act. */
const JOBS: PaletteEntry[] = [
  { id: "job_2d90bb", section: "jobs", label: "job_2d90bb — coalesce the session refresh" },
  { id: "job_8f2a1c", section: "jobs", label: "job_8f2a1c — split the settings reducer" },
  { id: "job_41c07e", section: "jobs", label: "job_41c07e — drop the stale socket on reconnect" },
];

/** A setting states its current value, in mono, right of the label. */
const SETTINGS: PaletteEntry[] = [
  { id: "set-model", section: "settings", label: "Default model", value: "sonnet", icon: Settings },
  { id: "set-headroom", section: "settings", label: "Concurrent drones", value: "4", icon: Settings },
  { id: "set-history", section: "settings", label: "Job history kept", value: "30 days", icon: Settings },
  { id: "set-theme", section: "settings", label: "Appearance", value: "dark", icon: Settings },
];

/** One registry row, as a palette entry. Nothing is re-spelled on the way. */
function asEntry(section: string) {
  return (action: Action): PaletteEntry => ({
    id: action.id,
    section,
    label: action.verb,
    shortcut: action.shortcut,
    ...(action.icon === null ? {} : { icon: action.icon }),
    ...(ALIASES[action.id] === undefined ? {} : { aliases: ALIASES[action.id] }),
    ...(action.destructive ? { destructive: true } : {}),
    ...(action.unbuilt === null ? {} : { dormant: `not built · ${action.unbuilt}` }),
  });
}

function sections(title: string): PaletteSection[] {
  return [
    { id: "context", title },
    { id: "navigation", title: "Navigation" },
    { id: "jobs", title: "Jobs" },
    { id: "settings", title: "Settings" },
  ];
}

/**
 * The contents in the order the contract fixes: acts on the current context,
 * navigation, jobs, settings.
 *
 * Navigation is the rail's destinations plus the global acts the rail is not.
 * `bridge_surfaces` is one registry row over four destinations, because the
 * rule there is rail order rather than the digits — so the rail is what
 * expands it, and Helm arrives from the registry as the digit after the last
 * of them.
 */
function entriesFor(context: "board" | "detail"): PaletteEntry[] {
  return [
    ...actsIn(context).map(asEntry("context")),
    ...RAIL,
    ...globalActs().map(asEntry("navigation")),
    ...JOBS,
    ...SETTINGS,
  ];
}

/** What the host says its index covered, for the state where nothing matched. */
const SEARCHED =
  "every act on this job, every place in Bridge, every job on this Manifest at every " +
  "status, and every setting";

const board = {
  open: true,
  sections: sections(CONTEXT),
  entries: entriesFor("board"),
  searched: SEARCHED,
} as const;

/**
 * **Empty query: the cheat sheet.** Every section head and every binding, which
 * is the whole mechanism by which forty shortcuts get learned without a help
 * screen — a person opens this to do one thing and reads three on the way past.
 *
 * The acts are the registry's, filtered to what the board offers, so this is
 * the real map rather than a fixture of one. Rows with no glyph draw an empty
 * slot that holds its width: thirteen acts have no registered silhouette and
 * inventing one is a decision for the icon registry.
 */
export const EveryBindingAtRest: Story = {
  args: board,
  /**
   * The claim is that nothing is missing, and the only way to state it is to
   * count the registry rather than a number typed here — a literal would go
   * stale the day an act is added, which is the exact failure this surface
   * exists to prevent.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("option")).toHaveLength(board.entries.length);
    for (const section of board.sections) {
      // `getAllBy`, because the context block is titled with the job it acts
      // on and that job is also a row under Jobs — which is the contents
      // order working rather than a duplicate.
      await expect(canvas.getAllByText(section.title)[0]).toBeVisible();
    }
  },
};

/**
 * The same palette on job detail. **The context block changes; a key never
 * does.** `r` is Review on both, and the acts a list cannot offer — Restart
 * step, Observe, Report this job, the diff, the stage — are here because this
 * is where they are offered.
 *
 * Two rows are worth reading: **Pilot appears and is not in the board's row
 * map**, which is the superset rule doing its work, and it draws disabled with
 * the issue that answers it, because a row a person presses and gets nothing
 * from is worse than one that is absent.
 */
export const OnJobDetail: Story = {
  args: {
    open: true,
    sections: sections(CONTEXT),
    entries: entriesFor("detail"),
    searched: SEARCHED,
  },
};

/**
 * **Top-anchored, at two result counts.** The anchor holds and only what is
 * below it shrinks. A centred dialog rises as results fall away, so the row
 * under the cursor changes while a person is still typing.
 */
export const TopAnchoredAtTwoCounts: Story = {
  args: board,
  /**
   * A rendering cannot show this: one frame is a palette, and the claim is
   * about two of them. So the geometry is read off the same palette before and
   * after the list is narrowed — the input's top edge and the first row's top
   * edge, both to the pixel.
   *
   * Broken on purpose once by setting `align-items: center` on the layer, which
   * moved both by 84px.
   */
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("combobox");
    const wide = canvas.getAllByRole("option").length;
    const inputTop = field.getBoundingClientRect().top;
    const rowTop = canvas.getAllByRole("option")[0]!.getBoundingClientRect().top;

    await userEvent.type(field, "job");

    await expect(canvas.getAllByRole("option").length).toBeLessThan(wide);
    await expect(field.getBoundingClientRect().top).toBe(inputTop);
    await expect(canvas.getAllByRole("option")[0]!.getBoundingClientRect().top).toBe(rowTop);
  },
};

/**
 * **Matching across sections.** One word returning acts, a place, three jobs
 * and a setting. The section heads are what stop them reading as one list —
 * without them a person cannot tell the Job Board surface from a job whose
 * title carries the word from the setting that decides how long jobs are kept.
 */
export const MatchingAcrossSections: Story = {
  args: { ...board, defaultQuery: "job" },
  /**
   * The claim is about the heads, not the rows: four kinds of result under
   * four names. A palette that dropped the heads would pass any assertion
   * counting matches and would be the flat list this state exists to refuse.
   */
  play: async ({ canvas }) => {
    for (const title of [CONTEXT, "Navigation", "Jobs", "Settings"]) {
      await expect(canvas.getAllByText(title)[0]).toBeVisible();
    }
  },
};

/**
 * **An alias hit.** The query is "terminate", which is Kill's alias — and the
 * row reads Kill, because the alias never renders. Other queries mark the
 * matched span; this one has none to mark, and faking one would render the
 * alias.
 */
export const AnAliasFindsTheLexiconTerm: Story = {
  args: { ...board, defaultQuery: "terminate" },
  /**
   * Read off the row, which is the only place the rule can be seen. An alias
   * that leaked into what renders would teach the person the word Armada does
   * not use, from the surface whose whole job is teaching them the words it
   * does — so the assertion is on all three: the term renders, the alias does
   * not, and nothing is marked.
   *
   * The job whose title carries "terminate" is a real match on a real label
   * and is expected beside it; it is the reason the mark assertion is scoped
   * to Kill's own row rather than to the list.
   */
  play: async ({ canvas }) => {
    const kill = canvas.getByRole("option", { name: /^Kill/ });
    await expect(kill).toHaveAccessibleName("Kill x");
    await expect(kill.querySelector("mark")).toBeNull();
    await expect(canvas.queryByText("terminate")).toBeNull();
  },
};

/**
 * **A place with no binding is still a place.** The held worktrees are reached
 * from the Board's head and sit in no rail slot, so there is no digit to draw
 * beside them — and the row is here anyway, because the rule the palette keeps
 * is that nothing exists outside it, not that everything has a key.
 *
 * The query is "disk", the word on the control that has reached this screen
 * since it shipped. The row reads `Held worktrees`, which is the title of the
 * screen it lands on.
 */
export const APlaceWithNoBindingIsStillFound: Story = {
  args: { ...board, defaultQuery: "disk" },
  /**
   * Two things a rendering cannot state: the row was reached by a word that is
   * not in its label, and its accessible name carries no binding where every
   * other Navigation row's ends in one. The second is read after the query is
   * cleared, because "disk" narrows Job Board away — and Job Board is the row
   * the claim is against.
   *
   * Broken on purpose by dropping the aliases from the entry, which loses the
   * row entirely and fails on the first line rather than on the name.
   */
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("option", { name: /Held worktrees/ })).toHaveAccessibleName(
      "Held worktrees",
    );

    await userEvent.clear(canvas.getByRole("combobox"));

    // Two caps and so two words: a chord is drawn as one `Kbd` per key.
    await expect(canvas.getByRole("option", { name: /^Job Board/ })).toHaveAccessibleName(
      "Job Board ⌘ 1",
    );
    await expect(canvas.getByRole("option", { name: /Held worktrees/ })).toHaveAccessibleName(
      "Held worktrees",
    );
  },
};

/**
 * **Matching nothing.** Names the query and says what was searched. No
 * suggestions and no did-you-mean: the palette is the discovery surface, so
 * the honest answer to a miss is the extent of the index rather than a guess
 * at what was meant.
 */
export const MatchingNothing: Story = {
  args: { ...board, defaultQuery: "zzz" },
  play: async ({ canvas }) => {
    await expect(canvas.queryAllByRole("option")).toHaveLength(0);
    await expect(canvas.getByText(/Nothing matches “zzz”/)).toBeVisible();
    await expect(canvas.getByText(/every job on this Manifest at every status/)).toBeVisible();
  },
};

/**
 * **The first row is always active**, so a query narrowed to one result is a
 * two-key act: type enough, press Enter.
 */
export const TheFirstRowIsActive: Story = {
  args: { ...board, defaultQuery: "re" },
  /**
   * Asserted after a change to the query and not only on the resting state,
   * because the regression is a cursor left at position four in a list that
   * now holds two — which reads as a palette that ignored what was typed.
   */
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("option", { selected: true })).toHaveAccessibleName(/^Review/);

    await userEvent.keyboard("{ArrowDown}");
    await expect(canvas.getByRole("option", { selected: true })).not.toHaveAccessibleName(
      /^Review/,
    );

    await userEvent.clear(canvas.getByRole("combobox"));
    await userEvent.type(canvas.getByRole("combobox"), "kill");

    await expect(canvas.getAllByRole("option")[0]).toHaveAttribute("aria-selected", "true");
  },
};

/**
 * The safety rule, rendered. Selecting Kill from the palette does not kill —
 * every destructive action confirms, even from the keyboard, and in the
 * confirmation Cancel holds initial focus and `Enter` fires it.
 *
 * **Danger is the dropdown-menu treatment**: `--status-completed-failed` on
 * the label and the glyph, no background. A filled red row in a list of thirty
 * reads as a failed job rather than an act.
 */
export const DestructiveEntryConfirms: Story = {
  render: () => {
    const [pending, setPending] = useState<PaletteEntry | undefined>(undefined);
    return (
      <>
        <CommandPalette {...board} defaultQuery="kill" onConfirm={(entry) => setPending(entry)} />
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
  /**
   * **Selecting Kill does not kill.** The palette hands the entry to its host
   * and stays where it is; the host opens the confirmation over it. So the
   * assertion is that two things are true at once after one `Enter` — the
   * dialog is on screen, and the palette did not act and did not close.
   *
   * A palette that closed here would be the more natural thing to write and
   * the wrong one: the way back from a confirmation is the list you chose from.
   */
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("option", { selected: true })).toHaveAccessibleName(/^Kill/);
    await expect(canvas.queryByRole("dialog", { name: "Kill the drone on job 12?" })).toBeNull();

    await userEvent.keyboard("{Enter}");

    await expect(canvas.getByRole("dialog", { name: "Kill the drone on job 12?" })).toBeVisible();
    await expect(canvas.getByRole("dialog", { name: "Command palette" })).toBeVisible();
  },
};
