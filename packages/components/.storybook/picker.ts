// Turns the preview into a one-shot element/region picker on command from
// the Notes panel. Manager and preview are same-origin, so reaching into
// `contentDocument` would work — but it breaks on a preview reload and isn't
// the sanctioned crossing. The channel is, and it's the one manager.tsx
// already uses for notes.
//
// `storybook/highlight` (checked first) draws boxes for elements matched by
// a CSS selector — it has no point-in-space hit test, and its "hovered"
// state lights up every box whose rect contains the cursor, ancestors
// included. Finding the one element under the pointer still means calling
// `elementFromPoint` ourselves, so the overlay here is a plain positioned
// div rather than a detour through that machinery.
import { addons } from "storybook/preview-api";
import { PICK_CANCEL, PICK_CANCELLED, PICK_RESULT, PICK_START } from "./picker-types.ts";
import type { Picked, PickedElement } from "./picker-types.ts";

const ROOT_ID = "storybook-root";
const DRAG_THRESHOLD_PX = 4;
const REGION_CAP = 12;
const ACCENT = "#029CFD"; // Storybook's own highlight-menu accent — one visual language, not two.

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface DragState {
  startX: number;
  startY: number;
  dragging: boolean;
}

let active = false;
let drag: DragState | null = null;
let hoverBox: HTMLDivElement | null = null;
let dragBox: HTMLDivElement | null = null;

function root(): HTMLElement | null {
  return document.getElementById(ROOT_ID);
}

function depthOf(el: Element): number {
  let depth = 0;
  for (let node: Element | null = el; node; node = node.parentElement) depth++;
  return depth;
}

// Stable enough to point at a rule, not a uniqueness guarantee — nth-of-type
// disambiguates real siblings, nothing more.
function selectorFor(el: Element, stopAt: Element): string {
  const parts: string[] = [];
  let node: Element | null = el;
  while (node && node !== stopAt) {
    const parent: Element | null = node.parentElement;
    if (!parent) break;
    const tag = node.tagName.toLowerCase();
    const siblings = Array.from(parent.children).filter((c) => c.tagName === node!.tagName);
    const nth = siblings.length > 1 ? `:nth-of-type(${siblings.indexOf(node) + 1})` : "";
    const cls = node.classList[0] ? `.${node.classList[0]}` : "";
    parts.unshift(`${tag}${cls}${nth}`);
    node = parent;
  }
  return parts.join(" > ");
}

function describe(el: Element, stopAt: Element): PickedElement {
  return {
    classes: Array.from(el.classList),
    selector: selectorFor(el, stopAt),
    text: (el.textContent ?? "").trim().slice(0, 80),
    tag: el.tagName.toLowerCase(),
  };
}

function elementAt(x: number, y: number): Element | null {
  const el = document.elementFromPoint(x, y);
  const anchor = root();
  if (!el || !anchor || !anchor.contains(el)) return null;
  return el;
}

function normalizedRect(x1: number, y1: number, x2: number, y2: number): Rect {
  return {
    x: Math.min(x1, x2),
    y: Math.min(y1, y2),
    width: Math.abs(x2 - x1),
    height: Math.abs(y2 - y1),
  };
}

function applyRect(box: HTMLDivElement, r: { x: number; y: number; width: number; height: number }): void {
  box.style.left = `${r.x}px`;
  box.style.top = `${r.y}px`;
  box.style.width = `${r.width}px`;
  box.style.height = `${r.height}px`;
}

function overlayBox(id: string): HTMLDivElement {
  const box = document.createElement("div");
  box.id = id;
  Object.assign(box.style, {
    position: "fixed",
    pointerEvents: "none",
    zIndex: "2147483647",
    border: `2px solid ${ACCENT}`,
    background: "rgba(2, 156, 253, 0.08)",
    boxSizing: "border-box",
  });
  document.body.appendChild(box);
  return box;
}

function ensureHoverBox(): HTMLDivElement {
  hoverBox ??= overlayBox("armada-picker-hover");
  return hoverBox;
}

function removeHoverBox(): void {
  hoverBox?.remove();
  hoverBox = null;
}

function ensureDragBox(): HTMLDivElement {
  dragBox ??= overlayBox("armada-picker-drag");
  return dragBox;
}

function removeDragBox(): void {
  dragBox?.remove();
  dragBox = null;
}

// Deepest first: a region's highest-value field is the class list, and the
// element carrying the real component class is a leaf, not the wrapping div
// a drag most easily lands on.
function collectEnclosed(rect: Rect, anchor: Element): { elements: PickedElement[]; elided?: number } {
  const right = rect.x + rect.width;
  const bottom = rect.y + rect.height;
  const candidates = Array.from(anchor.querySelectorAll("*")).filter((el) => {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    return r.left >= rect.x && r.top >= rect.y && r.right <= right && r.bottom <= bottom;
  });
  candidates.sort((a, b) => depthOf(b) - depthOf(a));
  const kept = candidates.slice(0, REGION_CAP);
  const elided = candidates.length - kept.length;
  return { elements: kept.map((el) => describe(el, anchor)), elided: elided > 0 ? elided : undefined };
}

function onMouseMove(event: MouseEvent): void {
  event.stopPropagation();
  if (drag) {
    if (!drag.dragging) {
      const dx = event.clientX - drag.startX;
      const dy = event.clientY - drag.startY;
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
      drag.dragging = true;
      removeHoverBox();
    }
    applyRect(ensureDragBox(), normalizedRect(drag.startX, drag.startY, event.clientX, event.clientY));
    return;
  }
  const el = elementAt(event.clientX, event.clientY);
  if (!el) {
    removeHoverBox();
    return;
  }
  const r = el.getBoundingClientRect();
  applyRect(ensureHoverBox(), { x: r.left, y: r.top, width: r.width, height: r.height });
}

function onMouseDown(event: MouseEvent): void {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  drag = { startX: event.clientX, startY: event.clientY, dragging: false };
}

function onMouseUp(event: MouseEvent): void {
  if (!drag) return;
  event.preventDefault();
  event.stopPropagation();
  const finished = drag;
  drag = null;
  if (finished.dragging) {
    finishRegion(finished, event);
  } else {
    finishElement(event);
  }
}

// The click that follows this gesture's mouseup fires as its own event,
// immune to preventDefault on mousedown/mouseup — this is the only place a
// disclosure's own handler can be stopped from firing.
function onClick(event: MouseEvent): void {
  event.preventDefault();
  event.stopPropagation();
}

function onKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  event.preventDefault();
  event.stopPropagation();
  stop();
  emitCancelled();
}

function finishElement(event: MouseEvent): void {
  const el = elementAt(event.clientX, event.clientY);
  const anchor = root();
  stop();
  if (!el || !anchor) {
    emitCancelled();
    return;
  }
  const r = el.getBoundingClientRect();
  const picked: Picked = {
    mode: "element",
    rect: { x: r.left, y: r.top, width: r.width, height: r.height },
    elements: [describe(el, anchor)],
  };
  emitResult(picked);
}

function finishRegion(finished: DragState, event: MouseEvent): void {
  const rect = normalizedRect(finished.startX, finished.startY, event.clientX, event.clientY);
  const anchor = root();
  stop();
  if (!anchor || rect.width < 2 || rect.height < 2) {
    emitCancelled();
    return;
  }
  const { elements, elided } = collectEnclosed(rect, anchor);
  if (elements.length === 0) {
    emitCancelled();
    return;
  }
  const picked: Picked = { mode: "region", rect, elements, elided };
  emitResult(picked);
}

function start(): void {
  if (active) return;
  active = true;
  document.addEventListener("mousemove", onMouseMove, true);
  document.addEventListener("mousedown", onMouseDown, true);
  document.addEventListener("mouseup", onMouseUp, true);
  document.addEventListener("click", onClick, true);
  document.addEventListener("keydown", onKeyDown, true);
}

function stop(): void {
  active = false;
  drag = null;
  document.removeEventListener("mousemove", onMouseMove, true);
  document.removeEventListener("mousedown", onMouseDown, true);
  document.removeEventListener("mouseup", onMouseUp, true);
  document.removeEventListener("keydown", onKeyDown, true);
  // The click for this gesture's mouseup hasn't fired yet — it lands in the
  // same tick, a macrotask before this timer, so the listener catches it
  // before coming off.
  setTimeout(() => document.removeEventListener("click", onClick, true), 0);
  removeHoverBox();
  removeDragBox();
}

let emitResult: (picked: Picked) => void = () => {};
let emitCancelled: () => void = () => {};

addons.ready().then(() => {
  const channel = addons.getChannel();
  emitResult = (picked) => channel.emit(PICK_RESULT, picked);
  emitCancelled = () => channel.emit(PICK_CANCELLED);
  channel.on(PICK_START, start);
  channel.on(PICK_CANCEL, () => {
    stop();
    emitCancelled();
  });
});
