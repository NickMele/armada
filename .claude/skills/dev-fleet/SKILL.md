---
name: dev-fleet
description: Start a Fleet of your own for development — its own home, store and repository copy, and a Drone that does nothing — so you can read real wire data or record a Job without touching the owner's Fleet. Load before starting a Fleet to look at, record from, or test a surface against.
---

# A Fleet of your own

**`scripts/dev-fleet <scratch-dir>` starts one**, and
`docs/practices/running-locally.md` is what it prints. This file is when to
reach for it, and what it will not do for you.

## Why not the owner's, and why not a plain `armada serve`

**The owner's Fleet is the owner's loop.** Stopping it strands the Jobs he is
running, and `pnpm dev` and `scripts/dev` reinstall the binary under it.
`.claude/skills/armada-local/SKILL.md` says what is yours to run there.

**A plain `armada serve` is not isolated, even on another port.** Fleet keeps its
store and its runtime file under `$HOME/Library/Application Support/Armada`, and
nothing but `HOME` moves them. So a second Fleet shares the owner's store, and
at boot it reconciles. Every Job it finds `running` with no Drone behind it is
escalated `interrupted`. That is written into the owner's record, and it is not
undone by stopping the second Fleet. Served against the main checkout, it also
appends to the transcripts under `.armada/`.

`scripts/dev-fleet` avoids all three: a scratch `HOME`, a local clone with the
Fleet data copied in, and `ARMADA_AGENT_BINARY` set to a program that exits at
once. **A Job it dispatches escalates rather than spending anything.**

## Starting and stopping it

- **It runs beside the owner's Fleet.** Fleet claims its own listener port at
  startup rather than binding a constant, so a dev Fleet takes a free one and
  publishes it in its own runtime file. This bullet used to say it could not
  run at all, which was true while the port was hardcoded.
- **The scratch directory goes outside the repository.** The script refuses one
  inside it — a copy of the owner's transcripts is one `git add -A` from a public
  commit. Your session scratchpad is the right place.
- **`--copy-store` brings the owner's Jobs.** Leave it off for an empty store.
  The copy is taken once, with SQLite's online backup, so it is safe while his
  Fleet is running. A second start reuses what the first one set up.
- **Run it with `run_in_background` and keep the handle.** It ends by exec-ing
  `armada serve`, which runs until signalled. Stop it through that handle, and
  say you did.
- **Read the port from your own runtime file** —
  `<scratch-dir>/home/user/Library/Application Support/Armada/fleet.json` — never
  from the owner's.
- **macOS has no `timeout`.** A wait for the runtime file is a loop with a
  counter, or it fails on its first line and reads as a Fleet that never came
  up.

## What it is for, and what it cannot do

| For | Not for |
|---|---|
| Reading a Job's wire data as Fleet serves it | Watching a Drone do real work. The agent exits at once, so a dispatched Job escalates |
| Recording a Job for Storybook with `scripts/record-job.mjs` | Anything that must change the owner's Jobs. Every write lands on the copy and is thrown away with it |
| Testing a surface against a real Fleet without holding his screen | Replacing `armada-local`. That is still the owner's Fleet and the owner's Bridge |

**Throw the directory away when you are done.** It holds a copy of the owner's
transcripts and store, and nothing else needs it.
