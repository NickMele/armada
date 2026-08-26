import type { HTMLAttributes } from "react";

/**
 * ScrollArea — a bounded region that scrolls, with a scrollbar drawn from the
 * token set rather than from the platform.
 *
 * **Nothing in the sources draws one.** The contract names it once, as what
 * the command palette does past its max height, and the component sheet has no
 * scrolling region anywhere in it. So the scrollbar's own treatment — thumb,
 * track, width, whether it overlays or takes space — is decided here and needs
 * confirming. Reported.
 *
 * The frozen-Bridge failure this app was built to escape is a surface that
 * cannot show a diff, so a region that clips without a way to reach the rest
 * of its content is the failure mode, not the scrollbar's appearance.
 */
export type ScrollAreaProps = HTMLAttributes<HTMLDivElement> & {
  /** Where the region stops growing and starts scrolling, as a CSS length. */
  maxHeight?: string;
  /** Horizontal scrolling is off by default: a data column that scrolls
   * sideways hides fields, and no field is dropped at any width. */
  axis?: "vertical" | "both";
};

export function ScrollArea({
  className,
  maxHeight,
  axis = "vertical",
  style,
  ...rest
}: ScrollAreaProps) {
  return (
    <div
      className={className ? `armada-scroll-area ${className}` : "armada-scroll-area"}
      data-axis={axis}
      style={maxHeight ? { ...style, maxHeight } : style}
      {...rest}
    />
  );
}
