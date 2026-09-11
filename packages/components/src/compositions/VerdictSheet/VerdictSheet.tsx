import type { ReactNode } from "react";

/**
 * Verdict sheet — the record job detail shows at the one place a Job stops for
 * a person, and again once it is over. One page, read top to bottom: what was
 * asked for, what came back, what proves it, what was left alone, the figures,
 * then the buttons.
 *
 * **Chosen as Option B, 2026-09-08.** It is buildable on today's wire and it
 * moves between four scenarios — a gate before a pull request, a gate with one
 * open, a gate on a workflow that never delivers, and a finish nothing asked —
 * by adding or dropping a labelled block. Nothing else about the page shifts.
 * The cost, kept on record: a second arrangement on a screen whose whole
 * argument is that there is one.
 *
 * **The buttons decide nothing here.** `actions` is `Decide`'s own region,
 * unchanged — this component draws the record around it. Where `actions` is
 * absent, nothing is being asked and the dashed `recordNote` says so instead of
 * a row of controls that would have nothing to do.
 *
 * **`pullRequest` is the only block that comes and goes with a fact rather than
 * with the render.** It is drawn where a pull request is open, and it is not
 * there is where none is — a workflow that never delivers, or one whose Drone
 * has not pushed yet.
 */
export type VerdictFigure = {
  /** `Branch`, `Files`, `Took`, `Steps`, `Pull request`. */
  label: string;
  /** Absent where nothing serves it, which the caller says in words instead. */
  value?: ReactNode;
  absent?: ReactNode;
  /** Machine-derived — a branch, a count, a duration, a cost. */
  mono?: boolean;
};

export type VerdictSheetProps = {
  /**
   * The record's own headline, above "What you asked for". **Only where the
   * Job is closed and a person answered it** — the fifth arrangement, beside
   * the gate and the nothing-asked finish. Neither the word nor the moment is
   * chosen here: both are the screen's, off the transition that closed the Job.
   */
  header?: { done: ReactNode; when: ReactNode };
  /** What was asked for — the Job's own title. */
  title: ReactNode;
  /** The acceptance criteria the Job was frozen with, one line each. */
  criteria: readonly ReactNode[];
  /** What stands in for the criteria list where the Job carries none. */
  criteriaAbsent?: ReactNode;
  /** What came back — the Drone's own claim, `Submitted.claimed`. */
  cameBack: ReactNode;
  /** The deliverable this step kept, as a control that opens it. */
  deliverable?: ReactNode;
  /**
   * The pull request block, where this Job has one open. **Presence is the
   * whole of what draws it** — absent is a workflow that never delivers, or one
   * whose Drone has not pushed yet, and neither is drawn as an empty card.
   */
  pullRequest?: ReactNode;
  /** What proves it — a `CheckRuns` list, or the sentence that stands in for one. */
  provesIt: ReactNode;
  /** The line under the checklist, where a step's evidence is the whole of it. */
  provesItNote?: ReactNode;
  /** What it left alone — `Submitted.not_claimed`, or why there is nothing here. */
  leftAlone: ReactNode;
  /** The figures, in the order the drawing runs them. */
  figures: readonly VerdictFigure[];
  /** A standing sentence above the buttons — what merging costs, what ending here means. */
  note?: ReactNode;
  /** `Decide`'s own region. Absent is nothing being asked of anyone. */
  actions?: ReactNode;
  /** Drawn instead of `actions`, in the dashed frame an empty state takes. */
  recordNote?: ReactNode;
};

export function VerdictSheet({
  header,
  title,
  criteria,
  criteriaAbsent,
  cameBack,
  deliverable,
  pullRequest,
  provesIt,
  provesItNote,
  leftAlone,
  figures,
  note,
  actions,
  recordNote,
}: VerdictSheetProps) {
  return (
    <div className="armada-verdict">
      {header === undefined ? null : (
        <div className="armada-verdict__header">
          <span className="armada-verdict__done">{header.done}</span>
          <span className="armada-verdict__when">{header.when}</span>
        </div>
      )}
      <Block label="What you asked for">
        <p className="armada-verdict__lede">{title}</p>
        {criteria.length === 0 ? (
          criteriaAbsent === undefined ? null : (
            <p className="armada-verdict__said">{criteriaAbsent}</p>
          )
        ) : (
          <ul className="armada-verdict__criteria">
            {criteria.map((one, i) => (
              <li key={i}>{one}</li>
            ))}
          </ul>
        )}
      </Block>

      <Block label="What came back">
        <p className="armada-verdict__said">{cameBack}</p>
        {deliverable === undefined ? null : (
          <div className="armada-verdict__document">{deliverable}</div>
        )}
      </Block>

      {pullRequest === undefined ? null : (
        <Block label="The pull request">{pullRequest}</Block>
      )}

      <Block label="What proves it">
        {provesIt}
        {provesItNote === undefined ? null : (
          <p className="armada-verdict__said">{provesItNote}</p>
        )}
      </Block>

      <Block label="What it left alone">
        <p className="armada-verdict__said">{leftAlone}</p>
      </Block>

      <ul className="armada-verdict__figures">
        {figures.map((figure, i) => (
          <li className="armada-verdict__figure" key={i}>
            <span className="armada-verdict__figure-label">{figure.label}</span>
            <span className="armada-verdict__figure-value" data-mono={figure.mono || undefined}>
              {figure.value === undefined ? figure.absent : figure.value}
            </span>
          </li>
        ))}
      </ul>

      {note === undefined ? null : <p className="armada-verdict__said">{note}</p>}

      {actions === undefined ? (
        <div className="armada-verdict__record">
          {recordNote ?? "Nothing is asked of anyone. This is the Job's record."}
        </div>
      ) : (
        <div className="armada-verdict__actions">{actions}</div>
      )}
    </div>
  );
}

/** One labelled section. The caps label is the section's whole identity. */
function Block({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="armada-verdict__block">
      <span className="armada-verdict__label">{label}</span>
      {children}
    </div>
  );
}
