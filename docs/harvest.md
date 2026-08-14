# Harvest — the check engine, as behaviour

**What this is.** The behaviour of the source repo's check engine, written down so charkit can be
rebuilt from it without anyone reading the original. It is a specification of *what the code does*
and, more importantly, a written list of **every trap and bug-shaped branch found** — the branches
that exist because something broke in production once, and that a rewrite loses silently because a
bug fix looks like an unremarkable three-line conditional.

**Who reads it.** Phase 3's implementer, who never opens the source repo
([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.7). Also phase 4's, because a large part of what was
harvested is service lifecycle rather than check engine — see §3.

**There is no second harvest.** §2.7 licenses exactly one agent, in this phase, to read that repo.
Anything not written down here is not recoverable later without re-opening the clean room, which
the architecture does not provide for. That is why this document records phase-4 material in full
rather than deferring it, and why it records defects and dead ends as well as behaviour.

## 0. The three rules this file is held to

**It describes behaviour and never carries implementation.** Prose, tables, trap descriptions;
short config or regex fragments where a rule turns on one. No verbatim implementation code. The
test applied throughout: could this be pasted into `crates/` and compile? If yes, it was rewritten
as prose. Argv is described by its *properties* ("names the service explicitly", "carries no TTY")
rather than transcribed.

This matters more here than anywhere else in the repository. This file is **not** covered by the
contamination grep — [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4 exempts it deliberately, because its
job is to record the assumptions that must be stripped, and banning the words would forbid writing
them down. That makes this document the one place *structural* contamination can enter, and
[`PHASES.md`](PHASES.md) §8.1 says no grep can catch that. Review is the only defence. It is
committed rather than kept local for exactly that reason.

**It is published.** This repository is public. The standard held to here is the one the phase
plan's own source-material table was written to before it was retired: behaviour, file names,
traps and counts yes; the source repo's name or path, and any absolute local path, never. Paths
are written relative to a repo root or as `~/`.

**The privacy gate's exemption is a safety net, not a licence.** `cargo xtask privacy` exempts this
file from the configured-private-names rule and **not** from the machine-path rule. Nothing below
relies on the exemption: no private name is written here, so the file would pass without it.

Product and application names from the source repo are genericised throughout as `app-a`…`app-d`
and `pkg-ui`, with their structural role stated. Directory names that sit on charkit's own
contamination list — `backend/`, `web/`, `scripts/`, `.claude/`, `tilt`, `NEXT_PUBLIC` — **are**
named, because naming the assumption is how the implementer knows to strip it.

---

## 1. What was measured

The phase plan's source-material table warned that it had gone stale once already, by 1.4–2.4×,
and instructed re-measurement before scoping. Re-measured; **it was stale again, in the same
direction.** That table has since been retired from [`PHASES.md`](PHASES.md), which is now
organised around milestones rather than phases, so **this section is the only record of the
measurement** — the "plan said" column below is what the retired table carried, kept so the
drift is legible rather than erased.

| Path, relative to `scripts/` | Plan said | Measured | Delta |
|---|---:|---:|---|
| `char/check.py` | 3,383 | **3,556** | +173 |
| `char/_shared.py` | 337 | 337 | — |
| `char/baselines.py` | 762 | 762 | — |
| `char/worktrees.py` | 679 | **705** | +26 |
| `char/servers.py` | 436 | 436 | — |
| `char/__main__.py` | 521 | 521 | — |
| `char/tickets.py` | 51 | 51 | — |
| `char_mcp/server.py` | ~95 | **316** | 3.3× |
| `char_test/` | 2,694 | **5,948** | **2.2×** |
| `scripts/pyproject.toml` | *unlisted* | 47 | — |
| `scripts/uv.lock` | *unlisted* | 788 | — |
| **Harvestable total** | ~6,169 | **12,632** | ~2.0× |

`bin/char` is 21 lines and lives at the **repo root**, not under `scripts/` as the plan's path
implied.

`char_test/` is the correction that matters, and the plan predicted it: that row was explicitly
never re-measured and flagged as "assume stale by a similar factor". It is 2.2× larger, and one
file — `test_check.py`, 3,354 lines and 250 tests — is on its own larger than the figure the
plan carried for the entire suite. Full suite: 397 tests across 7 files.

**Consequence for scoping.** The ported-cases deliverable is roughly double what the phase plan
assumed, which reinforces its own instruction that the check-engine work lands as several
review-sized PRs rather than one. Suggested split in §11.

**Re-measure before scoping anything against this table.** It has now gone stale twice, both
times in the same direction.

**One structural find that shaped this harvest.** `test_check.py` is partitioned into 23 labelled
bands, and six carry multi-paragraph banner comments narrating a **dated production defect** and
what it broke. Those banners are where the otherwise-silent fixes are documented — not in the code
they protect. Any future reader of that suite should treat the banners as the primary source.

---

## 2. What has been stripped

Stated plainly, because the implementer cannot make these calls afterwards.

| Stripped | Replaced by |
|---|---|
| **`CHECK_CATALOG`** — a static, compiled-in list of 12 check specifications | The config loader. §4.2 renders what the catalogue *expressed* as a table of behaviour, so the schema can be checked against it; the list itself does not survive. |
| **`domain`** — a three-valued enum (`backend`, `frontend`, `tooling`) hardcoded throughout | `component`. Everywhere the original branches on a domain, charkit reads a component from config. The count of three, and the fact that each mapped to exactly one directory and one toolchain, is source-repo topology. |
| **Every repo-specific path** — `backend/`, `web/`, `scripts/`, `docs/`, `web/apps/*`, `web/packages/*`, `web/e2e/`, `web/visual/`, `.claude/check-runs/`, `backend/.env`, the container mount root | Component `root:` and `match:` globs; charkit's own state directory; declared mount points. |
| **Every turbo filter** — the monorepo task runner's git-aware `...[<ref>...HEAD]` package selector, its `--filter=` argument shape, and the branch that drops it inside a container | Free-text `cmd:`. charkit must not know that tool exists. The *rule* underneath survives as a trap (§5.4, CMD-1). |
| **Every interpreter-directory assumption** — a `--directory`-style flag that changes directory before invoking a tool, and the path re-basing it forces | Per-check working directory at the component root, with paths emitted relative to it by construction. This makes the re-basing trap (§5.4, CMD-8) unnecessary rather than load-bearing. |
| Hardcoded default-branch and remote names (`main`, `origin/main`) | Config. |
| App names, compose service names, compose profile names, per-worktree port variable names | Config; charkit's own port model. |
| Behaviour keyed on a **check id** — remediation hints, image freshening, parser selection | Config. **No behaviour in charkit may key on a check's id.** This is the single most repeated source-repo assumption and it appears in four separate subsystems. |

---

## 3. Phase boundaries — read this before building anything

A large fraction of what was harvested is not check-engine behaviour. Building it in phase 3 would
put a compose-shaped adapter inside the check engine, which is the contamination §8.1 is about.

| Build now (phase 3) | Build in phase 4 | Build never |
|---|---|---|
| Scope and selection (§4.1) | Container daemon availability and auto-start | The static catalogue |
| Admission control, costs, exclusives (§4.3) | Compose environment provisioning, port export | The three-domain model |
| Command construction, the runner decision (§4.4) | Renamed-container husk sweeping | Per-tool parsers as core logic |
| The process seam — deadlines, groups, abort (§4.5) | Anonymous-volume sweeping | The pixel-diff visual aid (§6.2) |
| Result parsing as a *contract* (§4.6) | Service teardown and the ownership rule | Behaviour keyed on check id |
| Two-phase execution (§4.7) | Image freshening, of the service *and* its dependencies (§5.4, CMD-2 and CMD-3) | |
| Live output (§4.8) | Package-manager install preflight | |
| Run state, status, leases, `--again`, detach (§4.9, §4.10) | | |

**Phase 3 owes phase 4 exactly two seams**, and then owes it nothing else:

- a **preflight hook**, run before any check, which may fail the run with a typed error;
- a **cleanup hook**, run from the abort path, which may **never** fail the run.

Every piece of docker lifecycle in §5.7 lands behind those two without the check engine changing
again. This is the whole of the phase-3/phase-4 interface for this subsystem.

---

## 4. Behaviour

### 4.1 Scope and selection

Three mutually exclusive scope inputs collapse to a `(files, is_all)` pair, and five pure
classifiers turn that pair into *which checks run* and *what paths they are handed*.

**Scope resolution.** `--all` gives an empty file list and the all-flag set. An explicit file list
is split on commas and trimmed. Otherwise the changed set is computed as the union of a three-dot
name-only diff against a ref — so commits landing on the ref after divergence are excluded — and
the working-tree status including untracked files. The union is **order-preserving and
deduplicating**, diff entries first.

**The ref is resolved dynamically** when not given explicitly: verify a local default branch exists;
if the verification command exits zero use it, otherwise fall back to the remote-tracking one. Only
the exit status is consulted. Three states must be representable — flag absent, flag present with
no value, flag present with a value — and conflating the middle two is a live defect (§5.1, SC-1).

**Classification is conservative by design.** Two path registries drive it: *owned* trees, each of
which owns its own checks, and *inert* trees, which own no checks and are read by none. A file
under neither is **ambiguous**, and ambiguity schedules everything. A domain is excluded only when
every scoped file is provably under another owned tree or under an inert one.

| Scope | Runs |
|---|---|
| files cleanly under one owned tree | that tree's checks only |
| files under an inert tree only | nothing |
| any file outside every registered tree | **everything** |
| inert + owned | the owned tree's checks; the inert file contributes nothing |
| all-flag, or an empty file list | everything |

The empty-list case sharing a short-circuit with the all-flag is load-bearing: a branch with no diff
must not report `PASS` having run nothing. charkit expresses the same rule differently and more
strictly — `PLAN.md` §4.1's empty-`${files}` rule skips the check, and `--all-files` is the
documented way to ask for the whole tree — so the *conclusion* transfers and the mechanism does not.

**Sub-scoping within a tree** narrows a tool's positional arguments to the touched sub-directories,
with one exception that is a fix: **a file sitting directly at the tree root forces the whole tree**,
because shared infrastructure affects every sub-directory.

**Per-suite scoping** (browser and screenshot suites) resolves to a list drawn from a declared suite
order, and falls back to the **full list** in five distinct cases — unknown sub-directory, shared
package, suite's own directory, a file at the tree root, and no files of that kind at all. Each
fallback is a separate proof-obligation failure and each is a trap (§5.1).

### 4.2 The catalogue, as configuration

`CHECK_CATALOG` is a static list of 12 specifications. It is stripped; what it *expressed* is the
schema charkit's config must be able to state. Rendered as behaviour, with source names genericised:

| Check | Category | Workers | Footprint | Exclusive | Fixable | All-only | Runner | Timeout |
|---|---|---:|---:|---|---|---|---|---:|
| server lint | lint | 1 | 1 | — | yes | no | host | 600 |
| server format | lint | 1 | 1 | — | yes | no | host | 600 |
| migration drift | test | 1 | 1 | — | no | **yes** | host | 600 |
| server tests | test | 4 | **5** | `db` | no | no | container | **1200** |
| web lint | lint | 2 | 2 | — | yes | no | container | 600 |
| style lint | lint | 1 | 1 | — | yes | no | container | 600 |
| typecheck | **lint** | 2 | 2 | — | no | no | container | 600 |
| web unit tests | test | 4 | **6** | — | no | no | container | **1200** |
| browser e2e | test | 4 | **8** | `db` | no | no | container, **no host topology** | **1800** |
| screenshot diff | test | 4 | **6** | — | no | no | container | **1200** |
| engine lint | lint | 1 | 1 | — | yes | no | **host** | 600 |
| engine tests | test | 1 | 1 | — | no | no | **host** | 600 |

Four things the table cannot carry, all of which are decisions rather than data:

- **Typecheck is categorised `lint` despite being a compile step.** Category is a user-facing run
  selector, not a taxonomy. charkit's equivalent is the conventional-name set in `PLAN.md` §3.2.
- **Workers and footprint are two different numbers** and merging them is a documented outage
  (§5.3, SCH-1). The first is handed to the tool's own parallelism flag; the second is what the
  scheduler reserves.
- **The engine's own checks must stay on the host.** They are the suite that tests the container
  machinery, so running them through it lets a break in that seam take down the tests that detect
  it. Generalised: *a check engine's own suite must not run inside the containers it drives.*
- **Timeouts are per-check with a generous default.** The default's job is to turn "hung forever"
  into "failed loudly", not to police slow tools. charkit's default is 900 s (`PLAN.md` §4.1).

### 4.3 Admission control

Two orthogonal constraints over a slot budget and a set of named exclusive resources.

**The budget** is an explicit override when given, otherwise **detected cores minus two, floored at
one**. The reservation is not politeness: a check run shares the machine with the OS, the editor,
the agent driving the run, and the container VM. Budgeting every core targets 100 % utilisation by
design and the machine's owner then cannot use it — reported twice by the user before it was fixed.

**A slot is not a core.** One slot's work is often a whole build that will use every core it can see
inside its container. The budget bounds how many such units run at once; a separate per-service CPU
ceiling in the deployment topology bounds how much each may take. Both are required.

**Admission order** is descending footprint, ties broken by ascending id. Heaviest-first because a
large check admitted late waits for a large contiguous block; the id tie-break exists so two runs of
the same set schedule identically and a flaky run can be reproduced.

**Reservation** clamps the request to the total budget, waits until both the slots are available and
none of the requested exclusive names is held, then takes both **atomically** and returns the
granted figure. Release happens on every path including failure, and wakes all waiters. The granted
figure — not the declared cost — is what sizes the tool.

**A check whose cost exceeds the whole budget is clamped and run alone**, never rejected and never
left waiting. "Needs more than everything" is honestly read as "runs by itself".

There is **no fairness guarantee**; heaviest-first ordering is the only mitigation. Recorded as a
known limitation, not a required fix — charkit's `waiting_on` field (`PLAN.md` §3.1) surfaces the
symptom to the caller, which the original had no equivalent of.

### 4.4 Command construction and the runner seam

**The host-vs-container decision is declarative and per-check**: the check declares a runner, and a
global escape hatch can force everything to the host. Nothing is auto-detected — no probe chooses
the runner. A check declaring it has **no host topology** is *refused*, not silently run, because
running a browser suite against nothing produces an unrelated, misleading failure.

**Path translation is one-directional and applies to one path.** The run's artefact directory is
embedded as an absolute path in generated commands; on the container path it is re-rooted from the
host workspace root to the fixed container mount point, preserving the sub-path. Nothing is ever
translated the other way, and the translation is conditioned on *whether the check actually runs in
a container*, not on what it declares (§5.4, CMD-5).

**The container wrapper, as the *source* builds it**, adds in order: an ephemeral-run verb,
remove-on-exit, a **forced build** so a stale image cannot silently pass, an environment signal
marking "inside a check container", a domain-scoped environment signal selecting a shared database,
and the service name — after which the original command is passed through byte-identical. Working
directory is always the workspace root, because that is where the compose document lives and because
the project name is derived from it.

**charkit's shape is different, and only the last three of those transfer.** `PLAN.md` §4.1's `in:`
execs into a service that is *already running*, brought up by `needs:` / `char up`, so the
invocation charkit builds is an `exec -T` — no ephemeral-run verb, no remove-on-exit, no build flag.
What survives verbatim is: the environment signals are passed, the service is named, the original
command is passed through byte-identical after it, and `-T` is mandatory (`PLAN.md` §4.1 —
omitting it allocates a TTY and hangs). Image freshness does not disappear; it moves to whatever
brings the service up (§3, and §5.4 CMD-2).

**One logical check becoming several tool invocations** is a source-repo workaround that charkit
should not reproduce. That repo needed one check id to cover N workspace packages whose task runner
interleaved unparseable output, so it generated a shell script that ran the tool once per package,
redirected each report to its own file, latched the exit status, and concatenated the reports to
stdout. charkit expresses the same thing as configuration: N components each with their own check,
or one command that fans out itself. `PLAN.md` §4.1 argv-splits every `cmd:` by default and confines
`shell: true` to a per-entry opt-in, so a generated multi-command script is the shape the config
contract already rejects.

**Do not build the aggregation primitive.** Its single most dangerous trap (§5.4, CMD-6) becomes
unrepresentable if you don't.

### 4.5 The process seam

The original injects one callable; charkit has three seams behind a `Ctx`. The deadline,
process-group and abort behaviour has to survive that change of shape, and it is the most directly
portable area in the harvest.

**Spawn discipline.** Every check subprocess starts **in its own session** — new session, new process
group, no controlling terminal. A new session is what makes a *group* kill possible, and a group
kill is what makes a deadline end a process tree rather than one shell. This confirms
[`traps.md`](traps.md)'s measured entry that `killpg` against a `setsid`'d group reaches
grandchildren, and independently arrives at `setsid` over `process_group(0)`, which that file
measured as mutually exclusive.

**The deadline**, given *T* seconds: wait *T* for the process; on expiry signal the **group** with
SIGTERM, wait a grace period, escalate to SIGKILL; then **drain** the pipes with a separate bounded
ceiling; then synthesise a result that states it timed out. Omitting the deadline means no deadline
at all, not a default one.

**A timed-out result reports exit code 124** — GNU `timeout`'s code for "killed for running too
long", reused so the number means the same thing here as everywhere else on the system, and chosen
because the obvious alternative (a negative sentinel) collides with the source language's
died-by-signal encoding. In Rust that collision does not exist, but 124 stays free and stays
distinct from `128+N`, which is the property that made it a good choice.

**The result states its reason in text, not only in its code.** The consumer is usually an agent
that sees only a parsed summary, and "timed out" versus "the tool failed" call for opposite next
moves. charkit has a typed home for this — class `timeout`, exit 4 ([`ARCHITECTURE.md`](ARCHITECTURE.md)
§1.7) — so the *requirement* transfers and the in-band text does not.

**Partial output survives a timeout**, and this is a design requirement rather than a side effect:
what the tool emitted before it hung is often the only clue to *where* it hung. In Rust it must be
explicit — accumulate into buffers owned by reader threads from spawn, so the timeout path reads
accumulated state rather than issuing a fresh read.

**A registry of in-flight processes** exists, and it is the unavoidable cost of session isolation.
Before checks ran in their own sessions, a terminal interrupt reached the whole tree for free;
detached, it reaches only the supervisor and every server, worker and browser it started sails on.
Abort snapshots the registry, releases the lock, and group-kills each live entry.

### 4.6 Result parsing and failure reporting

**The contract**, which is the part that generalises: every parser returns a summary string and an
ordered list of failures. Parsing runs **only on a non-zero exit**; a zero exit yields nothing and a
timeout is never parsed at all.

**Message budget:** at most 3 lines per failure message, at most 240 characters per line, at most 10
failures carrying messages. Truncation is **never silent** — a trimmed message appends a line stating
how many were dropped, and a failure past the cap says its message was *omitted* rather than looking
identical to one that never had a message. ANSI escapes are stripped before trimming, and blank
lines are dropped before counting.

**Several JSON documents can arrive on one stream** when a command fans out. The scanner decodes one
value at a time and, on a decode failure *or a decoded scalar*, skips forward to the next brace or
bracket and retries. Both recovery cases are traps (§5.5).

**The post-parse guard is the load-bearing rule in this area.** If the exit code is non-zero, the
failure list is empty, *and* the summary is one of the benign "nothing wrong" strings, the summary is
replaced with the first meaningful error line from stderr-then-stdout. It is the **first** line, not
the last: for a crash the opening line is the error and the tail is stack frames.

**Per-tool field paths are trivia and are dropped.** charkit names arbitrary commands, so parser
selection must be declared in config and default to a generic fallback. What survives is the guard,
the budget, the scanner, the two-rules-for-two-questions split (first line for a crash, last line
for a normal summary), and the requirement that failures carry identifier and message as **separate
fields** — `PLAN.md` §3.1 already fixes that shape, and it makes the original's
recover-the-id-from-the-first-line trap unrepresentable.

### 4.7 Execution lifecycle

**Two phases.** Phase 0 runs fixers, and only when fix mode is requested; phase 1 runs verifiers.
The boundary guarantee is that **the entire fixer pool is drained** — every fixer exited, output
flushed, log written — before the first verifier is submitted, so a verifier never reads a tree a
fixer is mid-write on. Fixer verdicts are **discarded**; a fixer's failure is reported only through
the verifier re-discovering it. Fixer logs get a distinct filename so a fixer cannot clobber the log
its verifier is about to write.

**Skipped checks bypass admission entirely** and return immediately. A skip costs nothing and must
never queue behind a heavyweight check.

**One pool worker per check**, deliberately, so the pool cannot impose a second concurrency limit
that ignores declared costs and exclusive names. Admission is solely the scheduler's job.

**Cleanup runs from the abort path and can never change the verdict.** The case where stale state
matters most is the run that did not finish normally.

### 4.8 Live output

Enabled only when a flag is set **and** stdout is a real terminal. Colour is gated on the terminal
alone, with no flag — the primary consumer is an agent reading captured output, which must get zero
extra bytes.

**Initial state matters:** checks the skip predicate already rejects start as *skipped*, not
*queued*, and marking one running must never overwrite a known skip. Otherwise the table shows work
that will never run and never settles.

**Rows render in the original list order**, never re-sorted, and the status cell is padded *before*
colouring so escape bytes never count toward column width.

**The redraw model** is: render lines, emit a cursor-up by the previously printed line count, emit
each line prefixed with erase-to-end-of-line, flush, record the new count. Three things it must
never emit — a separator between the cursor-up and the first line's content, a line without
erase-to-end-of-line, and any output at all when not attached to a terminal. All three are traps
(§5.6) and two were reproduced against a real terminal emulator.

### 4.9 Run state, status and the lease

**Two documents, not one.** A *results record* written once after everything finishes, and a *live
pointer* written at run start and on every transition. The whole subsystem exists because there was
once only the first: a mid-flight poll read the **previous** run's `PASS`, and an agent concluded a
still-running or already-crashed run had succeeded.

**The state machine.** `idle` and `dead` are *derived at read time and never written*. `starting` is
published by a detaching parent before the child exists; `running` by the run itself before the first
check; `pass`/`fail` at finish; `aborted` by the signal handler. Terminal means the watcher stops
waiting.

**`dead` must be derived**, because SIGKILL and a vanishing sandbox never reach any handler. A
reported `running` whose pid is gone resolves to `dead`. A `starting` with no pid yet is tolerated
within a **startup grace**, and beyond it resolves to `dead` — without the grace every detached run
reads as already-failed for its first seconds; without the expiry a spawn that never produced a child
sits `starting` forever and blocks every waiter.

**Ownership.** Every mutation first checks that the document's run id equals its own and silently
discards the write otherwise. This is what stops a late write from one run overwriting another's
live status.

**Progress counting.** Skipped checks are excluded from both numerator and denominator. A timed-out
check counts as **done** and as **failed** — done so progress can reach completion, failed so a hang
can never report green.

**The lease.** Its point is the **identity record** — without one, the only answerable question is
"does a lock exist", which is not the question anyone has. Liveness is a **conjunction**: the
recorded pid resolves to a live process **and** the heartbeat is fresh. Either half alone
reintroduces a documented outage — a dead run poisoning the workspace permanently, or a recycled pid
holding a lease forever. The heartbeat is refreshed on every check transition from the same
timestamp as the transition, so a legitimately long check never looks hung.

charkit's store is machine-global SQLite with lease rows (`PLAN.md` §4.3), not a lock directory. Most
of the above is **a requirement expressed in the wrong medium**: the requirement survives, the
directory does not. §5.8 marks which is which.

**Reaping a dead holder's process group** is permitted only when the record exists, both pid and
group id are integers, **the group id equals the pid**, and the pid is confirmed dead. That third
condition is the safety interlock: a detached run creates its own session and is a leader, but a
foreground run shares its invoking shell's group, and killing that takes out the user's shell.

### 4.10 `--again`, detach, entrypoint

**`--again` selects every check whose prior status was failed *or timed out*.** Including timeouts is
deliberate: a check killed without producing a verdict is the most important one to retry. The merge
keeps every non-rerun entry byte-identical and recomputes the overall verdict from the **merged**
map, so a rerun of one now-passing check cannot report a whole-run pass while three others still
fail. A staleness hint warns when currently-changed files fall outside the scope the rerun covered —
a green rerun does not mean the tree is green.

**Detach ordering is the specification**, and every step is a separately confirmed defect: refuse on
a live lease **before touching any shared state**; strip the detach flag from the forwarded arguments;
publish `starting` **before** spawning; spawn in a new session with output to a file and the run id
injected into the child's environment; patch the child's pid in **only if** the document is still
this run and still `starting`.

**The child adopts the parent's run id** rather than minting its own, or the handle the command hands
back correlates with nothing.

**Read verbs never take the lease.** Status and wait must not block on the run they exist to report
on. Status always exits 0 — it is a query, not a verdict. charkit already states this rule
(`PLAN.md` §3.1, and the read-verb row in [`AGENTS.md`](../AGENTS.md)); the original arrived at it
independently.

**The abort handler's ordering is guaranteed**: restore the default signal disposition **first**,
before any work that can block; kill the children **before** publishing `aborted`; then publish; then
re-raise so the exit status still reflects the signal.

---

## 5. Traps and bug-shaped branches

The list. **Marked** says whether the source flags it: **D** = commented with a dated incident,
**C** = commented, **S** = silent. The silent ones are the point — a reviewer of a rewrite has
nothing to notice their absence by.

### 5.1 Scope and selection

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| SC-1 | Bare `--changed` resolves to a sentinel, not to the literal default branch name | D | The two states become one string. In a clone with only a remote-tracking default branch — the CI gate's own worktree — every diff fails with *unknown revision*, on every run |
| SC-2 | Default ref prefers the local branch, falls back to remote-tracking, decided on exit status | D | Same crash. Reversing the direction silently diffs against a stale fetch |
| SC-3 | The engine's own tree is a registered owned tree | D | A one-line edit to the engine falls through *both* exclusion rules and schedules the entire suite — which then **failed on resource contention**, on tests that passed in isolation seconds later. The missing entry produces false failures, not just waste |
| SC-4 | The prose tree is a registered *inert* tree, checked in addition to owned trees | D | A prose file looks as ambiguous as a root config file and trips the conservative fallback. Observed: a three-file change including one note scheduled the complete suite |
| SC-5 | Unrecognised paths schedule everything | C | This is *why* classification is conservative. Exact per-file attribution silently skips the checks a shared config file breaks |
| SC-6 | Root-level prose files are **not** inert; only files inside the declared inert tree are | C | Widening to "any prose file" skips everything for a change to a root instruction file that does change behaviour |
| SC-7 | The engine's own domain uses a **positive** membership test, not the exclusion rule | D | With the exclusion rule its suite runs on every unrelated diff. With no rule at all — the state before the fix — editing the engine and running the gate reported **PASS having executed zero of its ~200 tests** |
| SC-8 | A file directly at a tree root forces the whole-tree scope, returning immediately | C | Shared infrastructure affects every sub-directory; per-directory scoping runs a fraction of the suite and passes |
| SC-9 | An empty sub-directory set also returns the whole tree | **S** | Reachable when a domain is in scope only because something *else* was ambiguous. Without it the tool gets an empty path list and lints the process working directory, or nothing |
| SC-10 | Rename lines take the path **after** the separator | C | The old path enters scope, no longer exists, and the tool hard-errors — while the new path is never checked |
| SC-11 | Status paths are extracted at a fixed offset and **not unquoted** | **S** | Correct for every status code, but a path with a space or non-ASCII byte arrives quoted and escaped. Known gap, not a fix — charkit reads NUL-delimited output (`PLAN.md` §4.1) and avoids it entirely |
| SC-12 | Empty scope means *everything in scope* in all five classifiers | **S** | A branch with no diff must not report `PASS` having run nothing |
| SC-13 | `.`, `""` and `./` short-circuit to the current directory | C | `.` contains no separator, falls through to branch resolution, and is rejected with *"is not a branch checked out in any worktree"* — for the most natural programmatic argument. Confirmed live against the MCP caller |
| SC-14 | Path comparison canonicalises **both** sides but returns git's recorded spelling | **S** | Without canonicalisation, symlinked temp roots never match. Returning the canonical form instead makes every downstream comparison against git's own output diverge |
| SC-15 | Suite scoping falls back to the full list on an **unknown** sub-directory | C | A newly added application has no table entry, contributes nothing, and its coverage silently vanishes |
| SC-16 | …and on a shared package, the suite's own directory, or a file at the tree root | C | Shared code can break any suite; attributing it to specific ones is the wrong direction with no dependency graph |
| SC-17 | …and when there are **no** files of that kind at all | C | Reads backwards but is load-bearing: when the domain is in scope only because something was ambiguous, returning empty skips the suite on exactly the change most likely to break it |
| SC-18 | A sub-directory with no suite of its own maps to the shared cross-cutting suite | C | Mapping it to nothing skips its only coverage; treating it as unknown re-runs everything |
| SC-19 | Suite output is re-ordered into the declared order, not accumulation order | C | Non-deterministic argv; cache keys and golden outputs both move |
| SC-20 | Screenshot scoping **continues** past an uncovered sub-directory rather than returning false | **S** | This is what makes "uncovered + covered ⇒ in scope" work. An early false drops the covered one |
| SC-21 | Preparation steps are gated on the **scheduled check set**, not on scope flags | C | A check can be scheduled regardless of which files changed; the two are different questions, and gating on the wrong one skips preparation and then fails every check that needed it |

### 5.2 The catalogue

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| CAT-1 | Category must partition into exactly two values | C | An unvalidated mode string returns an empty set and the run silently checks nothing |
| CAT-2 | The engine's own checks stay on the host | D | They are the suite that tests the container machinery; running them through it lets a break in that seam take down the tests that detect it. Documentation claimed "every check runs in a container" while five never did |
| CAT-3 | Every check declares a **positive** time budget | D | A check sat **26 minutes** inside a browser runner with zero child processes and nothing listening — its supervised servers had died and the runner never noticed. Nothing bounded it, and because it held the run lease throughout, no other run could start |
| CAT-4 | Deployment topology declares a per-service CPU ceiling below machine width | D | A ceiling equal to the core count is not a ceiling. The limits were *decided* on one date and two days later still did not exist while the machine sat at 100 %+ |

### 5.3 Admission control

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| SCH-1 | **Scheduling footprint is a separate number from worker count** | D | A containerised check declared 4 workers but booted six services with them. Weighted at 4 on a 10-slot budget the scheduler co-scheduled another check beside it and two **synchronous** tests failed — in a container where they pass 820/820 alone. **Starvation presents as an ordinary test failure**, which is why it costs so much to diagnose |
| SCH-2 | Raising the footprint must **not** raise the worker count | C | The obvious fix for SCH-1 is to raise the cost — which is handed straight to the tool's parallelism flag, so the check spawns *more* work. This is why the two numbers cannot be merged |
| SCH-3 | A second check under-declared the same way, in the opposite role | D | Over-admitted work beside it made unrelated suites fail with a placeholder stack trace and **no assertion message** — the signature of a worker that never reported back. All of them passed in isolation |
| SCH-4 | Admission orders by footprint, not by cost | C | A check with a small cost and a large footprint is admitted last, then blocks waiting for a contiguous block everything else has already filled |
| SCH-5 | Deterministic tie-break on id | C | Two runs of one set schedule differently and a flaky run cannot be reproduced |
| SCH-6 | Over-budget requests are **clamped**, not queued | C | A check declaring more than the machine has waits forever for capacity that cannot exist. Deadlock, not a slow run |
| SCH-7 | The clamped grant is returned and sizes the tool | C | Without it a check admitted against a clamped budget still launches its full declared parallelism, oversubscribing the machine the clamp protects. **Only the first half is portable — see §10.** charkit's contract has no way to carry the grant into a command, so no case pins the sizing and the clamp is decorative as specified |
| SCH-8 | Slots and exclusive names are taken in **one atomic step under a single guard**, released in a finally, waking all | partial | The source states no acquisition order because it has none to state — the whole reservation is one step, and its comment argues for *eliminating* the ordering question rather than answering it. The transferable rule is that **no part of a reservation is ever held while waiting for the rest**. Releasing outside a finally leaks slots and locks on a panic. Waking one leaves a waiter blocked on a *name* never re-evaluated after a *slot* release |
| SCH-9 | The wait predicate is re-checked in a loop after every wake | **S** | A single conditional admits two checks past the budget on a spurious or multi-waiter wake |
| SCH-10 | Cores are reserved rather than fully budgeted | D | The machine sits at 100 %+ while its owner is trying to work. Reported twice |
| SCH-11 | Explicit worker flags override each tool's own auto-sizing | C | Every parallel tool defaults to one worker per core; the budget means nothing if the tools ignore it. **No ported case pins this, and that is not an oversight — see §10.** In charkit the flag is entirely the config author's literal, so the engine can neither require it nor correct it |
| SCH-12 | An exclusive lock deleted when containerisation removed the sharing was **re-added under a new name** when a different sharing edge appeared | D | Containerising removed one shared resource but the stack still depended on a singleton service on a fixed port; two concurrent runs raced and the loser failed with *port already allocated*. **General rule: a lock is a sign two checks share something. Prefer removing the sharing — but confirm it is gone, not merely moved to a different dependency edge** |
| SCH-13 | A different retired lock is documented as **must not be re-added** | C | Its cause has already moved; re-adding it treats a symptom whose source is gone |
| SCH-14 | Moving a check back to the host must **re-declare** its exclusive resource | C | The locks were safe to remove *only because* those checks run in containers. Flipping one back without restoring its lock silently reinstates the original corruption, and the symptom is unrelated tests failing in the gate while passing in isolation |
| SCH-15 | A container-declared check must name a service | C | A malformed invocation at run time instead of a config error at load time |

**On SCH-8 and `PLAN.md` §4.3's acquisition order — they are not in conflict, and `PLAN.md` wins
anyway** ([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.8; this document is not in that precedence list).
Four things the implementer should have:

- **`PLAN.md` §4.3's sorted-exclusives-then-slots ordering solves a problem the source never had.**
  The source's mechanism is entirely in-process and in-memory, and no named resource crosses a
  process boundary, so there is no cycle for an order to break. SCH-8 is evidence about atomicity
  and release, not about ordering.
- **The source's only cross-process exclusion is its per-checkout whole-run lock** (ST-14 to ST-20),
  and that lock is precisely why an in-memory scheduler was ever sufficient there. Remove it and the
  scheduler's guarantees stop at the process edge.
- **The source's own dated TODO — the one recorded as X-1 — is that in-memory mechanism failing to
  cover a resource with real extent beyond the process.** It notes the container tool holds no
  cross-process lock of its own. That is independent empirical support for `PLAN.md` §4.3's premise,
  from the very system SCH-8 is harvested from.
- **Multi-name acquisition is unexercised in the source.** No test passes more than one exclusive
  name and no catalogue entry declares more than one, so the source's clean record on ordering is
  not evidence of safety for charkit, where `exclusive:` is a machine-wide list.

**Build `PLAN.md` §4.3's order.** SCH-8's contribution is the invariant it shares with it — nothing
is held while the rest of the reservation is pending — reached there by atomicity and here by a
total order across both lease classes.

### 5.4 Command construction and the runner seam

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| CMD-1 | The git-aware package filter is **dropped inside a container** and kept on the host | D | A worktree's git directory is a *file* pointing outside the bind mount, so the container has no usable git and the check fails with *not part of a git repository*. The asymmetry is what makes the fix safe: dropping it is merely less selective, keeping it is always fatal |
| CMD-2 | Every container run **forces a build** | C | A stale image silently tests the wrong code the moment a build input moves. Green on stale code is indistinguishable from real green. **Phase 4, not phase 3.** The source *runs* an ephemeral container per check, so the build flag rides on the check's own invocation; charkit **execs into an already-running service** (`PLAN.md` §4.1's `in:`, a `docker compose … exec -T`), and an exec has no build flag and no image of its own to freshen. The rule survives unchanged — a stale image must never silently pass — but it lands on whatever brings the service up (`needs:` / `char up`) beside CMD-3, not on the check engine. Nothing in a check invocation asserts it |
| CMD-3 | A **dependency** image whose code is baked in is force-built **separately** | D | A rebuild flag on the run covers the run target only; compose's default for a dependency is build-only-if-missing. An image built before a fix commit was reused for the rest of the day, so the browser check kept failing the exact assertion the fix targeted while the server suite passed against the same commit. 77 minutes lost across two review rounds |
| CMD-4 | The no-host-topology case both **refuses** and **skips** | C in test | With only the refusal, the error escapes the executor and **discards every other check's results for the run**. With only the skip, a direct caller gets a silent nonsense command. Both are needed |
| CMD-5 | Path translation keys on whether the check *runs* in a container, not on what it *declares* | **S** | A forced-host run writes reports into a host directory named after the container mount point — a stray absolute top-level directory — and reads nothing back |
| CMD-6 | The per-item exit status is latched **before** the report is concatenated to stdout | **S** | Concatenating an existing file always succeeds, so reversing two lines makes every aggregated check exit **zero** — every violation and every test failure reported as a pass. **The highest-severity silent branch in the harvest.** charkit avoids it by not having the mechanism (§4.4) |
| CMD-7 | The artefact directory is created on the host **before** the command is built | C | A containerised check writes its report through the bind mount into a directory that does not exist; the report lands nowhere and the check fails with nothing parseable |
| CMD-8 | Positional paths are re-based when the invocation changes directory first, with a whole-tree sentinel | C | The tool is handed a path relative to the wrong root, finds nothing, and reports a clean pass over zero files. Silent false green. charkit's per-check working directory makes this unnecessary rather than load-bearing |
| CMD-9 | A domain-scoped database signal is passed, and only to that domain | C | The consumer branches on it and never probes, so it must be *passed*. Without it the containerised check boots throwaway database containers *inside* the check container. Passing it to other containers is a lie about what they do |
| CMD-10 | A generic "inside a check container" signal is passed to every container | C | Host-only code shells out to git against a path that does not exist in the container and prints a fatal-looking line **once per worker** inside an otherwise-green run |
| CMD-11 | An unparseable package manifest is skipped during discovery | **S** | Listed as a *defect*, not a fix — see §7 |
| CMD-12 | Browser suites are invoked **directly**, never through a package-script indirection | C, epitaph | Forwarding a reporter flag through a script indirection passed a literal end-of-flags separator, so the reporter flag became a positional file filter, the run fell back to a human-readable reporter, and a genuine **15-test failure was masked** behind a parse error |
| CMD-13 | Suites are chained with **no wait step** between them | C | The wait loop existed when each suite booted its own server on one port; it is gone because the topology changed. Keeping it costs dead time and a host tool the container lacks — but the race returns if the topology regresses |
| CMD-14 | The full documented command is invoked, never a synthesised faster variant | D | Build-time-inlined configuration means a build-skipping variant serves whatever artifact exists, built under whatever environment was active then. Symptom: every smoke test times out waiting on a request the stale bundle never makes. **Generalised: never synthesise a faster variant of a documented command** |

### 5.5 Result parsing

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| PAR-1 | **A non-zero exit may never report a benign summary** | D | A bad flag, missing module or crashed worker aborts the tool before it runs anything; the parser sees no failures and says "no failing tests"; the report shows `FAILED` beside it. Confirmed live — one invalid flag aborted **all nine** packages and the report said exactly that. Applies to every tool, because every parser has a benign summary |
| PAR-2 | The crash summary takes the **first** meaningful line, not the last | C | The crash's error is the opening line and the tail is stack frames. Reusing the normal-summary rule yields a stack frame as the diagnosis |
| PAR-3 | Build-progress chatter is excluded from the crash-line scan | D | A containerised check's output opens with dozens of progress lines, so "first non-empty line" picks one of them; the real cause was thirty lines further down. **No test reaches this code — see §9** |
| PAR-4 | Report-level errors are surfaced **only when no suite has any spec** | C | Omit the branch and a pre-run failure — a configured server failing to boot — produces zero suites and reports "no failing tests" for a run that never started. Omit the *condition* and a benign warning overrides genuine results, so every green run reads as a failure |
| PAR-5 | A **decoded scalar** is skipped like a decode failure in the stream scanner | C | A bare digit in preamble text (a version string) decodes as a valid document; yielding it desyncs the scan and feeds a non-object to a parser that assumes one, losing every subsequent report. **No test covers this — see §9** |
| PAR-6 | A decode failure **skips forward** instead of aborting | C | One package printing a telemetry banner before its output makes a strict parse fail, and **every other package's real failures become invisible** |
| PAR-7 | Failures past the message cap say the message was *omitted* | C | Otherwise a capped failure is byte-identical to one that never had a message, and the reader cannot tell truncation from absence |
| PAR-8 | A trimmed message states how many lines it dropped | C | Three lines of a twelve-line trace read like the whole error; the reader concludes they have seen everything |
| PAR-9 | Escapes are stripped before trimming; blank lines dropped before counting | C / **S** | Escapes consume the character budget and can leave a dangling sequence colouring the rest of the terminal; counted blank lines spend the budget on nothing so the actual assertion never appears |
| PAR-10 | The message cap applies **across** concatenated documents, not per document | C | An N-package run emits up to 10×N messages and the budget is silently multiplied |
| PAR-11 | Empty output is **not** a parse failure | **S** | Reporting a parse error for an empty stream masks PAR-1, which only rewrites *benign* summaries — a crash with no stdout would report a parse error instead of its actual stderr |
| PAR-12 | A missing pass/fail marker on a spec is treated as **passed** | **S** | Treating absent as failed reports every spec in that report shape as a failure; the whole suite reads red |
| PAR-13 | Two different fields are read for a message, because versions populate one or the other | C | Read only one and half the installed versions produce failures with titles and no messages |
| PAR-14 | A rerun hint is built from **bare identifiers**, passed **positionally** | C | Splicing a whole multi-line failure into a command makes indented assertion text into argv. And the earlier form used a name-matching selector, so a path-qualified identifier never matched and the suggested rerun **silently ran zero tests and looked like a pass** |
| PAR-15 | The message lookup key is a **mangled** form of the identifier | C | The diagnosis lives in a different section keyed differently; keying by the raw identifier makes class-based tests silently lose their assertion text while others keep theirs |
| PAR-16 | Any new section banner ends the diagnosis section | C | Lines from a later section attach to the last failure, or a bucket accumulates past its section |
| PAR-17 | A remediation hint matches on **failure text**, not on "the check failed" | C | Advising regeneration on an ordinary visual regression is actively harmful — regenerating erases the regression you were meant to look at |

### 5.6 Execution and live output

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| EXE-1 | Phase 0 is fully drained before phase 1 begins | C | A verifier reads a tree a fixer is mid-write on; verdicts become timing-dependent |
| EXE-2 | Fixer verdicts are discarded and fixer logs are named distinctly | C | A fixer's exit code appears as a check result, and a fixer clobbers the log its verifier is about to write |
| EXE-3 | The fixer population is filtered by the **same** skip predicate | **S** | A check skipped for scope is still auto-fixed, mutating files nothing in this run was asked to touch |
| EXE-4 | Skips bypass admission | C | A free skip queues behind a fifteen-minute check and the table sits at "queued" on work that will never run |
| EXE-5 | The pool is sized to the number of checks | C | The pool re-imposes a second concurrency limit that ignores costs and exclusives, and the two disagree |
| EXE-6 | Cleanup runs from the abort path and never changes the verdict | C | A cleanup hiccup turns a passing run into a failure with an unrelated cause; and the aborted run — where stale state matters most — leaves the machine dirty |
| VIS-1 | Skipped checks start as **skipped**, not queued | C in test | Checks that will never run sit at "queued" and the table never settles |
| VIS-2 | Marking running **never overwrites a known skip**, and returns before redrawing | C | The skip flickers to running and back; the run appears to execute work it declined |
| VIS-3 | **No separator between the cursor-up sequence and the first line's content** | C, reproduced on a real terminal | The cursor drops a row before a byte is written; the table drifts one row per redraw and the drift compounds into dozens of stale header lines |
| VIS-4 | Redraw is serialised by a **dedicated** lock covering the write *and* the line-count update | D | Three independent sources call redraw; two writers computing cursor-up from a stale count desync the terminal from reality |
| VIS-5 | Every line carries erase-to-end-of-line | **S** | A shorter line leaves the tail of the previous longer one visible — a stale duration that looks current |
| VIS-6 | The first redraw emits no cursor-up | **S** | Cursor-up-by-zero either errors or scrolls existing terminal content into the table |
| VIS-7 | Stop joins the ticker with a bound, then redraws once more | **S** | A wedged ticker hangs the run at exit; without the final redraw the last state on screen is stale and may show a check still running |
| VIS-8 | The ticker is a daemon thread | **S** | It keeps the process alive after the run finishes |
| VIS-9 | Colour and the live table are gated on a real terminal | C | Escape bytes pollute every agent's captured output and the cursor control corrupts a piped log |
| VIS-10 | The status cell is padded **before** colouring | **S** | Escape bytes count toward column width and every row after the first misaligns |
| VIS-11 | The final report omits its own table when the live table ran | **S** | The same table prints twice |

### 5.7 Service lifecycle — **phase 4, not phase 3**

Recorded in full because there is no second harvest. Do not build these now.

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| SVC-1 | Teardown names services **explicitly** and never uses the project-wide down verb | D | A developer's interactive stack is destroyed by a check run that merely borrowed its database |
| SVC-2 | …but teardown **does** remove a service this run started | D | Finished runs strand services on their ports; the next run's service never starts and checks fail in under six seconds with startup noise instead of test results. Because each CI sandbox is a fresh directory, the port allocator restarts from zero every run, **guaranteeing** the collision |
| SVC-3 | Liveness matches on **exact token equality**, not substring | C | A service whose name merely *contains* the database name makes the run believe it inherited a service it did not, and SVC-2's incident returns silently |
| SVC-4 | Liveness is sampled **before any check runs** | C | Sampled at teardown, the run's own service is up and reads as inherited; nothing is ever cleaned |
| SVC-5 | An unreachable daemon reads as "not running" | C | The safe direction: the run then owns what it starts rather than stranding it |
| SVC-6 | Teardown swallows failures and never raises | C | A cleanup hiccup fails a passing run for a cause unrelated to anything under test |
| SVC-7 | The volume sweep filters on **dangling *and* an anchored anonymous-name shape** | D | Dangling alone deletes the developer's named database volume to reclaim space. Shape alone attacks volumes a running container uses. Neither is sufficient |
| SVC-8 | The sweep exists at all, and runs every run | D | Anonymous volumes accumulated to 165 volumes / 9.2 GB, after which the containerised database failed 36 tests with a disk-full error — **a disk problem wearing the costume of a code regression** |
| SVC-9 | The daemon is not probed unless a container check is scheduled | C | A lint-only run launches a desktop application it has no use for, costing minutes |
| SVC-10 | The daemon check is the **first** thing in provisioning | C | The failure surfaces as a connection error from whichever command ran first, naming a sweep or a config file rather than the actual problem |
| SVC-11 | Auto-start is confined to the one platform it was tested on | C | Guessing service-manager unit names elsewhere produces a confidently-executed wrong command instead of an honest error |
| SVC-12 | Renamed-container husks are swept before anything starts | D | One husk from an earlier failed recreate makes every later run die before a single check, with an error naming neither the problem nor the fix, that retrying can never clear |
| SVC-13 | The husk sweep is scoped by project **label**, never a name prefix, and requires proof of displacement | C | A prefix scope deletes another project's containers; without the proof, a live renamed container is killed |
| SVC-14 | Provisioning runs **unconditionally**, not only when its artefact is missing | D | An existing file short-circuits provisioning, the process never learns its port block, and the service binds a port another checkout already holds |
| SVC-15 | Ports are **read** from the allocator's output, never recomputed from its formula | C | A sibling implementation reimplemented the formula, got it wrong, and had to be corrected |
| SVC-16 | The **target's own** copy of the provisioning script is invoked | D | The script identifies which workspace it is provisioning from its own location, so another checkout's copy reports the wrong block and the caller binds a held port |
| SVC-17 | The artefact's existence is verified after provisioning, with its own error | **S** | Provisioning can exit zero without producing it; the service then refuses to start with a message nothing explains |
| SVC-18 | Dependency install runs **always**, never gated on the directory existing | D | "Exists" is not "current". A present-but-stale tree fails checks with a module-not-found that reads like a real code bug on an unrelated change |

### 5.8 Run state, lease, detach

Marked **R** where the behaviour is a *requirement charkit's SQLite lease store must still satisfy*,
and **M** where it is an artefact of the old mechanism that the store makes unnecessary.

| # | Branch | Marked | R/M | What breaks if absent |
|---|---|---|---|---|
| ST-1 | A live pointer distinct from the results record | D | R | A poller reads the **previous** run's `PASS` mid-flight. The founding defect of the subsystem |
| ST-2 | Atomic publish (write-then-rename) | C | M | A poller catches a half-written document |
| ST-3 | **The temporary name is unique per write** | D | M | With one fixed temp name, concurrent writers race: one renames the shared temp away, the next gets a missing-file error from its own rename, and **the whole run dies at 0/12 checks.** A shared temp name reads as perfectly correct in review |
| ST-4 | Read-modify-write is one critical section, under a **re-entrant** lock | C | R | Concurrent transitions drop the loser, leaving a check showing `running` forever. Re-entrancy matters because the abort handler runs on the main thread and can fire while that thread already holds the lock — a plain mutex self-deadlocks and converts an interrupt into a hang |
| ST-5 | Reads are total — missing, malformed and I/O-error all yield "no state" | **S** | R | A status query crashes exactly when the run is in trouble |
| ST-6 | Every mutation proves it owns the document first | D | R | A late write from one run silently overwrites another's live status |
| ST-7 | `idle` is a real, named answer with a terminal flag | C | R | A watcher cannot tell "nothing is running" from "I could not tell" — opposite correct responses |
| ST-8 | A vanished pid resolves to `dead` **at read time** | C | R | A killed run says `running` forever and every waiter hangs |
| ST-9 | The startup grace, **and** its expiry | C | R | Without the grace every detached run reads `dead` for its first seconds. Without the expiry a spawn that never produced a child blocks every waiter forever |
| ST-10 | Timeout counts toward **done** | C | R | Progress sits one short of total forever — an in-progress run with nothing running, the exact report the ceiling exists to end |
| ST-11 | Timeout counts toward **failed** | C | R | A hang reports green |
| ST-12 | Skipped counts toward neither | **S** | R | A scoped run can never reach completion and the denominator lies |
| ST-13 | A permission error on a liveness probe means **alive** | C | R | Another user's run has its lease stolen because you cannot signal it |
| ST-14 | Liveness is a **conjunction** of live pid and fresh heartbeat | C | R | Either half alone reintroduces a documented outage: a dead run poisoning the workspace permanently, or a recycled pid holding a lease forever |
| ST-15 | An unreadable owner record counts as reclaimable | C | R | A run that died mid-write leaves a lease nobody can ever reclaim |
| ST-16 | The heartbeat is refreshed from the transition's own timestamp | C | R | A legitimately long check looks hung and has its lease stolen mid-run |
| ST-17 | Liveness and reclaim run **only after losing** the atomic-create race | C | M | Checking first opens a window where both racers see no lock and both proceed. The store's conditional update is the same shape |
| ST-18 | After a reclaim, loop back to a fresh acquire | C | M | Two processes reclaiming the same stale lease both acquire it |
| ST-19 | The blocking error **names the holder** and the commands that watch or wait | C | R | "Locked" without identity is unactionable |
| ST-20 | Reap the dead holder's group — **only** when it is a true group leader | C | R | Orphans hold the ports the next run needs. But a foreground run shares its shell's group, and reaping that kills the user's shell. **The highest-consequence conditional in the harvest** |
| ST-21 | The waiter checks terminality **before** the deadline each iteration | **S** | R | A run finishing exactly at the deadline is reported as a timeout despite having a verdict |
| ST-22 | The waiter returns promptly on a dead run | C | R | It burns its full ceiling — up to thirty minutes — on a run that died in the first second |
| ST-23 | Status always exits 0; wait exits on the verdict | **S** | R | A query's exit code is about the query. charkit states this rule already |
| ST-24 | Status and wait never take the lease | C | R | They block on the very run they report on |
| ST-25 | `--again` includes **timed-out** checks | C | R | A killed check is never retried and silently inherits its stale timeout while the rerun reports clean |
| ST-26 | The merge preserves non-rerun entries byte-identical | **S** | R | The record stops being a faithful union |
| ST-27 | Overall is recomputed from the **merged** map | **S** | R | A rerun of one now-passing check reports a whole-run pass while three others still fail |
| ST-28 | The staleness hint spans **all** prior scopes, not just the rerun's | **S** | R | Files covered by a passing check are reported as newly changed on every rerun |
| ST-29 | A structured request **proves the record is its own** | D | R | A request that failed before running anything returned the **previous** run's results as a confident verdict for a run that never happened |
| ST-30 | The mismatch response is a distinct shape saying explicitly not to read it as a verdict | C | R | A caller treats an empty check map as "everything passed" |
| ST-31 | Detach **refuses on a live lease before touching any shared state** | D | R | Detach overwrote a healthy run's live pointer, the child then died on the lease seconds later, and the whole thing reported `dead` — one mistimed detach wrecked a healthy run's state *and* mislabelled its own failure |
| ST-32 | Detach publishes `starting` **before** spawning | D | R | During interpreter cold start the pointer still describes the previous run, so detach-then-immediately-status reads a stale terminal verdict |
| ST-33 | The detach flag is stripped from forwarded arguments | C | R | The child detaches again, forever |
| ST-34 | The child adopts the parent's run id | C | R | The printed handle and the reported id disagree, and the handle is worthless |
| ST-35 | The pid is patched in **only if** still this run and still `starting` | **S** | R | A child that already published `running` is dragged backwards |
| ST-36 | The child gets a new session | C | R | It dies with the terminal — and, since leadership is what licenses reaping (ST-20), its orphans could never be cleaned |
| ST-37 | The run id is minted **before** the lease is taken | C | R | The lease records an unknown id and a reclaim message correlates with nothing |
| ST-38 | Callers are told to watch the status surface and **never tail the log** | C | R | The log is not a state surface; a run that dies leaves it silent forever |

### 5.9 The process seam

| # | Branch | Marked | What breaks if absent |
|---|---|---|---|
| PRO-1 | Each check spawns in its **own session**, and the kill targets the **group** | C | Killing only the child left a build shell and a job worker alive holding ports, so the next run failed to bind and blamed its own topology. **Confirms** [`traps.md`](traps.md)'s measured `killpg`-reaches-grandchildren entry |
| PRO-2 | `setsid`, not "new process group" | C | Independently arrives at the choice [`traps.md`](traps.md) measured as forced — the two are mutually exclusive |
| PRO-3 | The post-kill drain is **bounded** | C, with a deliberate reversion experiment | An unreaped grandchild holds the write end of the pipe, so **no end-of-file ever arrives** and an unbounded drain blocks forever — the timeout mechanism *becoming* the hang it exists to prevent. It did not merely fail the suite; it hung the suite |
| PRO-4 | The streaming runner's deadline is enforced by **waiting on the process**, never by the read loop | C | A tool that has stopped emitting entirely still holds its pipe open, so a line-driven loop blocks forever and never reaches a clock check. That silent hang is exactly what the timeout is for, and `--verbose` would be the one mode where a check can run forever |
| PRO-5 | Partial output survives the timeout | C | It is often the only clue to *where* the tool hung. In Rust this must be explicit: accumulate from spawn, do not re-read after the kill |
| PRO-6 | A timeout is a distinct verdict with a distinct code, stating its reason | C | A parser handed a hung tool's partial output reliably reports "no failing tests" — a passing check that somehow failed |
| PRO-7 | The registry shrinks on **every** exit path | C in test | A registry that only grows has abort signalling long-dead pids, which the OS recycles onto unrelated processes |
| PRO-8 | The abort path snapshots under lock, **releases**, then kills | C | Holding the lock across the kill is what makes re-entrancy necessary at all. Doing it this way is the structural fix, and it is better than a re-entrant lock — which Rust's standard library does not offer |
| PRO-9 | Children are killed **before** `aborted` is published | C | A waiter is told the machine is clear while servers still hold its ports |
| PRO-10 | The handler restores default signal disposition **first** | C | Everything after it acquires locks; a handler that blocks while still installed swallows every further signal, and the run becomes unkillable by anything but a hard kill |
| PRO-11 | The handler re-raises the signal at itself | C | The process exits cleanly and the caller believes it was a clean exit. Interacts with [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.6's signal carve-out — re-raising after restoring the default is how you get `128+N` |
| PRO-12 | A non-zero exit **never raises** | C | A failing check is data, not an exception. This is what lets every caller stay a plain function |
| PRO-13 | One convenience call is **prohibited outright** for checks | C in test | The standard "run with a timeout and give me output" helper neither creates a session — so its timeout kills the direct child only — nor bounds its own cleanup read, so a surviving grandchild makes it block **forever**. The Rust analogue is any wait-with-timeout on a `Command` that did not `setsid` |

---

## 6. The Playwright traps — the plan corrected

The plan recorded two, one in each file, and attributed the snapshot trap to `baselines.py`. The
attribution was right and the count and the description were not. This section is the evidence
behind both corrections, and — since the plan's source-material table has been retired — the
only place they are recorded.

### 6.1 There are three distinct traps, and the shared one is the live one

| Trap | Where | Status |
|---|---|---|
| **Unbounded browser workers manufacture failures** | Both `check.py` and `baselines.py` | **Live.** One root cause, discovered and fixed twice independently |
| **Snapshot-write inversion** | `baselines.py` | **Live** |
| **Flag forwarding through a script indirection** | `check.py` | Historical; preserved as a regression assertion |

**Worker starvation is the dangerous one.** Measured in the source: the screenshot suite reported
**17 of 29 specs failing** with workers unbounded, and the same specs passed in isolation; bounded to
the check's declared cost the run went to 28/29 and from 1.9 minutes to 24 seconds. It manufactures
failures indistinguishable from real visual regressions — there is no signal separating them. In
charkit this is fixture configuration (`cost:` plus an explicit worker flag in `cmd:`), not engine
code, exactly as the plan says the Playwright traps should translate.

**Corollary, also live:** a worker flag passed unconditionally to every package is only safe while
the whole workspace is on one major version of the tool. It briefly was not — on the older major that
flag makes the runner collect **zero** test files and exit non-zero, silently dropping that package's
tests. The chosen fix was to pin the invariant with a test rather than special-case the builder.

### 6.2 The snapshot trap is inverted from the plan's description

The plan says the default update mode writes an absent snapshot and passes, so a first containerised run
reported 29/29 having compared 17 brand-new images against themselves. That is true of the tool's
default and it is the right lesson — but the source repo's config already reads
`updateSnapshots: CI ? "none" : "missing"` and its check image sets `CI=true`, so **the check side is
already defended.**

The trap bites the **generation** side: the aid that writes missing baselines must *deliberately clear
`CI`* to be allowed to write at all. Without that, generation runs, exits without error, and produces
**zero baselines** — a silent no-op rather than a false pass. Both failures are silent; they are
different failures.

**This is the one baselines finding with a real engine consequence:** an `env:` override whose value
is the **empty string** must be transmitted as *set to empty*, not pruned as *unset*. A config layer
that drops empty values reintroduces the bug exactly.

*Verified against charkit as it stands:* `crates/core/schema/char.schema.json` places no `minLength`
and no `pattern` on an env value, so `env: { CI: "" }` validates today. **No schema change is
needed.** The residual risk is downstream — resolution, and the spawn adapter — so it becomes a test
the implementer writes rather than a config gap.

### 6.3 The remaining generation traps

All of these become fixture configuration, not engine code.

| # | Trap | What breaks if absent |
|---|---|---|
| BAS-1 | Update mode pinned to fill-missing-only | The committed side of every comparison is replaced with a fresh copy of itself; every pair reports identical; the review page looks perfect and the differences it exists to surface are gone |
| BAS-2 | The **bare** update flag is destructive too — it must always carry its explicit value | The same total loss, reached by what looks like a harmless simplification of a command string. The sub-trap a rewriter is most likely to hit |
| BAS-3 | Generation runs in the **same image** the check uses | Running on a developer machine produces baselines for the wrong platform. They look like progress, they get committed, and they can never satisfy the check |
| BAS-4 | Generation is coupled to the review by construction | *If you can generate without reviewing, eventually someone will* |
| BAS-5 | A non-zero exit is reported but **not** treated as failure | The tool reports a spec as failed when it *writes* a snapshot instead of matching one, so a successful first generation exits non-zero. Aborting on it aborts before showing the review — exactly when the review matters most |
| BAS-6 | …but the non-zero exit is **never asserted to be benign** | Both readings are common; saying "expected" and printing nothing makes a broken container indistinguishable from a successful generation |
| BAS-7 | Full output goes to a log whose path is printed | D. The first version captured everything and printed **none** of it, so a genuine failure surfaced only as a message saying the non-zero exit was expected. Reassurance without evidence |
| BAS-8 | The printed tail is bounded and skips blank lines | The full transcript buries the review it precedes; whitespace lines consume tail slots and push the real error out of an already-small window |
| BAS-9 | Package-manager chatter is dropped **from the tail only**, never from the log | D. The package manager prints its nag *after* the real output, so it is always last and occupied a third of the tail — exactly the lines someone reads to find out what broke |
| BAS-10 | Both streams are captured | The runner splits reporting across them; one stream gives a confident, incomplete story |
| BAS-11 | Labels are **never verdicts** — enforced by a forbidden-word assertion | No pixel statistic separates a re-rasterised glyph from an element that moved. A label reading as pass/fail gets deferred to, the human stops looking, and a real regression ships behind a green word |
| BAS-12 | Zero differences short-circuit to *identical* before the spread test | Zero rows is trivially below the spread threshold, so a byte-identical pair gets the loudest label and sorts to the **top** of the page |
| BAS-13 | Greyscale is **refused**, not compared | Channel-stride offsets misalign, so *every* pixel reads as differing and the tool confidently reports a total regression that does not exist |
| BAS-14 | Unsupported encodings are refused rather than best-effort decoded | Garbage pixels do not surface as a crash; they surface as a *pixel difference*, in a tool whose entire output is pixel differences |
| BAS-15 | Three distinct empty-case messages, never a silent success | A mistyped platform suffix and an ungenerated platform produce the same empty page, and the reader concludes there is nothing to review when there is everything to review |

### 6.4 Two incidents from the project file, and why they generalise

The source's `scripts/pyproject.toml` opens with 25 lines of commentary recording two dated
incidents. Both are the same genus — *something ambient was mistaken for something configured* — and
both generalise past their language.

**A test suite that silently ran nothing.** Per-file inline dependency headers are honoured only when
the runner is handed a single file, never when it collects a directory. The documented command
pointed at the directory, collection failed on a dependency that *was* declared, and the suite ran
**zero tests** — reported as nothing rather than as failure.

**A gate grading against rules nobody chose.** With no project file above the directory, the linter
fell back to ambient discovery: **415 enabled rules**, none selected by the repo, none reproducible
from anything committed.

charkit's exposure to the same two shapes:

| charkit risk | Engine behaviour it argues for |
|---|---|
| A selector matching zero checks and exiting 0 | Already handled by `PLAN.md` §3.2's conventional-name rule — but the count that *was* selected should always be reported |
| A `match:` glob matching nothing | Distinct, loud verdict; never silence. `PLAN.md` §4.1's empty-`${files}` rule is the same instinct |
| A check whose command collects nothing and exits 0 | The generalised rule: **a check that runs and finds nothing must be distinguishable from a check that passes** |
| A gate pattern that *cannot* match | Already learned twice by [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4's own contamination grep. Independent confirmation from a second codebase |
| A tool invoked without repo-pinned configuration | Configuration is committed and passed explicitly; never rely on upward discovery |

---

## 7. Defects in the original — do not port

Bug-shaped branches that are bug-shaped because they *are* bugs. Porting them faithfully imports the
defect.

| # | Defect | Why it must not be reproduced |
|---|---|---|
| X-1 | **An exclusive lock that does not actually hold.** A dated open TODO records that two checks sharing it both logged container creation followed by a port-already-allocated failure, reproduced twice, with the reservation logic reading correct on inspection | Suspected cause: container creation escapes the reservation window. **Action: make the reservation provably enclose the entire lifecycle, and write a test that fails if any resource-creating step escapes it** |
| X-2 | **The fix phase bypasses admission entirely** — no reservation, no exclusives, no budget | Fixers are cheap today, so it has not bitten. A fixable check with a real footprint would bypass the scheduler completely. Route both phases through one admission path |
| X-3 | **`--again` publishes no live state**, and its finish write is then silently discarded by the ownership guard because the document still belongs to the earlier run | A rerun is invisible to status and wait for its entire duration, reading as the previous run's terminal verdict. An omission wearing the shape of a design |
| X-4 | **The ownership guard has an opt-out**: a caller supplying no run id is allowed through unconditionally | It exists to make tests convenient and it disables the only protection on that surface for any call site that forgets to thread the id. In charkit the holder id is always known — make it non-optional and give tests a constructor |
| X-5 | **Group leadership is asserted, not observed** — the group id is recorded as the pid whenever the pid is not the writer's own | It makes ST-20's interlock, the one guard protecting the user's shell, satisfiable by unverified data. Record leadership only when the writer created the session |
| X-6 | **The heartbeat is written outside the state critical section, non-atomically** | Two concurrent transitions can tear the owner record; a third party reads it as malformed, treats it as no owner (ST-15), and **reclaims a live lease**. Low probability, catastrophic outcome. SQLite removes it — do not reintroduce an out-of-transaction heartbeat |
| X-7 | **An unparseable package manifest is silently skipped** during discovery | A broken manifest is a real error someone wants to hear about; skipping it removes that package's entire check with no trace |
| X-8 | **An explicit empty file list falls through to a full changed-file run** | The emptiness test is on the raw string, not the parsed list, so a caller passing a computed-and-possibly-empty list gets the *whole diff* rather than nothing |
| X-9 | **`--again` mixes every domain's files into one domain's path scope** | Recorded scopes are per-domain; unrelated paths reach a tool that cannot use them |
| X-10 | **Strict text decoding at the process seam** | Measured: a command emitting two invalid bytes raises out of the runner before any result exists — no exit code, no partial output, no timeout verdict. The check crashes the run. Real tools emit invalid UTF-8. **Capture bytes; lossy-decode at the reporting boundary** |
| X-11 | **The streaming runner merges stderr into stdout** | `--verbose` must not change the shape of what a check's parser sees. A mode-dependent verdict change |
| X-12 | **All partial output is discarded when the drain expires** | It throws away the evidence precisely in the worst case. Keep what was already read |
| X-13 | Skip-decision and skip-*reason* orderings differ | A check skipped for two reasons is reported under the one that did not decide it. Unify, and derive the message from the predicate |
| X-14 | The changed-file count in the pre-run summary omits one domain's list | A run scoped entirely to that domain announces "0 changed files" while running its checks |

### 7.1 Three measured defects in the kill path

Measured directly against the source module during this harvest, on darwin. All three are dangerous
and all three are invisible to that suite. They carry ids — X-15, X-16, X-17 — because the ported
cases cite them and an unnumbered defect gets cited by section, which collides with the numbered
rows above.

**X-15 — the process-group id is looked up at kill time, not recorded at spawn.** For a child that has
exited but not been reaped — exactly the state after a deadline expires, since the wait that would
reap it is the one that just timed out — the lookup fails and **the entire kill is skipped**.
Measured: a grandchild survived the timeout, the drain burned its full ceiling so the deadline
overran by 15×, and all partial output was destroyed. Three published guarantees fail at once.
**Fix: record the group id at spawn. With `setsid` the group id *is* the child's pid, so it is known
the instant the child exists and can never become unavailable.**

**X-16 — escalation to a hard kill is conditioned on the leader's exit, not the group's emptiness.** If the
leader dies on the soft signal while a group member ignores it, the hard signal is **never sent**.
Measured: the survivor was still alive after the call returned. This directly contradicts
[`traps.md`](traps.md)'s measured rule that escalation must be **unconditional, not a retry**,
because children inherit an ignored disposition and one uncooperative leader immunises its group.
**`traps.md` wins.**

**X-17 — only one error condition is treated as "already gone"**, where
[`traps.md`](traps.md) measured a *different* one on darwin for a zombie-only group and warns in
terms that branching on one specifically sees neither platform's answer. On the abort path that
escapes a signal handler, where nothing catches it.

> **These are not proposed as [`traps.md`](traps.md) entries.** That file's standard is measurement
> in the environment charkit actually runs in, and these were measured against a Python module.
> The underlying POSIX facts extend its existing zombie-group entry and are worth re-measuring in
> Rust — at which point they would earn a row. **Owner: whoever builds the process adapter.**

---

## 8. Keep versus drop, in one place

**Keep — every one earned by an incident.** SC-1 to SC-8, SC-10, SC-12 to SC-20 (scope
conservatism and every fallback); CAT-2, CAT-3 (positive time budgets, engine suite on the host);
SCH-1 to SCH-11, SCH-14 (footprint separate from workers, clamp-don't-wait, **no part of a
reservation held while waiting for the rest** — SCH-8's transferable invariant, which charkit
reaches by `PLAN.md` §4.3's order rather than by the source's single atomic step, see §5.3 —
reserved cores, re-declare the lock on moving back to the host; **SCH-7 and SCH-11 are kept but
have no ported case, because charkit's contract cannot express the worker half of either — §10**);
CMD-1 to CMD-4, CMD-7, CMD-9,
CMD-10, CMD-12, CMD-14 (drop the git filter in a container, refuse *and* skip, create
the artefact directory first, never synthesise a faster command — **CMD-2's forced build is kept but
lands on phase 4's `up`**, §5.4); PAR-1 to PAR-15 as *contract*
(the benign-summary guard above all); EXE-1 to EXE-6; VIS-1 to VIS-11; ST-1, ST-4 to ST-16, ST-19 to
ST-38 (as requirements, per §5.8's R/M column); PRO-1 to PRO-13; BAS-1 to BAS-15 as fixture config.

**Drop — source-repo shape, not knowledge.** The static catalogue and the three-domain model; every
hardcoded directory, app, service and profile name; per-tool parser field paths; behaviour keyed on
a check id; the generated aggregation script; the multi-package discovery globs; a process-wide
mutable registry ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.4 forbids it — it belongs on the run
adapter behind `Ctx`); git helpers bypassing the seam; ambient environment mutation to pass ports;
one exit code for every failure ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.6 replaces it); the
platform-specific daemon auto-start; the pixel codec, label taxonomy and HTML report; the two
label thresholds, tuned against 17 pairs on one platform pair; the unused run-id helper whose
format is duplicated inline at both call sites; source-text-reading assertions (§9).

**One comment/behaviour mismatch, recorded so nobody "fixes" it into a behaviour change.** A
threshold in the visual aid is commented as an override that the code does not implement — the
preceding test wins unconditionally. Implement the behaviour, not the comment. Impact is cosmetic.

---

## 9. Assertions that are vacuous or cannot fail

[`ARCHITECTURE.md`](ARCHITECTURE.md) §2.1.1 is explicit that a vacuous assertion is worse than none,
because it gets cited as evidence. The plan warns that a ported assertion that cannot fail is
invisible while looking like coverage. Found:

| Assertion | Why it cannot fail | Port as |
|---|---|---|
| The build-noise filter test | With no compiler-error line in the fixture the parser degrades to a fallback whose last-line rule already returns the right answer, so the guard never fires and **the filter is never called. Delete the filter and the test stays green.** | A fixture that routes through the guard: a benign summary, an empty failure list, and progress lines *ahead of* a real error that is not the last line |
| "A command that exits just before its deadline" | The deadline is 30 s and the command exits at once, so the kill path is **never entered**. A regression in the already-dead-group guards passes it unchanged. **This is the hole §7.1's first defect hid in** | A leader that exits while a group member survives, under a short deadline: expect the timeout code, the survivor killed, and output preserved |
| "The convenience call is never used for checks" | It reads the module's own source text and searches only for the *presence* of the correct construct, never for the forbidden one. It passes if only the second runner is right, and passes unchanged if the prohibition is violated | Observable behaviour: spawn a tree, kill it, assert nothing survives. Plus a structural lint that the process adapter is the only module constructing a command |
| The abort-handler ordering test | Splits the implementation file on a function signature and compares substring indices. Untranslatable and self-invalidating on any refactor | A real signal to a real child at the integration tier. **This is the highest-value untested behaviour in the harvest** |
| "The historically resource-sharing checks must declare an exclusive when host-run" | All three are container-run, so the loop body never executes | Keep it — it is a **tripwire that arms only when someone flips a runner** — and say so in a comment |
| Domain classification with an all-flag and an empty list | The function short-circuits on "all **or** empty", so the all-flag row cannot fail unless the empty row does | Test the all-flag with a **non-empty** list |
| "Install runs even when the directory exists" | Nothing in the code path stats the filesystem, so the assertion is byte-identical to the row above it | Assert that **no filesystem probe occurs** |
| Scheduler budget 4, four concurrent reservations of cost 3 | Cost 3 against budget 4 admits strictly one at a time, so the observed peak can only ever be 3 | Add a mixed-cost case (3 + 1 + 2) |
| "A check costing more than the budget still runs" | Asserts the body runs, never the granted value | Assert the grant equals the budget **and** that the tool is sized with it |
| Status-line rendering | Takes the last line of a single-line string, and is fed hand-built inputs so it cannot catch the status query and the renderer drifting apart | Drive it from real status values |
| Report-table suppression | A three-way substring disjunction whose first disjunct can never be true, because the result line itself contains the word | Assert the absence of the column-header row directly |
| Daemon-already-running | The second assertion cannot fail once the first holds, and would miss an absolute-path launcher | Drop the second assertion |
| Verbose-flag selection | Both executions pass the *same* double, so the verbose path is never exercised | Make one of them actually stream |
| Substring assertions of the form "contains *1 violation*" | `11 violations` also passes | Exact equality where the format is fixed |

**Two branches with no test at all**, both silent, both load-bearing: the scalar-skip recovery in the
stream scanner (PAR-5), and the empty-sub-directory-set fallback (SC-9). The pairing is the
harvest's argument in miniature — the code carries a fix nothing tests, and a test that pins nothing.

---

## 10. Candidate gaps in `PLAN.md` — for the implementer to raise, not to fix

Phase 2.5 was the last phase licensed to send changes back to `PLAN.md`
([`AGENTS.md`](../AGENTS.md)), so these are recorded rather than acted on.

| Observation | Why it might be a gap |
|---|---|
| A check whose tool legitimately exits non-zero on success has no way to say so | `PLAN.md` §4.1.1 lists keys deliberately not added, and an expected-exit-code key is not among them either way. **Recommendation: leave the engine strict.** The one harvested case (BAS-5) is a repo-local aid that becomes a `commands:` entry in phase 6, where the exit code passes through verbatim anyway |
| Per-check output tail length and a noise-prefix list | BAS-8, BAS-9 and PAR-3 all want them. `PLAN.md` §3.1 caps captured output at 10 MB but says nothing about which lines reach a summary |
| No stated rule that an empty-string `env:` value is preserved rather than pruned | §6.2. The schema already permits it; nothing states that resolution must not drop it |
| **No worker figure the engine computes can reach the tool — and it is the whole point of the budget.** The engine can derive a worker count exactly: `max(1, min(declared cost, granted))`. It has no way to deliver it. The worker flag is a **literal the config author writes into `cmd:`**, and `PLAN.md` §4.4 caps substitutions at `${port.NAME}`, `${files}`, `${component.root}`, `${workspace.id}` plus scoped `${env.NAME}` / `${ref}` — an unrecognised `${…}` is `bad_config`. There is no `${jobs}`-shaped variable, so a config can only hard-code the *declared* cost, and only that. Two kept traps fall out of it: **SCH-7's clamp is decorative** (an over-budget check is admitted against fewer slots and still launches its full declared parallelism — the exact oversubscription SCH-6's clamp exists to prevent), and **SCH-11 is unexpressible** (an explicit flag overriding a tool's auto-sizing is entirely the author's discipline; nothing in the engine can require or correct it). **SCH-7 and SCH-11 are the only kept traps in this harvest with no ported case behind them, and this row is why.** SCH-1/SCH-2's two numbers survive, but not as two declared keys with a default between them: `cost:` is the reservation and the worker count is the author's literal, so *nothing enforces that raising one does not raise the other* | **Why it matters, plainly: a scheduler that budgets the machine while every tool sizes itself to the whole machine is budgeting nothing.** SCH-1 and SCH-3 are the incidents that already cost this, and they present as ordinary test failures. The cap is deliberate and phase 3 is **not licensed to change `PLAN.md`** (phase 2.5 was the last), so **the implementer raises this against `PLAN.md` rather than inventing a key**. The engine-side half is cased — `sched.reserve.clamped-grant-is-the-clamped-figure` and `exec.grant.declared-cost-is-reserved-and-granted` assert the grant, and no case anywhere expects the engine to produce a worker count. The nearest thing is `exec.workers.bounded-browser-run-does-not-manufacture-failures`, which asserts the *fixture config declares* one — a config lint, and the only lever that exists today. Options: a granted-slots substitution admitted inside the cap; or accept that `cost:` must equal the tool's worker count, making clamping a scheduling-only concept and the author's literal the sole guarantee |

### 10.1 One gap that is not in `PLAN.md` — nothing machine-checks this document or `tests/cases/`

`cargo xtask doclint` works from a fixed corpus of six files (`xtask/src/docs.rs`), and neither
`docs/harvest.md` nor `tests/cases/README.md` is in it. So the ~95 `§` cross-references in this
file, the ~35 more the case files carry, the fenced YAML blocks in `tests/cases/README.md`, and
**every trap id that a `trap:` key under `tests/cases/` cites by name** are checked by nobody. All
of it was verified by hand while this document was written, and **hand-verification does not repeat
itself.**

**Why it matters, concretely.** The case files' `trap:` keys are a hard dependency on the ids in
§5: renumber a trap or delete a row and the link breaks silently, with no gate firing. That already
happened inside this change — three cases cited trap ids that resolved to unrelated defects, and
the check that was supposed to catch it confirmed only that the ids **existed**, never that they
were the right ones. That is the same shape as §9's vacuous assertions, and the same shape
[`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4 warns about twice: a green check that structurally cannot
fail.

**Closing it is not a one-line change.** This document cites its own sections, and it is not in
[`ARCHITECTURE.md`](ARCHITECTURE.md) §2.8's precedence list — so adding it to the corpus means
first deciding what an *unattributed* `§` means inside a document that does not rank. A cheaper
partial measure stands on its own and needs no such decision: **check that every `trap:` value
under `tests/cases/` resolves to a row in §5**, or — for the `"§6.1"` form the schema also permits
— to a section this document actually has.

| | |
|---|---|
| Owner | the implementer, or whoever next touches `xtask` |
| Status | **out of scope for phase 3** — recorded here deliberately, not closed |

---

## 11. The ported cases

The cases live in `tests/cases/`, as data, one file per subsystem, in the same YAML the config
contract already uses (`PLAN.md` §4.1.1 decision 5 pins the parser). `tests/cases/README.md` states
the schema. They are **cases, not translated test functions**: the original is `run_fn`-injected and
charkit has three seams behind a `Ctx`, so the assertions survive and the harness does not.

Assertions listed in §9 are **excluded** in their original form and present in their rewritten form
where a rewrite was possible; each exclusion is recorded in the case file so the omission is visible
rather than silent.

**Suggested PR split**, given §1's measurement that the harvest is ~2× the planned size:

| PR | Contents |
|---|---|
| 1 | This document plus `tests/cases/` — no Rust |
| 2 | Scope, selection, and the config-driven replacement for the catalogue |
| 3 | The process seam: deadlines, process groups, abort |
| 4 | Admission control and the scheduler reducer |
| 5 | Command construction, the runner decision, execution phases |
| 6 | Result parsing and the failure-reporting contract |
| 7 | Run state, leases, status, `--again`, detach |
| 8 | Live output |
