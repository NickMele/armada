import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

/**
 * Job outcome — what a finished Job produced, one row per part of it.
 *
 * **The region a finished Job is opened for.** A Job that stopped is read once,
 * to decide whether to take the work, and the parts of "produced" are the whole
 * of that decision: the branch, the commit, the pull request, the files that
 * changed, the evidence that was submitted.
 *
 * **A part nothing serves keeps its row and says so.** Four of the five are not
 * on the wire yet, and dropping them would draw a finished-looking outcome that
 * is a fifth of one. Each names the operation that would have to serve it, so
 * the hole is a finding rather than a silence — the same reason the screens
 * render a named absence instead of closing up.
 *
 * **Not `Absent` per row.** That treatment is a dashed `--status-completed-failed`
 * frame and it is right for a whole region with nothing in it; four of them
 * inside one region would make a Job that completed read as a Job that broke.
 * The row stays, the value is replaced by the sentence, and the weight drops to
 * `--fg-subtle` — which is what says "not here" without saying "wrong".
 *
 * **Marks run at 12px**, like every other row mark, and the registry sizes
 * `git-branch`, `git-commit-horizontal` and `git-pull-request` at 12 as well. A
 * part the registry has no glyph for keeps its mark column and renders it
 * empty, rather than borrowing a silhouette that means something else.
 */
export type JobOutcomePart = {
  /** What this part is: `Branch`, `Commit`, `Pull request`. Sentence case. */
  name: ReactNode;
  /** The glyph, where the registry has one for this part. */
  icon?: LucideIcon;
  /** The accessible name for the glyph, since the value beside it is machine text. */
  iconLabel?: string;
  /**
   * The value, where it is served. Mono, and it copies on click — a branch
   * name, a commit, a pull request reference are all things that get pasted
   * into a shell.
   */
  value?: string;
  /** What is known beside the value. Mono and neutral, never a count nothing measures. */
  meta?: ReactNode;
  /** Why there is no value. Said in words, never left as a blank. */
  absent?: ReactNode;
  /** A control that opens it. Secondary and unfilled: there is no decision here. */
  action?: ReactNode;
};

export type JobOutcomeProps = {
  /** Every part of what was produced, in the order a reader asks for them. */
  parts: JobOutcomePart[];
  /**
   * What the person still owes. **Armada pushes and opens a review, and does
   * not merge**, so this region says what is left rather than implying the work
   * landed — and rather than denying the two things it does.
   */
  note?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** Row marks are 12px at strokeWidth 2, like every mark below Job level. */
const ROW_ICON = 12;
const ROW_STROKE = 2;

export function JobOutcome({ parts, note, onCopied }: JobOutcomeProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied],
  );

  return (
    <div className="armada-outcome">
      <ol className="armada-outcome__parts">
        {parts.map((part, i) => (
          <li className="armada-outcome__part" key={i}>
            <span className="armada-outcome__mark">
              {part.icon ? (
                <part.icon size={ROW_ICON} strokeWidth={ROW_STROKE} aria-hidden />
              ) : null}
              {part.iconLabel ? (
                <span className="armada-outcome__sr">{part.iconLabel}</span>
              ) : null}
            </span>
            <span className="armada-outcome__name">{part.name}</span>
            {part.value === undefined ? (
              <span className="armada-outcome__absent">{part.absent}</span>
            ) : (
              /* The title carries the whole value however narrow the row gets,
                 and so does the clipboard: a copy that truncated with the
                 display would be worse than the overflow it was fixing. */
              <span
                className="armada-outcome__value"
                title={part.value}
                onClick={(event) => copy(event, part.value as string)}
              >
                {part.value}
              </span>
            )}
            <span className="armada-outcome__meta">{part.meta}</span>
            <span className="armada-outcome__action">{part.action}</span>
          </li>
        ))}
      </ol>
      {note === undefined ? null : <p className="armada-outcome__note">{note}</p>}
    </div>
  );
}
