import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";

/**
 * Studio name — a Studio's name where it is drawn, and the way a person gives
 * it one. `docs/concepts/studio.md`, Helm on a Studio; `#1364`.
 *
 * **Editable where it is drawn**, on the list row and on the open Studio. The
 * owner clicked an untitled Studio's name to name it and nothing happened; a
 * rename behind a menu would answer the report and not the gesture.
 *
 * **The name and the way in are two controls on a row and one on a heading.**
 * A row's name is what opens the Studio, so the rename sits beside it rather
 * than taking that press; a heading opens nothing and is itself the control.
 * What decides is `onOpen`.
 */

export type StudioNameProps = {
  /** The name Fleet holds, or `null` on a Studio nobody has named. */
  name: string | null;
  /** What an untitled Studio is called. The screen's word, not this one's. */
  untitled: string;
  /**
   * Nothing is renamed without this: a Studio reopened read-only, and a window
   * with no connection, both draw the name and no way to change it.
   */
  editable?: boolean;
  /** Drawn as the page's own heading rather than as a row's text. */
  heading?: boolean;
  /** Where the name is also the way into the Studio — the list row's press. */
  onOpen?: () => void;
  /** The name a person settled on. Never called with blank. */
  onRename: (name: string) => void;
  /** Out to Fleet: the field waits rather than taking a second press. */
  saving?: boolean;
  /** Why the last rename did not land, or absent. */
  refused?: string;
};

/** A name with nothing in it is no name, and Fleet refuses one. */
const settled = (draft: string): string | null => {
  const name = draft.trim();
  return name === "" ? null : name;
};

export function StudioName({
  name,
  untitled,
  editable = false,
  heading = false,
  onOpen,
  onRename,
  saving = false,
  refused,
}: StudioNameProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const field = useRef<HTMLInputElement>(null);
  const shown = name ?? untitled;
  const naming = draft !== null;

  // The cursor goes where the name was, with what is there selected: the first
  // thing a person does to "Untitled Studio" is replace all of it.
  useEffect(() => {
    if (naming) field.current?.select();
  }, [naming]);

  // The name arriving is the write having happened — Fleet answers every write
  // with the Studio whole — so nothing here holds a second copy of it open.
  useEffect(() => {
    if (!saving && refused === undefined) setDraft(null);
  }, [name]);

  function save(): void {
    const name = settled(draft ?? "");
    if (name !== null) onRename(name);
  }

  function keyed(event: KeyboardEvent<HTMLInputElement>): void {
    if (event.key === "Enter") {
      event.preventDefault();
      save();
      return;
    }
    // Esc abandons the rename rather than closing the surface behind it.
    if (event.key === "Escape") {
      event.stopPropagation();
      setDraft(null);
    }
  }

  if (naming) {
    return (
      <div className="armada-studio-name armada-studio-name--naming">
        <Input
          ref={field}
          // Labelled but not captioned: the name it is replacing is right
          // there, and a caption over one field in a heading row is noise.
          aria-label="Studio name"
          value={draft}
          disabled={saving}
          invalid={refused !== undefined}
          message={refused}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={keyed}
        />
        <div className="armada-studio-name__acts">
          <Button size="sm" variant="primary" pending={saving} disabled={settled(draft) === null} onClick={save}>
            Save
          </Button>
          <Button size="sm" variant="ghost" disabled={saving} onClick={() => setDraft(null)}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="armada-studio-name">
      {drawn({ heading, shown, untitled: name === null, onOpen })}
      {editable ? (
        <Button size="sm" variant="ghost" aria-label={`Rename ${shown}`} onClick={() => setDraft(name ?? "")}>
          Rename
        </Button>
      ) : null}
    </div>
  );
}

/** The name itself: a heading, a press into the Studio, or plain text. */
function drawn({
  heading,
  shown,
  untitled,
  onOpen,
}: {
  heading: boolean;
  shown: string;
  untitled: boolean;
  onOpen?: () => void;
}): ReactNode {
  const unnamed = untitled || undefined;
  if (heading) {
    return (
      <h2 className="armada-studio-name__heading" data-untitled={unnamed}>
        {shown}
      </h2>
    );
  }
  if (onOpen !== undefined) {
    return (
      <Button variant="ghost" size="sm" data-untitled={unnamed} onClick={onOpen}>
        {shown}
      </Button>
    );
  }
  return (
    <span className="armada-studio-name__text" data-untitled={unnamed}>
      {shown}
    </span>
  );
}
