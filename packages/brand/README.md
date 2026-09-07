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
| Accent | `#4A9EDB` | Rules, meta text, the accent app icon body |
| App body, dark | `#161C23` | The macOS icon body |

The mark is monotone. It is never given a gradient, a shadow, an outline, or a
second colour, with one exception: `ArmadaMarkDuo` holds the two trailing hulls
back, and only at 32px and above.

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
iconutil -c icns packages/brand/macos/AppIcon.iconset
```

The `.icns` here was assembled directly, so it can be regenerated on any
machine; `iconutil` is only needed if the iconset changes.

## The wordmark

IBM Plex Sans SemiBold, converted to outlines. Every lockup and wordmark file
carries paths, not a font reference, so nothing falls back to Times on a machine
that lacks Plex. If the wordmark needs resetting, install `@ibm/plex-sans` and
convert with fontTools' `SVGPathPen`. Do not trace it by eye.
