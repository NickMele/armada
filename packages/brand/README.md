# @armada/brand

The Countersign mark, and every rendering of it the app or the repository needs.

**The rules are not here.** `docs/contracts/iconography.md`, section *Brand mark
— the one custom glyph*, governs stroke, caps, clear space, minimum size and
colour, and `packages/icons/icons.toml` carries the mark's registry row. This
file says what is in the directory and how to rebuild it.

Two things worth knowing before touching anything, because both are easy to undo
by accident and neither is visible in a diff:

- **The stroke is 2 at every size**, and **the caps are butt, not round.** Round
  caps overhang each endpoint by a full unit, which closes the two-unit
  clearance the mark is built on — the mark stops being the mark.
- **The React components do not take `strokeWidth` or `linecap` as props.** That
  is deliberate: a prop is an invitation.

## What is here

| Path | |
|---|---|
| `svg/armada-mark.svg` | 24×24, `currentColor`. **The master** — every other rendering derives from it |
| `svg/armada-lockup-*.svg`, `armada-wordmark.svg` | Text outlined, so no font is needed to render them |
| `png/` | Transparent, light and dark, 16 through 1024; lockups at @2x |
| `web/` | `favicon.svg` responds to `prefers-color-scheme`; `.ico` carries 16/32/48 |
| `macos/` | `AppIcon.icns` ready for the bundle, plus the `.iconset` it is built from and the two SVG sources |
| `covers/` | Page covers at 2400×480 and the repository's social preview at 1280×640 |
| `src/` | `ArmadaMark`, `ArmadaLockup` |

## Rebuilding

The macOS icon, after any change to the mark:

```sh
iconutil -c icns packages/brand/macos/AppIcon.iconset
```

## Why the covers are the sizes they are

**2400×480 is ≈5:1** because that is the aspect a page cover is actually
displayed at, so nothing important lands outside the crop and the image needs no
repositioning. Both hold the lower-left quiet, because a page icon sits there and
overlaps the cover's bottom edge.

**1280×640 is GitHub's stated size** for a social preview, and everything sits
inside the 40pt safe border it recommends — so nothing is lost when the card is
re-cropped by a link unfurl. It is set under Settings → General → Social preview.
