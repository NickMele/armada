// What Fleet holds, as a proposal may name it. `crates/ipc/src/setup.rs`.
//
// Its own file for the reason the Rust side gives it one: these are what the
// three roster reads answer, and a proposal is the only thing that consumes
// them. Nothing on this page is about a Job that exists.
//
// It is also `protocol.ts`'s third split, after `history.ts` and `turn.ts`, and
// this one had a second reason: that file was at exactly the gate's 500-line
// warning, and `WorkflowStep` was about to take three fields. Splitting a
// module that already stands alone in Rust beats accepting a warning.
//
// Hand-written like `protocol.ts`, and a second statement of the Rust shapes
// for the same reason: the codegen that would emit both does not exist yet.

import type { DeclaredCheck, DeclaredJudge } from "./protocol";

/** One workflow Fleet holds. `crates/ipc/src/setup.rs`. */
export type WorkflowSummary = {
  /** What a proposal's `workflow_id` must name. Anything else is refused. */
  id: string;
  /** What a person reads. `bug`, where the id is what a Job points at. */
  name: string;
  version: number;
  /**
   * The steps, in order. Order is the semantics.
   *
   * **Objects since protocol 3, not step ids.** A field whose type changed is a
   * major bump, and the reason it changed is that a composer offering a
   * workflow could say how many steps it had and not whether any of them gates.
   */
  steps: WorkflowStep[];
  manifest_id: string;
};

/**
 * One step of a workflow, as the definition declares it.
 *
 * **The same three declarations `StepDetail` carries**, in the same shapes:
 * what the step checks, what it asks the Judge, and what it takes to advance.
 * What a person approves before a dispatch and what the rail shows during one
 * are one sentence read at two moments, so `rail.ts` and `preview.ts` spell
 * them through the same functions rather than twice.
 *
 * **None of them is optional here, and all three are on `StepDetail`.** A
 * workflow Fleet is serving is a workflow Fleet holds, so there is no "cannot
 * say" to spell — which is the whole of what the running rail's optionality is
 * for.
 */
export type WorkflowStep = {
  step_id: string;
  /**
   * What a person reads. **Never absent and never blank** — Fleet substitutes
   * the `step_id` where the definition declares no label, which is why a label
   * that *is* the id renders in mono rather than as a name.
   */
  label: string;
  /** Always a list, never absent. Empty is the ungated step. */
  checks: DeclaredCheck[];
  /**
   * What the step asks of the Judge, in the order it asks it. Counts and panel
   * sizes; the questions themselves do not cross.
   *
   * **Empty is "the Judge is never called on this step"**, which is most steps,
   * and an inert entry does not cross at all — a step that previewed as judged
   * with no Judge to call would be worse than one that previewed as neither.
   */
  judge_checks: DeclaredJudge[];
  /**
   * What it takes to advance past this step — `auto`, `auto_if_judge_passes`
   * or `human_always`.
   *
   * **This is what the preview was missing.** A workflow that will stop and
   * wait for a person at `handoff` is the substance of what somebody approves,
   * and it previewed as a step with nothing on it.
   */
  advance_gate: string;
};

/** One Manifest Fleet holds. */
export type ManifestSummary = {
  id: string;
  /**
   * The repository the Manifest was read from. **Not a name it declares** —
   * `armada.yml` has no key for one, so this is the directory, which is a fact
   * rather than an invention.
   */
  repository: string;
  path: string;
  version: number;
  checks: string[];
};

/** The models a Job may name, and the one it gets when it names none. */
export type ModelChoices = {
  models: string[];
  /** Always a member of `models`, so a picker selects it without a lookup. */
  default: string;
};
