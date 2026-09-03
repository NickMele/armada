// Every act Bridge offers: what it is called, what it is bound to, and the
// glyph it draws — or why it draws none.
//
// **`crates/core-model/domain/actions.toml` is the authority and this is a
// transcription of it**, in the same relationship `packages/screens/src/keys.ts`
// has to the same file and for the same reason: a TypeScript surface cannot
// read a TOML file in a browser, and the codegen that would emit this belongs
// to #232. `xtask`'s action rule reads the registry against the contract's key
// map; a third subject is what would close this file into the same loop, and
// until it does, a row edited here and not there is drift.
//
// **A Motion is here and is not an act.** `move_focus`, `open_focused` and
// `focus_chapter` move the cursor and act on nothing; the registry says they
// appear in no palette and carry no glyph, so they are transcribed for
// completeness and filtered out by anything that draws a list of acts.
//
// **A blank glyph is a fact, not a default.** Thirteen rows carry no icon:
// twelve because no registered silhouette means them and assigning one is a
// decision for `packages/icons/icons.toml`, and one — Copy debug info —
// because the contract decided it carries none. Both are spelled here, so a
// surface can say which kind of blank it is drawing rather than inventing a
// glyph to fill the column.
//
// **`unbuilt` names the issue that answers the key.** The registry is ahead of
// the app deliberately, because the map was settled by drawing. A palette that
// displays a binding beside every entry would otherwise offer a row a person
// presses and gets nothing from, which is worse than one that is absent.

import {
  ChevronRight,
  CornerUpRight,
  Eye,
  FileDiff,
  Filter,
  MessageSquare,
  Power,
  RotateCw,
  Search,
  Stamp,
  Terminal,
  X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

/** Whether the row is an act or a movement of the cursor. */
export type ActionKind = "Action" | "Motion";

/** Modifier-based and working anywhere, or single-key and on what is focused. */
export type ActionTier = "Global" | "Contextual";

/**
 * Where the binding is offered. The registry's own set, spelled the registry's
 * way — a second spelling here is the drift this file already risks once.
 */
export type ActionScope =
  | "anywhere"
  | "list"
  | "list and detail"
  | "detail"
  | "job board"
  | "piloted job"
  | "dispatch card";

/** Why an act's glyph column is empty. `null` where it is not. */
export type IconAbsence = "undecided" | "by design";

export type Action = {
  /** The id an implementation binds to. The registry's table key. */
  readonly id: string;
  readonly kind: ActionKind;
  readonly tier: ActionTier;
  /** What a person reads, in the lexicon's word. Never the id. */
  readonly verb: string;
  /** The glyph, or `null` — in which case `iconAbsent` says why. */
  readonly icon: LucideIcon | null;
  readonly iconAbsent: IconAbsence | null;
  /** The binding, spelled as the contract's map spells it. */
  readonly shortcut: string;
  readonly scope: ActionScope;
  readonly destructive: boolean;
  readonly confirms: boolean;
  /** The issue that gives the binding an act, on a row nothing answers yet. */
  readonly unbuilt: string | null;
};

/**
 * The map, in the registry's order: global first, then contextual.
 *
 * Order is load-bearing in one place only — the palette groups by section and
 * keeps registry order inside each — so nothing here is sorted.
 */
export const ACTIONS: readonly Action[] = [
  {
    id: "command_palette",
    kind: "Action",
    tier: "Global",
    verb: "Command palette",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "⌘K",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "bridge_surfaces",
    kind: "Action",
    tier: "Global",
    verb: "Bridge surfaces",
    icon: null,
    iconAbsent: "by design",
    shortcut: "⌘1–⌘4",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "helm",
    kind: "Action",
    tier: "Global",
    verb: "Helm",
    icon: MessageSquare,
    iconAbsent: null,
    shortcut: "⌘5",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "toggle_sidebar",
    kind: "Action",
    tier: "Global",
    verb: "Toggle sidebar",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "⌘\\",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "history",
    kind: "Action",
    tier: "Global",
    verb: "Back / forward",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "⌘[ ⌘]",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "close",
    kind: "Action",
    tier: "Global",
    verb: "Close",
    icon: X,
    iconAbsent: null,
    shortcut: "Esc",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "move_focus",
    kind: "Motion",
    tier: "Contextual",
    verb: "Move focus",
    icon: null,
    iconAbsent: null,
    shortcut: "j / k / ↓ / ↑",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "open_focused",
    kind: "Motion",
    tier: "Contextual",
    verb: "Open the focused job",
    icon: null,
    iconAbsent: null,
    shortcut: "Enter",
    scope: "list",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "open",
    kind: "Action",
    tier: "Contextual",
    verb: "Open",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "o",
    scope: "list",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "review",
    kind: "Action",
    tier: "Contextual",
    verb: "Review",
    icon: Eye,
    iconAbsent: null,
    shortcut: "r",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "attest",
    kind: "Action",
    tier: "Contextual",
    verb: "Attest",
    icon: Stamp,
    iconAbsent: null,
    shortcut: "t",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "redirect",
    kind: "Action",
    tier: "Contextual",
    verb: "Redirect",
    icon: CornerUpRight,
    iconAbsent: null,
    shortcut: "d",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "approve",
    kind: "Action",
    tier: "Contextual",
    verb: "Approve",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "a",
    scope: "dispatch card",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "restart_step",
    kind: "Action",
    tier: "Contextual",
    verb: "Restart step",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "s",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "pilot",
    kind: "Action",
    tier: "Contextual",
    verb: "Pilot",
    icon: Terminal,
    iconAbsent: null,
    shortcut: "p",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: "#250",
  },
  {
    id: "copy_debug_info",
    kind: "Action",
    tier: "Contextual",
    verb: "Copy debug info",
    icon: null,
    iconAbsent: "by design",
    shortcut: "c",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "report_job",
    kind: "Action",
    tier: "Contextual",
    verb: "Report this job",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "b",
    scope: "detail",
    destructive: false,
    confirms: true,
    unbuilt: null,
  },
  {
    id: "kill",
    kind: "Action",
    tier: "Contextual",
    verb: "Kill",
    icon: Power,
    iconAbsent: null,
    shortcut: "x",
    scope: "list and detail",
    destructive: true,
    confirms: true,
    unbuilt: null,
  },
  {
    id: "kill_and_redispatch",
    kind: "Action",
    tier: "Contextual",
    verb: "Kill & redispatch",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "X",
    scope: "detail",
    destructive: true,
    confirms: true,
    unbuilt: "#291",
  },
  {
    id: "new_job",
    kind: "Action",
    tier: "Contextual",
    verb: "New job",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "n",
    scope: "anywhere",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "search",
    kind: "Action",
    tier: "Contextual",
    verb: "Search",
    icon: Search,
    iconAbsent: null,
    shortcut: "/",
    scope: "list",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "state_filter",
    kind: "Action",
    tier: "Contextual",
    verb: "State filter",
    icon: Filter,
    iconAbsent: null,
    shortcut: "1–5",
    scope: "job board",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "observe",
    kind: "Action",
    tier: "Contextual",
    verb: "Observe",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "v",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "submit_for_verification",
    kind: "Action",
    tier: "Contextual",
    verb: "Submit for verification",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "u",
    scope: "piloted job",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "redispatch",
    kind: "Action",
    tier: "Contextual",
    verb: "Redispatch as a new job",
    icon: RotateCw,
    iconAbsent: null,
    shortcut: "e",
    scope: "list and detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "disclose",
    kind: "Action",
    tier: "Contextual",
    verb: "Expand and collapse",
    icon: ChevronRight,
    iconAbsent: null,
    shortcut: "h / l / ← / →",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "focus_chapter",
    kind: "Motion",
    tier: "Contextual",
    verb: "Move between chapters",
    icon: null,
    iconAbsent: null,
    shortcut: "[ ]",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "open_log",
    kind: "Action",
    tier: "Contextual",
    verb: "Open the log",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "Enter",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "open_diff",
    kind: "Action",
    tier: "Contextual",
    verb: "Open the diff",
    icon: FileDiff,
    iconAbsent: null,
    shortcut: "f",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
  {
    id: "open_stage",
    kind: "Action",
    tier: "Contextual",
    verb: "Open the stage",
    icon: null,
    iconAbsent: "undecided",
    shortcut: "g",
    scope: "detail",
    destructive: false,
    confirms: false,
    unbuilt: null,
  },
];

/**
 * What a person might type looking for an act, and never what renders.
 *
 * **Bridge's, not the registry's.** `actions.toml` carries no alias column and
 * should not: it is the authority on what an act *is called*, and the lexicon's
 * whole point is that one word wins. The search index is a different artifact —
 * the contract says so in as many words, that the index may carry aliases so
 * "terminate" finds Kill and the alias never renders.
 *
 * So a word here is a way in, and adding one costs nothing. What it may never
 * do is reach the screen: the palette is where a person learns Armada's
 * vocabulary, and a surface that answers "terminate" with the word "terminate"
 * has taught them the wrong one.
 */
export const ALIASES: Readonly<Record<string, readonly string[] | undefined>> = {
  kill: ["terminate", "stop", "abort", "end"],
  kill_and_redispatch: ["terminate and retry", "restart the job"],
  redispatch: ["retry", "again", "rerun"],
  redirect: ["steer", "correct", "instruct"],
  restart_step: ["retry the step", "run the step again"],
  new_job: ["dispatch", "start", "compose"],
  copy_debug_info: ["clipboard", "support", "envelope"],
  report_job: ["bug", "file", "wrong"],
  observe: ["watch", "transcript", "terminal"],
  open_diff: ["patch", "changes", "files"],
  open_log: ["activity", "turns", "history"],
  open_stage: ["gate", "phase", "check"],
  state_filter: ["tab", "narrow", "status"],
  toggle_sidebar: ["rail", "hide the rail"],
  submit_for_verification: ["hand back", "verify"],
};

/** One act, by the id an implementation binds to. */
export const ACTION: Readonly<Record<string, Action | undefined>> = Object.fromEntries(
  ACTIONS.map((action) => [action.id, action]),
);

/**
 * The two contexts the palette is drawn in.
 *
 * **Not the same thing as a scope.** A scope says where a binding is offered
 * and there are seven of them; a context is where a person is standing, and
 * there are two. `dispatch card` and `piloted job` are conditions inside the
 * detail rather than places of their own.
 */
export type ActionContext = "board" | "detail";

/** Which scopes each context admits. */
const ADMITS: Readonly<Record<ActionContext, readonly ActionScope[]>> = {
  board: ["anywhere", "list", "list and detail", "job board"],
  detail: ["anywhere", "list and detail", "detail", "dispatch card", "piloted job"],
};

/**
 * The acts a context offers on what is in front of you, in registry order.
 *
 * **Contextual tier only.** The global acts reach a surface rather than a Job,
 * so they do not belong under a block titled with the job it acts on — a
 * palette that says `job_2d90bb` and then offers Toggle sidebar under that
 * heading is a heading that lies. `globalActs` is where they are.
 *
 * **Motions are dropped.** A Motion acts on nothing, so a row for it would be
 * a row that does nothing when chosen — the same reading the contract gives
 * unbuilt rows, one step further along.
 */
export function actsIn(context: ActionContext): readonly Action[] {
  return ACTIONS.filter(
    (action) =>
      action.kind === "Action" &&
      action.tier === "Contextual" &&
      ADMITS[context].includes(action.scope),
  );
}

/**
 * The global acts, which reach a surface rather than a Job.
 *
 * **Two rows are left out and neither is an omission.** `command_palette` is
 * the one act that cannot be reached from inside the palette, since the
 * palette is what is already open; and `bridge_surfaces` is one registry row
 * standing for the whole rail, because the contract's rule there is rail order
 * rather than the digits. A palette draws the rail's destinations one per row,
 * from the rail — which is the only thing that knows what `⌘3` currently
 * reaches.
 */
export function globalActs(): readonly Action[] {
  return ACTIONS.filter(
    (action) =>
      action.kind === "Action" &&
      action.tier === "Global" &&
      action.id !== "command_palette" &&
      action.id !== "bridge_surfaces",
  );
}
