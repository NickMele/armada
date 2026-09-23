import { MessageSquare } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { BoardEmptyState } from "../BoardEmptyState/BoardEmptyState";
import { FleetPanel, type FleetPanelProps } from "../FleetPanel/FleetPanel";
import { Sidebar, type SidebarItem } from "../Sidebar/Sidebar";
import { StatsPanel, type StatsPanelProps } from "../StatsPanel/StatsPanel";
import { TitleBar } from "../TitleBar/TitleBar";
import { Button } from "../../primitives/Button/Button";
import { KbdCmd } from "../../primitives/Kbd/Kbd";
import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { HelmSheet } from "./HelmSheet";

/**
 * The shell — the left column, panel and dock. Bridge/1088 replaced the rail
 * and the status bar with three rounded panels — Navigation, Stats and
 * Fleet — stacked in one column. Spend and advice left with the bar and
 * appear nowhere here.
 *
 * **The roster is the caller's.** One surface is in the rail because one
 * surface exists, so the surfaces arrive as a prop and this component counts
 * nothing.
 *
 * The panel scrolls; the left column and the dock do not. `min-height: 0` on
 * the scrolling child is what stops the window growing instead.
 */
export type TheShellProps = {
  /** Beneath the rail's own section label — nothing draws here since #1087
   *  moved the Manifest picker to the title row; kept for a caller that wants
   *  the rail to carry something else. */
  railHeader?: ReactNode;
  /**
   * The repository picker, in the title row. **Moved out of the rail by
   * #1087** — the row that used to be macOS's grey bar now carries it beside
   * the traffic lights. Absent draws none.
   */
  repositoryPicker?: ReactNode;
  /** Opens the command palette from the title row's search field. Absent draws no field. */
  onSearch?: () => void;
  /** Opens the composer from the title row's Dispatch control. Absent draws no control. */
  onDispatch?: () => void;
  /** Disabled while nothing is connected to dispatch into. */
  dispatchDisabled?: boolean;
  surfaces: SidebarItem[];
  activeId?: string;
  /**
   * The 48px icon rail — the column's narrowest, and the narrowest it ever
   * gets. The surface decides, from two terms: the window is under
   * `--layout-breakpoint`, or the person asked for it (#1591). **The dock's
   * state is not one of them** and was until #1583 — a second bound collapsed
   * the column while the dock was taking 380px off the content, so opening
   * Helm made the content wider.
   *
   * **There is no third width.** #1435 added one — the column absent
   * altogether — and the owner corrected it on 18 Sep 2026: the panels
   * collapse into an icon-size column, they are not hidden away. A person's
   * press reaches this same rail, never a second narrow state.
   */
  collapsed?: boolean;
  /**
   * The press that collapses the column and brings it back — drawn in
   * Navigation's own head by `Sidebar`. **Absent draws no control**, which is
   * what the shell's caller passes under `--layout-breakpoint`: the rail is
   * the only width there, so there is no choice to offer. Remembering the
   * answer across a restart is the caller's, as `leftWidth` already is.
   */
  onCollapsedChange?: (collapsed: boolean) => void;
  /** `toggle_sidebar`'s binding, for that control's tooltip. */
  collapseBinding?: string;
  onSelect?: (id: string) => void;
  /** The panel body. The one region that scrolls. No head above it: #1090
   *  ended the panel head every screen used to spend on its own name — the
   *  rail already says where you are, so a screen's own controls sit at the
   *  top of its content instead. */
  children: ReactNode;
  /** Helm's dock, on every surface. Absent draws none. */
  dock?: TheShellDock;
  /** The left column's second panel. Open state is the surface's to persist. */
  stats: Omit<StatsPanelProps, "narrow">;
  /** The left column's third panel, what the status bar used to read. */
  fleet: Omit<FleetPanelProps, "narrow">;
  /**
   * The left column's resting width in px, for Navigation, Stats and Fleet
   * together — one width, the same way `.armada-shell__left` already shares
   * it among the three (#1124). Absent draws `--sidebar-default`. Ignored
   * while `collapsed`, which always draws `--sidebar-rail` — there is nothing
   * to size at 48px.
   */
  leftWidth?: number;
  /**
   * Drags and arrow-key nudges the trailing-edge handle, clamped between
   * `--sidebar-min` and `--sidebar-max` — `clampLeftWidth`. **Absent draws no
   * handle at all**, the dock's own reasoning: an edge that looks grabbable
   * and does nothing is worse than no edge. Hidden outright while `collapsed`,
   * the same way the dock's handle disappears while folded — nothing to drag
   * at the rail's 48px. Persisting the result across a restart is the
   * caller's, the same way the dock's own `width` is.
   */
  onResizeLeft?: (width: number) => void;
};

/**
 * Helm's dock (#948). **Over the content, never a column in it** (#1583): the
 * panel keeps its frame and its place on the trailing edge, and the screen
 * underneath keeps the window's width open or shut. It starts shut.
 *
 * **Closed draws nothing at all beyond the layout breakpoint** — #1094 dropped
 * the edge strip that used to sit there at any width, since the title row's
 * own Helm button (#1087) is already the way back. Folded keeps its strip:
 * under the breakpoint the title row has no room for the button.
 */
export type TheShellDock = {
  /** The panel over the content, or the sheet when folded. */
  open: boolean;
  /** Below `--layout-breakpoint`. A prop, because a media query cannot read the token. */
  folded?: boolean;
  /** Questions waiting on the person. Zero draws no count. */
  questions?: number;
  /** The binding, beside the close and in the strip's tooltip. */
  binding?: string;
  onOpen: (open: boolean) => void;
  /**
   * The resting width in px. Absent draws `--w-dock`. Read while the panel is
   * over the content; a folded dock is the `Sheet`'s own width, never a drag.
   */
  width?: number;
  /**
   * Drags and arrow-key nudges the leading-edge handle, clamped between
   * `--w-dock-min` and whatever the window leaves once the left column and
   * `--w-work-min` are accounted for — `clampDockWidth`. **Absent draws no
   * handle at all** — an edge that looks grabbable and does nothing is worse
   * than no edge. Persisting the result across a restart is the caller's, the
   * same way `open` is.
   */
  onResize?: (width: number) => void;
  /**
   * The dock's own act, beside Close — Helm's *Start fresh*, put there by the
   * owner on 18 Sep 2026: it ends the conversation the dock holds, which is
   * the head's business rather than the composer's, and the composer's own row
   * had run out of width for it. **One control**, and the head is the same
   * layout without it. Drawn in the folded sheet's head too, through `Sheet`'s
   * own `controls` slot — folding the dock must not take an act away.
   */
  action?: ReactNode;
  /** Absent draws one quiet line until the questions (#935) and the conversation (#944) arrive. */
  children?: ReactNode;
};

export function TheShell({
  railHeader,
  repositoryPicker,
  onSearch,
  onDispatch,
  dispatchDisabled,
  surfaces,
  activeId,
  collapsed,
  onCollapsedChange,
  collapseBinding,
  onSelect,
  children,
  dock,
  stats,
  fleet,
  leftWidth,
  onResizeLeft,
}: TheShellProps) {
  // The effective width whether or not a caller passed one — read by the
  // handle's own `width` and by the dock's chrome calc below, so a dock
  // dragged wide never assumes the column is narrower than it actually is.
  const leftWidthValue = clampLeftWidth(leftWidth ?? defaultLeftWidth());
  return (
    <ShortcutRevealProvider>
      <div className="armada-shell">
        <TitleBar
          repositoryPicker={repositoryPicker}
          onSearch={onSearch}
          onDispatch={onDispatch}
          dispatchDisabled={dispatchDisabled}
          helm={helmButtonOf(dock)}
          // Every width. The panel carries the same state from the same two
          // fields; the owner settled the duplication on 18 Sep 2026 — keep
          // it, drawn always, rather than mounting chrome on a resize.
          fleet={{ state: fleet.state, label: fleet.label }}
        />
        <div className="armada-shell__body">
          <div
            className="armada-shell__left"
            style={collapsed || leftWidth === undefined ? undefined : { width: `${leftWidthValue}px` }}
          >
            <Sidebar
              header={railHeader}
              sectionLabel={null}
              surfaces={surfaces}
              activeId={activeId}
              collapsed={collapsed}
              {...(onCollapsedChange === undefined ? {} : { onCollapsedChange })}
              {...(collapseBinding === undefined ? {} : { collapseBinding })}
              // The column holds the width. Left to its own 200px default, the
              // nav stayed put while Stats and Fleet followed a drag.
              width="100%"
              onSelect={onSelect}
            />
            <StatsPanel {...stats} narrow={collapsed} />
            <FleetPanel {...fleet} narrow={collapsed} />
          </div>
          {collapsed || onResizeLeft === undefined ? null : (
            <LeftHandle width={leftWidthValue} onResize={onResizeLeft} />
          )}
          <div className="armada-shell__work">
            <div className="armada-shell__panel">
              <div className="armada-shell__mount">{children}</div>
            </div>
            {dock === undefined ? null : <Dock {...dock} leftWidth={leftWidthValue} />}
          </div>
        </div>
      </div>
    </ShortcutRevealProvider>
  );
}

const DOCK_TITLE = "Helm";

// Fallbacks only for a caller with no stylesheet loaded (a bare unit test);
// the tokens are the real source and are read fresh on every drag. The max
// fallback stands in for a whole computed ceiling, not one token, since a
// caller with no stylesheet has no window figure worth trusting either.
const DOCK_WIDTH_MIN_FALLBACK = 320;
const DOCK_WIDTH_MAX_FALLBACK = 640;
const DOCK_WIDTH_DEFAULT_FALLBACK = 380;

/**
 * The dock's drag range. The floor is a token — `--w-dock-min` keeps the
 * composer's head row from clipping. **The ceiling has no token**, since
 * #1171/#1176: `availableWidth` (the caller's `window.innerWidth`) minus the
 * chrome around the dock that isn't the panel or the dock itself — the left
 * column's width, its margin, the gap, the handle's hit area and the dock's
 * own margin, four `--space-4` gutters and the left column between them, all
 * from `TheShell.css` — minus `--w-work-min`. `leftWidth` is the column's
 * actual width now that it resizes too; absent falls back to
 * `--sidebar-default`, what this always read before.
 *
 * **The arithmetic did not move when the dock did** (#1583). `--w-work-min`
 * was the panel's floor beside the dock; it is what stays uncovered under it.
 */
function dockWidthBounds(availableWidth: number, leftWidth?: number): { min: number; max: number } {
  if (typeof document === "undefined") {
    return { min: DOCK_WIDTH_MIN_FALLBACK, max: DOCK_WIDTH_MAX_FALLBACK };
  }
  const style = getComputedStyle(document.documentElement);
  const min = parseFloat(style.getPropertyValue("--w-dock-min"));
  const sidebar = leftWidth ?? parseFloat(style.getPropertyValue("--sidebar-default"));
  const workMin = parseFloat(style.getPropertyValue("--w-work-min"));
  const gutter = parseFloat(style.getPropertyValue("--space-4"));
  const floor = Number.isFinite(min) ? min : DOCK_WIDTH_MIN_FALLBACK;
  if (![sidebar, workMin, gutter].every(Number.isFinite)) {
    return { min: floor, max: DOCK_WIDTH_MAX_FALLBACK };
  }
  const chrome = sidebar + 4 * gutter;
  const dynamicMax = availableWidth - chrome - workMin;
  return { min: floor, max: Math.max(floor, dynamicMax) };
}

/** The window's own width, read live — the one figure here no CSS token can
 *  name, and the reason the dock's ceiling has to be computed rather than
 *  read off the theme. Updates on resize so a dock already open shrinks with
 *  the window instead of pushing it into overflow. */
function useWindowWidth(): number {
  const [width, setWidth] = useState(() => (typeof window === "undefined" ? 0 : window.innerWidth));
  useEffect(() => {
    function read(): void {
      setWidth(window.innerWidth);
    }
    window.addEventListener("resize", read);
    return () => window.removeEventListener("resize", read);
  }, []);
  return width;
}

/** The dock's own resting width, in px, for a caller with none of its own to remember yet. */
export function defaultDockWidth(): number {
  if (typeof document === "undefined") return DOCK_WIDTH_DEFAULT_FALLBACK;
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--w-dock"));
  return Number.isFinite(value) ? value : DOCK_WIDTH_DEFAULT_FALLBACK;
}

/**
 * Clamped to the drag range `dockWidthBounds` computes from `availableWidth`
 * — the caller's `window.innerWidth` — so a width read back from storage —
 * stale, from a narrower window, or from a build that carried different
 * tokens — never draws past what today's window and today's tokens allow.
 */
export function clampDockWidth(width: number, availableWidth: number, leftWidth?: number): number {
  const { min, max } = dockWidthBounds(availableWidth, leftWidth);
  return Math.min(max, Math.max(min, width));
}

/** One `--space-4` per arrow press — the same step the dock's own padding uses. */
const DOCK_WIDTH_STEP = 16;

const LEFT_WIDTH_MIN_FALLBACK = 160;
const LEFT_WIDTH_MAX_FALLBACK = 320;
const LEFT_WIDTH_DEFAULT_FALLBACK = 200;

/**
 * The left column's drag range — Navigation, Stats and Fleet resize as one,
 * so the range is the rail's own tokens: `--sidebar-min` and `--sidebar-max`,
 * the same ceiling Sidebar's own `max-width` already enforces (#1124).
 * **No window-relative ceiling here**, unlike the dock — the dock holds
 * whatever a Drone hands back with no natural size of its own; the left
 * column is three panels of fixed content the design system already sized.
 */
function leftWidthBounds(): { min: number; max: number } {
  if (typeof document === "undefined") {
    return { min: LEFT_WIDTH_MIN_FALLBACK, max: LEFT_WIDTH_MAX_FALLBACK };
  }
  const style = getComputedStyle(document.documentElement);
  const min = parseFloat(style.getPropertyValue("--sidebar-min"));
  const max = parseFloat(style.getPropertyValue("--sidebar-max"));
  return {
    min: Number.isFinite(min) ? min : LEFT_WIDTH_MIN_FALLBACK,
    max: Number.isFinite(max) ? max : LEFT_WIDTH_MAX_FALLBACK,
  };
}

/** The left column's own resting width, for a caller with none of its own to remember yet. */
export function defaultLeftWidth(): number {
  if (typeof document === "undefined") return LEFT_WIDTH_DEFAULT_FALLBACK;
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--sidebar-default"));
  return Number.isFinite(value) ? value : LEFT_WIDTH_DEFAULT_FALLBACK;
}

/** Clamped to `leftWidthBounds`, the same reasoning `clampDockWidth` uses for the dock. */
export function clampLeftWidth(width: number): number {
  const { min, max } = leftWidthBounds();
  return Math.min(max, Math.max(min, width));
}

/** One `--space-4` per arrow press, matching the dock's own step. */
const LEFT_WIDTH_STEP = 16;

/**
 * The dock's leading-edge handle. A drag or an arrow key moves it; both read
 * the same clamp so neither can push the dock past what a mouse could reach.
 *
 * **Left widens the dock, right narrows it** — the dock sits on the window's
 * trailing edge, so dragging toward the content is dragging the edge that
 * grows it, the same direction a mouse drag moves. Home and End match: Home
 * (the leftmost position a splitter can take) is the widest the dock gets.
 */
function DockHandle({
  width,
  availableWidth,
  leftWidth,
  onResize,
}: {
  width: number;
  availableWidth: number;
  leftWidth: number;
  onResize: (width: number) => void;
}) {
  const drag = useRef<{ pointerId: number; startX: number; startWidth: number } | null>(null);
  // Only for the line's own intensified colour while dragging — `:hover` drops
  // the moment the cursor leaves the 8px hit area, which a fast drag does
  // almost at once, and the grip going dim mid-drag would read as let go.
  const [dragging, setDragging] = useState(false);

  function pointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, startX: event.clientX, startWidth: width };
    setDragging(true);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    // Suppressing the drag's own text selection also suppresses the focus a
    // click would otherwise grant — put back by hand, so the keyboard still
    // works right after a press finds the handle.
    event.currentTarget.focus();
    event.preventDefault();
  }

  function pointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current === null || drag.current.pointerId !== event.pointerId) return;
    const delta = drag.current.startX - event.clientX;
    onResize(clampDockWidth(drag.current.startWidth + delta, availableWidth, leftWidth));
  }

  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    const { min, max } = dockWidthBounds(availableWidth, leftWidth);
    if (event.key === "ArrowLeft") onResize(clampDockWidth(width + DOCK_WIDTH_STEP, availableWidth, leftWidth));
    else if (event.key === "ArrowRight") onResize(clampDockWidth(width - DOCK_WIDTH_STEP, availableWidth, leftWidth));
    else if (event.key === "Home") onResize(max);
    else if (event.key === "End") onResize(min);
    else return;
    event.preventDefault();
  }

  const { min, max } = dockWidthBounds(availableWidth, leftWidth);
  return (
    <div
      className="armada-shell__dock-handle"
      role="separator"
      aria-orientation="vertical"
      aria-label={`Resize ${DOCK_TITLE}`}
      aria-valuenow={Math.round(width)}
      aria-valuemin={Math.round(min)}
      aria-valuemax={Math.round(max)}
      data-dragging={dragging || undefined}
      tabIndex={0}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onKeyDown={keyDown}
    />
  );
}

/**
 * The left column's trailing-edge handle — one handle for Navigation, Stats
 * and Fleet together, never one per panel. Right widens the column, since it
 * sits on the window's leading edge and dragging toward the content is
 * dragging the edge that grows it; Home and End match a plain slider's own
 * sense, unlike the dock's inverted one, since this column is not mirrored
 * to the opposite edge the way the dock is.
 */
function LeftHandle({ width, onResize }: { width: number; onResize: (width: number) => void }) {
  const drag = useRef<{ pointerId: number; startX: number; startWidth: number } | null>(null);
  const [dragging, setDragging] = useState(false);

  function pointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, startX: event.clientX, startWidth: width };
    setDragging(true);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    event.currentTarget.focus();
    event.preventDefault();
  }

  function pointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current === null || drag.current.pointerId !== event.pointerId) return;
    const delta = event.clientX - drag.current.startX;
    onResize(clampLeftWidth(drag.current.startWidth + delta));
  }

  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    const { min, max } = leftWidthBounds();
    if (event.key === "ArrowRight") onResize(clampLeftWidth(width + LEFT_WIDTH_STEP));
    else if (event.key === "ArrowLeft") onResize(clampLeftWidth(width - LEFT_WIDTH_STEP));
    else if (event.key === "Home") onResize(min);
    else if (event.key === "End") onResize(max);
    else return;
    event.preventDefault();
  }

  const { min, max } = leftWidthBounds();
  return (
    <div
      className="armada-shell__left-handle"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the left column"
      aria-valuenow={Math.round(width)}
      aria-valuemin={Math.round(min)}
      aria-valuemax={Math.round(max)}
      data-dragging={dragging || undefined}
      tabIndex={0}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onKeyDown={keyDown}
    />
  );
}

function Dock({
  open,
  folded = false,
  questions = 0,
  binding,
  width,
  onResize,
  onOpen,
  action,
  children,
  leftWidth,
}: TheShellDock & { leftWidth: number }) {
  const availableWidth = useWindowWidth();
  const body = children ?? (
    <BoardEmptyState quiet>Questions waiting on you, and Helm, will be here.</BoardEmptyState>
  );

  if (open && !folded) {
    // Clamped before it reaches the handle, not after: the window can narrow
    // between one render and the next, and a `width` still carrying what fit
    // the old one would show the handle's own aria-valuenow, its drag anchor
    // and its arrow-key math a number the aside on screen already disagrees
    // with. Clamping once here keeps every reader of `restingWidth` — the
    // aside's style included — looking at what is actually drawn.
    const restingWidth = clampDockWidth(width ?? defaultDockWidth(), availableWidth, leftWidth);
    // **The handle rides on the layer with the dock, not in the flow beside
    // it.** Left behind in `__work` it would still be a 16px track, so the
    // content would move by that much on every open — the same defect as the
    // dock's own 380, one order of magnitude down and harder to see.
    return (
      <div className="armada-shell__dock-layer">
        {onResize === undefined ? null : (
          <DockHandle width={restingWidth} availableWidth={availableWidth} leftWidth={leftWidth} onResize={onResize} />
        )}
        <aside
          className="armada-shell__dock armada-glass"
          aria-label={DOCK_TITLE}
          style={width === undefined ? undefined : { width: `${restingWidth}px` }}
        >
          <div className="armada-shell__dock-head">
            <span className="armada-shell__dock-chip" aria-hidden><MessageSquare size={16} strokeWidth={2} /></span>
            <h2 className="armada-shell__dock-title">{DOCK_TITLE}</h2>
            {action}
            <Button variant="secondary" size="sm" ground="card" onClick={() => onOpen(false)}>
              Close
              {binding === undefined ? null : <KbdCmd shortcut={binding} />}
            </Button>
          </div>
          <div className="armada-shell__dock-body">{body}</div>
        </aside>
      </div>
    );
  }

  // Closed, at width: nothing. The title row's Helm button is the one way back — #1094.
  if (!folded) return null;

  const waiting = questions > 0 ? `, ${questions} ${questions === 1 ? "question" : "questions"} waiting` : "";
  return (
    <>
      <button
        type="button"
        className="armada-shell__strip"
        aria-label={`Open ${DOCK_TITLE}${waiting}`}
        aria-expanded={open}
        title={binding === undefined ? `Open ${DOCK_TITLE}` : `Open ${DOCK_TITLE} — ${binding}`}
        onClick={() => onOpen(true)}
      >
        <MessageSquare size={16} strokeWidth={2} aria-hidden />
        {questions > 0 ? <span className="armada-shell__strip-count">{questions}</span> : null}
      </button>
      <HelmSheet
        open={open && folded}
        title={DOCK_TITLE}
        binding={binding}
        controls={action}
        onClose={() => onOpen(false)}
      >
        {body}
      </HelmSheet>
    </>
  );
}

/**
 * Helm's title-row reopen control, from the same `dock` shape the edge strip
 * reads — so the caller states the dock once and both controls agree on it.
 *
 * **Absent whenever the full dock is showing beside the content.** Drawn
 * while folded (the strip's own state) and while closed outright — the latter
 * is the only way back once #1094 stopped drawing a strip there at all.
 */
function helmButtonOf(
  dock: TheShellDock | undefined,
): { questions: number; binding?: string; onOpen: () => void } | undefined {
  if (dock === undefined) return undefined;
  if (dock.open && dock.folded !== true) return undefined;
  return {
    questions: dock.questions ?? 0,
    ...(dock.binding === undefined ? {} : { binding: dock.binding }),
    onOpen: () => dock.onOpen(true),
  };
}
