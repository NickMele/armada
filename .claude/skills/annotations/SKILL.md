---
name: annotations
description: Work through the notes the owner left on Bridge with the annotation layer — read each open one, find the code it points at, ask what is his to decide, dispatch a subagent to make each change, verify it, and mark it done. Load when the owner says /annotations, or asks you to read or act on his annotations.
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
| `source` | The file and line of the JSX that drew the element, from the repository root. **Open this first** — it is the exact line, where the layer had it. A note left in Bridge under `pnpm dev` has no `source`: only the mock's build stamps it |
| `component`, `owners` | Search for these names where there is no `source`. `ownersFrom: "parent"` (Bridge under `pnpm dev`) is the tree it sits in, not who rendered it — walk it outward until a name is a component this repository defines |
| `element.text`, `selector` | Confirms you found the right instance, where a component draws in several places |
| `screen`, `layer`, `scenario` | Where to look at it: `pnpm -C apps/desktop mock` with `?scenario=` reproduces a mock note exactly |

**Look before changing anything.** A note is a reaction to what was on screen,
and the screen may have moved since. If the element is gone or already reads the
way the note asks, say so rather than inventing a change.

### 3. Decide, one note at a time

| The note is | Do |
|---|---|
| Small and clear — a word, a spacing, a missing state | Dispatch it, step 4 |
| A design decision — a new illustration, a changed layout, a contract rule | Ask with `asking-a-person`, citing the contract it touches (`docs/contracts/design-system.md`, `iconography.md`). Dispatch it once he has answered. Do not decide it |
| A defect | `armada-bug` |
| Bigger than a session, or better run as a Job | Tell the owner to press *Send to Fleet* on it, or file it as an issue if he prefers |

**Ask before dispatching, never after.** A subagent cannot put a question to the
owner. A note that touches a decision he made (a contract line, say) is settled
in this session first, and the brief carries his answer as a given.

### 4. Dispatch each change to a subagent

**The main session does not make the change.** The owner asked for this on
17 Sep 2026, after three notes in a row were each built inline. The main session
reads, asks, briefs, verifies and marks done. The agent builds.

**One agent per change**: a note, or a group of notes on one component.
`bridge-engineer` for anything under `apps/` or `packages/`, with
`isolation: "worktree"`. `work-issue`'s *Dispatching several agents at once*
section applies in full. Scopes must be disjoint to run in parallel, and two
changes that touch one file run one after the other.

The brief is the only context the agent has, so it carries:

| | |
|---|---|
| The note | Its file path under `.armada/annotations/` in the main checkout, and the owner's `text` verbatim |
| Where it is | The component and file already found in step 2, and the mock `?scenario=` that reproduces it |
| The decision | The option the owner picked, and its stated cost, word for word |
| The proof | `armada-components`: a story `play` or a mock test through `App`, run once against the change broken on purpose. A screenshot is how it is looked at, not the proof |
| The landing | Commit and push after each piece that passes, open a PR, **do not merge**. Run heavy commands in the foreground and wait. A decision it runs into goes in a single `**QUESTION:**` line at the end, and nothing that depends on the answer gets built |

**Verify what comes back yourself.** Read the diff, run its test, and look at
the screen in the mock. An agent's report of green has been wrong here. Then
report the PR to the owner and merge only when he asks. Whoever merges gives the
worktree back, as `work-issue` says.

**A component change is still a component change.** `armada-components` applies:
the contract wins, and the proof is a story or a mock test, not a screenshot.

### 5. Mark it done

Set `status` to `"done"` and `updatedAt` to now, in the file, once its change is
merged or its question answered. The layer re-reads the files each time it is
turned on, so the pin is gone there and the numbering closes over it — the bar
says how many are done and shows them again on request. **Never delete a note**,
unless the owner says to. He deletes his own.

### 6. Report

One line per note: its text, what you did, and the PR, issue or question it
became. Notes you left open, and why, go first. Name any agent still running.
