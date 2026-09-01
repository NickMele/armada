import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * The Job's patch, on the layer that can hold it — Journey 4, frame `4j`.
 *
 * **The patch is the Job's, not the step's.** Fleet commits once at the end, so
 * there is no per-step patch to draw: the header names the branch, and the only
 * step-scoped fact a diff has is which step wrote each file, which is what the
 * rail carries. What a step chooses is *where the diff opens* — opening from a
 * step's `Produced` scrolls the reading to that step's first file.
 *
 * **A sheet, because a patch in a panel is a patch in a 602px column.** That
 * was the reading this replaced, and a diff read in a column that narrow is a
 * decision taken on a line that wrapped.
 */
/**
 * One row of the rail. **Not a `DiffFile`** — the patch is drawn by the
 * caller's own diff component, and what the rail needs is the reading's counts
 * and the step that produced the file, which a patch does not carry.
 */
export type JobDiffFile = {
  /** Repository-relative, exactly as git spells it. */
  path: string;
  /** Lines added and removed, as the reading counted them. */
  added: number;
  removed: number;
  /**
   * The step that wrote it. The one step-scoped fact a Job's patch carries —
   * and **absent where nothing says which step that was**. Fleet's footprint
   * carries `planned_by`, which is the step that *promised* a path, and that is
   * a different claim: a file no step declared would then read as a file no
   * step wrote. Absent draws the counts alone rather than a guess. Reported.
   */
  step?: ReactNode;
};

export type JobDiffSheetProps = {
  open: boolean;
  /** The branch the work is on. Mono: git spelled it. */
  branch: ReactNode;
  /** Every file in the patch, in the order the reading found them. */
  files: JobDiffFile[];
  /** Which file the rail has selected, by path. */
  selected?: string;
  onSelect?: (path: string) => void;
  /**
   * The step the diff was opened at, for the rail's heading. Absent where the
   * reading is not scoped to one — which needs the attribution above.
   */
  openedAt?: ReactNode;
  /**
   * What the rail says about itself, under the files. The default is why every
   * file in one sheet belongs to one patch; a caller whose reading cannot
   * attribute a file to a step says that instead.
   */
  note?: ReactNode;
  /**
   * The patch. **A slot, so the sheet does not choose the diff component** —
   * Bridge already draws one against its own read, and a sheet that imported
   * `UnifiedDiff` would be drawing the patch twice from two answers.
   */
  children: ReactNode;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

/** Why every file in one sheet belongs to one patch, said once over the rail. */
const ONE_PATCH =
  "Fleet commits once at the end, so the patch is the Job's. Each file names the step that " +
  "wrote it.";

export function JobDiffSheet({
  open,
  branch,
  files,
  selected,
  onSelect,
  openedAt,
  note = ONE_PATCH,
  children,
  floor = false,
  onClose,
}: JobDiffSheetProps) {
  const added = files.reduce((sum, file) => sum + file.added, 0);
  const removed = files.reduce((sum, file) => sum + file.removed, 0);

  return (
    <Sheet
      open={open}
      contained
      size="widest"
      floor={floor}
      title="Job diff"
      subtitle={
        <>
          <span className="armada-diff-sheet__mono">{branch}</span>
          {` · ${files.length} ${files.length === 1 ? "file" : "files"} · `}
          <span className="armada-diff-sheet__added">{`+${added}`}</span>{" "}
          <span className="armada-diff-sheet__removed">{`−${removed}`}</span>
          {" · uncommitted, in the worktree"}
        </>
      }
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      onClose={onClose}
    >
      <div className="armada-diff-sheet">
        <div className="armada-diff-sheet__rail">
          {openedAt === undefined ? null : (
            <span className="armada-diff-sheet__rail-head">
              {"Opened at "}
              {openedAt}
              {"'s files"}
            </span>
          )}
          {files.map((file) => (
            <button
              key={file.path}
              type="button"
              className="armada-diff-sheet__file"
              data-on={file.path === selected || undefined}
              onClick={() => onSelect?.(file.path)}
            >
              {/* Truncated from the front: the end of a path is what names the
                  file, and the front of it is the tree it sits in. */}
              <span className="armada-diff-sheet__path">
                <bdi>{file.path}</bdi>
              </span>
              <span className="armada-diff-sheet__counts">
                <span className="armada-diff-sheet__added">{`+${file.added}`}</span>
                {file.removed === 0 ? null : (
                  <>
                    {" "}
                    <span className="armada-diff-sheet__removed">{`−${file.removed}`}</span>
                  </>
                )}
                {file.step === undefined ? null : (
                  <>
                    {" · "}
                    {file.step}
                  </>
                )}
              </span>
            </button>
          ))}
          <span className="armada-diff-sheet__rail-note">{note}</span>
        </div>
        <div className="armada-diff-sheet__patch">{children}</div>
      </div>
    </Sheet>
  );
}
