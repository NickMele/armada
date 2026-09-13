// The Manifest surface's forms, held — Journey 9's *Editing*. `form-edits.ts`
// is the arithmetic; this holds the draft, one save at a time, and what past
// Jobs cost.
//
// **Held by the app and not by the screen**, for `manifest-file.ts`'s reason:
// the screen unmounts whenever the rail moves, and a draft it owned would go
// with it.

import { useEffect, useRef, useState } from "react";
import type { ManifestFormDraft, ManifestFormProps } from "@armada/components";
import type { EditManifest, ManifestDeclared, ManifestReading, ManifestSpend } from "@armada/protocol";

import { clockOf } from "./duration";
import type { ManifestEditAnswer, ManifestFileRead, ManifestSpendRead } from "./editing";
import { budgetWarningsOf, draftOf, editsOf, problemsOf } from "./form-edits";
import { unlandedSaying } from "./manifest-file";

/** What the forms ask of the host. */
export type ManifestFormSlice = {
  /** Whether the Manifest surface is on screen. Nothing is read while it is not. */
  showing: boolean;
  /** Fleet's last reading of the file, whose instant is what re-reads it. */
  reading: ManifestReading | null;
  onReadFile: () => Promise<ManifestFileRead>;
  onEditManifest: (body: EditManifest) => Promise<ManifestEditAnswer>;
  onReadSpend: () => Promise<ManifestSpendRead>;
};

/** The forms, as the screen draws them. */
export type ManifestForming =
  | { state: "reading" }
  | { state: "failed"; saying: string }
  /** The file does not load, so there is no form to draw; the file view corrects it. */
  | { state: "unloadable"; path: string }
  | { state: "open"; props: ManifestFormProps };

type Refused = NonNullable<ManifestFormProps["refused"]>;

type Held =
  | { state: "none" }
  | { state: "reading" }
  | { state: "failed"; saying: string }
  | { state: "unloadable"; path: string }
  | {
      state: "open";
      path: string;
      /** The text the draft was drawn from — `EditManifest.read`. */
      read: string;
      declared: ManifestDeclared;
      draft: ManifestFormDraft;
      saving: boolean;
      /** When the last save landed, as a clock reads it. */
      saved?: string;
      refused?: Refused;
      moved?: { onDisk: string | null };
    };

/**
 * A read, folded in. **A draft with changes in it is never replaced** — the
 * disk moving under it is Fleet's to refuse at Save, where it can be said.
 */
export function formAnswered(held: Held, answer: ManifestFileRead): Held {
  if (!answer.ok) {
    return held.state === "open" ? held : { state: "failed", saying: unlandedSaying(answer.outcome) };
  }
  if (held.state === "open") {
    const touched = editsOf(held.declared, held.draft).length > 0;
    if (held.saving || held.moved !== undefined || touched) return held;
  }
  const { path, text, declared } = answer.file;
  if (declared === undefined) return { state: "unloadable", path };
  const saved = held.state === "open" ? held.saved : undefined;
  return {
    state: "open",
    path,
    read: text,
    declared,
    draft: draftOf(declared),
    saving: false,
    ...(saved === undefined ? {} : { saved }),
  };
}

/** An edit's answer, folded in. **What was written is what the form redraws from.** */
export function editAnswered(held: Held, answer: ManifestEditAnswer): Held {
  if (held.state !== "open") return held;
  switch (answer.state) {
    case "edited": {
      const { edited } = answer;
      return {
        state: "open",
        path: edited.path,
        read: edited.text,
        declared: edited.declared,
        draft: draftOf(edited.declared),
        saving: false,
        saved: clockOf(edited.at),
      };
    }
    case "moved":
      return { ...held, saving: false, saved: undefined, refused: undefined, moved: { onDisk: answer.onDisk } };
    case "refused":
      return { ...held, saving: false, refused: { saying: answer.saying, faults: answer.faults } };
    case "failed":
      return { ...held, saving: false, refused: { saying: unlandedSaying(answer.outcome), faults: [] } };
  }
}

export function useManifestForm({
  showing,
  reading,
  onReadFile,
  onEditManifest,
  onReadSpend,
}: ManifestFormSlice): ManifestForming {
  const [held, setHeld] = useState<Held>({ state: "none" });
  const [spend, setSpend] = useState<ManifestSpend | null>(null);
  // What a press reads, and one save at a time — `manifest-file.ts`'s two refs.
  const current = useRef(held);
  current.current = held;
  const out = useRef(false);

  function read(): void {
    setHeld((prev) => (prev.state === "open" ? prev : { state: "reading" }));
    void onReadFile().then((answer) => setHeld((prev) => formAnswered(prev, answer)));
  }

  function readAgain(): void {
    setHeld({ state: "reading" });
    void onReadFile().then((answer) => setHeld((prev) => formAnswered(prev, answer)));
  }

  useEffect(() => {
    if (showing) read();
  }, [showing, reading?.at]);

  useEffect(() => {
    if (!showing) return;
    void onReadSpend().then((answer) => {
      if (answer.ok) setSpend(answer.spend);
    });
  }, [showing]);

  function save(over: boolean): void {
    const at = current.current;
    if (at.state !== "open" || out.current) return;
    // Over a moved file, the edits go onto what is on disk now — by key, so
    // what they touch is still only what the form changed.
    const against = over ? at.moved?.onDisk : at.read;
    if (against === undefined || against === null) return;
    const sent: EditManifest = { read: against, edits: editsOf(at.declared, at.draft) };
    if (sent.edits.length === 0) return;
    out.current = true;
    setHeld((prev) =>
      prev.state === "open" ? { ...prev, saving: true, refused: undefined, moved: undefined } : prev,
    );
    void onEditManifest(sent).then((answer) => {
      out.current = false;
      setHeld((prev) => editAnswered(prev, answer));
    });
  }

  if (held.state === "failed") return { state: "failed", saying: held.saying };
  if (held.state === "unloadable") return { state: "unloadable", path: held.path };
  if (held.state !== "open") return { state: "reading" };

  const { draft, declared, saved, refused, moved } = held;
  const changed = editsOf(declared, draft).length > 0;
  const receipt = changed
    ? saved === undefined
      ? "Not saved."
      : `Saved ${saved}. Changed since.`
    : saved === undefined
      ? undefined
      : `Saved ${saved}.`;
  return {
    state: "open",
    props: {
      path: held.path,
      draft,
      onDraft: (next) => setHeld((prev) => (prev.state === "open" ? { ...prev, draft: next } : prev)),
      autoMergeWords: declared.auto_merge.offered,
      reviewGateWords: declared.review_gate.offered,
      problems: problemsOf(draft),
      budgetWarnings: budgetWarningsOf(draft, spend),
      changed,
      saving: held.saving,
      onSave: () => save(false),
      onDiscard: () =>
        setHeld((prev) =>
          prev.state === "open" ? { ...prev, draft: draftOf(prev.declared), refused: undefined } : prev,
        ),
      ...(receipt === undefined ? {} : { receipt }),
      ...(refused === undefined ? {} : { refused }),
      ...(moved === undefined
        ? {}
        : {
            moved: {
              gone: moved.onDisk === null,
              onReadAgain: readAgain,
              ...(moved.onDisk === null ? {} : { onSaveOver: () => save(true) }),
            },
          }),
    },
  };
}
