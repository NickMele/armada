import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Separator } from "../../primitives/Separator/Separator";

/**
 * Sidebar — Bridge's navigation rail.
 *
 * **Two levels, rendered structurally.** Bridge is a section label above its
 * surfaces; a rule, then Helm as a sibling beneath — not one more peer in a
 * flat list. That is the app/surface-group hierarchy made visible, and a flat
 * nav quietly contradicts it.
 *
 * **The rail never disappears.** 48px is cheap and losing navigation entirely
 * is worse than losing 48px, at any width. Below the breakpoint it collapses
 * to that rail; the ⌘-digit bindings reach every surface without labels, which
 * is what makes the collapsed state more usable than it looks.
 *
 * **The roster is Bridge's, never this component's.** A surface earns a place
 * where a journey needs one, so the list arrives as a prop.
 *
 * The header is a drag region: frameless `hiddenInset` chrome insets the macOS
 * traffic lights over the sidebar's top, which reclaims vertical space and
 * costs a custom drag region.
 */
export type SidebarItem = {
  id: string;
  label: string;
  /** 16px in navigation, `--fg-muted` at rest and `--fg-default` when active. */
  icon: LucideIcon;
  /**
   * A count beside the label. **Never an escalation or approval count** — the
   * status bar already carries both on every surface, and duplicating them
   * here creates two places to check and two chances to disagree.
   */
  count?: number;
};

export type SidebarProps = {
  /** Bridge's surfaces, in rail order. ⌘1…⌘n follow this order. */
  surfaces: SidebarItem[];
  /** Helm, the sibling beneath the rule. ⌘ the digit after the last surface. */
  sibling?: SidebarItem;
  /** The label above the surfaces. "Bridge". */
  sectionLabel?: ReactNode;
  activeId?: string;
  /** The app name, and the drag region the traffic lights sit in. */
  appName?: ReactNode;
  /** Anything beneath the app name — the scope picker on M1's shell. */
  header?: ReactNode;
  /** 48px icon rail. Auto below the breakpoint, and toggled by ⌘\ above it. */
  collapsed?: boolean;
  /**
   * A resting width inside the 160–320px drag range, as a CSS length. Ignored
   * when collapsed. Width and collapsed state survive app restart, which is
   * the surface's to persist rather than this component's.
   */
  width?: string;
  onSelect?: (id: string) => void;
};

/** Navigation glyphs are 16px at strokeWidth 2. Never 11, 14, 18 or 20. */
const NAV_ICON = 16;
const NAV_STROKE = 2;

function Item({
  item,
  active,
  collapsed,
  onSelect,
}: {
  item: SidebarItem;
  active: boolean;
  collapsed: boolean;
  onSelect?: (id: string) => void;
}) {
  return (
    <button
      type="button"
      className="armada-sidebar__item"
      data-active={active || undefined}
      aria-current={active ? "page" : undefined}
      // The label is the accessible name in both states; collapsing hides it
      // visually and nothing else.
      aria-label={collapsed ? item.label : undefined}
      onClick={() => onSelect?.(item.id)}
    >
      <item.icon size={NAV_ICON} strokeWidth={NAV_STROKE} aria-hidden />
      {collapsed ? null : <span className="armada-sidebar__label">{item.label}</span>}
      {!collapsed && item.count !== undefined ? (
        <span className="armada-sidebar__count">{item.count}</span>
      ) : null}
    </button>
  );
}

export function Sidebar({
  surfaces,
  sibling,
  sectionLabel = "Bridge",
  activeId,
  appName,
  header,
  collapsed = false,
  width,
  onSelect,
}: SidebarProps) {
  return (
    <nav
      className="armada-sidebar"
      data-collapsed={collapsed || undefined}
      style={{ width: collapsed ? "var(--sidebar-rail)" : (width ?? "var(--sidebar-default)") }}
    >
      {appName ? <div className="armada-sidebar__chrome">{appName}</div> : null}
      {!collapsed && header ? <div className="armada-sidebar__header">{header}</div> : null}

      {!collapsed && sectionLabel ? (
        <div className="armada-sidebar__section">{sectionLabel}</div>
      ) : null}

      <div className="armada-sidebar__group">
        {surfaces.map((item) => (
          <Item
            key={item.id}
            item={item}
            active={item.id === activeId}
            collapsed={collapsed}
            onSelect={onSelect}
          />
        ))}
      </div>

      {sibling ? (
        <>
          {/* The rule is the only thing stating the boundary between a surface
              group and a sibling app, so it keeps its role. */}
          <Separator decorative={false} className="armada-sidebar__rule" />
          <div className="armada-sidebar__group">
            <Item
              item={sibling}
              active={sibling.id === activeId}
              collapsed={collapsed}
              onSelect={onSelect}
            />
          </div>
        </>
      ) : null}
    </nav>
  );
}
