import type { CSSProperties, KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * Studio capture — the layer a person points through to put a Note on the open
 * Studio. `docs/concepts/studio.md`, Notes; `#1290`.
 *
 * **It draws and it does not look.** What is under the pointer, what was
 * pressed and what the Note keeps are read from the page by
 * `apps/desktop`'s capture; this takes boxes and text and puts them on screen.
 *
 * **The card sits against the element, and reading and writing are one place.**
 * A person points, sees which component they hit, and types into the card over
 * it — never a panel elsewhere.
 */

/** A rectangle in the window, in CSS pixels. */
export type CaptureBox = { x: number; y: number; width: number; height: number };

export type StudioCaptureProps = {
  /** What the pointer is over, and what to call it. Absent once one is held. */
  hovered?: { box: CaptureBox; named: string };
  /** The element the note is being written on, and the chain that found it. */
  held?: { box: CaptureBox; chain: string };
  /** What is being typed. Controlled: the layer owns the draft. */
  note: string;
  onNote: (note: string) => void;
  onSave: () => void;
  onCancel: () => void;
  /** Out to Fleet: the card waits rather than taking a second press. */
  saving?: boolean;
  /** Why the last capture did not land, or absent. */
  refused?: string;
  /** What capture is aimed at — the Studio's name — or why it is aimed at nothing. */
  aim: string;
  /** Whether `aim` names a Studio. A capture with none saves nothing. */
  aimed: boolean;
  /** The binding that turns capture off, from the action registry. */
  binding: readonly string[];
};

const place = (box: CaptureBox): CSSProperties =>
  ({
    "--capture-x": `${box.x}px`,
    "--capture-y": `${box.y}px`,
    "--capture-w": `${box.width}px`,
    "--capture-h": `${box.height}px`,
  }) as CSSProperties;

/** Under the element where there is room below it, and over it where there is not. */
function cardAt(box: CaptureBox, height: number): { style: CSSProperties; side: "below" | "above" } {
  const below = box.y + box.height < height * 0.6;
  const y = below ? box.y + box.height : box.y;
  return { style: place({ ...box, y }), side: below ? "below" : "above" };
}

export function StudioCapture(props: StudioCaptureProps) {
  const { hovered, held, note, onNote, onSave, onCancel, saving = false, refused, aim, aimed, binding } = props;
  const savable = aimed && !saving && note.trim() !== "";

  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onCancel();
    } else if (event.key === "Enter" && event.metaKey && savable) {
      event.preventDefault();
      event.stopPropagation();
      onSave();
    }
  }

  const card = held === undefined ? null : cardAt(held.box, window.innerHeight);

  return (
    <div className="armada-capture" data-armada-capture="" onKeyDown={keyed}>
      {hovered === undefined || held !== undefined ? null : (
        <div className="armada-capture__outline" style={place(hovered.box)}>
          <span className="armada-capture__named">{hovered.named}</span>
        </div>
      )}

      {held === undefined || card === null ? null : (
        <>
          <div className="armada-capture__outline" data-held="" style={place(held.box)} />
          <div
            className="armada-capture__card"
            role="dialog"
            aria-label="Capture a note"
            style={card.style}
            data-side={card.side}
          >
            <p className="armada-capture__chain">{held.chain}</p>
            {/* The card opens on a press, so the cursor belongs in it without a second one. */}
            <Textarea autoFocus label="Note" value={note} onChange={(event) => onNote(event.target.value)} />
            {refused === undefined ? null : <p className="armada-capture__refused">{refused}</p>}
            <div className="armada-capture__acts">
              <Button variant="ghost" size="sm" onClick={onCancel}>
                Cancel
              </Button>
              <Button variant="primary" size="sm" pending={saving} disabled={!savable} onClick={onSave}>
                {saving ? "Capturing" : "Capture"}
              </Button>
            </div>
          </div>
        </>
      )}

      <div className="armada-capture__bar" role="status">
        <span>Capturing</span>
        <span>{aim}</span>
        <KbdChord keys={[...binding]} aria-label={`${binding.join(" ")} turns capturing off`} />
      </div>
    </div>
  );
}
