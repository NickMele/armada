import type { ReactNode } from "react";
import { Sidebar, type SidebarItem } from "../../compositions/Sidebar/Sidebar";
import { StatusBar, type StatusBarProps } from "../../compositions/StatusBar/StatusBar";

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
  /** The app name, in the drag region the traffic lights sit in. */
  appName?: ReactNode;
  /** Beneath the app name — the Manifest picker, on M1's shell. */
  railHeader?: ReactNode;
  surfaces: SidebarItem[];
  activeId?: string;
  /** The 48px icon rail. The surface decides, from the window's width. */
  collapsed?: boolean;
  onSelect?: (id: string) => void;
  /** The panel's own name. `Active jobs`. */
  title: ReactNode;
  /** The sentence under it — the counts, or what this view does. */
  summary?: ReactNode;
  /** The controls at the head's trailing edge. `New job` is the primary. */
  actions?: ReactNode;
  /** The panel body. The one region that scrolls. */
  children: ReactNode;
  status: StatusBarProps;
};

export function TheShell({
  appName = "Armada",
  railHeader,
  surfaces,
  activeId,
  collapsed,
  onSelect,
  title,
  summary,
  actions,
  children,
  status,
}: TheShellProps) {
  return (
    <div className="armada-shell">
      <div className="armada-shell__body">
        <Sidebar
          appName={appName}
          header={railHeader}
          surfaces={surfaces}
          activeId={activeId}
          collapsed={collapsed}
          onSelect={onSelect}
        />
        <div className="armada-shell__panel">
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
          <div className="armada-shell__mount">{children}</div>
        </div>
      </div>
      <StatusBar {...status} />
    </div>
  );
}
