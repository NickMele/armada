import type { ReactNode } from "react";

/**
 * Active jobs list — the framed list of Job rows, and the header above it.
 *
 * **Ordering carries the trigger, not a control.** Rows that need a person sort
 * above rows that do not, and within each group the oldest first, because the
 * thing waiting longest is the thing most likely to have gone wrong. The list
 * renders the order it is handed: sorting is Fleet's, and a component that
 * re-sorted would be a second definition of the rule.
 *
 * The header states the count and how many need you. That sentence is written
 * where the counts are known; this composition places it and never composes it.
 *
 * **The empty state is `Board empty state`**, its own row in
 * `components.toml`, and it is not built here. The `empty` slot is where it
 * mounts — Fleet running with no jobs, and Fleet not running, read differently
 * and the difference is that component's to carry.
 */
export type ActiveJobsListProps = {
  /** The surface's name. Lowercase anything countable: "Active jobs". */
  heading?: ReactNode;
  /** The count sentence: "6 jobs. 1 awaiting approval." */
  summary?: ReactNode;
  /**
   * The surface's one primary action, beside the heading and outside the
   * frame. A list row never takes one; the surface may.
   */
  action?: ReactNode;
  /** `Job row (stacked)` rows, in the order Fleet supplied. */
  children?: ReactNode;
  /** Where `Board empty state` mounts when there are no rows. */
  empty?: ReactNode;
};

export function ActiveJobsList({ heading, summary, action, children, empty }: ActiveJobsListProps) {
  const rows = Array.isArray(children) ? children.filter(Boolean) : children;
  const isEmpty = rows === undefined || rows === null || (Array.isArray(rows) && rows.length === 0);

  return (
    <section className="armada-active-jobs">
      {heading || summary || action ? (
        <header className="armada-active-jobs__header">
          <div className="armada-active-jobs__titles">
            {heading ? <h2 className="armada-active-jobs__heading">{heading}</h2> : null}
            {summary ? <p className="armada-active-jobs__summary">{summary}</p> : null}
          </div>
          {action ? <div className="armada-active-jobs__action">{action}</div> : null}
        </header>
      ) : null}
      <div className="armada-active-jobs__frame" role="list">
        {isEmpty ? empty : rows}
      </div>
    </section>
  );
}
