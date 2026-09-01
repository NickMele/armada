import { ChevronRight, Copy, ExternalLink } from "lucide-react";
import type { ReactNode } from "react";
import { useCallback } from "react";

/**
 * One row of *Where things are* — a label, the machine value beside it, and
 * the one thing the row does.
 *
 * **The label column is why this is a row and not a list of paths.** The build
 * drew glyphs where the labels go and left a reader to work out from the shape
 * of a string whether it was a worktree, a branch or a log. A 74px column
 * naming each one costs nothing and answers it before the value is read.
 *
 * **A path opens where it lives; an identifier copies.** Those are the two
 * acts, and the trailing glyph is which one — `external-link` against `copy`.
 * A third exists for a value that leads somewhere inside Bridge, and it is
 * `chevron-right`, the same mark a row that navigates carries anywhere else.
 *
 * **These rows are the milestone's escape hatch, not its subject.** The screen
 * above them exists so nobody needs a worktree path; they are here for when
 * somebody wants one anyway, which is why they are quiet, mono and last.
 */

/**
 * What the row does when it is pressed.
 *
 * `open` reveals the file or directory where it lives, outside Bridge.
 * `copy` puts the value on the clipboard, for a value that names something
 * rather than locating it — a branch, a Drone id.
 * `into` goes to another surface in Bridge, which is what the Workflow row
 * does: the workflow as it stood when the Job was dispatched.
 */
export type WhereRowAct = "open" | "copy" | "into";

export type WhereRowProps = {
  /** `Worktree`, `Branch`, `Manifest`, `Job log`, `Drone`. Sans, sentence case. */
  label: ReactNode;
  /**
   * The value. Mono, because everything in this region is machine-derived, and
   * clipped from the right — a worktree path and a branch both read from their
   * start, so this is the one place in job detail where an ordinary clip is
   * the correct one.
   */
  value: ReactNode;
  /**
   * What the value is, where the value does not say — `as it was at 14:20`.
   * Sans and `--fg-subtle`, so it reads as a note about the value.
   */
  note?: ReactNode;
  act: WhereRowAct;
  /**
   * What `copy` writes, where `value` has been shortened for the row. Defaults
   * to `value` when that is a string; a row whose value is not a string and
   * copies must supply this, or there is nothing to write.
   */
  copyValue?: string;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (value: string) => void;
  /** What `open` and `into` do. Absent draws the row as a label. */
  onAct?: () => void;
  /**
   * What pressing the row does, in words. The row is a control whose visible
   * label is a path, and the glyph beside it is 12px — neither is an
   * accessible name on its own.
   */
  actLabel?: string;
};

/** The trailing glyph is 12px at strokeWidth 2, as every mark on this screen is. */
const GLYPH = 12;
const STROKE = 2;

/** What each act says, where the caller writes nothing better. */
const SAYS: Record<WhereRowAct, string> = {
  open: "Open where this lives",
  copy: "Copy this value",
  into: "Open this",
};

const MARK = { open: ExternalLink, copy: Copy, into: ChevronRight };

export function WhereRow({
  label,
  value,
  note,
  act,
  copyValue,
  onCopied,
  onAct,
  actLabel,
}: WhereRowProps) {
  const writes = copyValue ?? (typeof value === "string" ? value : undefined);
  const press = useCallback(() => {
    if (act !== "copy") {
      onAct?.();
      return;
    }
    if (writes === undefined) return;
    void navigator.clipboard.writeText(writes).then(
      () => onCopied?.(writes),
      // A failed clipboard write is otherwise indistinguishable from a dead
      // control, and the row has already said it copied.
      () => onCopied?.(writes),
    );
  }, [act, onAct, onCopied, writes]);

  const actable = act === "copy" ? writes !== undefined : onAct !== undefined;
  const Mark = MARK[act];
  const says = actLabel ?? SAYS[act];

  const body = (
    <>
      <span className="armada-wrow__k">{label}</span>
      <span className="armada-wrow__v">
        {value}
        {note === undefined ? null : <span className="armada-wrow__note">{note}</span>}
      </span>
      <Mark size={GLYPH} strokeWidth={STROKE} className="armada-wrow__mark" aria-hidden />
    </>
  );

  if (!actable) {
    return (
      <div className="armada-wrow" data-act={act}>
        {body}
      </div>
    );
  }

  return (
    <button type="button" className="armada-wrow" data-act={act} onClick={press} title={says}>
      {body}
      <span className="armada-wrow__sr">{says}</span>
    </button>
  );
}
