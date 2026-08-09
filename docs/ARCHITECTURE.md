# charkit — architecture

> **Status:** agreed in Phase 0 (see [`PHASES.md`](PHASES.md)). No source code exists yet.
>
> This document records **principles and the reasoning behind them**. The reasoning is the
> load-bearing part: a rule without its reason gets discarded the first time it is
> inconvenient. If you want to change a principle, argue with the rationale — not the rule.
>
> **Precedence: where this document and [`PLAN.md`](PLAN.md) disagree, this one wins.** PLAN is
> the specification of what to build; this is the record of what was decided about how. A
> conflict is a defect in one of them — fix it, and say which was wrong.

## Contents

| § | | |
|---|---|---|
| **1** | Architecture principles | 1.1 seams · 1.2 pure core & the reducer · 1.3 no logic in commands · 1.4 no ambient state · 1.5 dependencies inward · 1.6 machine-readable output & exit codes · 1.7 typed failures · 1.8 secrets |
| **2** | SDLC principles | 2.1 TDD scope · 2.2 branching · 2.3 commits · 2.4 the merge gate & contamination grep · 2.5 versioning · 2.6 dogfooding · 2.7 phase 3 clean-room · **2.8 document ownership & precedence** |
| **3** | Decisions recorded, and the test tiers | |
| **4** | What was deliberately not decided | |

---

## 1. Architecture principles

### 1.1 Three injected seams: subprocess, clock, network

Every interaction with the outside world that is **slow, nondeterministic, or external** is
reached through a function passed in, never imported. There are exactly three:

| Seam | Covers |
|---|---|
| `run` | every subprocess — and therefore docker, git, and every `cmd:` from `char.yml` |
| `now` | timeouts, heartbeat mtime staleness, `claimed_at` |
| `fetch` | `http` and `tcp` ready-checks |

They travel together with the workspace in one frozen dataclass, passed as the first
argument:

```rust
pub struct Ctx<R: Run, C: Clock, F: Fetch> {
    pub workspace: Workspace,
    pub run: R,      // every subprocess
    pub now: C,      // timeouts, heartbeats, claimed_at
    pub fetch: F,    // http and tcp ready-checks
}
```

Traits rather than boxed closures, so the fake is a zero-cost substitution the compiler checks
and the production path pays nothing for. A test constructs `Ctx<FakeRun, FixedClock,
FakeFetch>`; nothing is patched and nothing is dynamic.

**Why injection at all.** This is the one pattern worth copying wholesale from the source
repo. Chariot's 2,694 test lines run hermetically with no mocking framework, because
`run_fn` is a *parameter*. Nothing patches `subprocess.run`; the test simply passes a
different function.

**Why three and not six.** The plan originally proposed six (adding filesystem, docker and
git). Three, because:

- **Docker and git are subprocess calls.** Giving them their own ports means three different
  ways to fake a shell command, and tests that disagree about which one ran.
- **Faking at the `run` level keeps argv assertable, and argv is where the bugs are.**
  charkit's central correctness claim is *"`clean` releases exactly what this workspace
  owns."* When that breaks it looks like a label filter written `--filter
  label=char.workspace` instead of `label=char.workspace=<id>`, or a compose project name at
  `up` that doesn't match the one at `down`. Those are argv bugs. A test that fakes `run`
  asserts the exact command and catches them. A test that fakes a `DockerPort` catches none
  of them — the fake returns whatever you told it to, and the argv-building code has no test
  at all. Mocking at the higher layer would hide precisely the bug class the tool exists to
  prevent.
- **The filesystem must not be faked, and neither must SQLite.** Machine-global state lives
  in `~/.char/char.db` (PLAN.md §4.3), so char depends on real transaction semantics for
  port claims and lease acquisition, and on real files for logs and `.char/`. A fake gives
  you a green test over your own fake's concurrency model and proves nothing about the real
  one — in the one area where [`PHASES.md`](PHASES.md) §11 names corruption under concurrent claims as a live risk.
  Two threads against a real database in a `tempfile::TempDir` is both more faithful and less code.

**Where stack diversity actually lands.** It does not land here. A Rails repo and a Go repo
differ in *what string char runs*, not in *how char runs a string*. Most of what charkit
shells out to it has never heard of — `bundle exec`, `turbo run`, `go test`,
`./scripts/kind-up.sh` — because those arrive as free text from `char.yml`. `owns:` does the
same for cleanup, with a label selector as a string. No typed port can model any of it.
Config absorbs diversity; the I/O layer never sees it.

**Reversibility.** If docker argv assembly turns out to be scattered and error-prone,
promoting it to a typed port later is a mechanical refactor. The reverse is not true: once
tests are written against port fakes, the argv assertions are gone and rewriting the suite to
recover them is real work. Three is both the cheaper bet and the reversible one.

**Consequently, `docker` and `git` are ordinary adapter modules** that build argv and call
`ctx.run`. Git's real risk is *parsing* — `git worktree list --porcelain`,
`rev-parse --git-common-dir` — and a parser is a pure function tested against recorded output
strings. It needs no injection.

---

### 1.2 Pure core, imperative shell

Config resolution, scope computation, scheduling, port selection and verdict aggregation are
pure functions over data. Spawning, writing, labelling and killing live at the edge. Most of
the test suite then needs no fixture, no tmpdir and no seams — just values in, values out.

**Sub-rule, scoped to the scheduler and the claim/lease loop:** the core is a **reducer**.

```rust
pub fn step(state: State, event: Event) -> (State, Vec<Action>)
```

**`Event` and `Action` are enums, and `step` matches them exhaustively.** This is the reason
the language decision landed where it did (PLAN.md §10.1). Adding a variant without handling
it is `error[E0004]`, unconditionally — measured, not assumed. The types are not documentation
of the scheduler's specification; they *are* the specification, and the compiler checks it.

Two rules follow, and both exist to keep that property:

- **Never add a catch-all `_ =>` arm in `step`.** It converts the compile error into silence
  and forfeits the entire benefit. If a variant genuinely needs no action, write it out and
  return no actions — the explicitness is the point.
- **`State` is owned and returned, not mutated in place.** A `&mut State` reducer can be
  called for its side effects and its return value ignored, which is how the pure core starts
  leaking into the shell.

The shell executes actions and feeds results back as events. **The core proposes, the shell
attempts, failures return as events.**

**Why the scheduler cannot be a planner.** It holds four constraints at once: a cost budget
capped by CPU slots, `exclusive:` resources that are mutexes rather than counts, `needs:`
ordering against services, and — in the real fixtures — a 3-second-to-15-minute duration
spread. Whether the next check can start depends on **which checks have already finished**,
which is unknowable until runtime. A genuinely static plan can only express batches, and
batching a 15-minute check alongside 3-second ones is a large wall-clock regression, not
merely an architectural smell.

So the choice was never "reactive versus simple." It was **where the reactive part lives**. A
planner does not remove the reactive scheduler; it relocates it into the shell, where the
pure test suite cannot reach it — and [`PHASES.md`](PHASES.md) §11 names exactly the bug that would hide
there: *"the scheduler deadlocks when two exclusive resources overlap under load."* That is
the one class of failure a greenfield build structurally cannot catch, because there is no
continuous real-repo validation until phase 6. Making it reachable in a unit test is the
highest-leverage thing this principle buys.

**Two further benefits.** Every `(state, event) → (state, actions)` transition can be logged,
so a production deadlock replays verbatim as a regression test. And port claiming has the
identical shape — choosing a block is pure, the claim can lose a race, and losing means
re-deciding — so one pattern covers lease acquisition and the machine-global store too.

**Costs, accepted knowingly.** What would be local variables (in-flight set, remaining
budget, held exclusives, start times) become explicit `State` fields, which is more verbose
to read. And a pure reducer cannot call `time.monotonic()`, so `now` is carried on every
event and the shell sleeps until the next deadline computed from state.

**One deadlock is deliberately outside the reducer's reach, and is prevented rather than
tested.** Once `exclusive:` resources are machine-wide (PLAN.md §4.3), a cycle can span two
*processes* — and `step()` models one run, so no unit test can construct it. The answer is to
acquire exclusives in sorted name order, which makes a cycle impossible for any interleaving
rather than unlikely. Recording it here because it is the one gap in the claim above: the
reducer makes *intra-run* scheduling deadlocks unit-testable, and the *inter-run* case needed a
different kind of answer.

**This is deliberately not a general rule.** Reducers apply to the scheduler and the claim
loop. Everywhere else in the core, a plain function is a plain function.

---

### 1.3 No logic in command functions

Every command is: **parse args → call into the core → render.** The MCP server calls the same
functions the CLI does.

This is forced by a decision already made — the MCP server runs in-process and cannot shell
out to itself, so two callers need the same logic. It is stated anyway because the failure
mode is gradual: one command function does slightly too much, and eventually the logic lives
in the argument parser.

**The source repo arrived at this independently**, which is the strongest evidence the
principle is not merely imposed. Its `baselines.py` opens with: *"Nothing here prints or
exits — `__main__.py` renders the report, and an MCP wrapper can call `build_report()` for
the same data."* Same two callers, same conclusion, reached before this document existed.

**The public contract is the CLI surface and the `--json` payloads. Internal modules are
internal.** The plan originally called this "a thin wrapper over an importable library," and
the word *library* was dropped deliberately: it invites `from charkit import ...`, which
creates a stability obligation to a consumer that does not exist. External consumers get the
CLI. Note this interacts with versioning (§2.5) — charkit stays 0.x, so no stability is
promised on any surface, but the CLI and `--json` are the ones intended to be depended on.

---

### 1.4 No ambient state

Stated in a form that can actually be violated visibly:

> **No `static mut` and no global `OnceCell`/`LazyLock` holding mutable state. No
> `std::env::current_dir()` and no `std::env::var()` below the entrypoint.**

The workspace is resolved once, at the entrypoint, and passed explicitly thereafter (it rides
on `Ctx`, §1.1).

**Why.** `--project` and `--all` mean operating on workspaces that are *not* the current
directory. If any function can re-derive "the workspace" from cwd, then scoping to a
different one requires lying about cwd — and you are one `os.chdir` away from a race between
two concurrent runs on the same machine, which is the exact scenario charkit exists to make
safe.

The abstract phrasing ("no ambient state") was rejected because everyone agrees with it and
nobody can tell when it has been broken. The concrete phrasing is a lint rule.

**Honest note:** a `Ctx` parameter threaded everywhere is ambient state with better
testability. That is the trade, made on purpose, and it is why the seams and the workspace
travel together rather than as seven separate arguments.

---

### 1.5 Dependencies point inward

```
charkit/
  core/       pure. depends on std and its own traits. nothing else.
  adapters/   docker, git, filesystem, process. import core protocols only.
  cli/        the ONLY module that imports both, and wires them together.
```

Enforced mechanically by the crate graph itself, plus a `cargo-deny`/`clippy` boundary check in
the merge gate from phase 1 —
before there is anything to untangle.

**Why this is stated as a correction.** The plan phrased it "dependencies point one way:
core → adapters," which reads as *the core imports the adapters* — the opposite of what
§1.1 and §1.2 require, and the opposite of the note printed directly beneath it in the plan.
An agent implementing that arrow literally would wire the system inside-out while every
stated rationale still appeared satisfied. Dependencies point **inward**, toward the core.

---

### 1.6 Every verb answers in a machine-readable shape

`--json` on every verb, without exception. The renderer is the only thing that differs
between human and agent output.

A flag is not a contract, so three things make it one.

**Exit codes.** Missing from the plan entirely, and the larger gap: the plan defines terminal
states per verb but never says what the process exits with. Agents read exit codes far more
reliably than they parse stdout, and the difference between "your config is wrong" and "your
tests failed" is the difference between fixing the right thing and retrying forever.

**There is exactly one mapping: `exit code = f(error.class)`, or `0` when `error` is null.**
An earlier draft had two — one from terminal state, one from error class — and asserted that
the class won, while printing a table keyed on state. They disagreed, and nothing would have
caught it: golden snapshots capture stdout, not exit status.

| Code | Error class | Meaning |
|---:|---|---|
| `0` | *(none)* | success |
| `1` | `tool_failed` | the thing char ran failed on its own terms — a real result, not char's fault |
| `2` | `bad_invocation` | unknown verb or flag |
| `3` | `bad_config` | `char.yml` is wrong |
| `4` | `timeout` | char's own deadline elapsed |
| `5` | `aborted` | cancelled, or the run's holder died |
| `70` | `char_bug` | internal error; retrying will not help |
| `130` | *(signal)* | SIGINT |
| `141` | *(signal)* | SIGPIPE — `char status \| head` and friends |

**Terminal state describes *what happened*; error class states *why*; the code follows the
class.** They are not the same axis, which is why one cannot be derived from the other:

| Case | State | Class | Code |
|---|---|---|---:|
| `char up`, `char.yml` names a compose file that does not exist | `FAILED` | `bad_config` | 3 |
| `char check`, the tests genuinely fail | `FAILED` | `tool_failed` | 1 |

Same state, different codes, and that is correct — an agent must fix the config in one case
and read the test output in the other. `DEAD` (the run's holder died) maps to `aborted`,
because the useful next action is the same as for a cancellation: try again.

**Verified rather than assumed** — though against Typer 0.27.1, *before* the language decision, so this is now evidence about conventions rather than about charkit's own stack (`traps.md` marks it historical): `KeyboardInterrupt`
already exits **130**, and usage errors already exit **2**. Those two are genuinely free. A
report that Click collapses them to `1` was checked and is false for this version — but check
again if the framework is upgraded, because it would be silent.

`70` is not a claim to implement BSD `sysexits`. It is chosen because it sits far from char's
own low codes *and* from the codes a child process is likely to return, so "char itself broke"
stays distinguishable from everything else.

**Signals are the one carve-out from `exit code = f(error.class)`.** A process killed by a
signal exits `128+N` and has no error class at all — `130` for SIGINT, `141` for SIGPIPE.
State that explicitly, because the rule as written has no room for them and an implementer
following it literally would map them into a class.

**Broken pipe is now resolved: `141`.** `char status | head` must not read as a failure, and
Rust's runtime sets `SIGPIPE` to `SIG_IGN` at startup, so without intervention the process
*panics* with exit 101 — worse than nothing. Restoring the default disposition in `main` gives
the ordinary Unix behaviour: silent death, exit 141. That is one of exactly two `unsafe` blocks
the design permits; both are recorded in `traps.md`.

**The envelope**, fixed in phase 1 alongside the config contract and for the same reason —
four things consume it and none can invent it independently. Full definition in PLAN.md §3.1.

```json
{ "schema_version": 1, "verb": "check", "workspace": "a3f91c02",
  "status": "FAILED", "error": null, "data": { } }
```

`data` is nested rather than flattened so the envelope is generically validatable — one
schema checks the wrapper, a per-verb schema checks the body — and so a verb added later can
carry a field named `status` or `error` without colliding. `workspace` is always the
*invoking* workspace, even under `--project` / `--all`, so the envelope shape never varies;
other workspaces live in `data`.

**`schema_version`.** One global version for the whole CLI contract, in every payload. Bump
rule: **adding a field does not bump; removing a field or changing its type does.** That rule
is checkable, and it lets a consumer say "I need ≥ 1" and be right.

Global rather than per-verb because six verbs ship in one binary and an agent uses all of
them — eight independently drifting version numbers works against the project's "learn it
once" thesis for a precision nobody needs.

Worth being honest: the field has no consumer today. The MCP server is in-process and always
the same version, and agents read JSON adaptively rather than branching on a version number.
It is included because it **cannot be retrofitted** — adding it in v0.4 does not help anyone
who wrote a script against v0.2, since their payloads never carried it. Every other decision
here is reversible; this one has a one-way door. It matters more than usual given charkit
stays 0.x indefinitely (§2.5), which means the package version communicates nothing about
compatibility, so `schema_version` is the only compatibility signal that exists.

**The envelope is flushed explicitly before any exit path.** Measured: a 491-byte payload
written through a `BufWriter` and followed by `std::process::exit` delivers **zero bytes** —
and a 20 KB payload delivers all of them, because it exceeds the buffer capacity and bypasses
it. So the failure is size-dependent: it passes a test with a large fixture and silently
empties a small real payload, for the one consumer this contract exists to serve. See
`traps.md`.

**One golden snapshot per verb**, in `tests/golden/`. No `--update-snapshots` flag.

These catch the one thing nothing else does: **key renames.** Every other test asserts on
value objects from the pure core, so renaming `verdict` to `status` in the renderer breaks
no test, runs fine locally, and surfaces in someone else's repo. A checked-in snapshot turns
that into a visible diff.

Three ways golden tests rot, and what is done about each:

| Failure | Mitigation |
|---|---|
| Auto-update reflex — a flag turns the test into "regenerate, commit, don't read" | Do not ship the flag. Regenerating by hand is annoying enough to make you look. |
| Nondeterminism — timestamps, run ids, durations, paths, allocated ports | Largely solved by construction: `now` is injected (§1.1) and the workspace path is passed in (§1.4). Ports and run ids get a small redaction helper. |
| Over-coverage — snapshot everything, and every intentional change is a 40-file diff nobody reads | One canonical snapshot per verb and per auxiliary command, never per test case. Deliberately not stated as a count — an earlier draft said "eight files", which was already wrong against the six verbs plus `config scan`, `config verify` and `agents-md`. |

Note the dependency: snapshots are what tell you *when* to bump `schema_version`. Without
them, the version field relies on remembering, which is how version fields become
decorative.

---

### 1.7 Failures are typed and attributed

Every error carries **which class of failure it is, where, and what to do next.**

```json
{
  "schema_version": 1,
  "verb": "config verify",
  "error": {
    "class": "bad_config",
    "where": "char.yml:components.api.checks.lint.cmd",
    "message": "`ruff` not found on PATH",
    "next_action": "add ruff to the api component's setup:, or correct the cmd"
  }
}
```

| Class | Meaning | Agent's correct response | Exit |
|---|---|---|---:|
| `bad_invocation` | the command itself was wrong | fix the command | `2` |
| `bad_config` | `char.yml` is wrong | fix the config | `3` |
| `tool_failed` | the underlying tool failed | that is a real result — report it | `1` |
| `timeout` | char's own deadline elapsed | raise the timeout, or investigate why it is slow | `4` |
| `aborted` | cancelled, or the run's holder died | try again | `5` |
| `char_bug` | charkit broke | stop; retrying will not help | `70` |

`timeout` and `aborted` are classes rather than only terminal states so that the class enum
covers every non-zero exit. Without them the mapping in §1.6 would have holes, and a hole is
where a second, competing mapping grows back.

`next_action` is **required for `bad_config`** and optional elsewhere. That is the one class
where char genuinely knows the fix, because it has just validated the file and knows what it
expected. Elsewhere a remediation string would usually be generic, and a field that is often
empty teaches agents to ignore it.

**Why this is a principle rather than a detail.** Phase 5's entire premise is that an agent
authors a config and `config verify` tells it what is wrong. That loop needs a defined
vocabulary to speak in, and without this there isn't one — six verbs and five terminal states
say nothing about whose fault a failure is.

---

### 1.8 A resolved secret never appears in anything char writes

`char.yml` declares a *reference*; char resolves it at spawn time and injects it into the
child's environment (PLAN.md §4.7). From the architecture's point of view that creates one
invariant, and it cuts across the renderer, the log writer and every error path:

> A resolved secret value is never written to **anything char writes** — stdout, stderr,
> `--json`, `.char/`, **`~/.char/char.db`**, or argv.

**The scope is deliberate, and the boundary is "what char writes."** char cannot make a
secret unreadable to something it hands the secret to. A value injected into a container is
in Docker's store, and anyone who can reach the daemon can `docker exec ... env` or read any
file inside — daemon access is root-equivalent. No mechanism char could build changes that,
so promising it would be false assurance. What char *can* guarantee is that its own outputs,
files and database never contain one.

`~/.char/char.db` was missing from an earlier draft of this list, which mattered: `PLAN.md` §6.1's
`owns.release:` is resolved at `init` and recorded there, so the obvious way to express a
teardown command with a password wrote a plaintext credential to a machine-global store that
survives `clean` **by design**.

Four consequences for the code:

- **Resolution happens in the shell, at spawn, and the value never enters the core.** The
  core deals in secret *names* and references. A pure function that has never seen a value
  cannot leak one, which is most of the enforcement for free.
- **Injection is via the child's environment, never argv** — argv is world-readable through
  `ps`.
- **The renderer and log writer scrub known resolved values** before writing. This is the
  one place the value is deliberately held, so it is the one place that needs the filter.
  **char reads raw and writes scrubbed** — ready-check regexes, `parse:` keys and exit-code
  interpretation all see real bytes; logs, `--json` and the terminal see redacted ones.
  Scrubbing the stream instead would break a ready-check whose regex spans a redacted value.
  char can only scrub what it can see, which is why `stdio:` (PLAN.md §4.5) is declarable
  per entry rather than inferred.
- **No verb returns a secret.** There is deliberately no `char secret get`. An agent can
  *use* a secret by running `char up`; it cannot *obtain* one. That asymmetry is the point,
  and it is a property of the verb surface rather than of any implementation.

**Stated as an architecture principle rather than a feature detail** because it is an
invariant about *output*, and §1.6 already made output a contract. Every verb answers in a
machine-readable shape — this says what that shape may never contain.

**Honest limit:** scrubbing is defense-in-depth, not a proof. It cannot defeat an encoded
value, and char does not control commands invoked outside it. What it guarantees is that the
default path is safe.

---

## 2. SDLC principles

### 2.1 TDD, scoped

| Where | Rule |
|---|---|
| `core/` — scheduler, scope resolution, verdict aggregation, port selection, config resolution | **Mandatory.** Failing test first, minimal implementation, passing test. |
| `adapters/`, `cli/` | **Test-alongside.** Same PR, order not policed. |
| Phase 1 schema | **Exempt.** The fixtures *are* the tests. |
| Phase 3 engine | **Exempt.** The test cases arrive with the harvest (§2.7). |

**Why scoped rather than absolute.** "TDD throughout, no exceptions" collides with two phases
the plan has already specified. Phase 1 is explicitly design-by-example — the plan says to
*expect the schema to change while writing the fixtures*, which is the phase working
correctly. Phase 3 arrives with a ported suite; you cannot write a failing test first for
behaviour you are transcribing.

A rule with a stated exception survives meeting it. An absolute rule gets quietly broken in
phase 4 and then constrains nothing anywhere. And at the adapter boundary a test-first is
either asserting on your own fake or requires real docker — neither of which is what TDD is
for.

---

### 2.2 Feature branch → PR → main. Tag each phase.

No phase branches. `main` is the only base. Each completed phase gets a git tag (`phase-1`,
`phase-2`, …).

**Why not phase branches.** An earlier draft argued this from "there is no server-side merge
gate, so the phase→main merge is the one diff nothing ever validates." **That premise is
false** — §3 chose a GitHub Actions matrix two sections later, which is a server-side gate,
and it would run on the phase merge like any other. The argument was written before the CI
decision and never revisited.

What actually survives is weaker but still decisive for this project:

- **One rebase base.** `no-mistakes` rebases onto base automatically; a phase branch is a
  second moving target that someone maintains by hand.
- **No stacked branches.** This is where agent implementers reliably go wrong, and agents are
  doing the work.
- **The agent review runs twice.** `no-mistakes`' review step is agent-driven and slow; a
  phase→main merge either re-reviews the accumulated diff or is waved through.

The conclusion stands; the confidence should be lower than the original phrasing implied.

**Cost, accepted:** `main` can sit part-way through a phase. Tags recover per-phase rollback,
and publishing a release is tag-triggered and deliberate, so `main` being mid-phase never
ships.

**Phase 1 still lands alone.** That is a sequencing rule, not a branching one, and it is
better served this way — the config contract is visibly on `main` before phase 2 opens.

---

### 2.3 Conventional commits

Scopes are **module names**: `core`, `adapters`, `cli`, `schema`, `fixtures`, `docs`. Not
phase numbers, which expire.

Cheap, matches the source repo's existing history, and feeds the changelog (§2.5).

---

### 2.4 The merge gate

**The GitHub Actions matrix is the authoritative gate** — it runs on every PR and nothing
merges without it (§3). **`no-mistakes` is the pre-flight**, not a requirement: it runs an
agent code review plus test, lint and docs locally, then pushes and opens the PR, so problems
surface before CI rather than after. It is how this repo is maintained day to day, it is
installable by anyone, and a contributor who skips it is judged by the same checks.

An earlier draft of this section called `no-mistakes` "the primary gate", which contradicted
both README and AGENTS.md — and the precedence rule only covers PLAN ↔ ARCHITECTURE, so
nothing resolved it.

The gate is:

1. **lint** — `cargo clippy -- -D warnings`, plus `cargo fmt --check`
2. **typecheck** — the compiler. `cargo build` failing *is* the typecheck
3. **tests** — unit, integration and e2e tiers
4. **coverage ratchet** — may never drop
5. **crate boundaries** — the layers contract from §1.5, enforced by the crate graph
6. **the contamination grep**

**The contamination grep.** This document is its **single owner** — PLAN, AGENTS and README
link here rather than restating it, because a fact stated in four places drifts, and this one
already drifted into a form that cannot match anything (see below).

```sh
grep -riE "chariot|tilt|NEXT_PUBLIC|\.claude|backend/|web/" \
     crates/ tests/ --exclude-dir=fixtures --exclude-dir=target
```

It must return nothing. **There is no allowlist.** If it fires, the code changes, not the
pattern.

> **A copy of this pattern in a markdown table cell is unrunnable.** Table cells require `|`
> to be escaped as `\|`, which renders as a pipe to a human but is a *literal* pipe in an
> ERE — so the pattern searches for one long string and matches nothing, ever. That is a
> permanently green gate that looks correct in the rendered document. Verified. **Keep the
> pattern in a fenced block, and nowhere else.**
>
> **The gate carries a test that the pattern still matches a known-bad string** *placed in a
> real crate source path*. A grep that cannot match is indistinguishable from a clean
> repository — and this has now failed twice for different reasons: once because markdown
> escaping made the pattern unrunnable, once because it pointed at `src/`, which does not
> exist in a Cargo workspace. The test must assert a **match**, not merely that the command
> runs.

> **Paths, not just the pattern.** Sources live at `crates/*/src/`; there is no top-level
> `src/`. Pointing the grep at `src/` makes it warn, scan nothing, and exit 2 — which most
> shell idioms read as "no contamination found."**

**`tests/fixtures/` is exempt, and the rest of `tests/` is not.** A fixture config is *data
describing a hypothetical repository* — the `django-next` fixture naturally says `backend/`,
because the repo it models has one, and that is the fixture doing its job. Ported test *cases*,
which are the reason `tests/` was covered at all, live elsewhere under `tests/` and stay
covered.

**Why `tests/` is covered at all:** [`PHASES.md`](PHASES.md) §9 calls the source repo's test suite the single most
valuable asset and its *cases* are ported in phase 3, which makes them the second
transcription vector. An earlier draft greped `src/` only and missed them entirely.

*What it catches:* phase 3 is the only phase permitted to read the Chariot repo, and this is
its acceptance test made permanent. `chariot` is a leftover import or path; `tilt` means a
vendor assumption got into code rather than staying in config; `NEXT_PUBLIC` means Next.js
knowledge was baked in; `.claude` means char assumed where worktrees live instead of asking
git; `backend/` and `web/` are the source repo's package directories, so a hardcoded one
means the `components:` abstraction did not take.

*Why permanent:* from phase 3 onward the plan runs charkit **against** the Chariot checkout
in read-only parallel to compare verdicts, and phase 6 is a Chariot PR. Pasting a real
Chariot path into `src/` to reproduce a mismatch is an entirely ordinary thing to do, and
this is what catches it.

*The known cost, chosen deliberately:* `tilt`, `backend/` and `web/` are ordinary words the
plan itself uses as illustrations. With no allowlist, `src/` simply may not contain them —
including in docstrings and test fixture strings. Use neutral examples instead (`foreman
start`, `root: services/api`). This was chosen over an allowlist because an allowlist is a
weakening mechanism that gets used under deadline pressure and never reviewed afterwards.

*What it cannot cover, and why:* **`docs/harvest.md`.** That document is the designed conduit
from the source repo into the implementer's context — the one place where the banned strings
are most likely to appear and most likely to matter. It is deliberately **not** greped,
because it legitimately has to say things like *"`check.py` assumes `uv run --directory
backend`, which must not survive the rewrite."* Banning the word would forbid recording the
very assumption the implementer needs to strip.

So the harvest doc gets a positive rule rather than a prohibition:

> **The harvest doc describes behaviour. It does not carry implementation.** Prose, tables and
> trap descriptions; no verbatim implementation code. Short fragments of *config* or a *regex*
> are fine. The test: could this be pasted into `src/` and compile?

The reasoning is that the two contamination types have different vectors. **Transcription** —
a Chariot path ending up in the code — is already caught downstream by the grep over `src/`
and `tests/`. **Structure** — the shape of `check.py` reproducing itself in a rewrite — is
carried by pasted code, and is exactly what [`PHASES.md`](PHASES.md) §8.1 says no grep can catch.

**This one is policed, not structural, and that is not a failure to fix.** No mechanical rule
can distinguish a legitimate mention of the source repo from a leaked one inside a document
whose entire job is to discuss the source repo. It is a review rule with a stated test, and
the honest thing is to say so rather than to invent a gate that would only produce false
confidence.

*What it does not catch:* **anything subtle.** A green grep means no crude contamination and
nothing more. The higher-severity risk — an abstraction shaped around Django+Next because
that is the only repo anyone saw — is invisible to it. That is what the six fixture configs
are for.

---

### 2.5 Versioning: 0.x indefinitely

charkit publishes at `0.x` and does not commit to a `1.0`. A changelog is maintained from the
first publish.

No stability is promised, which is honest for a tool with one or two consumers. The
consequence is recorded in §1.6: since the package version carries no compatibility signal,
**`schema_version` is the only one**, which raises its importance rather than lowering it.

---

### 2.6 Dogfooding: a test through phase 6, the gate after it

This is staged deliberately. The end state is charkit gating itself with itself; the interim
arrangement exists only while charkit is still being built.

**Phases 3–6 — dogfooding is a test.** charkit has its own `char.yml` from phase 3. The gate
runs the **raw tools** — `cargo clippy`, `cargo fmt --check`, `cargo test`. A single test,
runs `char check --json` and asserts it reaches the same verdict and that every check id
resolves.

**Once phase 6 lands — `char check` becomes the gate.** Phase 6 is Chariot adopting charkit:
`scripts/char/check.py` deleted, the dependency taken, `char check --all` green. That is the
point at which a real repository is already trusting `char check` as its own merge gate — so
it is trustworthy enough for this one. The dogfood test is then replaced by the real thing,
and the raw commands stay documented in the README as the fallback.

**Why not gate on `char check` from phase 3.** If `char check` is the gate and `char check`
breaks, every PR fails — including the PR that fixes it. The alternative is a written
break-glass procedure, and break-glass procedures have three problems: they are rarely
exercised so they rot, they demand careful judgment at the moment of least judgment, and a
`--skip=lint,test` in your shell history does not stay confined to emergencies.

Deferring the flip is not giving up the forcing function — break `char check` during phases
3–6 and the dogfood test fails, so you still cannot merge until you fix it. What it buys is
that a bug in a tool still under construction cannot lock its own repository, during exactly
the period when such bugs are most likely.

**A caveat worth writing down:** charkit's own `char.yml` is one component, pure Rust, no
services — structurally the *simplest* fixture shape. Dogfooding therefore pulls the design
toward that shape. "It works on charkit" is not evidence the abstraction generalises. The six
fixtures and phase 8 are.

---

### 2.7 Phase 3 is clean-room

The plan called phase 3 a copy. It is a **clean-room rewrite**, split across two agents:

| | Reads | Produces |
|---|---|---|
| **Harvester** | `~/Development/chariot` | `docs/harvest.md` — a behaviour spec plus a written list of every trap and bug-shaped branch found. Plus the ported test **cases**. |
| **Implementer** | `PLAN.md`, this document, the fixtures, the harvest doc, the tests. **Never opens the Chariot repo.** | `src/` |

**Why rewrite rather than copy.** Two reasons already force it. The scheduler is a reducer
(§1.2) and the original's almost certainly is not, so the hardest part was being rewritten
regardless. And reshaping 3,383 lines of foreign code into `core`/`adapters`/`cli` with a
`Ctx` and three seams is usually more work than writing to the principles directly.

**Why the harvest step is mandatory.** The value in those lines is not the code — it is the
**empirically discovered bug fixes**, branches that exist because something broke in
production once. Two are named in the plan ("load-bearing comments about two Playwright
traps"). The ones that are *not* commented are the danger: a bug fix looks like an
unremarkable three-line conditional, and nobody reviewing a rewrite notices it is missing.
This matters more here than in a normal rewrite, because charkit gives up continuous
real-repo validation until phase 6 — so knowledge lost cannot be re-derived by running the
thing. You would rediscover those bugs in phase 6, the one PR the plan already flags as
carrying the most rework.

**Why the split.** *Structural guarantees beat policed ones* — the plan's own argument for
building greenfield, applied one level deeper. Structural contamination becomes very hard
when the agent typing the code has never seen the structure.

#### Making it structural rather than aspirational

"The implementer never opens the Chariot repo" is a sentence in a prompt, and a sentence in a
prompt is policed, not structural — the weaker thing this section exists to reject. Three
mechanisms, in descending order of how much weight they carry:

1. **A `PreToolUse` hook, keyed on `agent_type`.** This is the only one that actually
   enforces. Verified in phase 0: the hook input carries `agent_type` and `agent_id`, and a
   hook denies a call by returning `permissionDecision: "deny"`. Default-deny the source-repo
   path for **every** agent, and allow it for the harvester alone — an allowlist, so a new
   agent added later is denied by default rather than silently permitted.
2. **A narrow `tools:` list** on the implementer. Real but coarse: `tools:` is an allowlist
   and cannot express paths, so it reduces surface without addressing the actual boundary.
3. **Prompt instructions.** Documentation of intent. They are why the agent *wants* to
   comply; they are not why it *cannot* violate.

The hook must inspect `tool_input` as a whole rather than a `file_path` field, because the
path can arrive through `Read`, `Glob`, `Grep`, or a `Bash` command containing `rg`, `find`,
`cat`, or `python -c`. Anything less covers the polite failure and misses the interesting one.

**It ships in phase 1**, with the rest of the repo scaffolding, so it exists before phase 3
needs it. A guard added at the moment it is first needed has already been unenforced for
every commit before that.

**The test cases are ported, not rewritten.** The source suite is 2,694 lines asserting on
behaviour rather than implementation, and the plan calls it the single most valuable asset.
Rewriting it from scratch discards the valuable part and keeps the cheap part. One honest
caveat: it is built around `run_fn` injection and charkit uses three seams behind a `Ctx`, so
it is **port the cases, rewrite the harness** — the assertions survive, the setup does not,
and the scheduler tests change shape because the scheduler did.

---

### 2.8 Which document owns which fact

Every defect found in the second review round was the same shape: **a fact stated in several
documents, where one copy drifted.** The contamination grep drifted into a pattern that could
not match. The error-class enum lost two members in the document agents read first. A deleted
file stayed cited as live in six places. None was a reasoning error; all were duplication.

So two rules, and the second is the one that prevents rather than resolves.

#### Precedence, when two documents disagree

```
traps.md            measured    — beats everything; it is the only document with evidence
ARCHITECTURE.md     decided     — how, and why
PLAN.md             specified   — what to build
PHASES.md           sequenced   — in what order
AGENTS.md README.md derived     — never authoritative, ever
```

A disagreement is a **defect in the lower document**, not a judgement call. Fix it, and say
which one was wrong.

#### Single ownership

| Fact | Owner |
|---|---|
| Contamination grep pattern and its scope | `ARCHITECTURE.md` §2.4 |
| Exit-code map and the signal carve-out | `ARCHITECTURE.md` §1.6 |
| Error classes | `ARCHITECTURE.md` §1.7 |
| `schema_version` and its bump rule | `ARCHITECTURE.md` §1.6 |
| The merge gate's checks | `ARCHITECTURE.md` §2.4 |
| Test tiers | `ARCHITECTURE.md` §3 |
| Harvester / implementer split | `ARCHITECTURE.md` §2.7 |
| `--json` envelope and `data.results[]` | `PLAN.md` §3.1 |
| Terminal-state enum | `PLAN.md` §3 |
| Measured environment behaviour | `traps.md` |

> **A derived document may not state a fact that is absent upstream.** `AGENTS.md` and
> `README.md` are summaries: they may restate an owner's fact in shorter form, and they may
> link. They may not be the only place something is written down — if a rule exists only in a
> derived document, it does not exist.

**Where a summary is allowed to remain**, it is because agents read `AGENTS.md` first and
sending them elsewhere for the exit-code map on every lookup is a worse trade than one
maintained copy. That is a deliberate exception, and it is why the precedence order exists:
when the copy and the owner disagree, the owner wins and the copy is the bug.

---

## 3. Decisions recorded (PLAN.md §0.3)

| Question | Answer | Reasoning kept |
|---|---|---|
| Public or private | **Public** | Phase 7 publishes release binaries and the install story is a `curl` one-liner, so it is public in effect regardless — a public package with a private source repo has no issue tracker and no source link. Also makes Actions free. |
| License | **Apache-2.0** | Explicit patent grant, clears corporate legal review, no adoption cost. |
| CI | **Both** — `no-mistakes` primary, minimal Actions matrix alongside | Actions supplies the one thing a local gate cannot: a machine that is not yours, and Linux as well as macOS. Process groups, signals and file locks are load-bearing; verifying real process-group kill only on macOS leaves the platform most users are on untested. Free on a public repo. `no-mistakes` keeps the agent review step, which is the only actual review in a solo repo. |
| Typing | **The compiler** | The decision that produced "mypy strict from commit one" is satisfied for free and more strongly: there is no gradual-typing escape hatch and no `Any` leaking in from untyped dependencies. Deny `unsafe` crate-wide except where `traps.md` records a required exception. |
| Rust edition / MSRV | **2021 edition** | MSRV pinned in `Cargo.toml`, raised deliberately. Users are unaffected either way — they receive a static binary, so no toolchain is required to run `char`. |
| Test layers | **Unit + integration + e2e** | Hermetic unit tests mean nothing exercises real process-group kill, real concurrent claim races or real docker labels — the exact failures char exists to prevent. The e2e tier turns phase 4's done-when scenario from a manual check into a test. |
| Coverage | **Gated, ratchet floor** | Floor is set by the first real measurement and may only rise; a PR that lowers coverage fails. Chosen over a fixed percentage because no project data exists to ground a number — 80 and 90 are convention, not evidence. `#[coverage(off)]`, or a documented exclusion, with a reason comment, is the escape for genuinely untestable lines. |

### Test tiers

| Tier | Contents | Speed |
|---|---|---|
| `tests/unit/` | Pure core. Scheduler reducer, scope resolution, verdict aggregation, config resolution, port selection. No I/O of any kind. | fast |
| `tests/integration/` | Real subprocesses and real files. Process-group spawn and kill with no orphans surviving; two directories claiming ports concurrently; compose up/down with labels verified gone. | slow |
| `tests/e2e/` | The real CLI against scratch repos, end to end. | slowest |

Integration and e2e run on both `ubuntu-latest` and `macos-latest` in the Actions matrix.

---

## 4. What was deliberately not decided here

- **Anything in PLAN.md §10.** Those decisions were made with rationale recorded. Do not
  relitigate them.
- **Module layout below the three-package split.** `core`/`adapters`/`cli` is fixed; what
  lives inside them is a phase-1 and phase-2 concern.
- **The `char.yml` schema.** That is phase 1's entire output, and the plan is explicit that
  it should change while the six fixtures are being written.
