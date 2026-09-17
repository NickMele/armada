// The annotation layer, #1226. Dev only: `main.tsx` loads this chunk only when a
// save path exists, and a packaged build has none.

import { useCallback, useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { Button, KbdChord, Textarea } from "@armada/components";

import { byCreation, type Annotation, type Box } from "../../../shared/annotations";
import { capture, locate } from "./capture";
import { componentsOf, fiberOf } from "./fiber";
import { sendToFleet, unsendable } from "./send";
import type { Sink } from "./sink";
import "./annotate.css";

/** ⌥⌘A. Free in Bridge's key map and in Electron's default menu, beside ⌥⌘I for devtools. */
export function isToggle(event: Pick<globalThis.KeyboardEvent, "code" | "metaKey" | "altKey" | "ctrlKey" | "shiftKey">): boolean {
  return event.code === "KeyA" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
}

/** Survives a reload, so pins are still drawn after one. Per window, not per machine. */
const ON_KEY = "armada.annotate.on";

/** Pointer events the layer takes from the app while it is on, so a click annotates rather than acts. */
const SWALLOWED = ["pointerdown", "pointerup", "mousedown", "mouseup", "click", "dblclick", "contextmenu"] as const;

type Draft = { note: Annotation; element: Element; text: string };

const boxOf = (element: Element): Box => {
  const r = element.getBoundingClientRect();
  return { x: r.x, y: r.y, width: r.width, height: r.height };
};

const place = (box: Box): CSSProperties =>
  ({
    "--annotate-x": `${box.x}px`,
    "--annotate-y": `${box.y}px`,
    "--annotate-w": `${box.width}px`,
    "--annotate-h": `${box.height}px`,
  }) as CSSProperties;

/** A card sits under its element when there is room, and over it when there is not. */
function cardPlace(box: Box): { style: CSSProperties; side: "below" | "above" } {
  const below = box.y + box.height < window.innerHeight * 0.6;
  const y = below ? box.y + box.height : box.y;
  return { style: place({ ...box, y }), side: below ? "below" : "above" };
}

function inLayer(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  return element?.closest("[data-armada-annotate]") != null;
}

/** What the pointer is over, taking an SVG as the whole glyph rather than one path in it. */
function targetOf(target: EventTarget | null): Element | null {
  if (!(target instanceof Element)) return null;
  return target.closest("svg") ?? target;
}

export function Layer({ sink }: { sink: Sink }) {
  const [on, setOn] = useState(() => sessionStorage.getItem(ON_KEY) === "1");
  const [notes, setNotes] = useState<Annotation[]>([]);
  const [hovered, setHovered] = useState<Element | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [frames, setFrames] = useState<Record<string, Box | null>>({});
  // Which send is out: one note's id, or every open note on this screen.
  const [sending, setSending] = useState<string | null>(null);
  const cannotSend = unsendable(sink);
  const draftRef = useRef(draft);
  draftRef.current = draft;

  const fail = useCallback((what: string, cause: unknown) => {
    setError(`${what} failed through ${sink.via}: ${cause instanceof Error ? cause.message : String(cause)}`);
  }, [sink.via]);

  useEffect(() => {
    function pressed(event: globalThis.KeyboardEvent): void {
      if (!isToggle(event)) return;
      event.preventDefault();
      event.stopPropagation();
      setOn((was) => !was);
    }
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  }, []);

  // Read the files again every time the layer comes on, so a note an agent marked done shows as done.
  useEffect(() => {
    sessionStorage.setItem(ON_KEY, on ? "1" : "0");
    if (!on) {
      setHovered(null);
      setDraft(null);
      setOpened(null);
      return;
    }
    setError(null);
    sink.list().then((read) => setNotes([...read].sort(byCreation)), (cause) => fail("Reading notes", cause));
  }, [on, sink, fail]);

  useEffect(() => {
    if (!on) return undefined;
    function moved(event: PointerEvent): void {
      if (inLayer(event.target)) return;
      setHovered(targetOf(event.target));
    }
    function swallowed(event: Event): void {
      if (inLayer(event.target)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.type !== "click") return;
      const element = targetOf(event.target);
      // A note half written is not thrown away by a stray click.
      if (element === null || (draftRef.current !== null && draftRef.current.text.trim() !== "")) return;
      setOpened(null);
      setDraft({ note: capture(element, new Date()), element, text: "" });
    }
    window.addEventListener("pointermove", moved, true);
    for (const type of SWALLOWED) window.addEventListener(type, swallowed, true);
    return () => {
      window.removeEventListener("pointermove", moved, true);
      for (const type of SWALLOWED) window.removeEventListener(type, swallowed, true);
    };
  }, [on]);

  // Pins follow their elements through scrolling, resizing and re-rendering; compared before set, so a still page costs no render.
  useEffect(() => {
    if (!on) return undefined;
    let frame = 0;
    let last = "";
    const tick = (): void => {
      const next: Record<string, Box | null> = {};
      for (const note of notes) {
        const element = locate(note);
        next[note.id] = element === null ? null : boxOf(element);
      }
      if (hovered !== null) next["hovered"] = hovered.isConnected ? boxOf(hovered) : null;
      if (draft !== null) next["draft"] = draft.element.isConnected ? boxOf(draft.element) : draft.note.box;
      const key = JSON.stringify(next);
      if (key !== last) {
        last = key;
        setFrames(next);
      }
      frame = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(frame);
  }, [on, notes, hovered, draft]);

  async function save(): Promise<void> {
    if (draft === null || draft.text.trim() === "") return;
    const note = { ...draft.note, text: draft.text.trim(), updatedAt: new Date().toISOString() };
    try {
      await sink.save(note);
      setNotes((was) => [...was.filter((n) => n.id !== note.id), note].sort(byCreation));
      setDraft(null);
      setError(null);
    } catch (cause) {
      fail("Saving", cause);
    }
  }

  async function setStatus(note: Annotation, status: Annotation["status"]): Promise<void> {
    const next = { ...note, status, updatedAt: new Date().toISOString() };
    try {
      await sink.save(next);
      setNotes((was) => was.map((n) => (n.id === note.id ? next : n)));
    } catch (cause) {
      fail("Saving", cause);
    }
  }

  /** One note to Fleet. Written back with its Job only once Fleet took it. */
  async function send(note: Annotation): Promise<boolean> {
    const box = frames[note.id] ?? note.box;
    try {
      const answer = await sendToFleet(note, box, sink, window.armada, new Date());
      if (!answer.ok) {
        setError(`Not sent: ${answer.saying}`);
        return false;
      }
      const next = { ...note, sent: answer.sent, updatedAt: answer.sent.at };
      await sink.save(next);
      setNotes((was) => was.map((n) => (n.id === note.id ? next : n)));
      setError(null);
      return true;
    } catch (cause) {
      fail("Sending", cause);
      return false;
    }
  }

  async function sendOne(note: Annotation): Promise<void> {
    setSending(note.id);
    await send(note);
    setSending(null);
  }

  /** Every open, unsent note on this screen, one Job each, stopping at the first Fleet refuses. */
  async function sendHere(): Promise<void> {
    setSending("here");
    for (const note of notes.filter((n) => n.status === "open" && n.sent === undefined && frames[n.id] != null)) {
      if (!(await send(note))) break;
    }
    setSending(null);
  }

  async function remove(note: Annotation): Promise<void> {
    try {
      await sink.remove(note.id);
      setNotes((was) => was.filter((n) => n.id !== note.id));
      setOpened(null);
    } catch (cause) {
      fail("Deleting", cause);
    }
  }

  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      setDraft(null);
      setOpened(null);
    } else if (event.key === "Enter" && event.metaKey && draft !== null) {
      event.preventDefault();
      event.stopPropagation();
      void save();
    }
  }

  if (!on) return null;

  const openNote = notes.find((n) => n.id === opened) ?? null;
  const openBox = openNote === null ? null : frames[openNote.id] ?? null;
  const hoveredBox = draft === null ? frames["hovered"] ?? null : null;
  const draftBox = draft === null ? null : frames["draft"] ?? draft.note.box;
  const offScreen = notes.filter((n) => frames[n.id] == null).length;
  const open = notes.filter((n) => n.status === "open").length;
  const sendableHere = notes.filter((n) => n.status === "open" && n.sent === undefined && frames[n.id] != null).length;

  return (
    <div className="armada-annotate" data-armada-annotate="" onKeyDown={keyed}>
      {hoveredBox !== null && hovered !== null && (
        <div className="armada-annotate__outline" style={place(hoveredBox)}>
          <span className="armada-annotate__tag">
            {componentsOf(fiberOf(hovered)).component ?? hovered.tagName.toLowerCase()}
          </span>
        </div>
      )}

      {notes.map((note, i) => {
        const box = frames[note.id];
        if (box == null) return null;
        return (
          <button
            key={note.id}
            type="button"
            className="armada-annotate__pin"
            data-status={note.status}
            data-sent={note.sent === undefined ? undefined : ""}
            style={place(box)}
            aria-label={`Note ${i + 1}, ${note.status}${note.sent === undefined ? "" : `, sent as ${note.sent.handle}`}: ${note.text}`}
            onClick={() => {
              setDraft(null);
              setDeleting(false);
              setOpened(opened === note.id ? null : note.id);
            }}
          >
            {i + 1}
          </button>
        );
      })}

      {draft !== null && draftBox !== null && (
        <>
          <div className="armada-annotate__outline" data-held="" style={place(draftBox)} />
          <div className="armada-annotate__card" role="dialog" aria-label="New note" {...cardProps(draftBox)}>
            <p className="armada-annotate__meta">{chain(draft.note)}</p>
            <Textarea
              label="Note"
              autoFocus
              value={draft.text}
              onChange={(event) => setDraft({ ...draft, text: event.target.value })}
            />
            <div className="armada-annotate__actions">
              <Button variant="ghost" size="sm" onClick={() => setDraft(null)}>
                Cancel
              </Button>
              <Button variant="primary" size="sm" disabled={draft.text.trim() === ""} onClick={() => void save()}>
                Save note
              </Button>
            </div>
          </div>
        </>
      )}

      {openNote !== null && openBox !== null && (
        <div className="armada-annotate__card" role="dialog" aria-label="Note" {...cardProps(openBox)}>
          <p className="armada-annotate__meta">{chain(openNote)}</p>
          <p className="armada-annotate__text">{openNote.text}</p>
          {openNote.sent !== undefined && (
            <p className="armada-annotate__meta">Sent to Fleet as {openNote.sent.handle}, waiting on approval on the Board</p>
          )}
          {openNote.sent === undefined && cannotSend !== null && <p className="armada-annotate__meta">{cannotSend}</p>}
          <div className="armada-annotate__actions">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => (deleting ? void remove(openNote) : setDeleting(true))}
            >
              {deleting ? "Press again to delete" : "Delete"}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void setStatus(openNote, openNote.status === "open" ? "done" : "open")}
            >
              {openNote.status === "open" ? "Mark done" : "Reopen"}
            </Button>
            {openNote.sent === undefined && openNote.status === "open" && (
              <Button
                variant="primary"
                size="sm"
                disabled={cannotSend !== null || sending !== null}
                onClick={() => void sendOne(openNote)}
              >
                {sending === openNote.id ? "Sending…" : "Send to Fleet"}
              </Button>
            )}
          </div>
        </div>
      )}

      <div className="armada-annotate__bar" role="status">
        <span>Annotating</span>
        <span>
          {open} open, {notes.length - open} done
          {offScreen > 0 ? `, ${offScreen} not on this screen` : ""}
        </span>
        {cannotSend === null && sendableHere > 0 && (
          <Button variant="secondary" size="sm" disabled={sending !== null} onClick={() => void sendHere()}>
            {sending === "here" ? "Sending…" : `Send ${sendableHere} to Fleet`}
          </Button>
        )}
        {error !== null && <span className="armada-annotate__error">{error}</span>}
        <KbdChord keys={["⌥", "⌘", "A"]} aria-label="Option Command A turns annotating off" />
      </div>
    </div>
  );
}

function cardProps(box: Box): { style: CSSProperties; "data-side": "below" | "above" } {
  const { style, side } = cardPlace(box);
  return { style, "data-side": side };
}

/** `JobRowStacked ← ActiveJobsList ← Board`, then the selector, for the person checking what was picked. */
function chain(note: Annotation): string {
  const names = [note.component, ...note.owners].filter((n): n is string => n !== null).slice(0, 4);
  return `${names.length > 0 ? names.join(" ← ") : note.element.tag} · ${note.selector}`;
}
