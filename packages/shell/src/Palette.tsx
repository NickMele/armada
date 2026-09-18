// The command palette, wired: what goes in it, in what order, and what a
// chosen row means.
//
// **A shell holds a screen; it is not one** — and the palette is the same
// whatever surface is open, which is exactly what makes it a layer. It also
// spans both screens: the acts on the board and the acts on one job read whole
// are different sets under one artifact, so a palette owned by either screen
// would be a second one the moment the other needed it.
//
// # The contents, in the order the contract fixes
//
// Acts on the current context, navigation, jobs, settings. The context block
// is **titled with the job it acts on**, so a palette that will Kill something
// says which something before the first row is read.
//
// # Everything here is the registry's, and nothing is decided here
//
// The verb, the glyph, the binding and whether an act is destructive all come
// from `actions.ts`, which transcribes `crates/core-model/domain/actions.toml`.
// This file decides which of them a context offers — and even that is the
// registry's `scope` column read through `actsIn`. What it adds is one thing
// the registry cannot know: whether *this app, right now* can reach the act.
//
// **`dormant` is that answer and the caller supplies it.** A row nothing
// answers is drawn dimmed with the reason beside its binding rather than left
// out, because the palette is the discovery surface: a binding a person cannot
// find is a binding they never learn, and one they press to no effect is worse
// than a row that says why.
//
// # ⌘K is bound once, here
//
// `useCommandPalette` owns the binding. It is a modified key, so it does not
// go through the contextual tier's suppression — the whole point of the
// modifier is that it works while a text field has focus, which is when a
// person most wants out.

import { useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";

import { actsIn, ALIASES, globalActs, CommandPalette } from "@armada/components";
import type { Action, ActionContext, PaletteEntry, PaletteSection } from "@armada/components";

/** A Job as a search result. It carries no binding, and that is not a gap. */
export type PaletteJob = { id: string; label: string };

/**
 * A destination in Bridge, with the digit that reaches it where it has one.
 *
 * **The digit is optional and that is a gap, named rather than filled.** The
 * contract binds `⌘1–⌘9` to *Bridge surfaces in rail order*, and Helm is on
 * `⌘J`, which toggles the dock rather than taking a digit. So a digit is a
 * position in the rail and nothing else. A destination that is not in the rail
 * has no position, and inventing one for it would take a digit the rail owes
 * to a surface named in `docs/concepts/bridge.md` — which changes what a
 * published binding says, and is not a wiring decision.
 *
 * So a destination without a rail row is reached by name in here and by no
 * digit. That is a smaller thing than it looks: the palette is the discovery
 * surface, and the rule it exists to keep is that no action lives outside it.
 */
export type PaletteSurface = {
  id: string;
  label: string;
  shortcut?: string;
  /**
   * Words that find it, never drawn. Only for a place a person already knows
   * by another word — the control they reached it by before it was in here.
   */
  aliases?: readonly string[];
  icon: LucideIcon;
};

/**
 * One thing the Manifest surface can run — a Check, a Command, or a server.
 *
 * **A row per entry, not one row spelling "run something".** Journey 9's own
 * table says the palette carries one entry per Check and Command, for the
 * reason the Board's filters are five rows rather than the registry's single
 * `state_filter`: the set is what a person is choosing from, and a row that
 * named none of them would leave every Check reachable only by finding the
 * surface first.
 *
 * **Its `run` line is the value**, drawn in mono to the right the way a
 * setting's current value is — so a palette that will run `pnpm -C
 * apps/desktop typecheck` says so before it is chosen.
 */
export type PaletteRunnable = {
  /** The entry id the Manifest surface uses — `check:format`, `command:fmt`. */
  id: string;
  /** The Manifest's own name for it. */
  label: string;
  /** The `run` or `serve` line, exactly as declared. */
  value: string;
};

/**
 * A setting, which states its current value in mono right of the label where
 * it has one.
 *
 * **`value` is optional, and a row without one is not a display.** Fleet
 * settings is the first row this section carries: choosing it opens the
 * sheet rather than stating a field, because the section is where the app's
 * standing configuration lives, whether that configuration is read here or
 * changed behind its own screen.
 */
export type PaletteSetting = { id: string; label: string; value?: string };

/**
 * One of the Board's state filters, with the digit that sets it.
 *
 * **They appear as rows and the registry's single `state_filter` does not.**
 * That row spells its binding `1–5` over five tabs, for the reason
 * `bridge_surfaces` spells `⌘1–⌘4` over four destinations — the rule is tab
 * order rather than the digits. A palette drawing that one row would name no
 * filter and set none, which would leave `1`–`5` the one part of the app
 * learnable only by finding the tabs.
 */
export type PaletteFilter = { id: string; label: string; shortcut: string };

/** What a chosen row was. The section it came from decides what happens. */
export type PaletteChoice =
  | { of: "act"; id: string }
  | { of: "job"; id: string }
  | { of: "surface"; id: string }
  | { of: "run"; id: string }
  | { of: "setting"; id: string }
  | { of: "filter"; id: string };

export type PaletteProps = {
  open: boolean;
  onClose: () => void;
  /** Where the person is standing: the board, or one Job read whole. */
  context: ActionContext;
  /**
   * The Job the acts act on, as a person reads it. `null` where nothing is
   * focused — and then the block says so rather than being titled with
   * nothing, because a heading that names no object over rows that need one is
   * the ambiguity the title exists to remove.
   */
  on: string | null;
  surfaces: readonly PaletteSurface[];
  /** The Board's state filters, where the surface has them. */
  filters?: readonly PaletteFilter[];
  /**
   * What the Manifest surface can run, where it is the surface showing.
   *
   * **Empty everywhere else, and that is the whole of the scoping.** Choosing
   * one runs it in the checkout, which is an act on a surface rather than on a
   * Job — so a row offered from the Board would be a run started from a
   * surface that cannot show its output.
   */
  runnables?: readonly PaletteRunnable[];
  jobs: readonly PaletteJob[];
  settings: readonly PaletteSetting[];
  /** Why an act cannot be chosen here, by action id. Absent means it can. */
  dormant?: Readonly<Record<string, string | undefined>>;
  /**
   * Acts left out here rather than drawn dimmed, by action id. A binding the
   * registry marks `unbuilt` is drawn whatever this says, since its reason
   * holds everywhere and not just here.
   */
  absent?: readonly string[];
  /**
   * A row was chosen. **Destructive acts do not come through here** — they go
   * to `onConfirmAct`, because every destructive action confirms even from the
   * keyboard, and the palette stays open behind the confirmation.
   */
  onChoose: (choice: PaletteChoice) => void;
  onConfirmAct: (actionId: string) => void;
};

/** The section ids. Their order is the contract's contents order. */
const CONTEXT = "context";
const NAVIGATION = "navigation";
const RUN = "run";
const JOBS = "jobs";
const SETTINGS = "settings";

/** The registry row the Board's filters stand in for. */
const STATE_FILTER = "state_filter";

/** What the block is titled where nothing is under the cursor. */
const NOTHING_FOCUSED = "No job focused";

/**
 * Whether ⌘K is down, and the state it opens.
 *
 * **Bound on `window` and never suppressed by a focused field.** The
 * contextual tier is suppressed while a text input holds focus so that typing
 * "axe" cannot approve, kill and open something; a modified key is a different
 * tier and that is the whole separation. `Esc` closes it, and the palette
 * itself owns that key while it is up.
 */
export function useCommandPalette(): {
  open: boolean;
  onOpen: () => void;
  onClose: () => void;
} {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.key !== "k" && event.key !== "K") return;
      if (!(event.metaKey || event.ctrlKey)) return;
      event.preventDefault();
      // Toggled, not opened. ⌘K twice is how a person closes something they
      // opened by reflex, and `Esc` is not what they reach for while their
      // hand is still on the modifier.
      setOpen((was) => !was);
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  return { open, onOpen: () => setOpen(true), onClose: () => setOpen(false) };
}

export function Palette({
  open,
  onClose,
  context,
  on,
  surfaces,
  filters = [],
  runnables = [],
  jobs,
  settings,
  dormant = {},
  absent = [],
  onChoose,
  onConfirmAct,
}: PaletteProps) {
  const sections: PaletteSection[] = [
    { id: CONTEXT, title: on ?? NOTHING_FOCUSED },
    { id: NAVIGATION, title: "Navigation" },
    // **Below Navigation and above Jobs, and it is a fifth block.** The
    // contract's contents order names four, and none of them is this: a Check
    // is not an act on the Job under the cursor, not a place, not a Job and
    // not a setting. It sits where it does because it is the one block scoped
    // to the surface showing — nearer the acts than the index of everything.
    ...(runnables.length === 0 ? [] : [{ id: RUN, title: "Run in the checkout" }]),
    { id: JOBS, title: "Jobs" },
    { id: SETTINGS, title: "Settings" },
  ];

  const entries: PaletteEntry[] = [
    ...actsIn(context).flatMap((action) =>
      action.unbuilt === null && absent.includes(action.id)
        ? []
        : action.id === STATE_FILTER && filters.length > 0
        ? // Expanded into its five, for the reason the rail expands
          // `bridge_surfaces` into its four. The one registry row spells a
          // range over a set whose order is the rule; only the Board knows
          // what `3` currently narrows to.
          filters.map((filter) => ({
            id: `tab:${filter.id}`,
            section: CONTEXT,
            label: filter.label,
            shortcut: filter.shortcut,
            ...(action.icon === null ? {} : { icon: action.icon }),
          }))
        : [entryOf(action, CONTEXT, dormant)],
    ),
    // The rail as it actually is, one row per destination. The registry keeps
    // `bridge_surfaces` as a single row on purpose — its rule is rail order
    // rather than the digits — so the rail is what expands it, and a palette
    // built from that row alone would name no destination at all.
    //
    // **Places that are not in the rail come through here too**, which is what
    // the optional digit above is for. They are destinations by every other
    // measure — a title, a glyph in the registry's Navigation group, a screen
    // that replaces the panel — and leaving them out is the exact failure this
    // section is here to prevent.
    ...surfaces.map((surface) => ({
      id: `nav:${surface.id}`,
      section: NAVIGATION,
      label: surface.label,
      icon: surface.icon,
      ...(surface.shortcut === undefined ? {} : { shortcut: surface.shortcut }),
      ...(surface.aliases === undefined ? {} : { aliases: surface.aliases }),
    })),
    ...globalActs()
      .filter((action) => action.unbuilt !== null || !absent.includes(action.id))
      .map((action) => entryOf(action, NAVIGATION, dormant)),
    ...runnables.map((runnable) => ({
      id: `run:${runnable.id}`,
      section: RUN,
      label: runnable.label,
      // The `run` line, mono and to the right — a setting's own treatment,
      // because it answers the same question: what this row currently is.
      value: runnable.value,
    })),
    ...jobs.map((job) => ({ id: `job:${job.id}`, section: JOBS, label: job.label })),
    ...settings.map((setting) => ({
      id: `set:${setting.id}`,
      section: SETTINGS,
      label: setting.label,
      ...(setting.value === undefined ? {} : { value: setting.value }),
    })),
  ];

  return (
    <CommandPalette
      open={open}
      sections={sections}
      entries={entries}
      searched={searchedSentence(jobs.length, settings.length, runnables.length)}
      onClose={onClose}
      onSelect={(entry) => onChoose(choiceOf(entry.id))}
      onConfirm={(entry) => onConfirmAct(bare(entry.id))}
    />
  );
}

/** One registry row as a palette row. Nothing is re-spelled on the way. */
function entryOf(
  action: Action,
  section: string,
  dormant: Readonly<Record<string, string | undefined>>,
): PaletteEntry {
  // Two kinds of dormancy and one rendering: a binding nothing answers
  // anywhere, which the registry names with an issue, and an act this app
  // cannot reach at this moment, which only the app knows.
  const why =
    action.unbuilt === null ? dormant[action.id] : `not built · ${action.unbuilt}`;
  return {
    id: `act:${action.id}`,
    section,
    label: action.verb,
    shortcut: action.shortcut,
    ...(action.icon === null ? {} : { icon: action.icon }),
    ...(ALIASES[action.id] === undefined ? {} : { aliases: ALIASES[action.id] }),
    ...(action.destructive ? { destructive: true } : {}),
    ...(why === undefined ? {} : { dormant: why }),
  };
}

/** The id without its section prefix. */
function bare(id: string): string {
  return id.slice(id.indexOf(":") + 1);
}

/** What a chosen row was, read off the prefix its section gave it. */
function choiceOf(id: string): PaletteChoice {
  const rest = bare(id);
  if (id.startsWith("act:")) return { of: "act", id: rest };
  if (id.startsWith("nav:")) return { of: "surface", id: rest };
  if (id.startsWith("job:")) return { of: "job", id: rest };
  if (id.startsWith("tab:")) return { of: "filter", id: rest };
  if (id.startsWith("run:")) return { of: "run", id: rest };
  return { of: "setting", id: rest };
}

/**
 * What was searched, for the state where nothing matched.
 *
 * **It names the extent of the index rather than apologising**, and there is
 * no did-you-mean: the palette is the discovery surface, so the useful answer
 * to a miss is what was looked through. The counts are the real ones — a
 * sentence claiming settings on a build that indexes none would be the labelled
 * blank this app refuses everywhere else.
 */
function searchedSentence(jobs: number, settings: number, runnables: number): string {
  const held = [
    "every act available here",
    "every place in Bridge",
    runnables === 0 ? null : "every Check and Command this Manifest declares",
    jobs === 0 ? null : "every job on this Manifest, at every status",
    settings === 0 ? null : "every setting",
  ].filter((part): part is string => part !== null);
  return `${held.slice(0, -1).join(", ")} and ${held[held.length - 1]}`;
}
