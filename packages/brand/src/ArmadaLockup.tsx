import type { HTMLAttributes } from 'react'
import { ArmadaMark } from './ArmadaMark'

/**
 * Mark plus wordmark, set live in IBM Plex Sans rather than as a flat asset so
 * it inherits colour and stays crisp at any size.
 *
 * Geometry is expressed in mark-units (u = size / 24), the same unit the mark
 * is constructed in:
 *
 *   cap height   13u
 *   gap          5u from the drawn mark's right edge (21u) to the first glyph
 *   baseline     centred on the mark's optical centre (12u)
 *
 * For print, export, or anywhere IBM Plex Sans may not load, use the outlined
 * SVG at brand/svg/armada-lockup-horizontal.svg instead.
 */

export interface ArmadaLockupProps extends HTMLAttributes<HTMLSpanElement> {
  /** Mark box edge length in px. The wordmark scales from it. */
  size?: number
  orientation?: 'horizontal' | 'stacked'
}

export function ArmadaLockup({
  size = 32,
  orientation = 'horizontal',
  style,
  ...props
}: ArmadaLockupProps) {
  const u = size / 24

  if (orientation === 'stacked') {
    return (
      <span
        {...props}
        style={{
          display: 'inline-flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 4 * u,
          color: 'inherit',
          ...style,
        }}
      >
        <ArmadaMark size={size} />
        <span
          style={{
            fontFamily: '"IBM Plex Sans", system-ui, sans-serif',
            fontWeight: 500,
            fontSize: 8 * u / 0.698,
            lineHeight: 1,
            letterSpacing: '0.18em',
            textIndent: '0.18em',
            textTransform: 'uppercase',
          }}
        >
          Armada
        </span>
      </span>
    )
  }

  return (
    <span
      {...props}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 2 * u, // 5u from the drawn edge, less the mark box's own 3u padding
        color: 'inherit',
        ...style,
      }}
    >
      <ArmadaMark size={size} />
      <span
        style={{
          fontFamily: '"IBM Plex Sans", system-ui, sans-serif',
          fontWeight: 600,
          fontSize: 13 * u / 0.698, // 13u cap height
          lineHeight: 1,
          letterSpacing: '-0.03em',
        }}
      >
        Armada
      </span>
    </span>
  )
}
