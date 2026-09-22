// The verbs and tokens the draft's own enum values render as. Draft, for
// `crates/core-model/domain/enum-verbs.toml`.
//
// Source of truth today: `packages/components/src/generated/vocabulary.ts`,
// generated from that registry, which is where every word a surface renders
// comes from. Nothing here is a second copy of a word that file already has —
// each entry below is a value **no registry declares yet**.
//
// **A draft word goes through `enum-verbs.toml` at promotion** (#1532), and the
// entry here is deleted in the same change. Until then this is where a draft
// value gets a verb, so no board types one inline — which is the drift
// `lib/job-states.js` was deleted for.
//
// **Every token is one `packages/tokens/src/status.css` already defines.**
// None of the boards' off-token hexes reaches code (#1530).

import type { Rendering } from "@armada/components/src/generated/vocabulary";

import type { CaseRunOutcome, CaseState } from "./cases";
import type { GroupState } from "./group";
import type { TaskState } from "./task";

/**
 * A draft word, shaped exactly like a generated `Rendering` minus its glyph.
 *
 * **No icon.** `docs/contracts/iconography.md` defaults to none, and only
 * glyphs in `packages/icons/icons.toml` may be drawn — picking one for a value
 * that has no registry row would be minting vocabulary in the place this file
 * exists to stop.
 */
export type DraftWord = Omit<Rendering, "icon" | "hint">;

/**
 * `failed` is the one task state the wire cannot produce.
 *
 * The other four are `TaskState` on the wire and are rendered from the
 * generated vocabulary as before; they are repeated here only so a board has
 * one map to read rather than two.
 */
export const TASK_STATE_WORDS: Readonly<Record<TaskState, DraftWord>> = {
  open: { verb: "open", badgeStatus: "not-started", statusToken: "--status-not-started" },
  working: { verb: "working", badgeStatus: "running", statusToken: "--status-running" },
  done: {
    verb: "done",
    badgeStatus: "completed-success",
    statusToken: "--status-completed-success",
  },
  failed: {
    verb: "failed",
    badgeStatus: "completed-failed",
    statusToken: "--status-completed-failed",
  },
  dropped: { verb: "dropped", badgeStatus: "killed", statusToken: "--status-killed" },
};

/**
 * The group states. **None of these has a registry row**, because no registry
 * has a group in it yet — `job-statuses.toml` is the Job's machine and
 * `step-states.toml` is the step's, and a group is between them.
 */
export const GROUP_STATE_WORDS: Readonly<Record<GroupState, DraftWord>> = {
  pending: {
    verb: "not started",
    badgeStatus: "not-started",
    statusToken: "--status-not-started",
  },
  running: { verb: "running", badgeStatus: "running", statusToken: "--status-running" },
  joining: { verb: "joining", badgeStatus: "running", statusToken: "--status-running" },
  checking: { verb: "checking", badgeStatus: "running", statusToken: "--status-running" },
  passed: {
    verb: "passed",
    badgeStatus: "completed-success",
    statusToken: "--status-completed-success",
  },
  failed: {
    verb: "failed",
    badgeStatus: "completed-failed",
    statusToken: "--status-completed-failed",
  },
  retrying: {
    verb: "retrying",
    badgeStatus: "awaiting-review",
    statusToken: "--status-awaiting-review",
  },
  landed: {
    verb: "landed",
    badgeStatus: "completed-success",
    statusToken: "--status-completed-success",
  },
};

/**
 * `classifying` is #1159's `proposing` status by the word the boards use. It is
 * the one Job status in this file, and it is here because the registry spells
 * it the other way.
 */
export const CLASSIFYING_WORD: DraftWord = {
  verb: "classifying",
  badgeStatus: "awaiting-approval",
  statusToken: "--status-awaiting-approval",
};

/**
 * What became of a run — **never of the work**.
 *
 * `run_failed` is the harness falling over and says nothing about whether the
 * change is right. A case with no spec is `not_run`, which reads as not
 * covered.
 */
export const CASE_RUN_OUTCOME_WORDS: Readonly<Record<CaseRunOutcome, DraftWord>> = {
  ran: { verb: "ran", badgeStatus: "running", statusToken: "--status-running" },
  run_failed: {
    verb: "the run failed",
    badgeStatus: "completed-failed",
    statusToken: "--status-completed-failed",
  },
  not_run: {
    verb: "not covered",
    badgeStatus: "not-started",
    statusToken: "--status-not-started",
  },
};

/** What a task owes. `dropped` is a case that fell out, never one that passed. */
export const CASE_STATE_WORDS: Readonly<Record<CaseState, DraftWord>> = {
  owed: { verb: "owed", badgeStatus: "awaiting-review", statusToken: "--status-awaiting-review" },
  dropped: { verb: "dropped", badgeStatus: "killed", statusToken: "--status-killed" },
  added: { verb: "added", badgeStatus: "not-started", statusToken: "--status-not-started" },
};

/**
 * Every draft word in one list, so `#1545` can read what this module owes the
 * registry without opening each map.
 */
export const DRAFT_VOCABULARIES: readonly {
  readonly vocabulary: string;
  readonly words: Readonly<Record<string, DraftWord>>;
}[] = [
  { vocabulary: "task_state", words: TASK_STATE_WORDS },
  { vocabulary: "group_state", words: GROUP_STATE_WORDS },
  { vocabulary: "case_run_outcome", words: CASE_RUN_OUTCOME_WORDS },
  { vocabulary: "case_state", words: CASE_STATE_WORDS },
  { vocabulary: "job_status", words: { classifying: CLASSIFYING_WORD } },
];
