// The Manifest file, as the Manifest surface holds it — Journey 9's *Editing*,
// the half behind the toggle named by the file's path.
//
// **The wire half and nothing drawn.** What a read and a save come to, and how
// each folds into what the view is holding. No React and no component type, so
// main can import the shapes it answers with and a node test can call every
// fold — `manifest-file.ts` is the hook that draws from them.
//
// **What the edit started from is carried whole, and is never rebased
// silently.** `SaveManifestFile.read` is what Fleet compares the disk against,
// so a view that moved `read` forward under an unsaved edit would be the one
// thing that let a `git checkout` be overwritten without anybody seeing it.

import type {
  ManifestEdited,
  ManifestFile,
  ManifestReading,
  ManifestSaved,
  ManifestSpend,
  Outcome,
  SaveManifestFile,
} from "@armada/protocol";

/** `GET /manifest/file`, read into the app. */
export type ManifestFileRead = { ok: true; file: ManifestFile } | { ok: false; outcome: Outcome };

/**
 * `POST /manifest/save_file`, read into the app.
 *
 * **Three answers and not two**, Fleet's own reason: a file that moved under
 * the edit is not a failure — nothing broke, and what a person does next is
 * reconcile rather than retry.
 */
export type ManifestSaveAnswer =
  | { state: "saved"; saved: ManifestSaved }
  /** What is on disk now, or `null` where the file is no longer there. */
  | { state: "moved"; onDisk: string | null }
  | { state: "failed"; outcome: Outcome };

/**
 * `POST /manifest/edit`, read into the app.
 *
 * **`refused` is Fleet naming what it would not write** — a result that would
 * not load, a name the file does not hold, a shape the writer does not edit —
 * and `faults` is empty for the last two, whose sentence says it all.
 */
export type ManifestEditAnswer =
  | { state: "edited"; edited: ManifestEdited }
  /** What is on disk now, or `null` where the file is no longer there. */
  | { state: "moved"; onDisk: string | null }
  | { state: "refused"; saying: string; faults: { key: string; fault: string }[] }
  | { state: "failed"; outcome: Outcome };

/** `GET /manifest/spend`, read into the app. */
export type ManifestSpendRead = { ok: true; spend: ManifestSpend } | { ok: false; outcome: Outcome };

/**
 * Which of the surface's views is showing: running, the forms, or the file.
 * The toggle named by the path goes on switching to `file`.
 */
export type ManifestView = "run" | "form" | "file";

/** What the file view is holding. */
export type HeldFile =
  | { state: "none" }
  | { state: "reading" }
  | { state: "failed"; outcome: Outcome }
  | {
      state: "open";
      path: string;
      /** What the edit started from — `SaveManifestFile.read`. */
      read: string;
      /** The edit. */
      text: string;
      saving: boolean;
      /** The last save that landed. */
      saved?: ManifestSaved;
      /** The last save met a different file. */
      moved?: { onDisk: string | null };
      /** The last save did not reach the disk. */
      failure?: Outcome;
    };

/**
 * The file's own name, off the path Fleet resolved — what the toggle is
 * called. **Never the format**: the lexicon bans naming a Manifest as one.
 */
export function fileNameOf(path: string): string {
  const parts = path.split("/").filter((part) => part !== "");
  return parts[parts.length - 1] ?? path;
}

/**
 * Whether Fleet has read the file since this save.
 *
 * **Strictly after.** Both instants are Fleet's own clock at millisecond
 * precision, and the reading follows the write by the settle window — so a
 * reading at or before the write's instant is of the file before it.
 */
export function settledBy(saved: ManifestSaved, reading: ManifestReading | null): boolean {
  if (reading === null) return false;
  const read = Date.parse(reading.at);
  const wrote = Date.parse(saved.at);
  if (Number.isNaN(read) || Number.isNaN(wrote)) return false;
  return read > wrote;
}

/**
 * A read, folded into what is held.
 *
 * - **An edit in progress is never replaced.** The disk moving under it is
 *   Fleet's to refuse at save, where both texts can be shown; replacing the
 *   text here would lose the edit, and moving `read` would defeat the guard.
 * - **A failed read does not blank an open file.** The text on screen is still
 *   the person's.
 * - **A file that came back** fills the side of a moved save that said it was
 *   gone.
 */
export function fileAnswered(held: HeldFile, answer: ManifestFileRead): HeldFile {
  if (!answer.ok) {
    return held.state === "open" ? held : { state: "failed", outcome: answer.outcome };
  }
  const file = answer.file;
  if (held.state !== "open") {
    return { state: "open", path: file.path, read: file.text, text: file.text, saving: false };
  }
  if (held.moved !== undefined) {
    return { ...held, moved: { onDisk: file.text } };
  }
  if (held.saving || held.text !== held.read) return held;
  return { ...held, path: file.path, read: file.text, text: file.text };
}

/**
 * A save's answer, folded into what is held.
 *
 * **`read` moves to what was sent, not to what is on screen.** A person can go
 * on typing while a save is out, and those keystrokes are an edit of the saved
 * file — so they stay unsaved rather than being counted as written.
 */
export function saveAnswered(
  held: HeldFile,
  sent: SaveManifestFile,
  answer: ManifestSaveAnswer,
): HeldFile {
  if (held.state !== "open") return held;
  switch (answer.state) {
    case "saved":
      return {
        state: "open",
        path: answer.saved.path,
        read: sent.text,
        text: held.text,
        saving: false,
        saved: answer.saved,
      };
    case "moved":
      // The receipt of an earlier save is about a file that is no longer
      // there, so it goes with the conflict rather than sitting beside it.
      return { ...held, saving: false, saved: undefined, failure: undefined, moved: { onDisk: answer.onDisk } };
    case "failed":
      return { ...held, saving: false, failure: answer.outcome };
  }
}

/**
 * Throw the edit away and carry on from what is on disk now. Only where there
 * is a disk to carry on from.
 */
export function takenOnDisk(held: HeldFile): HeldFile {
  if (held.state !== "open" || held.moved === undefined || held.moved.onDisk === null) return held;
  const onDisk = held.moved.onDisk;
  return { state: "open", path: held.path, read: onDisk, text: onDisk, saving: false };
}
