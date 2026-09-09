import type { ReactNode } from "react";

/**
 * Judge citations — every pointer the panel made, as selectors.
 *
 * **A judgment is mostly a set of pointers into other artifacts.** The verdict
 * grid says which way each judge went; this says what each of them looked at
 * to get there. Pressing one puts that artifact in the viewer, scrolled to the
 * cited region — which makes this the densest navigation surface on the
 * screen.
 *
 * **A judge that met a criterion is here too**, wherever there is anything to
 * put in a row. That is what separates *a judge caught something* from *the
 * criterion is ambiguous*, and without it a lone dissent is unreadable — a
 * list that showed only refusals would throw away the half of the comparison
 * that does the work.
 *
 * **What the wire cannot fill that in from, today.** `Judged.cited` places
 * every quotation a member made, and a `met` answer is one line under the
 * Judge's answer format — no prose, so nothing to quote and nothing to place.
 * So a screen fed by Fleet draws refusals here and a judge that met a criterion
 * appears only where something else recorded what it read. `JudgeInputs` is the
 * reading that does carry across both verdicts.
 *
 * **Rows are grouped by nothing and sorted by nothing.** They are in the order
 * the panel recorded them, per criterion, per judge — a citation list that
 * reordered itself would be a second index over a record whose own order is
 * evidence of how the panel ran.
 *
 * **The wire serves this.** `Judged.cited` arrived in protocol 8.3: one entry
 * per quotation, carrying the labelled part of the brief that holds it and the
 * lines it is on, which is what `where` renders. `screens`' `cited.ts` is what
 * builds the rows. This block said fixtures only until 2026-09-09.
 */

export type JudgeCitation = {
  /** What a selection names — the artifact this points into. */
  id: string;
  /** Which judge and which criterion — `j1 · 02`. Mono, and it leads the row. */
  who: ReactNode;
  /** The criterion, in the requester's words. Sans. */
  criterion: ReactNode;
  /** What was cited, in mono — `check:test_suite lines 2007–2008`. */
  where: ReactNode;
  /** Which way that judge went on that criterion. */
  named: "met" | "not_met";
  /** The word for it — `refusal`, `met`. */
  verdict?: ReactNode;
  onOpen?: (citedId: string) => void;
};

export type JudgeCitationsProps = {
  rows: JudgeCitation[];
  /** The label over the list, where it stands on its own. */
  label?: ReactNode;
  /** What a panel that cited nothing says. Never an empty frame. */
  emptyNote?: ReactNode;
};

export function JudgeCitations({ rows, label, emptyNote }: JudgeCitationsProps) {
  return (
    <div className="armada-judge-citations">
      {label ? <span className="armada-judge-citations__label">{label}</span> : null}
      {rows.length === 0 ? (
        <p className="armada-judge-citations__empty">{emptyNote}</p>
      ) : (
        <ul className="armada-judge-citations__list">
          {rows.map((row, at) => (
            <li className="armada-judge-citations__row" key={at} data-named={row.named}>
              {/* The whole row is the control. A citation is one pointer and
                  splitting it into a label and a target would make the words
                  beside the target look like something else. */}
              <button
                type="button"
                className="armada-judge-citations__open"
                onClick={() => row.onOpen?.(row.id)}
              >
                <span className="armada-judge-citations__who">{row.who}</span>
                <span className="armada-judge-citations__what">
                  <span className="armada-judge-citations__criterion">{row.criterion}</span>
                  <span className="armada-judge-citations__where">{row.where}</span>
                </span>
                {row.verdict === undefined ? null : (
                  <span className="armada-judge-citations__verdict">{row.verdict}</span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
