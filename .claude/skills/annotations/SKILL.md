---
name: annotations
description: Work through the notes the owner left on Bridge with the annotation layer — read each open one, find the code it points at, then fix it, ask, or leave it for Fleet, and mark it done. Load when the owner says /annotations, or asks you to read or act on his annotations.
---

# Working through annotations

**A note is a file.** The annotation layer (⌥⌘A in Bridge under `pnpm dev`, or in
`pnpm -C apps/desktop mock`) writes one JSON file per note under
`.armada/annotations/` at the repository root, gitignored. In a worktree it is
that worktree's root, so read the checkout the owner ran Bridge from — usually
the main one. The shape is `Annotation` in `apps/desktop/src/shared/annotations.ts`.

**Two ways a note reaches work.** In the layer, *Send to Fleet* proposes it as a
Job at the approval gate, and the note carries `sent` with that Job's handle.
This skill is the other way: a session reads the notes itself. Never do both to
one note — a note with `sent` belongs to its Job.

## The loop

### 1. Read the open notes

```
ls .armada/annotations/
```

Read every file. Skip `status: "done"`, and skip any with `sent` — say which Job
each went to, so the owner knows it is not lost. Group what is left by
`component`: three notes on one component are one change.

### 2. Find the code from the note, not from a guess

| Field | What it tells you |
|---|---|
| `text` | What the owner wants. The only field he wrote |
| `component`, `owners` | Search for these names first. `ownersFrom: "parent"` (Bridge under `pnpm dev`) is the tree it sits in, not who rendered it — walk it outward until a name is a component this repository defines |
| `element.text`, `selector` | Confirms you found the right instance, where a component draws in several places |
| `screen`, `layer`, `scenario` | Where to look at it: `pnpm -C apps/desktop mock` with `?scenario=` reproduces a mock note exactly |

**Look before changing anything.** A note is a reaction to what was on screen,
and the screen may have moved since. If the element is gone or already reads the
way the note asks, say so rather than inventing a change.

### 3. Decide, one note at a time

| The note is | Do |
|---|---|
| Small and clear — a word, a spacing, a missing state | Fix it. `work-issue` owns where: a worktree, a PR, merged as the owner has asked |
| A design decision — a new illustration, a changed layout, a contract rule | Ask with `asking-a-person`, citing the contract it touches (`docs/contracts/design-system.md`, `iconography.md`). Do not decide it |
| A defect | `armada-bug` |
| Bigger than a session, or better run as a Job | Tell the owner to press *Send to Fleet* on it, or file it as an issue if he prefers |

**A component change is still a component change.** `armada-components` applies:
the contract wins, and the proof is a story or a mock test, not a screenshot.

### 4. Mark it done

Set `status` to `"done"` and `updatedAt` to now, in the file, once its change is
merged or its question answered. The layer re-reads the files each time it is
turned on, so the pin turns grey there. **Never delete a note** — the owner
deletes his own.

### 5. Report

One line per note: its text, what you did, and the PR, issue or question it
became. Notes you left open, and why, go first.
