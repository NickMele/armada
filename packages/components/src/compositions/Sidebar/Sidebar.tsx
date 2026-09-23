import { PanelLeftClose, PanelLeftOpen, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { KbdCmd } from "../../primitives/Kbd/Kbd";
import { useShortcutReveal } from "../../shortcut-reveal";

/**
 * Sidebar — Bridge's navigation rail.
 *
 * **One level, Bridge's own.** It lists Bridge's surfaces and nothing else.
 * Helm left it for the dock (#948), so there is no second tier beneath the
 * surfaces, and no rule to draw one. `design-system.md` → Left column.
 *
 * **The rail never disappears.** 48px is cheap and losing navigation entirely
 * is worse than losing 48px, at any width. Below the breakpoint it collapses
 * to that rail; the ⌘-digit bindings reach every surface without labels, which
 * is what makes the collapsed state more usable than it looks.
 *
 * **A person can ask for the rail, too** — #1591. The toggle at the top of
 * this panel is `toggle_sidebar` built, and there is still one collapsed
 * state rather than two: it reaches the same 48px the breakpoint does.
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
  /**
   * 16px, in a 28px chip — `--accent-hover` on `--accent-faint` at rest,
   * `--fg-inverse` on solid `--accent` when active. One tint for every
   * section, never a hue per section. `iconography.md` → Navigation.
   */
  icon: LucideIcon;
  /**
   * A count beside the label. **Never an escalation or approval count** — the
   * status bar already carries both on every surface, and duplicating them
   * here creates two places to check and two chances to disagree.
   */
  count?: number;
  /**
   * The `⌘`-digit that reaches this row, `bridge_surfaces`'s own binding —
   * `surfaces.ts`'s `digitOf`, never retyped. Drawn only while Cmd is held
   * (`useShortcutReveal()`) and only past the collapsed rail, which has no
   * room for a badge beside a bare glyph and needs none: the digit already
   * reaches every surface without a label to read.
   */
  shortcut?: string;
};

export type SidebarProps = {
  /** Bridge's surfaces, in rail order. ⌘1…⌘n follow this order. */
  surfaces: SidebarItem[];
  /** The label above the surfaces. "Bridge". */
  sectionLabel?: ReactNode;
  activeId?: string;
  /** The app name, and the drag region the traffic lights sit in. */
  appName?: ReactNode;
  /** Anything beneath the app name — the scope picker on M1's shell. */
  header?: ReactNode;
  /** 48px icon rail. Auto below the breakpoint, and asked for by ⌘\ above it. */
  collapsed?: boolean;
  /**
   * Collapses the column to its rail, and brings it back — the press half of
   * `collapsed`, which stays controlled. **Absent draws no control at all**,
   * the shell's own handle reasoning: a button that looks pressable and does
   * nothing is worse than none. That is what the shell passes below
   * `--layout-breakpoint`, where the rail is the only width there is and
   * there is no choice to offer.
   */
  onCollapsedChange?: (collapsed: boolean) => void;
  /** `toggle_sidebar`'s binding, in the control's tooltip. Read from the registry, never retyped. */
  collapseBinding?: string;
  /**
   * A CSS length: a resting width inside the 160–320px drag range, or `100%`
   * where a column already holds the width, which is what the shell passes.
   * Ignored when collapsed. Width and collapsed state survive app restart, which is
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
  const revealing = useShortcutReveal();
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
      {/* The chip is the column's, not the glyph's: the palette draws the same
          registry icons bare. The button around it stays the click target, so
          the rail's hit area is the whole row rather than the 28px chip. */}
      <span className="armada-sidebar__chip" aria-hidden>
        <item.icon size={NAV_ICON} strokeWidth={NAV_STROKE} />
      </span>
      {collapsed ? null : <span className="armada-sidebar__label">{item.label}</span>}
      {!collapsed && item.count !== undefined ? (
        <span className="armada-sidebar__count">{item.count}</span>
      ) : null}
      {/* Mounted whenever there is a binding to show, whatever `revealing` is,
          so the row's own width does not shift the moment Cmd goes down —
          `data-revealing` toggles `visibility` in `Sidebar.css`, the same
          shape `JobRowStacked`'s own held-key `visibility` rule uses. */}
      {!collapsed && item.shortcut !== undefined ? (
        <span className="armada-sidebar__kbd" data-revealing={revealing || undefined}>
          <KbdCmd shortcut={item.shortcut} />
        </span>
      ) : null}
    </button>
  );
}

/**
 * The column's own collapse control — #1591, `toggle_sidebar` built.
 *
 * **It says which state it is in**, in three channels rather than one:
 * `panel-left-close` while the column is open and `panel-left-open` while it
 * is at the rail, `aria-expanded` for a reader, and the verb in the tooltip
 * and the accessible name. The registry carries both glyphs as a pair; a
 * single mark for both states makes a person press it to find out which way
 * it goes, which at 48px is the only chrome they have left.
 *
 * `title` rather than the `Tooltip` primitive, following
 * `.armada-shell__strip` — the same one-line hint with the same binding in
 * it, and one control's hover is not worth a second layer in the column.
 */
function CollapseToggle({
  collapsed,
  binding,
  onCollapsedChange,
}: {
  collapsed: boolean;
  binding?: string;
  onCollapsedChange: (collapsed: boolean) => void;
}) {
  const said = collapsed ? "Expand the left column" : "Collapse the left column";
  const Glyph = collapsed ? PanelLeftOpen : PanelLeftClose;
  return (
    <button
      type="button"
      className="armada-sidebar__collapse"
      aria-label={said}
      aria-expanded={!collapsed}
      title={binding === undefined ? said : `${said} — ${binding}`}
      onClick={() => onCollapsedChange(!collapsed)}
    >
      <Glyph size={NAV_ICON} strokeWidth={NAV_STROKE} aria-hidden />
    </button>
  );
}

export function Sidebar({
  surfaces,
  sectionLabel = "Bridge",
  activeId,
  appName,
  header,
  collapsed = false,
  width,
  onSelect,
  onCollapsedChange,
  collapseBinding,
}: SidebarProps) {
  return (
    <nav
      className="armada-sidebar armada-glass"
      data-collapsed={collapsed || undefined}
      style={{ width: collapsed ? "var(--sidebar-rail)" : (width ?? "var(--sidebar-default)") }}
    >
      {appName ? <div className="armada-sidebar__chrome">{appName}</div> : null}
      {/* Above the surfaces, at the column's trailing edge open and centred at
          the rail — the one place that is the same place in both states. */}
      {onCollapsedChange === undefined ? null : (
        <div className="armada-sidebar__tools">
          <CollapseToggle
            collapsed={collapsed}
            {...(collapseBinding === undefined ? {} : { binding: collapseBinding })}
            onCollapsedChange={onCollapsedChange}
          />
        </div>
      )}
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
    </nav>
  );
}
