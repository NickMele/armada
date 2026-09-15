import { MessageSquare } from "lucide-react";
import { useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { BoardEmptyState } from "../../compositions/BoardEmptyState/BoardEmptyState";
import { FleetPanel, type FleetPanelProps } from "../../compositions/FleetPanel/FleetPanel";
import { Sidebar, type SidebarItem } from "../../compositions/Sidebar/Sidebar";
import { StatsPanel, type StatsPanelProps } from "../../compositions/StatsPanel/StatsPanel";
import { TitleBar } from "../../compositions/TitleBar/TitleBar";
import { Button } from "../../primitives/Button/Button";
import { Kbd, KbdChord } from "../../primitives/Kbd/Kbd";
import { Sheet } from "../../primitives/Sheet/Sheet";

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
  /** The 48px icon rail. The surface decides, from the window's width. */
  collapsed?: boolean;
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
};

/**
 * Helm's dock (#948). **Closed draws nothing at all beyond the layout
 * breakpoint** — #1094 dropped the edge strip that used to sit there at any
 * width, since the title row's own Helm button (#1087) is already the way
 * back and a strip that says the same thing a second time is residue, not a
 * second door. Folded still keeps its strip: under the breakpoint the title
 * row has no room to carry the button, so the strip is the only way in.
 */
export type TheShellDock = {
  /** Beside the content, or the sheet when folded. */
  open: boolean;
  /** Below `--layout-breakpoint`. A prop, because a media query cannot read the token. */
  folded?: boolean;
  /** Questions waiting on the person. Zero draws no count. */
  questions?: number;
  /** The binding, beside the close and in the strip's tooltip. */
  binding?: string;
  onOpen: (open: boolean) => void;
  /**
   * The resting width in px. Absent draws `--w-dock`. Read while beside the
   * content; a folded dock is the `Sheet`'s own width, never a drag.
   */
  width?: number;
  /**
   * Drags and arrow-key nudges the leading-edge handle, clamped to
   * `--w-dock-min`/`--w-dock-max`. **Absent draws no handle at all** — an
   * edge that looks grabbable and does nothing is worse than no edge.
   * Persisting the result across a restart is the caller's, the same way
   * `open` is.
   */
  onResize?: (width: number) => void;
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
  onSelect,
  children,
  dock,
  stats,
  fleet,
}: TheShellProps) {
  return (
    <div className="armada-shell">
      <TitleBar
        repositoryPicker={repositoryPicker}
        onSearch={onSearch}
        onDispatch={onDispatch}
        dispatchDisabled={dispatchDisabled}
        helm={helmButtonOf(dock)}
      />
      <div className="armada-shell__body">
        <div className="armada-shell__left">
          <Sidebar
            header={railHeader}
            sectionLabel={null}
            surfaces={surfaces}
            activeId={activeId}
            collapsed={collapsed}
            onSelect={onSelect}
          />
          <StatsPanel {...stats} narrow={collapsed} />
          <FleetPanel {...fleet} narrow={collapsed} />
        </div>
        <div className="armada-shell__work">
          <div className="armada-shell__panel">
            <div className="armada-shell__mount">{children}</div>
          </div>
          {dock === undefined ? null : <Dock {...dock} />}
        </div>
      </div>
    </div>
  );
}

const DOCK_TITLE = "Helm";

// Fallbacks only for a caller with no stylesheet loaded (a bare unit test);
// the tokens are the real source and are read fresh on every drag.
const DOCK_WIDTH_MIN_FALLBACK = 320;
const DOCK_WIDTH_MAX_FALLBACK = 640;
const DOCK_WIDTH_DEFAULT_FALLBACK = 380;

function dockWidthBounds(): { min: number; max: number } {
  if (typeof document === "undefined") {
    return { min: DOCK_WIDTH_MIN_FALLBACK, max: DOCK_WIDTH_MAX_FALLBACK };
  }
  const style = getComputedStyle(document.documentElement);
  const min = parseFloat(style.getPropertyValue("--w-dock-min"));
  const max = parseFloat(style.getPropertyValue("--w-dock-max"));
  return {
    min: Number.isFinite(min) ? min : DOCK_WIDTH_MIN_FALLBACK,
    max: Number.isFinite(max) ? max : DOCK_WIDTH_MAX_FALLBACK,
  };
}

/** The dock's own resting width, in px, for a caller with none of its own to remember yet. */
export function defaultDockWidth(): number {
  if (typeof document === "undefined") return DOCK_WIDTH_DEFAULT_FALLBACK;
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--w-dock"));
  return Number.isFinite(value) ? value : DOCK_WIDTH_DEFAULT_FALLBACK;
}

/**
 * Clamped to the drag range `--w-dock-min`/`--w-dock-max` names, so a width
 * read back from storage — stale, or from a build that carried different
 * tokens — never draws past what the tokens allow today.
 */
export function clampDockWidth(width: number): number {
  const { min, max } = dockWidthBounds();
  return Math.min(max, Math.max(min, width));
}

/** One `--space-4` per arrow press — the same step the dock's own padding uses. */
const DOCK_WIDTH_STEP = 16;

/**
 * The dock's leading-edge handle. A drag or an arrow key moves it; both read
 * the same clamp so neither can push the dock past what a mouse could reach.
 *
 * **Left widens the dock, right narrows it** — the dock sits on the window's
 * trailing edge, so dragging toward the content is dragging the edge that
 * grows it, the same direction a mouse drag moves. Home and End match: Home
 * (the leftmost position a splitter can take) is the widest the dock gets.
 */
function DockHandle({ width, onResize }: { width: number; onResize: (width: number) => void }) {
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
    onResize(clampDockWidth(drag.current.startWidth + delta));
  }

  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    const { min, max } = dockWidthBounds();
    if (event.key === "ArrowLeft") onResize(clampDockWidth(width + DOCK_WIDTH_STEP));
    else if (event.key === "ArrowRight") onResize(clampDockWidth(width - DOCK_WIDTH_STEP));
    else if (event.key === "Home") onResize(max);
    else if (event.key === "End") onResize(min);
    else return;
    event.preventDefault();
  }

  const { min, max } = dockWidthBounds();
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

function Dock({ open, folded = false, questions = 0, binding, width, onResize, onOpen, children }: TheShellDock) {
  const body = children ?? (
    <BoardEmptyState quiet>Questions waiting on you, and Helm, will be here.</BoardEmptyState>
  );

  if (open && !folded) {
    const restingWidth = width ?? defaultDockWidth();
    return (
      <>
        {onResize === undefined ? null : <DockHandle width={restingWidth} onResize={onResize} />}
        <aside
          className="armada-shell__dock"
          aria-label={DOCK_TITLE}
          style={width === undefined ? undefined : { width: `${clampDockWidth(width)}px` }}
        >
          <div className="armada-shell__dock-head">
            <h2 className="armada-shell__dock-title">{DOCK_TITLE}</h2>
            <Button variant="secondary" size="sm" ground="sunken" onClick={() => onOpen(false)}>
              Close
              {binding === undefined ? null : <Chord binding={binding} />}
            </Button>
          </div>
          <div className="armada-shell__dock-body">{body}</div>
        </aside>
      </>
    );
  }

  // Closed, at width: nothing. The title row's Helm button is the one way
  // back — #1094.
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
      <Sheet
        open={open && folded}
        title={DOCK_TITLE}
        contained
        closeLabel="Close"
        closeBinding={binding}
        onClose={() => onOpen(false)}
      >
        {body}
      </Sheet>
    </>
  );
}

/** One cap per key, as `KbdChord` draws a chord. */
function Chord({ binding }: { binding: string }) {
  return (
    <KbdChord>
      {Array.from(binding).map((key, at) => (
        <Kbd key={at}>{key}</Kbd>
      ))}
    </KbdChord>
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
