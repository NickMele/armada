---
id: 002
title: Tasks
status: BUILT
module: helm
raised: design pass, writing Helm's inbox mechanism
---

# 002 — Tasks

> **BUILT** — `armada task "<sentence>"` captures, `armada tasks` lists, `armada
> tasks start <id>` puts a Job on one, `armada tasks clear` discards. **One
> thing shipped differently from what is written below and it is deliberate:
> the store.** See *What was built, and the one place it diverges*.

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

---

## What was built, and the one place it diverges

**The verbs.** `task` files and `tasks` lists, exactly as
[`014`](014-report-what-you-know-went-wrong.md)'s `report` files and `failures`
lists — one quoted sentence cannot be confused with a sub-verb when the two live
under different words.

| Verb | What it is |
|---|---|
| `armada task "<sentence>"` | written down, with an id, no model call and one `git` call |
| `armada tasks` | the list; navigable at a terminal, plain through a pipe, `--json` either way |
| `armada tasks show <id>` | one task whole, and the prompt a Job would get |
| `armada tasks start <id>` | `armada fleet spawn`, with the task as its prompt |
| `armada tasks clear <id>` \| `--all` | discarded |

**The divergence: `~/.armada/tasks/<project>.yml` is not what shipped.** A task
is an `Origin::Written` row in `~/.armada/failures.jsonl` — the same store,
fold, id space and promotion path a failure and a report already share.

**This document's own argument is what changed the answer.** It says a task and
a raised item are *"the same object from two directions"* and that they *"should
not be built as two lists"*, and then, four paragraphs earlier, it puts them in
two files. Between this being written and being built,
[`014`](014-report-what-you-know-went-wrong.md) settled the same question the
other way for reports, on
[`001`](001-raised-items-need-identity.md)'s grounds: a second store is a second
id space, a second `show` and a second promotion path, and *a thing needing
attention is useless until it has an id you can act on one at a time*.

Every reason given above for `~/.armada/tasks/<project>.yml` survives and is met:

| Wanted | Because | What shipped |
|---|---|---|
| not in the repository | nothing to gitignore; no half-formed thought a teammate reads | it is under `~/.armada/` |
| not in `.armada/` | `clean` removes that directory (`PLAN.md` §4.2) | machine state; `clean` never touches it |
| one list per checkout, not per worktree | a Drone in a throwaway worktree sees what you wrote in `main` | one list per **machine**, so it does |
| never synced | *what describes you syncs, what describes this machine does not* (`PLAN.md` §13.1) | only `guild/` syncs, and this is not in it |

**The project identity is kept as a column rather than as a filename.** Capture
resolves `git rev-parse --path-format=absolute --git-common-dir` and records its
parent, which is `PLAN.md` §2.2's project: every worktree of a repository writes
the same value, so a task written by a Drone under `.claude/worktrees/` names the
checkout a Job can still branch from a week later.

### The workspace column, added after the fact

**A repository and a workspace are not the same identity, and `cwd` above only
ever answers the first.** A monorepo is one git repository and may declare
several workspaces (`workspaces: […]` in the root `armada.yml`), each its own
`armada.yml` — so a task written in one workspace and a task written in a
sibling both record the same `cwd` and the listing could not tell them apart.

`Entry` and `Line::Written` each gained a second, optional field —
`workspace` — carrying whichever `armada.yml` capture found on the way up
from `cwd`, tilde'd. **Reused rather than reimplemented**: it is the same
walk `armada manifest status` resolves against
(`armada_manifest::discovery::resolve`), so the two can never disagree about
what a workspace is. `armada tasks` draws it as a `WORKSPACE` column, dropped
entirely when no row in the listing has one (`render/table.rs`'s "empty in
every row" rule, unaffected by this feature) and left blank rather than
guessed on a row whose task was written outside any `armada.yml` — a
candidate directory that only resolves its own dependencies is not a
workspace until a config file claims it. Scope is unchanged: this is a
column, not a second axis to filter on, and every existing record without the
field keeps reading as `workspace: null`.

## The three questions this left open, answered

**Does a task belong to a repository or to the machine? To the machine, with the
repository as a column.** He works across repositories and a task written in one
is often about another; a list you have to be standing in the right directory to
read is a list you stop reading. The row still knows which repository it is
about, which is what `start` branches from — it is the listing that does not
care.

**What happens to a task when the Job it became finishes? Nothing, until you say
so.** The promotion line puts the Job's handle on the row and the state reads
`FIXING`; it does not close when the Job ends. A Drone reaching the end of its
workflow is not the same claim as the thing being done —
[`012`](012-a-drones-progress-through-its-workflow.md) records exactly that
distinction — and a task that closed itself on the weaker of the two would
quietly retire work nobody checked. `armada fleet ls` answers whether it landed;
`armada tasks clear` is the one keystroke that ends it.

**Are tasks and failures listed together or separately? Separately, out of one
store.** Two lenses over one file, split on one function
(`armada_core::failure::Origin::is_fault`), so a fourth origin cannot arrive
without deciding which listing it belongs in. A flat list would mix a
`bad_config` from Tuesday with *"rename the port allocator"* and make both harder
to find. **The id space is not split**: `armada failures show <a task id>`
answers, because a reader holding an id has already said which row they mean.

## What was not built

**Helm proposing a task is still reserved.** *"Helm may also propose one, when
you ask what to do next"* wants Helm, and the confirm-below-threshold rule
(`PLAN.md` §15.4) it would lean on is `fleet spawn`'s — which `armada tasks
start` already goes through when no `--workflow` is named. The Bridge row is
[`003`](003-bridge-command-centre.md)'s.
