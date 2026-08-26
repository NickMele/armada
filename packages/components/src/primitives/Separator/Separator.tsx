import type { HTMLAttributes } from "react";

/**
 * Separator — a 1px `--border-subtle` rule. The dividers in a dropdown menu,
 * the rule between Bridge's surfaces and Helm in the sidebar, the edge above a
 * card's field run.
 *
 * It carries no margin. Spacing between a rule and what it separates belongs
 * to the thing doing the separating: a dropdown sets `--space-1`, a sidebar
 * sets more, and a default here would be wrong in one of them.
 */
export type SeparatorProps = HTMLAttributes<HTMLDivElement> & {
  orientation?: "horizontal" | "vertical";
  /**
   * A rule that only reinforces a boundary already stated by layout is
   * decorative and takes no role, so a screen reader does not announce it.
   * A rule that is the only thing stating the boundary keeps `separator`.
   */
  decorative?: boolean;
};

export function Separator({
  className,
  orientation = "horizontal",
  decorative = true,
  ...rest
}: SeparatorProps) {
  return (
    <div
      className={className ? `armada-separator ${className}` : "armada-separator"}
      data-orientation={orientation}
      role={decorative ? "none" : "separator"}
      aria-orientation={decorative ? undefined : orientation}
      {...rest}
    />
  );
}
