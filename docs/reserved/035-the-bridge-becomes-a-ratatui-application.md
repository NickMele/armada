---
id: 035
title: The Bridge becomes a ratatui application
status: RESERVED
module: helm
raised: the owner asking why scrolling was described as unavailable, 2026-08-17
---

# 035 — The Bridge becomes a ratatui application

**What this settles.** The Bridge draws every box, border and column by composing strings by hand
and paints the result as **one `Paragraph`**. It uses almost none of the library it depends on. That
is why it cannot scroll, why it breaks on resize, and why a per-section background tint is not
expressible. Decided: adopt ratatui properly, and take the golden coverage that makes the change
safe with it.

## How this was found

The owner asked whether the inbox list could scroll. It was reported back to him as *no, and it is
already losing rows today*, with three findings under it. He pushed back in one line:

> *"Thats not true. Are we not using Ratatui?"* — with `ratatui.rs/examples/widgets/scrollbar` and
> `ratatui.rs/examples/layout/flex`

He was right, and the correction is the more useful finding. Everything needed is **already in the
pinned tree**, with no new dependency, no feature flag and no version bump:

| Available now | Where |
|---|---|
| `Scrollbar`, `ScrollbarState`, `ScrollbarOrientation`, `ScrollDirection` | re-exported by the facade, `ratatui-0.30.2/src/widgets.rs:684` |
| `Paragraph::scroll((y, x))` | a `const` builder, `ratatui-widgets-0.3.2/src/paragraph.rs:233` |
| `Layout`, `Constraint`, `Flex` | `ratatui-core-0.1.2/src/layout/flex.rs` |
| `Block`, `Borders`, background styling | `ratatui-widgets-0.3.2/src/block.rs` |
| `TestBackend` — renders into an in-memory `Buffer` | `ratatui-core-0.1.2/src/backend.rs:112`, ungated |

**The accurate finding is narrower and worse than the one it replaces.** Armada uses none of it. The
whole screen is `Paragraph::new(lines)` over `f.area()` with no `.scroll()`
(`crates/helm/src/bridge.rs:700`), and every `┌─┐` is drawn character by character in
`crates/helm/src/render/frame.rs`. The only ratatui widget used beyond `Paragraph` anywhere in the
workspace is `Block`/`Borders`/`Wrap`, in the prose editor — not the Bridge.

## Three recorded complaints are one fact

Each was filed separately. Each is the hand-composed screen.

| Signal | Complaint | Mechanism |
|---|---|---|
| `80d452a5` | *"Resizing the terminal window breaks the Bridge TUI layout"* | `f.area()` already follows the real terminal, but content is composed at a width captured in `main` (`render/term.rs:62`, *"`main` only"*) and never re-read; `Event::Resize` is discarded at `bridge.rs:541`. `Layout` would have done this for free |
| `14cd98ab` | *"Bridge needs more color, side borders per section, and a distinct background tint per section"* | `Span` is `{ text, role, bold }` (`render/table.rs:207`) and `live::paint` only ever sets `.fg()`. A background is not expressible in the type the shared composition returns; `Block` takes one |
| — | the list cannot scroll | no panel caps its rows, one `Paragraph` with no offset, and `Cursor` is a bare wrapping index (`crates/core/src/fleet/bridge.rs:238`) that can already sit on a row nobody drew |

`fa770bfc` — *"KEYS legend is illegible"* — is adjacent: the shipped legend is one unboxed line
because overflow is handled by *dropping* pairs into a separate `Keys` mode
(`core/fleet/bridge.rs:503`), where [`033`](033-the-command-centre-designed.md) specifies a bordered
two-line panel. A `Block` with a real height budget removes the reason it was one line.

## Why it was built this way, which is not laziness

`render::command_centre` returns `Vec<Vec<Span>>` and **both audiences consume the same value** —
`--once` turns it into text, the Bridge paints it into ratatui. `crates/helm/src/render.rs:885-893`
records getting burned when those two diverged before:

> *"It used to be a bare `ARMADA BRIDGE` heading over one `bridge_table`… so after `033` landed the
> panels in the live path only, `--once` and the screen were two layouts."*

Adopting ratatui's `Table` and `Block` in the Bridge breaks that sharing by construction. That is
the real cost, and it is the reason this is a design rather than a chore.

## Decided: full adoption, and the goldens come with it

**The objection to full adoption was that it gives up golden coverage of the screen** — the fixtures
are what hold the two audiences together, and there would be nothing to hold. That objection was
wrong for the same reason the scrollbar claim was wrong: it reasoned about ratatui instead of reading
it. `TestBackend` renders into a `Buffer` that can be dumped and snapshotted, so a fully-adopted
Bridge gets **better** coverage than it has now.

That is not a small distinction. Today **no golden fixture can see any interactive surface**: all
three Bridge fixtures route through `render.rs:882`, the `--once` path, which passes `None` for the
cursor and `false` for keys (`render.rs:901-902`). Measured:

```
$ grep -l "▸" tests/golden/render/*.plain       → no golden contains the focus marker
$ grep -l "─ KEYS" tests/golden/render/*.plain  → no golden contains a KEYS box
```

The cursor, the KEYS box, the detail pane, the reap preview, the keys page, the compose box and the
filter line have no byte-level fixture — only inline `paint()` unit tests. **Every one of the six
recorded TUI complaints lives in exactly the region the golden suite cannot see.** So the coverage is
not a nicety attached to this change; it is the thing that makes the change reviewable, and it
lands with it.

| | Before | After |
|---|---|---|
| The screen | one `Paragraph` of composed strings | `Layout` rects, `Block` borders, real widgets |
| Overflow | silently clipped by ratatui | `Scrollbar` + a windowed list |
| Resize | width captured in `main` | `f.area()` per frame |
| Per-section tint | not expressible | `Block::style` |
| Golden coverage of the screen | none | `TestBackend` buffer snapshots |
| `--once` | the same composed value the screen used | its own composition, deliberately |

**`--once` keeps its own goldens and its own composition.** The two audiences stop sharing one
layout, which is the thing `render.rs:885-893` warned about — so what replaces the shared value as
the anti-drift mechanism is **two fixture suites over one payload**: `ShowData`/`BridgeData` stay the
single source of the facts, and each audience has a snapshot proving what it renders. That is a
weaker guarantee than one shared function and a stronger one than today's, where the screen has no
snapshot at all.

## What this does not decide

**The six complaints are their own pass.** This document is the adoption and the coverage; the
freeze (`f1b22f05` — `read_all` calls `doctor::run` synchronously on every redraw,
`verbs/bridge.rs:204`), the `Enter` binding (`a8f36d45`), boarding into cmux (`1668b794`) and the
KEYS layout (`fa770bfc`) are fixed on top of fixtures that can prove the screen still draws. The
owner chose that order explicitly. `docs/HANDOVER.md` puts the freeze first in his own priority
order, and it stays first *after* this.

**`Table::spans` is a prerequisite, not a footnote.** `Table::render` draws a hanging note under the
second column (`render/table.rs:364-372`); `Table::spans` never reads `row.note` (`:414-419`), so
every ratatui surface loses it and the parity test at `:658` cannot catch it — it compares against a
line `spans` never emits. Any row that wants a second line needs this first.

## The risk

**Two rendering idioms in one crate during the transition**, and `ARCHITECTURE.md` §1.9 across seven
panels — the same risk [`033`](033-the-command-centre-designed.md) and
[`020`](020-the-tui-decided.md) both name as their largest, and one `cargo xtask boundaries` cannot
see, because the Bridge may legitimately read every module. The backstop is the same: review at the
`implement` gate, reading this section.
