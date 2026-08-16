---
id: 022
title: Armada cleans up after itself in Docker
status: BUILT
module: cross-cutting
raised: real use, 2026-08-15
---

# 021 — Armada cleans up after itself in Docker

> **Built.** Four pieces: two rows in `armada doctor`, a finding about the ownership store,
> `armada manifest prune`, and a Job that releases what it holds when it *finishes*. What follows
> is the design, and the arguments that lost — because the worktree decision in piece 4 is the
> one a later change is most likely to get wrong by reverting it without knowing it was decided.

**The complaint this exists to fix.** A macOS storage warning, on a machine holding **171 local
volumes and 12.0 GB, 100% reclaimable — and not one of them carrying an Armada label.** Two facts
in one measurement, and they point in opposite directions: most of the disk was not Armada's, and
Armada had no verb that would have told anybody either way.

## The four pieces

| # | Piece | Where |
|---|---|---|
| 1 | `armada doctor` reports docker disk as **two rows**, never summed | `crates/helm/src/verbs/doctor.rs` |
| 2 | The ownership store never recorded docker resources — reclaiming is **entirely by label** | finding; `crates/manifest/src/db.rs`, `crates/helm/src/app.rs` |
| 3 | `armada manifest prune` — reclaim disk, including disk that is not Armada's | `crates/helm/src/verbs/prune.rs`, [`prune.md`](../commands/manifest/prune.md) |
| 4 | A Job that **finishes** releases what it holds | `crates/helm/src/verbs/fleet.rs` |

The parsing and the safety rules are `armada_core::disk` — pure, and testable against the exact
bytes docker emitted on the measured machine. Running `docker` is not pure and lives in
`armada_manifest::docker` ([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.5).

## 1 — Two rows, because there are two remedies

| Row | Whose | What to run |
|---|---|---|
| the machine's | everything docker holds | `docker volume prune` — **the reader's to run** |
| Armada's | what carries an Armada owner label | `armada manifest clean --all` |

**They may never be added together.** A single "reclaimable" figure on the measured machine would
have pointed the reader at Armada's verb for a problem Armada did not cause — or, reversed, at
`docker volume prune` for resources a live workspace is still using. One number is one remedy, and
the wrong one is destructive in both directions.

**Neither row is ever a warning**, and the second half of that is the less obvious one.

- **Docker being absent is normal, not a fault.** Most machines running `armada doctor` are not
  running docker at that moment. A check that warned every time would make the command unsafe to
  put in a shell prompt, which is the property `doctor` exists to keep.
- **Reclaimable disk is a fact about the machine, not drift Armada can fix.** `clean` removes a
  workspace's resources when that workspace is *dead*, so a live worktree's 2 GB is legitimately
  held and a row calling it a problem would be wrong every day the reader is working.

**The precedent is the existing `manifest.db` row**, which reports reclaimable rows as `ok` and
puts the command in the detail. A settled row may not carry a `remedy`, and the number is on the
screen either way — which is the whole of what was asked for.

The remedy is deliberately kept off the *machine's* row: at the 80 columns the render is frozen
at, a detail carrying both the figures and a command truncates mid-word, and the half that gets
cut is the command.

## 2 — The store was neither recording nor reclaiming; labels were doing both

This began as a suspicion that `clean` was missing volumes. It was not. **`clean` already
reclaimed labelled volumes**, and had all along. What was missing is one layer down:

**The ownership store never recorded them.** `manifest.db`'s `owned` table has six `kind` values —
`Container`, `Network`, `Volume`, `Image`, `Pgid`, `Release` — and only the last two are ever
written. Nothing in the product inserts a `Container`, `Network`, `Volume` or `Image` row; the
variants exist, parse, serialise and are read on the way out, and are never on the way in.

So docker resources are found by **label, not by row**, everywhere:

| Question | Answered by |
|---|---|
| what does this workspace own in docker | `docker … ls --filter label=…`, asked of the daemon |
| what does this workspace own in processes | the `owned` table's `Pgid` rows |
| what would `owns.release:` run | the `owned` table's `Release` row — recorded, never executed |

Recorded here as a **finding rather than a defect**, because the label path is the trustworthy one
and the row path would be a second answer to the same question. The daemon knows what labels a
volume carries; `manifest.db` only knows what Armada last wrote down, and the case that matters —
reclaiming after the directory is gone, after a crash, after a machine reboot — is exactly the
case where the last write may not have happened. Two consequences worth stating so nobody
"fixes" this by populating the table:

- **A label is applied by the thing that creates the resource**, atomically with creating it.
  A row is a second write that can fail on its own.
- **Two sources of ownership would disagree**, and the disagreement would show up as either a
  leak or a wrongful deletion. Neither failure announces itself.

The unwritten variants stay. `manifest.db` is forward-compatible across 0.x and an unused `kind`
costs nothing an older binary cannot ignore — but a reader tracing "where does the `Volume` row
get written" should find this paragraph rather than a bug.

## 3 — `armada manifest prune`

Full reference: [`prune.md`](../commands/manifest/prune.md). The design in one table, because the
three rules are the whole verb:

| Rule | Encoded in |
|---|---|
| A preview is mandatory — rows toggle, enter confirms, esc touches nothing | the run itself; `--dry-run` is the ordinary run with the confirmation withheld |
| Armada's own **idle** volumes open ticked; nothing else does | `armada_core::disk::default_ticks` |
| An unlabelled volume goes only on a per-run confirmation from a person at a terminal | `armada_core::disk::permitted` |

**Volumes only.** Images are `docker image prune`, which already does the right thing and which
Armada has no ownership story for. **Separate verb, not a flag on `clean`**: `clean` is defined by
what it can prove, and a flag that reaches past that boundary is one word away from a verb people
run without reading.

**The terminal is read at the entrypoint and nowhere below it.** `interactive` is
`terminal.can_ask() && !json`, decided once and passed down — a verb that sniffed a stream's
tty-ness itself would be reading ambient state and would also be untestable.

## 4 — A Job that finishes releases what it holds

**The happy path was the leak.** `spawn`'s rollback reclaimed, `kill` reclaimed, `reap` reclaimed —
and the workflow loop's success path reclaimed nothing. Every Job that worked left its containers,
its volumes and its port block behind, which is how a machine comes to hold 171 volumes.

`end`'s loop body became a shared `tear_down`. `kill`, `reap` and the finishing pass all arrive
there and **differ only in an `Ending`** — the state to write, whether the branch is kept, whether
the worktree is kept, whether to observe first. A second copy of that loop would be a second
answer to what Armada orders and what it tolerates.

| | `kill` / `reap` | finishing |
|---|---|---|
| containers, networks, volumes, images | released | released |
| port block | released | released |
| **branch** | deleted | **always kept — it is the deliverable** |
| **worktree** | removed | removed **only when git says there is nothing in it to lose** |

**The branch is never touched on a finish.** A Job that reached its last step produced commits,
and those commits are the entire reason it ran; deleting the branch would make the loop's success
indistinguishable from its failure.

### The worktree decision, and both arguments that lost

This was genuinely arguable, and it is recorded in full because the losing arguments are good
ones and will be made again.

| Option | The case for it | Why it lost |
|---|---|---|
| **Keep it; let `reap` take it** | `reap` exists for exactly this, already offers a `DONE` Job **ticked by default**, and a reader who wants to see what the Job did has somewhere to look | **Nobody runs the deferred verb.** That is this project's own evidence and the identical argument the user made about `clean` — answering *"why is my disk full"* with a command he was already not running is not an answer |
| **Remove it always** — what `kill` does | one rule, no git call, no branch in the code | `worktree::remove` **forces**, and forcing is right when a person asked by name and wrong when a background pass decided. Uncommitted work destroyed by a loop nobody was watching is work nobody agreed to lose |

**So it goes when git says the tree is clean, and stays when it is not** — and the row says which,
because a directory that survives with nothing explaining it reads as a broken removal. A Job
whose tree is dirty stays offered to `armada fleet reap`, where taking it is a deliberate act in
front of a preview.

`worktree::holds_uncommitted_work` is the question, and it is conservative in both directions:

- **Untracked files count.** A Drone that wrote a scratch file nobody added is the ordinary case,
  and `--force` deletes it with the directory. `--untracked-files=normal`, so an ignored
  `node_modules` is not counted as work somebody loses.
- **A git that will not answer is dirty.** A tree Armada could not inspect is not a tree Armada
  may throw away — the same rule `armada_core::reap` holds for a path that will not `stat`, and
  for the same reason: guessing wrong in that direction is unrecoverable.
- `--porcelain`, because the human `git status` is explicitly not a format to parse.

**A `FAILED` or halted Job never reaches this path.** `Next::Halt` leaves a Job `PAUSED` and asks
a person; its worktree is the evidence for the question it just raised.

## The standing constraint: `docker system df` output is not a stable API

It is a human report with a `--format` template bolted on, and
[`traps.md`](../traps.md)'s *"Docker disk usage — `docker system df`"* section records what it
actually emits. Three rules hold throughout, and each exists because the alternative is silent:

| Rule | The alternative, and why it is silent |
|---|---|
| **Parse the `--format '{{json .}}'` template, never the human columns** | A renamed field makes the template exit non-zero and announces itself on the first run after an upgrade. A renamed column shifts a `split_whitespace` by one and reports a count as a size — and `Local Volumes` is two words, so it does that on the row holding all the disk |
| **An unreadable size is unknown, never `0`** | `0 B` is a confident claim that there is nothing to reclaim, which is the one wrong answer that stops the reader looking. One unreadable contributor makes the *total* unknown too, rather than a smaller total that reads as complete |
| **A type Armada does not recognise is carried by name, not dropped** | A future `docker system df` row is a thing holding disk, and a parser that silently skips it under-reports for ever. Only the reader can tell whether it matters |

Two more that fall out of the same section: sizes are **base 1000** with a lowercase `k` in `kB`
(reading it as 1024 is a 2.4% error that survives review and makes a gigabyte figure wrong), and
`docker system df` **needs the daemon** — there is no client-side answer, so a dead daemon is an
`environment` failure to report and never a zero to display.

## What it is downstream of

[`clean.md`](../commands/manifest/clean.md)'s premise — ownership recorded machine-globally, so
reclaiming is a query rather than a memory — is what makes piece 4's safety net real: if the
teardown order is ever reversed or a step fails, `armada manifest clean --all` still reclaims what
was left. That is the reason Manifest sits underneath Fleet, and it is why nothing in `tear_down`
raises.

[010](010-armada-records-its-own-failures.md)'s rule about `$HOME` applies unchanged here: a
volume name and a workspace path both reach the screen and the payload, and this repository is
public permanently.

## A bug this branch found in its own review, and fixed

**`prune` built its envelope `ok` regardless, so a run in which docker refused to remove a volume
exited `0`.** It was written that way by analogy with `clean` — one resource that will not release
must not abort the rest — but the analogy was to the wrong half. `clean` carries the failure on the
row *and* derives an error from it through `envelope::aggregate`, precisely so the exit code still
moves.

The rule this broke is the one in [`AGENTS.md`](../../AGENTS.md): **the code is a function of
`error.class`, or `0` when there is no error**, and the enum covers every non-zero exit. A verb
that reclaims disk and cannot tell a caller it failed to is a verb nothing can gate on.

The argument for leaving it was that a failing run could not be told apart from a `SKIPPED`-heavy
one. That argument does not survive contact with the rule: `SKIPPED` produces no error and so
still exits `0`, while a refused *removal* is a `tool_failed` and always was. The two were never
in danger of colliding — they are different axes, which is the whole reason terminal state does
not determine the exit code.

Fixed on this branch. A refused removal is now `tool_failed`, naming every handle that would not
go, with `next_action` telling the caller to remove them by hand and re-run — the same shape
`clean` already uses. `PARTIAL` is kept for "three of five went", because that and "none went"
demand different actions from a reader even though both exit `1`.
