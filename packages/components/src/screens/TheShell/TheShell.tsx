import { ArmadaLockupHorizontal, ArmadaMark } from "@armada/brand";
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
  status: StatusBarProps;
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
      </div>
      <StatusBar {...status} />
    </div>
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
