// Studio capture, shipped — #1290. The binding turns it on anywhere in Bridge,
// a press picks the element under the pointer, and what is typed lands on the
// open Studio as a Note fixed at capture.
//
// **A second layer beside the development one, which is unchanged.** `annotate/`
// keeps ⌥⌘A, its files and Send to Fleet; what is reused from it is its reading
// of the page, never its surface.

import { useCallback, useEffect, useRef, useState } from "react";
import { ACTION, StudioCapture as CaptureOverlay, type CaptureBox } from "@armada/components";
import type { Outcome, StudioCapture } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";

import { captureOf, chainOf, namedOf } from "./note";

/** The binding, from the action registry rather than retyped. `⌥⌘C`. */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

/** Pointer events capture takes from the app, so a press points rather than acts. */
const SWALLOWED = ["pointerdown", "pointerup", "mousedown", "mouseup", "click", "dblclick", "contextmenu"] as const;

/** What a capture aims at: the Studio a person last had open and continued. */
export type CaptureAim = { id: string; name: string } | null;

export type CaptureLayerProps = {
  aim: CaptureAim;
  /** Fleet's answer to a capture. `apps/desktop`'s `commands.ts` sends it. */
  onCapture: (studioId: string, said: string, capture: StudioCapture) => Promise<Outcome>;
};

type Held = { element: Element; capture: StudioCapture };

const boxOf = (element: Element): CaptureBox => {
  const rect = element.getBoundingClientRect();
  return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
};

function inLayer(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  return element?.closest("[data-armada-capture]") != null;
}

/** What the pointer is over, taking an SVG as the whole glyph rather than one path. */
function targetOf(target: EventTarget | null): Element | null {
  if (!(target instanceof Element)) return null;
  return target.closest("svg") ?? target;
}

/** `⌥⌘C`, as the registry spells it. Nothing else in Bridge's map holds it. */
export function isCaptureBinding(
  event: Pick<globalThis.KeyboardEvent, "code" | "metaKey" | "altKey" | "ctrlKey" | "shiftKey">,
): boolean {
  return event.code === "KeyC" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
}

/** What the bar says capture is pointed at, and whether a press can land. */
export function aimed(aim: CaptureAim): { aim: string; aimed: boolean } {
  return aim === null
    ? { aim: "No Studio open — open one on Studios and press Continue to capture onto it", aimed: false }
    : { aim: `Onto ${aim.name}`, aimed: true };
}

export function CaptureLayer({ aim, onCapture }: CaptureLayerProps) {
  const [on, setOn] = useState(false);
  const [hovered, setHovered] = useState<Element | null>(null);
  const [held, setHeld] = useState<Held | null>(null);
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  // Boxes are measured each frame, so an outline follows what it points at.
  const [boxes, setBoxes] = useState<{ hovered: CaptureBox | null; held: CaptureBox | null }>({
    hovered: null,
    held: null,
  });
  const noted = useRef("");
  noted.current = note;

  const close = useCallback(() => {
    setHeld(null);
    setNote("");
    setRefused(undefined);
  }, []);

  useEffect(() => {
    function pressed(event: globalThis.KeyboardEvent): void {
      if (!isCaptureBinding(event)) return;
      event.preventDefault();
      event.stopPropagation();
      setOn((was) => !was);
    }
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  }, []);

  useEffect(() => {
    if (on) return;
    setHovered(null);
    close();
  }, [on, close]);

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
      // A note half written is not thrown away by a stray press.
      if (element === null || noted.current.trim() !== "") return;
      setRefused(undefined);
      setHeld({ element, capture: captureOf(element, new Date()) });
    }
    window.addEventListener("pointermove", moved, true);
    for (const type of SWALLOWED) window.addEventListener(type, swallowed, true);
    return () => {
      window.removeEventListener("pointermove", moved, true);
      for (const type of SWALLOWED) window.removeEventListener(type, swallowed, true);
    };
  }, [on]);

  // Compared before it is set, so a still page costs no render.
  useEffect(() => {
    if (!on) return undefined;
    let frame = 0;
    let last = "";
    const tick = (): void => {
      const next = {
        hovered: hovered !== null && hovered.isConnected ? boxOf(hovered) : null,
        held: held !== null && held.element.isConnected ? boxOf(held.element) : null,
      };
      const key = JSON.stringify(next);
      if (key !== last) {
        last = key;
        setBoxes(next);
      }
      frame = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(frame);
  }, [on, hovered, held]);

  async function save(): Promise<void> {
    if (held === null || aim === null) return;
    setSaving(true);
    const outcome = await onCapture(aim.id, note.trim(), held.capture);
    setSaving(false);
    if (outcome.ok) {
      setOn(false);
      return;
    }
    setRefused(`Not captured: ${said(outcome)}`);
  }

  if (!on) return null;

  const where = aimed(aim);
  return (
    <CaptureOverlay
      {...where}
      binding={BINDING}
      note={note}
      onNote={setNote}
      onSave={() => void save()}
      onCancel={close}
      saving={saving}
      refused={refused}
      {...(hovered !== null && boxes.hovered !== null && held === null
        ? { hovered: { box: boxes.hovered, named: namedOf(hovered) } }
        : {})}
      {...(held !== null && boxes.held !== null
        ? { held: { box: boxes.held, chain: chainOf(held.capture) } }
        : {})}
    />
  );
}
