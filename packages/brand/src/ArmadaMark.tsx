/**
 * Armada brand mark — three hulls on a shallow arc.
 *
 * The API is `size` plus colour by inheritance. There is deliberately no
 * strokeWidth or strokeLinecap prop: the mark is a filled construction with no
 * stroke at all, and per-size tuning is not permitted by the design contract.
 */
import * as React from "react";

const HULLS = [
  { s: 0.46, x: 1.2, y: 30.0 },
  { s: 0.46, x: 19.6, y: 11.6 },
  { s: 0.46, x: 38.0, y: 25.4 },
] as const;

const HULL =
  "M32 4 C 40 17, 50 35, 53 44 L36 46 C 34 34, 33 24, 31 18 C 28 25, 24 38, 19 52 L2 54 C 11 36, 24 15, 32 4 Z";

// bbox of the composed mark, in authoring units
const BX = 2.1200, BY = 13.4400, BW = 60.2600, BH = 41.4000;
const K = 24 / BW;
const OY = (24 - BH * K) / 2;

const transformFor = (h: { s: number; x: number; y: number }) =>
  `translate(${((h.x - BX) * K).toFixed(4)},${(OY + (h.y - BY) * K).toFixed(4)}) scale(${(h.s * K).toFixed(6)})`;

export interface ArmadaMarkProps
  extends Omit<React.SVGProps<SVGSVGElement>, "children"> {
  /** Rendered width and height in pixels. Defaults to 24. Floor is 16. */
  size?: number | string;
  /** Accessible label. Omit for a decorative mark. */
  title?: string;
}

export const ArmadaMark = React.forwardRef<SVGSVGElement, ArmadaMarkProps>(
  function ArmadaMark({ size = 24, title, ...rest }, ref) {
    return (
      <svg
        ref={ref}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        width={size}
        height={size}
        fill="currentColor"
        role={title ? "img" : undefined}
        aria-hidden={title ? undefined : true}
        {...rest}
      >
        {title ? <title>{title}</title> : null}
        {HULLS.map((h, i) => (
          <path key={i} transform={transformFor(h)} d={HULL} />
        ))}
      </svg>
    );
  }
);

/**
 * Two-tone variant. The lead hull carries `currentColor`; the two following
 * hulls are held back. Never below 32px — the distinction stops reading.
 */
export interface ArmadaMarkDuoProps extends ArmadaMarkProps {
  /** Opacity of the two trailing hulls. Defaults to 0.45. */
  trailOpacity?: number;
}

export const ArmadaMarkDuo = React.forwardRef<SVGSVGElement, ArmadaMarkDuoProps>(
  function ArmadaMarkDuo({ size = 32, title, trailOpacity = 0.45, ...rest }, ref) {
    return (
      <svg
        ref={ref}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        width={size}
        height={size}
        fill="currentColor"
        role={title ? "img" : undefined}
        aria-hidden={title ? undefined : true}
        {...rest}
      >
        {title ? <title>{title}</title> : null}
        {HULLS.map((h, i) => (
          <path
            key={i}
            opacity={i === 1 ? 1 : trailOpacity}
            transform={transformFor(h)}
            d={HULL}
          />
        ))}
      </svg>
    );
  }
);

export default ArmadaMark;
