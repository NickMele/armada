---
id: 003
title: The Bridge as the command centre
status: RESERVED
module: helm
raised: real use — user request during a Helm/Bridge conversation
---

# 003 — The Bridge as the command centre

**The ask.** *"I feel like it should also have the ability for me to see the manifest and
the guild information. Like, I should be able to access anything from the bridge as if it's
a command centre."*

**Why it is coherent rather than scope creep.** PLAN.md §15.1 defines the Bridge as a renderer
over `fleet.*` that holds no state. Nothing in that definition is specific to Fleet — a renderer
over `manifest.*` and `guild.*` is the same object pointed at different data, and the four
modules already answer in one envelope (PLAN.md §3.1). The Bridge is where the user already is;
making him leave it to run `armada manifest status` is the same defect as `NEEDS YOU: YES` with
no reason attached — the data exists and the screen declines to show it.

**The constraint that makes it hard, and it is the interesting one.** The Bridge is
`ARCHITECTURE.md` §1.9-clean today precisely because it only reads Fleet. A screen that also
renders Manifest and Guild must not become the place where the three meet and start referring
to each other — Manifest and Guild are siblings and neither may reference the other. The
renderer may read all three; it may never let them read *through* it. Whether that survives
contact with a real layout is the open question.

**Design questions this leaves open:**

- **Navigation.** Fleet's rows are Jobs; Manifest's are components and checks; Guild's are
  files. Three shapes with no natural common cursor, and a tab bar is the lazy answer rather
  than the right one.
- **Whether it stays read-only.** PLAN.md §15.1's Bridge watches. A command centre that can
  edit a guild file or start a check is a different program with a different blast radius.
- **What it costs to keep live.** The redraw loop polls Fleet cheaply. Manifest state is a
  database read and Guild is a git worktree; neither wants a 250 ms cadence.

**Not scheduled.** Explicitly deferred by the user when raised — *"we don't have to build this
right now"* — and downstream of the Bridge's own bugs being fixed.
