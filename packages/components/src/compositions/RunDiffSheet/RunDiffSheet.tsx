import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { railOfPatch } from "../JobDiffSheet/JobDiffSheet";
import { UnifiedDiff, type DiffFile } from "../UnifiedDiff/UnifiedDiff";

/**
 * What one run in the main checkout changed — Journey 9's **Open the diff**,
 * on the Manifest surface.
 *
 * # Why this is not `JobDiffSheet`
 *
 * **There is no Job.** That sheet is titled *Job diff*, names a branch, and
 * says its counts are *since the branch was cut* — three claims, each false
 * here. A run in the main checkout is read against the snapshot Fleet took
 * just before it, and nothing else, because this tree holds a person's own
 * uncommitted work and a patch against `HEAD` would show all of it as the
 * run's. The wire says so in `against`, and the header says it in words.
 *
 * **No file rail.** `JobDiffSheet`'s rail exists to carry the one step-scoped
 * fact a Job-wide patch has — which step wrote each file. A run has no steps,
 * and the page this opens over already lists the run's files. The counts a
 * rail would have carried are summed into the header instead.
 *
 * **The patch is `UnifiedDiff`, not a second renderer.** Two diffs drawn by
 * two components is how two diffs in one app come to look different.
 *
 * # Nothing here is hued
 *
 * No run from this surface is a verdict. The only colour is the diff's own
 * `+` and `−`, which restate the marker rather than judge anything. An undone
 * run is a band of plain words; a snapshot that is gone is a sentence.
 *
 * **A read, not an act.** Opening it changes nothing, and it offers nothing to
 * press but Close.
 */
export type RunDiffReading =
  /** Fleet has been asked and has not answered. */
  | { state: "reading" }
  | {
      state: "read";
      /** Every file in the patch, in the order git wrote them. `[]` draws `emptyNote`. */
      files: DiffFile[];
      /** Where the patch was longer than the bound — `UnifiedDiff`'s own `cut`. */
      cut?: ReactNode;
      /** What the region says with nothing to draw. The caller knows which silence. */
      emptyNote: string;
    }
  /**
   * **The snapshot is gone, and nothing stands in for it.** Retention swept the
   * run's record, or none was taken. Never a fallback to `HEAD`.
   */
  | { state: "gone"; why: ReactNode }
  /** Fleet refused, or was not there to ask. */
  | { state: "failed"; saying: ReactNode };

export type RunDiffSheetProps = {
  open: boolean;
  /** The Check or Command that ran, as the Manifest names it. */
  name: ReactNode;
  /** When it started, as a clock — the page's own run rows carry no date. */
  ranAt: ReactNode;
  reading: RunDiffReading;
  /**
   * The run was undone, as a sentence — `Undone at 14:21:03.` **The diff is
   * still drawn**: Undo restores from the snapshot and keeps it, and a person
   * may want to read what a run did before deciding to run it again.
   */
  undone?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

/** What every reading of this sheet is measured against. The wire's `run_snapshot`. */
const AGAINST = "against the checkout just before the run";

/** Under the patch, saying what is not in it and why. */
const NOT_HEAD =
  "Read against the snapshot Fleet took just before this run, never against HEAD — so " +
  "uncommitted work that was already in the checkout is not shown as the run's, and " +
  "nothing written since the run is in it either.";

export function RunDiffSheet({
  open,
  name,
  ranAt,
  reading,
  undone,
  onCopied,
  floor = false,
  onClose,
}: RunDiffSheetProps) {
  return (
    <Sheet
      open={open}
      contained
      size="widest"
      floor={floor}
      title="What this run changed"
      subtitle={
        <>
          {name}
          {" · "}
          {ranAt}
          <Measured reading={reading} />
          {undone === undefined ? null : " · undone"}
        </>
      }
      bands={
        undone === undefined ? undefined : (
          <p className="armada-run-diff-sheet__undone" role="note">
            {undone} The checkout no longer holds these changes. This is what the run did, read
            from the snapshot Undo restored.
          </p>
        )
      }
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      onClose={onClose}
    >
      <div className="armada-run-diff-sheet">
        <Body reading={reading} onCopied={onCopied} />
      </div>
    </Sheet>
  );
}

/**
 * The header's second half. **Counted only off a reading**: a sum drawn over
 * `reading` or `gone` would be `0 files · +0 −0` asserted where nothing was
 * read, which is #310 on a different sheet.
 */
function Measured({ reading }: { reading: RunDiffReading }) {
  if (reading.state === "reading") return <>{" · reading"}</>;
  if (reading.state === "failed") return <>{" · no reading"}</>;
  if (reading.state === "gone") return <>{" · snapshot gone"}</>;
  const rail = railOfPatch(reading.files);
  const added = rail.reduce((sum, file) => sum + file.added, 0);
  const removed = rail.reduce((sum, file) => sum + file.removed, 0);
  return (
    <>
      {` · ${rail.length} ${rail.length === 1 ? "file" : "files"} · `}
      <span className="armada-run-diff-sheet__added">{`+${added}`}</span>{" "}
      <span className="armada-run-diff-sheet__removed">{`−${removed}`}</span>
      {` · ${AGAINST}`}
    </>
  );
}

function Body({
  reading,
  onCopied,
}: {
  reading: RunDiffReading;
  onCopied?: (value: string) => void;
}) {
  if (reading.state === "reading") {
    return <p className="armada-run-diff-sheet__said">Reading this run's diff.</p>;
  }
  if (reading.state === "failed") {
    return <p className="armada-run-diff-sheet__said">{reading.saying}</p>;
  }
  if (reading.state === "gone") {
    return (
      <div className="armada-run-diff-sheet__said">
        <p>
          There is no diff to read: {reading.why}. A run's diff is read against the snapshot it
          took, and without that snapshot nothing can say what the run itself changed.
        </p>
        <p>
          Nothing is drawn in its place. A diff against HEAD would show every uncommitted edit in
          the checkout as this run's.
        </p>
      </div>
    );
  }
  return (
    <UnifiedDiff
      files={reading.files}
      emptyNote={reading.emptyNote}
      {...(reading.cut === undefined ? {} : { cut: reading.cut })}
      note={NOT_HEAD}
      {...(onCopied === undefined ? {} : { onCopied })}
    />
  );
}
