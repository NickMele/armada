import { forwardRef, type SVGProps } from 'react'

/**
 * The Armada Countersign mark.
 *
 * Drawn on the same 24-unit grid and 2-unit stroke as lucide-react, so it sits
 * beside the icon set without restyling. Two rules are deliberately not props:
 *
 *   strokeWidth  — fixed at 2. The Design System forbids per-size tuning.
 *   linecap      — butt, not round. Round caps overhang each endpoint by a full
 *                  unit, which closes the 2-unit clearance the mark is built on.
 *
 * Colour comes from `currentColor`, as everywhere else in the app.
 */

export interface ArmadaMarkProps extends Omit<SVGProps<SVGSVGElement>, 'width' | 'height'> {
  /** Rendered edge length in px. Use 16 in chrome, 12 only alongside badge text. */
  size?: number
}

export const ArmadaMark = forwardRef<SVGSVGElement, ArmadaMarkProps>(
  ({ size = 16, ...props }, ref) => (
    <svg
      ref={ref}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      role="img"
      aria-label="Armada"
      {...props}
    >
      {/* the claim — three sides, unclosed */}
      <path
        d="M14 9 V4 H4 V14 H9"
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="butt"
        strokeLinejoin="miter"
      />
      {/* the countersign — a second act, held clear by exactly 2 */}
      <path d="M11 11 H21 V21 H11 Z" fill="currentColor" />
    </svg>
  ),
)

ArmadaMark.displayName = 'ArmadaMark'

/**
 * Two-tone variant. The claim recedes to --fg-muted, the countersign holds
 * --fg-default: asserted against evidenced, stated without a second hue.
 * Only for sizes >= 24 — below that the muted stroke drops under the contrast
 * floor and the mark reads as a single object.
 */
export const ArmadaMarkTwoTone = forwardRef<SVGSVGElement, ArmadaMarkProps>(
  ({ size = 32, ...props }, ref) => (
    <svg
      ref={ref}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      role="img"
      aria-label="Armada"
      {...props}
    >
      <path
        d="M14 9 V4 H4 V14 H9"
        fill="none"
        stroke="var(--fg-muted)"
        strokeWidth={2}
        strokeLinecap="butt"
        strokeLinejoin="miter"
      />
      <path d="M11 11 H21 V21 H11 Z" fill="var(--fg-default)" />
    </svg>
  ),
)

ArmadaMarkTwoTone.displayName = 'ArmadaMarkTwoTone'
