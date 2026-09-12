// What each member of a panel read, and what each of them was handed.
//
// **Two readings off the same rows, opposite questions.** `Judged.cited` is
// where in the brief a member's own words are quoted from — what it read.
// `Judged.given` is a digest of the text sent — what it was handed: the
// first says whether two refusals share lines, the second whether the
// panel was a panel at all.
//
// **Not in `gates.ts`, deliberately** — that file holds what two surfaces
// must not disagree about; nothing else reads these two, so a third
// reading beside the shared ones would make its claim harder to check.
//
// **Both served, both absent on old rows** — `cited` and `given` arrived
// in protocol 8.3; a Fleet before it sends neither, and every function
// here answers with nothing rather than a placeholder.

import { CRITERION_VERDICT_JUDGE } from "@armada/components";
import type { JudgeCitation, JudgeInputRow, JudgeInputsFor } from "@armada/components";
import type { Citation, Judged } from "@armada/protocol";

import { judgeNamed, type Panel } from "./gates";

/**
 * Every pointer the panel made, one row per quotation.
 *
 * **In the order the panel recorded them** — criterion, then member, then
 * quotation order. `panelsOf` holds the first two orderings, `Judged.cited`
 * the third, so nothing here sorts: a reordered list would be a second
 * index over a record whose own order is evidence of how the panel ran.
 *
 * **Empty and `null` are different answers.** Empty is a panel that quoted
 * nothing placeable — it met everything, or refused in its own words —
 * worth a sentence. `null` is a Fleet before 8.3, recording no citations for
 * anybody; the honest rendering is no region at all, since a note saying
 * the refusals described rather than quoted would be a claim about prose
 * nobody read.
 */
export function citationsOf(
  panels: readonly Panel[],
  panelSize: number,
  onOpen: (path: string) => void,
): JudgeCitation[] | null {
  if (!panels.some((panel) => panel.members.some((one) => one.cited !== undefined))) return null;
  const rows: JudgeCitation[] = [];
  for (const panel of panels) {
    for (const one of panel.members) {
      for (const citation of one.cited ?? []) {
        rows.push({
          // The brief is what a citation points into, and it is the one
          // artifact the row carries a path to. A member with no kept brief
          // still gets its row: where the words are is worth reading even
          // where the file they are in was not kept.
          id: one.brief_path ?? "",
          who: whoSaid(one, panel, panelSize),
          criterion: panel.criterion?.text ?? panel.criterionId,
          where: whereItIs(citation),
          named: one.verdict === "met" ? "met" : "not_met",
          verdict: CRITERION_VERDICT_JUDGE[one.verdict]?.verb ?? one.verdict,
          ...(one.brief_path === undefined
            ? {}
            : { onOpen: (path: string) => onOpen(path) }),
        });
      }
    }
  }
  return rows;
}

/**
 * Which judge and which criterion — `j2 · 02`.
 *
 * **No judge at a panel of one.** `Judged.member` is absent there and naming it
 * `j1` would be this screen inventing a position the record deliberately does
 * not carry. What is left is the criterion, which is the half a citation names
 * at any panel size.
 *
 * The criterion's frozen position, not its place on screen — and its id where
 * the Job carries no position for it, because a number cannot be invented.
 */
function whoSaid(one: Judged, panel: Panel, panelSize: number): string {
  const at = panel.ordinal === undefined ? panel.criterionId : ordinalSaid(panel.ordinal);
  return panelSize < 2 ? at : `${judgeNamed(one)} · ${at}`;
}

/** A criterion's frozen position, as the grid's own column spells it. */
function ordinalSaid(ordinal: number): string {
  return String(ordinal).padStart(2, "0");
}

/**
 * What was cited — `check:test_suite lines 2007–2008`.
 *
 * **The region and the lines, and no quoted words.** The words are in
 * `expected` and `produced` on the same record, four inches up the screen
 * inside the refusal; repeating them here would make the list a second copy of
 * the finding rather than an index into what it was read from.
 */
function whereItIs(citation: Citation): string {
  const lines =
    citation.from_line === citation.to_line
      ? `line ${citation.from_line}`
      : `lines ${citation.from_line}–${citation.to_line}`;
  return `${citation.region} ${lines}`;
}

/**
 * What one criterion's panel was handed, or nothing.
 *
 * **One block per criterion, never one per step.** Every criterion is its own
 * brief — the panel loop builds one and hands it to every member — so a digest
 * folded across criteria would differ for a reason that says nothing, and the
 * one comparison that matters would be lost inside it.
 *
 * **Nothing where no member recorded it**, which is every row a Fleet before
 * 8.3 wrote. An empty digest table would read as a panel handed nothing.
 */
export function givenTo(panel: Panel): {
  rows: JudgeInputRow[];
  each: JudgeInputsFor[];
  identical: boolean;
} | null {
  const handed = panel.members.filter((one) => one.given !== undefined);
  if (handed.length === 0) return null;
  const fields = FIELDS.map((field) => ({
    field,
    values: handed.map((one) => field.of(one)),
  }));
  const differing = fields.filter(({ values }) => new Set(values).size > 1);
  const rows = fields.map(({ field, values }) => ({
    name: field.name,
    // Where they agree, the value they agree on. Where they do not, every
    // value there was — a compare view that showed one of two would be
    // answering the question it exists to ask by picking a side.
    value: [...new Set(values)].join(" · "),
    ...(new Set(values).size > 1 ? { differs: true } : {}),
  }));
  const each = handed.map((one) => ({
    id: judgeNamed(one),
    judge: judgeNamed(one),
    rows: fields.map(({ field }) => ({
      name: field.name,
      value: field.of(one),
      // Marked in the per-judge view too. Reading one judge's object alone,
      // nothing about a digest says it is the odd one out.
      ...(differing.some((odd) => odd.field.name === field.name) ? { differs: true } : {}),
    })),
  }));
  return { rows, each, identical: differing.length === 0 && handed.length === panel.members.length };
}

/**
 * The three readings of what a member was handed.
 *
 * **Machine-derived, every one of them**, which is why the component draws
 * every value in mono: nothing here was written by anybody, it is what Fleet
 * handed the panel, read back.
 */
const FIELDS: readonly { name: string; of: (one: Judged) => string }[] = [
  { name: "brief digest", of: (one) => one.given?.digest ?? "" },
  { name: "brief size", of: (one) => `${one.given?.size ?? 0} characters` },
  { name: "model", of: (one) => one.given?.model ?? "" },
];
