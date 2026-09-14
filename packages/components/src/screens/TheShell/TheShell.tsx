import { MessageSquare } from "lucide-react";
import type { ReactNode } from "react";
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
  /**
   * The panel's own name. `Active jobs`.
   *
   * **Absent draws no head at all**, which is a view whose content names
   * itself: one Job read whole carries its badge, its title and its id in its
   * own header, and a bar above that naming the surface behind it spends the
   * top of the window to say the one thing the reader is not looking for.
   * Every other view here is named by nothing else and passes one.
   */
  title?: ReactNode;
  /** The sentence under it — the counts, or what this view does. */
  summary?: ReactNode;
  /** The controls at the head's trailing edge. `Dispatch` is the primary. */
  actions?: ReactNode;
  /** The panel body. The one region that scrolls. */
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
  title,
  summary,
  actions,
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
            {title === undefined ? null : (
            <div className="armada-screen__panel-head">
              <div className="armada-screen__titles">
                <span className="armada-screen__title">{title}</span>
                {summary === undefined ? null : (
                  <span className="armada-screen__summary">{summary}</span>
                )}
              </div>
              {actions === undefined ? null : (
                <div className="armada-shell__actions">{actions}</div>
              )}
            </div>
            )}
            <div className="armada-shell__mount">{children}</div>
          </div>
          {dock === undefined ? null : <Dock {...dock} />}
        </div>
      </div>
    </div>
  );
}

const DOCK_TITLE = "Helm";

function Dock({ open, folded = false, questions = 0, binding, onOpen, children }: TheShellDock) {
  const body = children ?? (
    <BoardEmptyState quiet>Questions waiting on you, and Helm, will be here.</BoardEmptyState>
  );

  if (open && !folded) {
    return (
      <aside className="armada-shell__dock" aria-label={DOCK_TITLE}>
        <div className="armada-shell__dock-head">
          <h2 className="armada-shell__dock-title">{DOCK_TITLE}</h2>
          <Button variant="secondary" size="sm" ground="sunken" onClick={() => onOpen(false)}>
            Close
            {binding === undefined ? null : <Chord binding={binding} />}
          </Button>
        </div>
        <div className="armada-shell__dock-body">{body}</div>
      </aside>
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
