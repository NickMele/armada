// Draws the macOS app icon and renders its iconset.
//
// The icon is Bridge's own canvas at icon scale: --bg-base under the accent
// pool at the top leading corner, a glass rim, and the mark in ink. Every
// value below is a token from packages/tokens/src, quoted here because an SVG
// master cannot read a CSS custom property. The mark itself stays one flat
// fill in one colour — the light sits behind it, never on it.
//
// Run: pnpm --filter @armada/brand build:appicon

import { Resvg } from '@resvg/resvg-js'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const BRAND = join(dirname(fileURLToPath(import.meta.url)), '..')
const MACOS = join(BRAND, 'macos')
const ICONSET = join(MACOS, 'AppIcon.iconset')

/** The hull, and the three positions of the arc. From svg/armada-mark.svg. */
const HULL =
  'M32 4 C 40 17, 50 35, 53 44 L36 46 C 34 34, 33 24, 31 18 C 28 25, 24 38, 19 52 L2 54 C 11 36, 24 15, 32 4 Z'
const HULLS = [
  [299.7099, 483.6947],
  [425.5115, 357.8931],
  [551.313, 452.2443],
]
const HULL_SCALE = 3.145038

/** Apple's grid: the art occupies 824 of 1024, and the inset carries the shadow. */
const BODY = { x: 100, y: 100, size: 824, radius: 185.4 }

const TOKEN = {
  bgBase: '#0F1419', // --bg-base, the canvas
  bgGlass: '#1A2330', // --bg-glass, a card's lit top
  accent: '#4A9EDB', // --accent
  ink: '#E9EEF4', // brand ink on dark grounds
}

const rect = (inset = 0) =>
  `x="${BODY.x + inset}" y="${BODY.y + inset}" width="${BODY.size - inset * 2}" ` +
  `height="${BODY.size - inset * 2}" rx="${BODY.radius - inset}" ry="${BODY.radius - inset}"`

const mark = (fill) =>
  `<g fill="${fill}">` +
  HULLS.map(
    ([x, y]) => `<path transform="translate(${x},${y}) scale(${HULL_SCALE})" d="${HULL}"/>`,
  ).join('') +
  '</g>'

/**
 * Every id is prefixed with the variant's own name. Two of these inlined into
 * one document would otherwise share `#pool` and `#rim`, and the second set of
 * defs would repaint the first icon — which is how the first contact sheet came
 * out with two accent bodies and no dark one.
 *
 * @param id     the variant's id prefix
 * @param body   the graded ground, top to bottom
 * @param pool   the pool of light's colour
 * @param halo   the mark's halo, one step lighter than the pool
 * @param glyph  the mark's flat fill
 * @param rim    the glass edge, brightest where the light falls
 */
const icon = ({ id, body, pool, halo, glyph, rim }) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024" role="img" aria-label="Armada">
  <defs>
    <clipPath id="${id}-body"><rect ${rect()}/></clipPath>
    <linearGradient id="${id}-ground" x1="0" y1="100" x2="0" y2="924" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${body[0]}"/>
      <stop offset="1" stop-color="${body[1]}"/>
    </linearGradient>
    <radialGradient id="${id}-pool" cx="0" cy="0" r="1" gradientTransform="translate(180 168) scale(760 520)" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${pool}" stop-opacity="0.46"/>
      <stop offset="0.48" stop-color="${pool}" stop-opacity="0.14"/>
      <stop offset="1" stop-color="${pool}" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="${id}-rim" x1="140" y1="110" x2="900" y2="920" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${rim[0]}" stop-opacity="${rim[1]}"/>
      <stop offset="0.5" stop-color="#FFFFFF" stop-opacity="0.06"/>
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0.03"/>
    </linearGradient>
    <filter id="${id}-halo" x="-70%" y="-70%" width="240%" height="240%" color-interpolation-filters="sRGB">
      <feGaussianBlur stdDeviation="9"/>
    </filter>
    <filter id="${id}-contact" x="-25%" y="-25%" width="150%" height="160%" color-interpolation-filters="sRGB">
      <feDropShadow dx="0" dy="14" stdDeviation="22" flood-color="#000000" flood-opacity="0.5"/>
    </filter>
  </defs>
  <g filter="url(#${id}-contact)"><rect ${rect()} fill="url(#${id}-ground)"/></g>
  <g clip-path="url(#${id}-body)">
    <rect ${rect()} fill="url(#${id}-pool)"/>
    <g filter="url(#${id}-halo)" opacity="0.75">${mark(halo)}</g>
  </g>
  ${mark(glyph)}
  <rect ${rect(1.5)} fill="none" stroke="url(#${id}-rim)" stroke-width="3"/>
</svg>
`

const MASTERS = {
  'appicon-dark.svg': icon({
    id: 'dark',
    body: [TOKEN.bgGlass, '#0C1116'],
    pool: TOKEN.accent,
    halo: '#7FC2F0',
    glyph: TOKEN.ink,
    rim: ['#A8D8F7', 0.34],
  }),
  'appicon-accent.svg': icon({
    id: 'accent',
    body: ['#6FB8EC', '#2A6E9F'],
    pool: '#FFFFFF',
    halo: TOKEN.bgBase,
    glyph: '#FFFFFF',
    rim: ['#FFFFFF', 0.45],
  }),
}

/** Apple's ten. `iconutil` refuses an iconset missing any of them. */
const ICONSET_SIZES = [
  ['icon_16x16', 16],
  ['icon_16x16@2x', 32],
  ['icon_32x32', 32],
  ['icon_32x32@2x', 64],
  ['icon_128x128', 128],
  ['icon_128x128@2x', 256],
  ['icon_256x256', 256],
  ['icon_256x256@2x', 512],
  ['icon_512x512', 512],
  ['icon_512x512@2x', 1024],
]

mkdirSync(ICONSET, { recursive: true })

for (const [name, svg] of Object.entries(MASTERS)) {
  writeFileSync(join(MACOS, name), svg)
  console.log(`macos/${name}`)
}

const dark = MASTERS['appicon-dark.svg']
for (const [name, width] of ICONSET_SIZES) {
  const png = new Resvg(dark, { fitTo: { mode: 'width', value: width } }).render().asPng()
  writeFileSync(join(ICONSET, `${name}.png`), png)
  console.log(`macos/AppIcon.iconset/${name}.png  ${width}px`)
}

console.log('\nNow: iconutil -c icns packages/brand/macos/AppIcon.iconset')
