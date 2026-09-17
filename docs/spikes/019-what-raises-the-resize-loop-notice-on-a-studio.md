# Spike 19 — What raises `ResizeObserver loop completed with undelivered notifications.` on a Studio's whiteboard?

**A whiteboard card whose own box changes as its words change, and nothing else.**
React Flow measures every node with a `ResizeObserver` and writes what it measured into
its own store; its components read that store through `useSyncExternalStore`, which
takes the sync lane and is flushed in the microtask between two of the observer's own
callbacks — so the commit, and the layout it forces, land *inside* the observer's
delivery loop. While the cards keep resizing the loop never settles and Chromium says
so. **Armada's own five `ResizeObserver`s raise nothing**, because each updates through
`useState`, which takes the default lane and commits in a scheduler task after the loop
has finished.

| A `ResizeObserver` callback updates through | Notices in ~1.5 s |
|---|---|
| `useState` — every Armada observer | 0, over 175 self-resizing renders |
| `useSyncExternalStore` — React Flow's store | 163 |

**One change settles; sustained change does not.** Over 320 frames on the board:

| What moves | Notices |
|---|---|
| A node's content, container fixed | 298 |
| The container, node content fixed | 0 |
| Both | 72 |

## What moved the box, measured

`StudioNode` drew its title under `-webkit-line-clamp: 3` with no height, so the card
was one, two or three lines tall according to what the title said. A chip row was a text
line box with an `inline-block` chip on its baseline, and what the strut's descent
rounded to moved by a pixel between frames.

Driven at one redraw per frame, `apps/desktop/src/renderer/src/whiteboard-resize.test.tsx`
over 150 passes on ten cards:

| The card's title box, its chip rows | Distinct heights | Notices |
|---|---|---|
| Both free — `main` at 13c67b95 | 40 to 41 | 148 |
| Title pinned to its clamp, chip rows free | 20 (a 1 px oscillation) | 0 |
| Both pinned | 10, one per node | 0 |

**A pixel of oscillation was already under the threshold**, so the title is what drove
it; the chip row is pinned with it because a fuse left in is a fuse.

## What was ruled out, on the app rather than on a harness

Every `ResizeObserver` callback in the window was recorded while a Studio was opened on
a mock Fleet, through select, arrow-key moves, a 40-step pointer drag, six zoom steps,
Fit, and 40 frames of the board's own pane being resized.

**Ten callbacks, all of them at open, and no notice at any point.** Nothing the app
does to a board that is already drawn changes a node's box: React Flow positions nodes
by transform, and a transform is not a resize. **What drives the box is a Studio being
republished with different words on a card** — Helm writing to the board, a read-in
naming a Link, a scout's sources landing — and nothing in `Studios.tsx` ticks.

**So what drove it in the owner's session was never reproduced**, and the remedy does
not depend on knowing: a card whose box is a function of its kind rather than its
content cannot raise the notice however often it is republished.

## Provenance

Measured on 2026-09-17 in Chromium under `vitest --project "renderer (browser)"` at
1440 × 900, against `@xyflow/react` as `pnpm-lock.yaml` pins it. The first two tables
are `#1386`'s measurement; the rest are this file's. The counted test is
`apps/desktop/src/renderer/src/whiteboard-resize.test.tsx` and the fix it measures is
`packages/components/src/compositions/StudioNode/StudioNode.css`.
