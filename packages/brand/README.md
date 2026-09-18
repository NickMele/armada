# Armada brand

The mark is three hulls on a shallow arc. It is one filled shape in one
colour, and it is the only custom glyph Armada owns.

## What is here

| Folder | Holds |
|---|---|
| `svg/` | The masters. Everything else is derived from these. |
| `png/` | Raster mark at nine sizes, plus lockups and wordmark at @2x, in both tones. |
| `web/` | Favicons. `favicon.svg` flips with `prefers-color-scheme`. |
| `macos/` | App icon on Apple's grid, as `.icns` and as a rebuildable `.iconset`. |
| `src/` | `ArmadaMark`, `ArmadaMarkDuo`, `ArmadaLockupHorizontal`, `ArmadaLockupStacked`. |
| `covers/` | Notion covers at 2400×480 and the GitHub card at 1280×640. |

## Colour

| Token | Value | Use |
|---|---|---|
| Ink | `#E4E9EF` | The mark on dark grounds |
| Ink, dark | `#0F1419` | The mark on light grounds |
| Accent | `#4A9EDB` | Rules, meta text, the app icon's pool of light |
| App body, dark | `#1A2330` → `#0C1116` | The macOS icon body, top to bottom |

The mark is monotone. It is never given a gradient, a shadow, an outline, or a
second colour, with one exception: `ArmadaMarkDuo` holds the two trailing hulls
back, and only at 32px and above.

**The macOS icon body is the exception, and only the body.** It was a flat
`#161C23` until the app grew depth, and a flat tile then read as a dead one
beside a window built from lit glass. It is now Bridge's own canvas at icon
scale: the `--bg-glass` to `--bg-base` grade, `--accent` in the same pool of
light at the top leading corner, a glass rim, and the shadow Apple's 824-of-1024
inset exists to hold. The mark inside it is still one flat fill in one colour.
**The light goes behind the mark, never on it** — no gradient in the hulls, no
glow drawn as part of the glyph. Nothing else in this package takes a gradient:
`svg/`, `png/`, `web/` and `src/` stay flat, because a mark with no body has
nothing to light.

## Clear space

Clear space on all four sides equals the height of one hull — roughly half the
mark's own height. Nothing enters it, including the wordmark in a hand-built
lockup. Use the supplied lockups rather than setting your own.

## Minimum sizes

| Context | Floor |
|---|---|
| Mark alone | 16px |
| Horizontal lockup | 20px cap height |
| Stacked lockup | 48px overall height |
| `ArmadaMarkDuo` | 32px |

At 16px the notches are about one pixel and the mark is at its limit; it is
comfortable from 24px. Below 16px the three hulls fuse into a single silhouette
and the mark should not be used — set the wordmark alone instead.

## What may never change

- **The hull count is three.** It is not a fleet size, a feature count, or a
  number to tune. Two hulls or four is a different mark.
- **The arc is shallow and the hulls are equal.** A size taper was tried; the
  third hull disappears below 24px.
- **The legs carry their weight.** The hulls were widened specifically so the
  mark survives 16px. Thinning them for elegance at display size breaks the
  favicon.
- **No enclosing shape.** No circle, no rounded square, no badge. The macOS icon
  body is Apple's grid requirement, not a container the mark may borrow.
- **The mark is filled, never stroked.** There is no stroke width to set, which
  is why the React components expose no stroke prop.

## Rebuilding the macOS icon

```
pnpm --filter @armada/brand build:appicon
iconutil -c icns packages/brand/macos/AppIcon.iconset
```

The first command draws both SVG masters and renders all ten iconset PNGs; the
second packs them. Run both after changing anything in
`scripts/build-appicon.mjs`, which is the only place the icon's geometry and
colour live — the masters and the PNGs are output, and editing one by hand is
undone by the next build.

**The rasteriser is `@resvg/resvg-js`, a devDependency of this package**, so the
build needs no system library and no Homebrew. It also needs one thing the
browser does not: **every filter carries
`color-interpolation-filters="sRGB"`.** Without it resvg converts each filter
region to linearRGB and back, and the conversion leaves a visible horizontal
seam where the region ends — two bands across the body, at the drop shadow's
edge and the halo's. WebKit hides the bug because it composites the whole page
the same way.

Only the dark master goes into the iconset. `appicon-accent.svg` is drawn
alongside it and nothing consumes it; it exists so the light-bodied tone stays
in step if it is ever wanted.

## The wordmark

IBM Plex Sans SemiBold, converted to outlines. Every lockup and wordmark file
carries paths, not a font reference, so nothing falls back to Times on a machine
that lacks Plex. If the wordmark needs resetting, install `@ibm/plex-sans` and
convert with fontTools' `SVGPathPen`. Do not trace it by eye.
