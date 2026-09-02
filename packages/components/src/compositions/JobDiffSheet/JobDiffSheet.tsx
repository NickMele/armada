import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import type { DiffFile } from "../UnifiedDiff/UnifiedDiff";

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
 * caller's own diff component, and the rail needs one number per file where the
 * patch holds one row per line.
 *
 * **It is derivable from the patch, and `railOfPatch` is how.** The counts were
 * once described here as the *reading's*, which sent callers to a footprint for
 * them; a footprint is written when a step submits, so mid-step there is none
 * and the rail came back empty beside a fully drawn patch — #310. The one thing
 * a patch genuinely does not carry is `step`.
 */
export type JobDiffFile = {
  /** Repository-relative, exactly as git spells it. */
  path: string;
  /** Lines added and removed, as the patch spells them. */
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

/**
 * The rail for a patch, counted off the same files the patch is drawn from.
 *
 * **One answer feeds the rail, the header and the body.** Hand this the array
 * you hand `UnifiedDiff` and the header cannot contradict what is beside it —
 * which is what #310 was: a rail and a count taken from a footprint while the
 * body was taken from the worktree, so a Job mid-step read `0 files · +0 −0`
 * above its own patch.
 *
 * **A cut patch counts what was drawn**, because that is what the reader is
 * looking at. `UnifiedDiff`'s `cut` is what says the rest exists; a header
 * counting lines nobody can see would be the second source again, one field
 * along.
 *
 * No `step`: a patch does not say which step wrote a file, and nothing is
 * guessed here. See `JobDiffFile.step`.
 */
export function railOfPatch(files: DiffFile[]): JobDiffFile[] {
  return files.map((file) => {
    let added = 0;
    let removed = 0;
    for (const line of file.lines) {
      if (line.kind === "added") added += 1;
      else if (line.kind === "removed") removed += 1;
    }
    return { path: file.path, added, removed };
  });
}

export type JobDiffSheetProps = {
  open: boolean;
  /** The branch the work is on. Mono: git spelled it. */
  branch: ReactNode;
  /**
   * Every file in the patch, in the order the reading found them — or `null`
   * where there is no reading at all.
   *
   * **Absent is not empty, and the header says which.** `[]` is a worktree that
   * opened and holds no change, which is a real answer and reads `0 files · +0
   * −0`. `null` is a Job with no worktree, a read that failed or one still in
   * flight, and there the header says it has no reading rather than summing an
   * empty list — a count of nothing asserted where nothing was read is #310 one
   * state over. Which silence it is belongs to `children` and to `note`.
   */
  files: JobDiffFile[] | null;
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

/**
 * What the header says where there is no reading behind it.
 *
 * **It states the absence and stops.** It does not say *why* — a Job with no
 * worktree, a read that failed and a read still in flight are three different
 * facts, and the sentence for each belongs to the caller, in the body. A header
 * that guessed between them would be inventing the one thing it does not know.
 *
 * Nor does it carry the *uncommitted, in the worktree* clause the counted
 * header ends on: that clause says where the patch came from, and there is no
 * patch.
 */
const NO_READING = "no reading";

/**
 * The counted half of the header: how many files, and what they gained and
 * lost.
 *
 * Its own component so the sum lives with the branch that draws it. Summing
 * before knowing whether there is a reading is what produced `+0 −0` under a
 * `null`.
 */
function Counted({ files }: { files: JobDiffFile[] }) {
  const added = files.reduce((sum, file) => sum + file.added, 0);
  const removed = files.reduce((sum, file) => sum + file.removed, 0);
  return (
    <>
      {` · ${files.length} ${files.length === 1 ? "file" : "files"} · `}
      <span className="armada-diff-sheet__added">{`+${added}`}</span>{" "}
      <span className="armada-diff-sheet__removed">{`−${removed}`}</span>
      {" · uncommitted, in the worktree"}
    </>
  );
}

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
          {files === null ? ` · ${NO_READING}` : <Counted files={files} />}
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
          {(files ?? []).map((file) => (
            <button
              key={file.path}
              type="button"
              className="armada-diff-sheet__file"
              data-on={file.path === selected || undefined}
              // Which file the patch is showing, said rather than only drawn.
              // `data-on` is the stylesheet's hook and reaches nobody who is
              // not looking at it; `StepRow` spells the same fact this way.
              aria-current={file.path === selected ? "true" : undefined}
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
