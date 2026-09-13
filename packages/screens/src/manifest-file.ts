// The Manifest surface's file view — Journey 9's *Editing*, held and drawn.
//
// `editing.ts` is what the wire answers and how each answer folds; this is the
// state that holds it, and the props `ManifestFile` takes.
//
// **Held by the app and not by the screen.** The screen unmounts whenever the
// rail moves, and an unsaved edit it owned would go with it — the one thing a
// file view must never do to somebody part way through a correction.

import { useEffect, useRef, useState } from "react";
import type { ManifestFileProps } from "@armada/components";
import type { ManifestReading, Outcome, SaveManifestFile } from "@armada/protocol";

import { clockOf } from "./checkout-runs";
import { said } from "./copy";
import {
  fileAnswered,
  fileNameOf,
  saveAnswered,
  settledBy,
  takenOnDisk,
  type HeldFile,
  type ManifestFileRead,
  type ManifestSaveAnswer,
  type ManifestView,
} from "./editing";

/**
 * The file's name before Fleet has said where it is — the first frame after
 * the surface opens. `crates/armada/src/setup.rs` declares it, and `RunPage`'s
 * *last edited* line already spells it the same way.
 */
const MANIFEST_NAME = "armada.yml";

/** What the file view asks of the host. */
export type ManifestEditingSlice = {
  /** Whether the Manifest surface is on screen. Nothing is read while it is not. */
  showing: boolean;
  /** Fleet's last reading of the file — `get_manifest_reading` or `manifest.reread`. */
  reading: ManifestReading | null;
  onReadFile: () => Promise<ManifestFileRead>;
  onSaveFile: (body: SaveManifestFile) => Promise<ManifestSaveAnswer>;
  /** Which view opens first. The app takes the default; a story opens the file. */
  initialView?: ManifestView;
};

/** The file view, as the screen draws it. */
export type ManifestEditing = {
  view: ManifestView;
  onView: (view: ManifestView) => void;
  /** What the toggle is called: the file's own name, never its format. */
  named: string;
  file:
    | { state: "reading" }
    | { state: "failed"; saying: string }
    | { state: "open"; props: ManifestFileProps };
};

/**
 * What a read or a save that did not land says. **Fleet's own sentence where
 * Fleet refused** — it names the path and the OS's reason — and `said`'s where
 * the request never had an answer.
 */
export function unlandedSaying(outcome: Outcome): string {
  if (outcome.ok) return "";
  return outcome.why === "refused" ? outcome.error.message : said(outcome);
}

/**
 * The file view's whole state machine: which view is showing, the file and the
 * edit, and one save at a time.
 */
export function useManifestEditing({
  showing,
  reading,
  onReadFile,
  onSaveFile,
  initialView = "run",
}: ManifestEditingSlice): ManifestEditing {
  const [view, setView] = useState<ManifestView>(initialView);
  const [held, setHeld] = useState<HeldFile>({ state: "none" });
  // What a press reads. **A save takes the text as it is at the press**, and
  // a closure over the render before it would send the edit minus its last
  // keystroke.
  const current = useRef(held);
  current.current = held;
  // **One save at a time, decided before the render that would disable Save.**
  // A second press inside that frame would send the same `read` again after
  // the first had moved the disk, and Fleet would refuse it as a file that
  // moved — a conflict with nobody but the person's own double-click.
  const out = useRef(false);

  function readFile(): void {
    setHeld((prev) => (prev.state === "open" ? prev : { state: "reading" }));
    void onReadFile().then((answer) => setHeld((prev) => fileAnswered(prev, answer)));
  }

  // Read on opening the surface, and again on every reading Fleet publishes:
  // a file changed by an editor, a checkout or a pull is shown as it is now —
  // **unless it is being edited**, which `fileAnswered` keeps.
  useEffect(() => {
    if (showing) readFile();
  }, [showing, reading?.at]);

  function save(over: boolean): void {
    const at = current.current;
    if (at.state !== "open" || out.current) return;
    // Saving over a moved file compares against what is on disk now, which is
    // the one thing the person pressing has just been shown.
    const read = over ? at.moved?.onDisk : at.read;
    if (read === undefined || read === null) return;
    const sent: SaveManifestFile = { read, text: at.text };
    out.current = true;
    setHeld((prev) => (prev.state === "open" ? { ...prev, saving: true, failure: undefined } : prev));
    void onSaveFile(sent).then((answer) => {
      out.current = false;
      setHeld((prev) => saveAnswered(prev, sent, answer));
    });
  }

  const path = held.state === "open" ? held.path : (reading?.path ?? MANIFEST_NAME);
  return {
    view,
    onView: setView,
    named: fileNameOf(path),
    file:
      held.state === "open"
        ? { state: "open", props: propsOf(held, reading) }
        : held.state === "failed"
          ? { state: "failed", saying: unlandedSaying(held.outcome) }
          : { state: "reading" },
  };

  function propsOf(open: Extract<HeldFile, { state: "open" }>, now: ManifestReading | null): ManifestFileProps {
    const moved = open.moved;
    return {
      path: open.path,
      text: open.text,
      onText: (text) => setHeld((prev) => (prev.state === "open" ? { ...prev, text } : prev)),
      changed: open.text !== open.read,
      saving: open.saving,
      onSave: () => save(false),
      ...(open.saved === undefined
        ? {}
        : { saved: { at: clockOf(open.saved.at), settled: settledBy(open.saved, now) } }),
      ...(now === null ? {} : { reading: now }),
      ...(moved === undefined
        ? {}
        : {
            moved: {
              onDisk: moved.onDisk,
              ...(moved.onDisk === null
                ? { onReadAgain: readFile }
                : {
                    onSaveOver: () => save(true),
                    onTakeOnDisk: () => setHeld((prev) => takenOnDisk(prev)),
                  }),
            },
          }),
      ...(open.failure === undefined ? {} : { failure: unlandedSaying(open.failure) }),
    };
  }
}
