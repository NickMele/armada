// The capture window's own document — #1294, `docs/practices/capture-window.md`.
//
// **Bridge's chrome, drawn outside the page it sits above.** The bar names the
// Run, the address and the Studio; while capture is armed this view covers the
// whole window, so the outline and the note card are drawn where the page
// cannot read them, restyle them or forge them — and a press never reaches the
// app underneath.
//
// Nothing here holds a capture. What a person points at is read by main in the
// page's own world and bounded there; this gets a box and one line.

import { useCallback, useEffect, useRef, useState } from "react";
import { ACTION, CaptureBar, StudioCapture as CaptureOverlay, type CaptureBox } from "@armada/components";
import { said } from "@armada/screens/src/copy";
import { UNTITLED_STUDIO } from "@armada/screens/src/studio";

import type { CaptureAimed, CaptureHeld, CaptureWindowState } from "../../../shared/capture-window";

/** The binding, from the action registry rather than retyped. `⌥⌘C`. */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

/** What each refusal is called on the bar. A page moves in five ways and each says which. */
const REFUSED_SAID: Record<string, string> = {
  navigation: "Navigation",
  redirect: "Redirect",
  frame: "Frame",
  window: "Popup",
  download: "Download",
};

/** How often the pointer is asked about, in frames. One ask per frame is one round trip per frame. */
type Pointer = { x: number; y: number };

export type CaptureWindowApi = {
  read: (onChanged: (state: CaptureWindowState) => void) => () => void;
  arm: (on: boolean) => Promise<CaptureWindowState | null>;
  aim: (x: number, y: number) => Promise<CaptureAimed | null>;
  hold: (x: number, y: number) => Promise<CaptureHeld | null>;
  release: () => Promise<void>;
  save: (said: string) => Promise<{ ok: boolean } & Record<string, unknown>>;
  reload: () => Promise<void>;
  followRefused: () => Promise<void>;
  scroll: (wheel: { x: number; y: number; deltaX: number; deltaY: number }) => void;
};

export function CaptureWindow({ api }: { api: CaptureWindowApi }) {
  const [state, setState] = useState<CaptureWindowState | null>(null);
  const [aimed, setAimed] = useState<CaptureAimed | null>(null);
  const [held, setHeld] = useState<CaptureHeld | null>(null);
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const bar = useRef<HTMLDivElement>(null);
  const pointer = useRef<Pointer | null>(null);

  useEffect(() => api.read(setState), [api]);

  const armed = state?.armed === true;
  const heldNow = held !== null;

  /** The page's viewport, which starts where the bar ends. Measured, never assumed. */
  const strip = useCallback((): number => bar.current?.offsetHeight ?? 0, []);

  const close = useCallback(() => {
    setHeld(null);
    setNote("");
    setRefused(undefined);
    void api.release();
  }, [api]);

  useEffect(() => {
    if (armed) return;
    setAimed(null);
    setHeld(null);
    setNote("");
    setRefused(undefined);
  }, [armed]);

  // The pointer is asked about once a frame while nothing is held, so the
  // outline follows without one round trip per pointer event.
  useEffect(() => {
    if (!armed || heldNow) return undefined;
    let frame = 0;
    let out = false;
    let last = "";
    const tick = (): void => {
      frame = requestAnimationFrame(tick);
      const at = pointer.current;
      if (out || at === null) return;
      const key = `${at.x},${at.y}`;
      if (key === last) return;
      last = key;
      out = true;
      void api.aim(at.x, at.y - strip()).then((answer) => {
        out = false;
        setAimed(answer);
      });
    };
    tick();
    return () => cancelAnimationFrame(frame);
  }, [api, armed, heldNow, strip]);

  useEffect(() => {
    if (!armed) return undefined;
    function moved(event: PointerEvent): void {
      pointer.current = { x: event.clientX, y: event.clientY };
    }
    function pressed(event: MouseEvent): void {
      // The card is Bridge's own surface and takes its own presses.
      if (event.target instanceof Element && event.target.closest("[data-armada-capture] *") !== null) return;
      if (event.clientY < strip()) return;
      event.preventDefault();
      event.stopPropagation();
      if (held !== null || note.trim() !== "") return;
      void api.hold(event.clientX, event.clientY - strip()).then((answer) => {
        if (answer !== null) setHeld(answer);
      });
    }
    // The page still scrolls under the outline: the wheel is taken here and
    // driven into the page by main, which is main moving the page rather than
    // the page reaching anything.
    function wheeled(event: WheelEvent): void {
      if (event.clientY < strip()) return;
      api.scroll({
        x: event.clientX,
        y: event.clientY - strip(),
        deltaX: -event.deltaX,
        deltaY: -event.deltaY,
      });
    }
    window.addEventListener("pointermove", moved, true);
    window.addEventListener("click", pressed, true);
    window.addEventListener("wheel", wheeled, { capture: true, passive: true });
    return () => {
      window.removeEventListener("pointermove", moved, true);
      window.removeEventListener("click", pressed, true);
      window.removeEventListener("wheel", wheeled, true);
    };
  }, [api, armed, held, note, strip]);

  async function save(): Promise<void> {
    setSaving(true);
    const outcome = await api.save(note.trim());
    setSaving(false);
    if (outcome.ok) {
      setNote("");
      setHeld(null);
      return;
    }
    setRefused(`Not captured: ${said(outcome as Parameters<typeof said>[0])}`);
  }

  if (state === null) return null;

  // The page starts below the bar, so a box measured in its viewport is drawn
  // that much lower in this document.
  const lowered = (box: CaptureBox): CaptureBox => ({ ...box, y: box.y + strip() });
  const studio = state.studio.name ?? UNTITLED_STUDIO;
  const aim = state.serving ? `Onto ${studio}` : "The run ended — nothing more is captured";

  return (
    <>
      <div ref={bar}>
        <CaptureBar
          run={state.served.name}
          address={state.served.address}
          studio={studio}
          serving={state.serving}
          armed={armed}
          framesRefused={state.framesRefused}
          {...(state.refused === undefined
            ? {}
            : {
                refused: {
                  address: state.refused.address,
                  said: REFUSED_SAID[state.refused.what] ?? "Navigation",
                  offerable: state.refused.offerable,
                },
              })}
          onArm={(on) => void api.arm(on)}
          onReload={() => void api.reload()}
          onFollowRefused={() => void api.followRefused()}
          binding={BINDING}
        />
      </div>
      {armed ? (
        <CaptureOverlay
          aim={aim}
          aimed={state.serving}
          binding={BINDING}
          note={note}
          onNote={setNote}
          onSave={() => void save()}
          onCancel={close}
          saving={saving}
          {...(refused === undefined ? {} : { refused })}
          {...(aimed !== null && held === null ? { hovered: { box: lowered(aimed.box), named: aimed.named } } : {})}
          {...(held === null ? {} : { held: { box: lowered(held.box), chain: held.chain } })}
        />
      ) : null}
    </>
  );
}
