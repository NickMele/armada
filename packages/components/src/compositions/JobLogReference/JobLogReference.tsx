import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

/**
 * Job log reference — where the work is, and where the log is.
 *
 * **Bridge names a job's log on every job from dispatch** — running, failed and
 * finished. The job worth reading a log for is usually the one still running
 * badly, and a failed job has already stopped, so its log is a post-mortem.
 *
 * The treatment is a path in mono that copies on click, what is known about it
 * beside it, and one secondary that opens the file. **Never a count nothing
 * measures** — the drawing shows "142 lines · 0 error" and nothing counts
 * either, so the row names the path and stops. **No viewer, no
 * level filter, no tail**: the Check output pane answers what the suite said,
 * the log answers what Fleet, the drone and Bridge each did, joined on
 * `job_id`. A reader is a later milestone's problem; knowing where the file
 * is, is not.
 *
 * On a failed job the same block carries the branch and the worktree above the
 * log, because the screen's four statements in order are what failed, that the
 * job is over, where the branch is, and where the log is. **All the controls
 * take you to the work; none offers to act on it.**
 *
 * UI copy says "log". Never "Log Envelope" — that name is outside the
 * sanctioned lexicon, and a surface is not the place to settle it.
 */
export type JobLogReferenceRow = {
  /**
   * The glyph. The `file-*` family is reserved to evidence, so the log row
   * takes the plain page outline rather than a new glyph.
   */
  icon?: LucideIcon;
  /** The accessible name for the glyph, since the value beside it is a path. */
  iconLabel?: string;
  /** The path, branch or directory, in mono. Machine-derived. */
  value: string;
  /**
   * What this row puts on the clipboard. A path, a branch and a worktree
   * directory all copy. The worktree was excluded here on the grounds that
   * there is nothing to paste it into; it is the first thing somebody chasing
   * a stopped Job needs in a shell, which is what `cd` is for.
   */
  copyValue?: string;
  /**
   * What is known beside it, in mono and neutral — that a file is not written
   * yet, or a file and line delta on a branch. Nothing below Job level
   * declares a hue for an error count, so it stays neutral.
   */
  meta?: ReactNode;
  /** A rule above this row, where it starts a second group. */
  separated?: boolean;
};

export type JobLogReferenceProps = {
  rows: JobLogReferenceRow[];
  /**
   * The sentence beneath. What the log holds, or that the worktree and branch
   * are left in place — stated, not implied.
   */
  children?: ReactNode;
  /** The controls. Secondary and unfilled: there is no decision here. */
  actions?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** Row glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const ROW_ICON = 12;
const ROW_STROKE = 2;

/**
 * A value split for a middle ellipsis: everything up to the last separator, and
 * the segment after it.
 *
 * **The head truncates and the tail never does.** Every path here ends in a
 * ULID or a filename, which is the only part that tells one Job's worktree from
 * another's — and a trailing ellipsis eats exactly that. A trailing separator
 * belongs to the last segment, so a directory keeps its name.
 */
function halves(value: string): [string, string] {
  const end = value.endsWith("/") ? value.length - 1 : value.length;
  const cut = value.lastIndexOf("/", end - 1);
  return cut <= 0 ? [value, ""] : [value.slice(0, cut + 1), value.slice(cut + 1)];
}

export function JobLogReference({ rows, children, actions, onCopied }: JobLogReferenceProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLSpanElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value),
      );
    },
    [onCopied],
  );

  return (
    <div className="armada-log-ref">
      {rows.map((row, i) => {
        const [head, tail] = halves(row.value);
        return (
          <div className="armada-log-ref__row" key={i} data-separated={row.separated || undefined}>
            <span className="armada-log-ref__mark">
              {row.icon ? <row.icon size={ROW_ICON} strokeWidth={ROW_STROKE} aria-hidden /> : null}
              {row.iconLabel ? <span className="armada-log-ref__sr">{row.iconLabel}</span> : null}
            </span>
            {/* The title carries the whole value however narrow the row gets,
                and so does the clipboard: a copy that truncated with the display
                would be worse than the overflow it was fixing. */}
            <span
              className="armada-log-ref__value"
              title={row.value}
              data-copies={row.copyValue !== undefined || undefined}
              onClick={
                row.copyValue !== undefined ? (e) => copy(e, row.copyValue as string) : undefined
              }
            >
              <span className="armada-log-ref__head">{head}</span>
              {tail === "" ? null : <span className="armada-log-ref__tail">{tail}</span>}
            </span>
            {row.meta ? <span className="armada-log-ref__meta">{row.meta}</span> : null}
          </div>
        );
      })}
      {children || actions ? (
        <div className="armada-log-ref__foot">
          {children ? <p className="armada-log-ref__note">{children}</p> : null}
          {actions ? <div className="armada-log-ref__actions">{actions}</div> : null}
        </div>
      ) : null}
    </div>
  );
}
