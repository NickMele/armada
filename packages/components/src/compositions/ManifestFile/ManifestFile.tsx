import type { ReactNode } from "react";
import type { ManifestReading } from "@armada/protocol";
import { worthSaying } from "@armada/protocol";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { ManifestNotice } from "../ManifestNotice/ManifestNotice";

/**
 * The Manifest as a file — Journey 9's *Editing*, the half behind the toggle
 * named by the file's path.
 *
 * # What this is for
 *
 * Correcting a Check used to mean leaving Bridge for an editor, and finding out
 * whether the correction parsed meant watching Fleet's console. This puts the
 * file and the answer to a save in one place: the text, one Save, and what
 * Fleet's next reading of the file came to, drawn above the text it is about.
 *
 * # Save writes the file and stops
 *
 * No staging and no commit. The file is tracked, so the edit shows in the
 * working tree like any other and is committed with everything else; Armada
 * committing on a person's behalf would be a surprise in the one place they are
 * most sensitive to one.
 *
 * # What a save comes to arrives later than the save
 *
 * Fleet's watch settles before it re-reads, so the write answers first and the
 * reading follows by up to the settle window. The receipt says which of the
 * two has happened, so a person is never left reading a refusal that belongs to
 * the file before their save.
 *
 * # A file that moved under the edit
 *
 * A `git checkout` or a pull landing while this is open changes the file under
 * the edit. Fleet refuses that save and hands back what is on disk now, and
 * both are drawn side by side so nobody loses either. Nothing overwrites the
 * incoming change unless a person presses the control that says it will.
 *
 * # No line numbers
 *
 * A line number appears in one place: `ManifestRefused.summary`, where the
 * document never became a document. A gutter here would put numbers beside
 * faults that are attributed by key, and invite a reading of one against the
 * other that nothing on the wire supports.
 */

/** What the last save came to. */
export type ManifestFileSaved = {
  /** When the bytes landed, as a clock reads it. */
  at: ReactNode;
  /**
   * Whether Fleet has read the file since. **Not whether it took** — that is
   * `reading`, which is drawn only once this is true.
   */
  settled: boolean;
};

/** A save Fleet refused because the file changed after the edit read it. */
export type ManifestFileMoved = {
  /**
   * What is on disk now, or `null` where the file is no longer there — a
   * different thing to do about, so a different thing to say.
   */
  onDisk: string | null;
  /** Save the edit over what is on disk now. Absent where there is no file. */
  onSaveOver?: () => void;
  /** Throw the edit away and carry on from what is on disk now. */
  onTakeOnDisk?: () => void;
  /** Read the file again — the one move left where it is not there. */
  onReadAgain?: () => void;
};

export type ManifestFileProps = {
  /** The file as Fleet resolved it. Mono: a path is a fact the system reported. */
  path: string;
  /** The edit, whole. */
  text: string;
  onText?: (text: string) => void;
  /**
   * Whether `text` differs from what was read. **Save is offered only then** —
   * the same bytes written back change nothing Fleet's watch can see, so no
   * reading would ever follow and the receipt would wait forever.
   */
  changed: boolean;
  saving?: boolean;
  onSave?: () => void;
  saved?: ManifestFileSaved;
  /**
   * Fleet's reading, where there is one to draw. `ManifestNotice` asks
   * `worthSaying` itself, so a reading with no news in it draws nothing.
   */
  reading?: ManifestReading;
  moved?: ManifestFileMoved;
  /** Why a save did not reach the disk, in a sentence. */
  failure?: ReactNode;
};

export function ManifestFile({
  path,
  text,
  onText,
  changed,
  saving = false,
  onSave,
  saved,
  reading,
  moved,
  failure,
}: ManifestFileProps) {
  return (
    <div className="armada-manifest-file">
      <div className="armada-manifest-file__head">
        <span className="armada-manifest-file__path">{path}</span>
        <span className="armada-manifest-file__receipt">{receiptOf(saved, reading, changed)}</span>
        {moved !== undefined ? null : (
          <Button
            variant="primary"
            size="sm"
            disabled={!changed || saving || onSave === undefined}
            onClick={onSave}
          >
            {saving ? "Saving" : "Save"}
          </Button>
        )}
      </div>

      {failure === undefined ? null : (
        <Alert tone="escalated" title="Not saved">
          {failure}
        </Alert>
      )}

      {/* The answer to the save sits above the text it is about. Held back
          while the save is settling: the reading on hand is of the file before
          it, and a refusal that has already been corrected must not read as
          the answer to the correction. */}
      {reading === undefined || (saved !== undefined && !saved.settled) ? null : (
        <ManifestNotice reading={reading} />
      )}

      {moved === undefined ? (
        <div className="armada-manifest-file__well">
          <Textarea
            aria-label={path}
            value={text}
            onChange={(event) => onText?.(event.target.value)}
            spellCheck={false}
            wrap="off"
          />
        </div>
      ) : (
        <Moved path={path} text={text} onText={onText} moved={moved} saving={saving} />
      )}
    </div>
  );
}

/**
 * The save that met a different file. Both texts, side by side, and the two
 * ways out named for what they do.
 */
function Moved({
  path,
  text,
  onText,
  moved,
  saving,
}: {
  path: string;
  text: string;
  onText?: (text: string) => void;
  moved: ManifestFileMoved;
  saving: boolean;
}) {
  const gone = moved.onDisk === null;
  return (
    <>
      <Alert
        tone="caution"
        title={gone ? "The file is no longer there" : "The file changed after you opened it"}
      >
        {gone
          ? "Nothing was saved, and Fleet will not put back a file something else removed. Your edit is still here."
          : "Nothing was saved. What is on disk now is beside your edit, and neither has been changed."}
      </Alert>

      <div className="armada-manifest-file__pair">
        <div className="armada-manifest-file__side">
          <span className="armada-manifest-file__side-label">On disk now</span>
          {gone ? (
            <p className="armada-manifest-file__gone">{`${path} is not on disk.`}</p>
          ) : (
            <div className="armada-manifest-file__well">
              <Textarea
                aria-label="On disk now"
                value={moved.onDisk ?? ""}
                readOnly
                spellCheck={false}
                wrap="off"
              />
            </div>
          )}
        </div>
        <div className="armada-manifest-file__side">
          <span className="armada-manifest-file__side-label">Your edit</span>
          <div className="armada-manifest-file__well">
            <Textarea
              aria-label="Your edit"
              value={text}
              onChange={(event) => onText?.(event.target.value)}
              spellCheck={false}
              wrap="off"
            />
          </div>
        </div>
      </div>

      <div className="armada-manifest-file__acts">
        {gone ? (
          moved.onReadAgain === undefined ? null : (
            <Button variant="secondary" onClick={moved.onReadAgain}>
              Read the file again
            </Button>
          )
        ) : (
          <>
            {moved.onTakeOnDisk === undefined ? null : (
              <Button variant="secondary" disabled={saving} onClick={moved.onTakeOnDisk}>
                Discard my edit
              </Button>
            )}
            {moved.onSaveOver === undefined ? null : (
              <Button variant="primary" disabled={saving} onClick={moved.onSaveOver}>
                Save my edit over it
              </Button>
            )}
          </>
        )}
      </div>
    </>
  );
}

/**
 * The line beside the path. **The save and the reading are two facts**, and
 * this says how many of them have arrived.
 */
function receiptOf(
  saved: ManifestFileSaved | undefined,
  reading: ManifestReading | undefined,
  changed: boolean,
): string {
  if (saved === undefined) return changed ? "Not saved." : "";
  const landed = `Saved ${String(saved.at)}.`;
  if (!saved.settled) return `${landed} Fleet reads it once the file settles.`;
  // The reading is the notice below. Where it has nothing to say — the same
  // `worthSaying` the notice asks — this is the one place that says the save
  // was read at all.
  const quiet = reading === undefined || !worthSaying(reading);
  const read = quiet ? `${landed} Fleet read it, and nothing it reads while running changed.` : landed;
  return changed ? `${read} Edited since.` : read;
}
