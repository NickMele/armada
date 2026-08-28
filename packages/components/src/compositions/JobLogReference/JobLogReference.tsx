import { ExternalLink } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { Fragment, useCallback, useState } from "react";

import { Button } from "../../primitives/Button/Button";

/**
 * Job log reference — where the work is, and where the log is.
 *
 * **Bridge names a job's log on every job from dispatch** — running, failed and
 * finished. The job worth reading a log for is usually the one still running
 * badly, and a failed job has already stopped, so its log is a post-mortem.
 *
 * The treatment is a path in mono that copies on click, what is known about it
 * beside it, and — where the surface can open it — one ghost row action that
 * hands it to the OS. **Never a count nothing measures** — the drawing shows
 * "142 lines · 0 error" and nothing counts either, so the row names the path
 * and stops. **No viewer, no level filter, no tail**: the Check output pane
 * answers what the suite said, the log answers what Fleet, the drone and
 * Bridge each did, joined on `job_id`. A reader is a later milestone's
 * problem; knowing where the file is, is not.
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
   * directory all copy — it is how a path reaches a shell, and a stopped job
   * is where somebody needs one there.
   *
   * **Copying stays on the value even where the row also opens**, and the open
   * is a separate control rather than a second reading of the same click. The
   * design contract's rule holds either way: a value that copies does not get
   * a button that copies it.
   */
  copyValue?: string;
  /** How this row is opened, where the surface can open it. Absent on a branch. */
  open?: JobLogReferenceOpen;
  /**
   * What is known beside it, in mono and neutral — that a file is not written
   * yet, or a file and line delta on a branch. Nothing below Job level
   * declares a hue for an error count, so it stays neutral.
   */
  meta?: ReactNode;
  /** A rule above this row, where it starts a second group. */
  separated?: boolean;
};

/**
 * Why a row did not open, or `null` because it did.
 *
 * **A sentence, not a code.** What Bridge can fail at here — a reclaimed
 * worktree, a job whose manifest is gone, an OS with no handler — are four
 * different things to say, and which one it was is known by the surface that
 * asked, not by this block.
 */
export type NotOpened = { because: string } | null;

/** How one row opens. */
export type JobLogReferenceOpen = {
  /** The control's accessible name — the glyph is its whole content. */
  label: string;
  /** Hand it to the OS. Resolves to why it did not open, or `null`. */
  go: () => Promise<NotOpened>;
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

/** `external-link` is a Chrome glyph and Chrome glyphs are 16px. */
const ACTION_ICON = 16;

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
  /**
   * The last open that did not happen, and which row it was.
   *
   * **One at a time, and it is the last press.** Two stale rows arguing on
   * screen is worse than the one the person just clicked, and a message that
   * outlived its press would report a path as missing after it came back.
   */
  const [unopened, setUnopened] = useState<{ row: number; because: string } | null>(null);
  /** The row with an open in flight. A second press does not send a second. */
  const [opening, setOpening] = useState<number | null>(null);

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

  const open = useCallback((row: number, go: () => Promise<NotOpened>) => {
    setOpening(row);
    setUnopened(null);
    void go()
      .then((why) => {
        // Nothing visible happens when a file opens behind the window, so the
        // silent case is the one that worked. The failure is the one that has
        // to speak, and it speaks on the row it was pressed on.
        if (why !== null) setUnopened({ row, because: why.because });
      })
      .finally(() => setOpening(null));
  }, []);

  return (
    <div className="armada-log-ref">
      {rows.map((row, i) => {
        const [head, tail] = halves(row.value);
        const failed = unopened !== null && unopened.row === i ? unopened.because : null;
        const opens = row.open;
        return (
          <Fragment key={i}>
            <div className="armada-log-ref__row" data-separated={row.separated || undefined}>
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
              {opens === undefined ? null : (
                <Button
                  size="sm"
                  variant="ghost"
                  iconOnly
                  aria-label={opens.label}
                  disabled={opening !== null}
                  onClick={() => open(i, opens.go)}
                >
                  <ExternalLink size={ACTION_ICON} strokeWidth={ROW_STROKE} aria-hidden />
                </Button>
              )}
            </div>
            {failed === null ? null : (
              <p className="armada-log-ref__unopened" role="status">
                {failed}
              </p>
            )}
          </Fragment>
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
