import type { HTMLAttributes } from "react";

/**
 * Card — elevation is surface, not shadow. `--bg-raised` with a
 * `--border-default` edge and no blur. Shadows are legal on floating layers
 * only, and a card is not one.
 *
 * The card owns its padding, so the parts below add none. That is what keeps
 * a header, a body and a footer sharing one left edge.
 */
export type CardProps = HTMLAttributes<HTMLDivElement>;

function joined(base: string, extra?: string) {
  return extra ? `${base} ${extra}` : base;
}

export function Card({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card", className)} {...rest} />;
}

/** A row: the label of the thing on the left, a status badge on the right. */
export function CardHeader({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card-header", className)} {...rest} />;
}

/** Panel heading step. Sentence case, like everything else. */
export function CardTitle({ className, ...rest }: HTMLAttributes<HTMLHeadingElement>) {
  return <h3 className={joined("armada-card-title", className)} {...rest} />;
}

/** Secondary prose beneath the title, at body default in `--fg-muted`. */
export function CardDescription({ className, ...rest }: HTMLAttributes<HTMLParagraphElement>) {
  return <p className={joined("armada-card-description", className)} {...rest} />;
}

export function CardContent({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card-content", className)} {...rest} />;
}

/** Actions. Every button in the group is the same height. */
export function CardFooter({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card-footer", className)} {...rest} />;
}
