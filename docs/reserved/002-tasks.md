---
id: 002
title: Tasks
status: RESERVED
module: helm
raised: design pass, writing Helm's inbox mechanism
---

# 002 — Tasks

**The thing being solved.** Working in a repo you notice something — a fix, a
question, a thing to look into — and you want it recorded without spending
anything on it yet, and without losing it. Today an agent writes it into a
markdown file somewhere in the repository, which is neither yours nor the
repository's and gets stale in both directions.

**Capture is free; execution is deliberate.** That is the whole point, and it is
what makes a task different from a Job. A Job has a worktree, a budget and a
Drone from the moment it exists. A task is a sentence.

#### Where they live

`~/.armada/tasks/<project>.yml` — machine-local, **keyed by project rather than
workspace** (PLAN.md §2.2's two identities).

Three things fall out of that choice, and each rules out an alternative:

- **Not in the repository.** Nothing to gitignore, nothing to commit by
  accident, and no half-formed thought becomes something a teammate reads.
- **Not in `.armada/`.** PLAN.md §4.2 says that directory deliberately holds nothing you
  would miss, and `clean` removes it. Tasks would die with the workspace.
- **Keyed by project, not workspace.** Every worktree of a repository shares one
  list, so a Drone in a throwaway worktree sees the same tasks the checkout you
  wrote them in does. Keyed by workspace, a task written in `main` would be
  invisible to the Job spawned to do it.

**They do not sync.** They are about one repository on one machine, and the
guild is about you. `PLAN.md` §13.1's line holds: what describes you syncs, what
describes this machine does not.

#### Becoming a Job

`armada task start <id>` — or Enter on the row in the Bridge — spawns a Job with
the task as its prompt and **links the two**, so the task shows the Job's state
and closes when it lands. Nothing ever runs because you wrote it down.

**Helm may also propose one**, when you ask what to do next. That is the one path
where work can begin without you naming it, so it proposes and does not start —
Helm's spawn rule (PLAN.md §15.4) already confirms below a confidence threshold, and a
task Helm chose for you is exactly that case.

#### Its relationship to raised-item identity (`001-raised-items-need-identity.md`)

**A task and a raised item are the same object from two directions.** You write
one; a Drone raises the other with `fleet.ask_human`. Both are a thing needing
attention, both want an id, and both want one keystroke to act on.
`001-raised-items-need-identity.md` reserved the identity problem from the inbox side; this is
the same record arriving from yours, and they should not be built as two lists.

**Not scheduled.** It wants the Bridge to be worth looking at, which is M3.
