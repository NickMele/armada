---
name: restart-app
description: Move the owner's Fleet — and Bridge, once it needs to — onto the working tree's build, so a merged fix runs without him leaving Bridge for a terminal. Load after merging a change to Fleet or Bridge, before running `scripts/restart`.
---

# Moving the owner's Fleet and Bridge onto a merged fix

**Run `scripts/restart` after a merge that touches Fleet or Bridge, when the
owner is going to look at the result.** A Job was left undelivered on 12 Sep
because Fleet kept serving a binary from before the fix had merged, and
nobody knew until a review read the logs. That is the incident this closes.

`docs/practices/running-locally.md` is what the script does and what it
refuses. Read it once; this file is only when to reach for the script and
what to tell the owner first.

## Never add it to an allow list

**The confirmation is Claude Code's own permission prompt, not a flag on the
script.** `scripts/restart` stops the owner's real Fleet and, when Bridge
needs it, reopens the window in front of him — the same reason
`.claude/skills/armada-local/SKILL.md` says an agent does not run
`scripts/dev`. Leaving it off every allow list is what turns "an agent ran
this" into a prompt he sees before it happens.

**Fast-forward the checkout first.** It builds what is in the working tree, not
what is on `origin/main`. On 17 Sep 2026 the checkout was four merges behind, so
the restart faithfully rebuilt the old code and Fleet came back on the protocol
version it started on — the one thing the restart was run to change. `git pull
--ff-only` before it, and read the version it prints at the end.

**Say what it will do before you call it.** Not "restarting Fleet" — whether
a Drone is working right now (it refuses if one is, naming the Job), and
whether Bridge is going to reopen (only if `apps/` or `packages/` changed
since it was last built). He is reading the prompt to decide, not you.

## When to run it

- After merging a fix to Fleet (`crates/`) or Bridge (`apps/desktop/`,
  `packages/`) that the owner is about to rely on.
- Not after every merge. A change nobody is watching for can wait for the
  owner's own next restart, and running it needlessly is one more prompt he
  has to read.

## What it refuses, and what it does not

**A Drone that is working.** `scripts/restart` reads the live roster and
each Drone's Job before touching anything; a Job at `running` refuses the
whole restart and names it. An escalated Job's idle Drone does not refuse —
Fleet's own restart reconciliation picks it back up, the way it already
does for a crash.

**Nothing else waits.** A queued Job, a Job at a human gate, a Job a person
is piloting — none of them hold a process the restart would interrupt.

**A restart already in progress.** Only one runs at a time; a second one
refuses at once, naming the pid of the one already running, rather than
racing it to stop and rebuild Fleet. If you see that refusal, wait for the
first to finish rather than retrying it.

**A restart that died holding the lock.** This refuses too, naming the pid
that died and the lock file to remove. It is never broken automatically —
tell the owner rather than removing it yourself, since a lock only outlives
its holder when a restart itself was killed.

## What it does not decide for you

It does not ask the owner anything itself — no prompt, no confirmation read
from a terminal. That is the point of never allow-listing it: the one
question it puts to him is Claude Code's own, and the one answer that
matters is whether he lets you run it.
