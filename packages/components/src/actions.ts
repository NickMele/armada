// What Bridge does with the act registry, and nothing the registry already says.
//
// **The map itself is generated.** `crates/core-model/domain/actions.toml` is
// the authority on what an act is called, what it draws and what it is bound
// to, and `./generated/actions.ts` is that file in TypeScript, emitted by
// `pnpm --filter @armada/desktop codegen` and held to the registry by
// `cargo xtask verify-foundations`. This file used to be a field-for-field
// transcription of the same table; a registry with two copies has two answers
// the day one of them is edited alone.
//
// What is left here is what the registry has no column for and should not: an
// alias a person might type, and which acts a palette offers where it is
// standing. Both are Bridge's readings of the map rather than facts about it.

import { ACTIONS } from "./generated/actions";
import type { Action, ActionScope } from "./generated/actions";

// The map, its types and `ACTION` reach `@armada/components`' consumers through
// here, so an importer neither knows nor cares which half of this pair a name
// came from. Moving the transcription into a generated file is not an API
// change and should not read as one at a call site.
export * from "./generated/actions";

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

/**
 * The two contexts the palette is drawn in.
 *
 * **Not the same thing as a scope.** A scope says where a binding is offered
 * and the registry decides how many there are; a context is where a person is
 * standing, and there are two. `dispatch card` and `piloted job` are conditions
 * inside the detail rather than places of their own.
 */
export type ActionContext = "board" | "detail";

/**
 * Which contexts each scope is offered in.
 *
 * **Keyed by scope rather than by context, so a new scope cannot be silent.**
 * `ActionScope` is generated from the rows of `actions.toml`, so a scope added
 * there widens this record's key set and the build stops here naming it. The
 * other direction — a list of scopes under each context — compiles clean and
 * drops the new scope out of every palette, which is the same defect the
 * registry exists to prevent, one layer down.
 */
const CONTEXTS_FOR: Readonly<Record<ActionScope, readonly ActionContext[]>> = {
  anywhere: ["board", "detail"],
  list: ["board"],
  "list and detail": ["board", "detail"],
  "job board": ["board"],
  detail: ["detail"],
  "dispatch card": ["detail"],
  "piloted job": ["detail"],
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
      CONTEXTS_FOR[action.scope].includes(context),
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
