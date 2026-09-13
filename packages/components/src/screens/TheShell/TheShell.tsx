import { ArmadaLockupHorizontal, ArmadaMark } from "@armada/brand";
import { MessageSquare } from "lucide-react";
import type { ReactNode } from "react";
import { BoardEmptyState } from "../../compositions/BoardEmptyState/BoardEmptyState";
import { Sidebar, type SidebarItem } from "../../compositions/Sidebar/Sidebar";
import { StatusBar, type StatusBarProps } from "../../compositions/StatusBar/StatusBar";
import { Button } from "../../primitives/Button/Button";
import { Kbd, KbdChord } from "../../primitives/Kbd/Kbd";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * The shell — rail, panel, status bar. The frame every journey mounts inside.
 *
 * **The status bar spans beneath the rail, not inside the panel.** The drawing
 * insets it to the content area; `docs/contracts/design-system.md` says the
 * opposite in as many words, on the grounds that the bar is app-level and
 * appears on Helm too. The contract wins and the disagreement is reported.
 *
 * **The roster is the caller's.** One surface is in the rail because one
 * surface exists — six disabled rows would be a promise Armada does not keep —
 * so the surfaces arrive as a prop and this component counts nothing.
 *
 * The panel scrolls; the rail and the bar do not. `min-height: 0` on the
 * scrolling child is what stops the window growing instead, which is v1's
 * "layout broke on resize" restated in CSS.
 */
export type TheShellProps = {
  /**
   * The app name, in the drag region the traffic lights sit in. **Absent
   * draws Armada's own**: the horizontal lockup, or the mark alone when the
   * rail is collapsed.
   */
  appName?: ReactNode;
  /** Beneath the app name — the Manifest picker, on M1's shell. */
  railHeader?: ReactNode;
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
  /** The controls at the head's trailing edge. `New job` is the primary. */
  actions?: ReactNode;
  /** The panel body. The one region that scrolls. */
  children: ReactNode;
  /** Helm's dock, on every surface. Absent draws none. */
  dock?: TheShellDock;
  status: StatusBarProps;
};

/** Helm's dock (#948). Closed draws the edge strip at any width, so both arrangements share one way back. */
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
  appName,
  railHeader,
  surfaces,
  activeId,
  collapsed,
  onSelect,
  title,
  summary,
  actions,
  children,
  dock,
  status,
}: TheShellProps) {
  return (
    <div className="armada-shell">
      <div className="armada-shell__body">
        <Sidebar
          appName={appName ?? brandOf(collapsed === true)}
          header={railHeader}
          surfaces={surfaces}
          activeId={activeId}
          collapsed={collapsed}
          onSelect={onSelect}
        />
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
      <StatusBar {...status} />
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
 * Armada's name in the drag region: the horizontal lockup, or the mark alone
 * when the rail collapses, which the owner asked for on 11 Sep 2026 after the
 * collapsed rail drew the wordmark clipped.
 *
 * **Decided here, not by the caller.** `Shell` used to choose, and this
 * component's own default was the word "Armada", so every story drew text the
 * app never shows. `packages/brand/README.md` floors the lockup at a 20px cap
 * height, 26px overall; a 48px rail has no room for it, and a clipped lockup
 * is nothing where the mark alone is still the app's name.
 *
 * `title` gives either its accessible name; without one both render
 * `aria-hidden`. Nothing here is clickable: the drag region cannot hold
 * anything interactive.
 */
function brandOf(collapsed: boolean): ReactNode {
  return collapsed ? (
    <ArmadaMark size={20} title="Armada" />
  ) : (
    <ArmadaLockupHorizontal height={26} title="Armada" />
  );
}
