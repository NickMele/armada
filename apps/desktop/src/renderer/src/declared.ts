// What a step declares, in the words a rail row uses for it.
//
// **One spelling, read at two moments.** `rail.ts` draws a running Job's steps
// from `StepDetail` and `preview.ts` draws a workflow's from `WorkflowStep`;
// the declarations are the same three on both, in the same shapes, because
// `crates/ipc/src/setup.rs` says so. Two copies of "judge · 2 criteria · panel
// of 3" would agree until one of them changed, which is the same reason
// `DeclaredJudge::firing` is one function in Fleet rather than the same filter
// at each call site.
//
// Nothing here reads a state or a result. A declaration is knowable before
// anything runs — that is the whole reason these rows exist — so what the gate
// found stays with the caller that has it.

import type { WorkflowRailDeclaration } from "@armada/components";

import { ADVANCE_GATE } from "../../shared/generated/vocabulary";
import type { DeclaredCheck, DeclaredJudge } from "../../shared/protocol";

/**
 * The Check's name, or the built-in's kind where it names none.
 * `diff_nonempty` is an assertion rather than a Manifest Check, so it carries
 * the kind and nothing invents a name for it.
 */
export function nameOf(check: DeclaredCheck): string {
  return check.name ?? check.kind;
}

/**
 * One Check as a gate row's mono command: the name, then what the frozen
 * workflow resolved it to.
 *
 * `build` says nothing about what gated the step; the command says what to run
 * to reproduce it. `diff_nonempty` runs nothing and carries none.
 */
export function commandOf(check: DeclaredCheck): string {
  return check.run === undefined ? nameOf(check) : `${nameOf(check)} · ${check.run}`;
}

/**
 * Which paths a Check covers, or nothing where it covers everything.
 *
 * **Absent means always, and it draws nothing.** Most Checks declare no `when`,
 * and a row saying "covers everything" on every one of them would bury the two
 * that say something. Fleet sends no key rather than an empty list, so there is
 * no empty case to disambiguate here.
 *
 * **This is the half that is only useful before the Check runs.** Once the gate
 * has skipped one, its `check_runs` row is `not run` and names the paths itself;
 * before, this is the only thing that tells a reader a Check they expect to see
 * will not be spent on this Job.
 */
export function coversOf(check: DeclaredCheck): string | undefined {
  return check.when === undefined ? undefined : `when ${check.when.join(", ")}`;
}

/**
 * One declared `judge_checks[]` entry, in counts.
 *
 * `judge` is the verification source named in text, which the iconography
 * contract settles for exactly this reason — Check, Judge and Attestation are
 * shorter and more precise as words than any glyph, and every verdict family is
 * reserved to a source rather than to a declaration.
 *
 * **Counts, never a question.** `DeclaredJudge` carries how many criteria are
 * asked and how many judges answer, and deliberately carries no prompt: a
 * question drawn on a rail is a prompt in a screenshot.
 *
 * **`panel_size` is absent at one**, so a value here always means a panel and
 * nothing compares against a default that is already the domain's. An entry
 * asking no criteria only looks for gaming, and says that and nothing else.
 */
export function judgeOf(judge: DeclaredJudge): string {
  const said = ["judge"];
  if (judge.criteria > 0) {
    said.push(`${judge.criteria} ${judge.criteria === 1 ? "criterion" : "criteria"}`);
  }
  if (judge.panel_size !== undefined) said.push(`panel of ${judge.panel_size}`);
  if (judge.gaming_check) said.push("gaming check");
  return said.join(" · ");
}

/** The one gate whose whole meaning is that the Checks above are the whole gate. */
const AUTO = "auto";

/**
 * What it takes to advance past the step, where that is more than its Checks.
 *
 * **`auto` draws no row.** It says the mechanical tier is the whole gate, which
 * the gate rows above already are — and a row on every step of every workflow
 * would bury the two values that matter and displace the sentence the design
 * contract requires on a step that genuinely checks nothing.
 *
 * **The word is the wire's own.** `enum-verbs.toml` carries no `advance_gate`
 * rows, so `ADVANCE_GATE` is empty and `human_always` renders as itself — the
 * same fallback `step_state` takes. A phrase chosen here would be the second
 * vocabulary the generated module exists to prevent. Reported.
 */
export function advanceOf(gate: string | undefined): WorkflowRailDeclaration | undefined {
  if (gate === undefined || gate === AUTO) return undefined;
  return { label: `advance_gate · ${ADVANCE_GATE[gate]?.verb ?? gate}` };
}
