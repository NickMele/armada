# Spike 8 — Does v1's detach mechanism leave the piped-stdin secrets path open?

**Answer: no, and the premise holds for only one of v1's two detached spawns.** v1 did have a
piped-stdin secrets path, and `setsid` did not leave it open: the write end is closed inside
`ProcessGroup::spawn` before that function returns, and the payload is read to EOF at the child's
entrypoint before the child spawns anything. That path served `armada manifest check --detach`.
**It was never on a Drone spawn.** v1's Drone got two environment variables, `Stdio::null()` on
stdin and no secret of any kind.

**Read, not measured.** Everything below is `v1-final` source at the paths and line numbers given.
The only measured fact is labelled as such: `ps -Eww` on darwin 27.0.

## What was read

| Path at `v1-final` | Lines | What it holds |
|---|---|---|
| `crates/helm/src/secrets.rs` | 1-77, 95-200 | `Vault`, `handoff`, `inherit`, `granted`, `mask` |
| `crates/core/src/secrets.rs` | 1-50, 170-200 | the half that never sees a value; `provider_argv` |
| `crates/manifest/src/process.rs` | 92-205 | `ProcessGroup::spawn` — stdio wiring, setsid, the write |
| `crates/manifest/src/posix.rs` | 78-123 | `restore_sigpipe`, `new_session` |
| `crates/helm/src/verbs/check.rs` | 395-440, 640-670, 1550-1575 | resolve, the detach handoff, the per-check grant |
| `crates/helm/src/main.rs` | 44, 898, 1777-1834 | SIGPIPE, entrypoint, `detach_handoff` |
| `crates/helm/src/app.rs` | 77-87 | `App::handoff` |
| `crates/fleet/src/drone.rs` | 40-51, 174-245 | `job_env`, `start` — the Drone spawn |
| `crates/helm/tests/secrets.rs` | whole | the end-to-end greps |
| `docs/reserved/013-secrets-must-resolve-before-the-run-detaches.md` | whole | the rule and its reasoning |
| `docs/traps.md` | 280-300, 620-640 | SIGPIPE, setsid, the `unsafe` carve-out |

Commits: `b25a248` landed the whole path in one go; `363ab1e` extended it to `commands:`. No
repeated fixes, so nothing here cost v1 a night.

## How v1 handed a secret to a spawned process

One channel per kind of child.

| Child | Channel | Code |
|---|---|---|
| A provider (`op read …`) | argv, `${ref}` as one element | `core/src/secrets.rs:170-200` |
| A check, attached or detached | environment, per-entry grant | `check.rs:1568` |
| The detached `check` child | JSON on stdin, one write | `check.rs:665`, `process.rs:189-195` |
| A Drone | **none** | `fleet/src/drone.rs:187-245` |

`Vault` (`secrets.rs:95`) is the only type that holds a value. It has no `Serialize` and a hand-written
`Debug` printing names. `handoff()` (`secrets.rs:178`) serialises to JSON and has exactly one caller.

## What detaching did to the path

`posix::new_session` (`posix.rs:108-123`) installs `libc::setsid()` in `pre_exec`. Each consequence
below is confirmed in the code rather than inferred.

**The pipe survives.** `setsid` changes session and process group, never file descriptors. Stdio is
wired at `process.rs:107-168`, before the fork; fd 0 is the pipe read end after exec.

**The write end closes before spawn returns.** `child.stdin.take()` at `process.rs:190` moves the
handle into the `if let`, which drops it at line 194. The child sees EOF immediately.

**The payload cannot take a second hop.** The detached child runs its checks under `StdioMode::Capture`
(`check.rs:1572`), and that arm gives a child `Stdio::null()` on stdin when the request carries no
`stdin` (`process.rs:107-118`). No grandchild inherits fd 0.

`detach_handoff` (`main.rs:1811-1834`) reads stdin to a `String` at the entrypoint, guarded:
`ARMADA_DETACH_RUN` must parse as a run id, the run must still be offering adoption, and stdin must not
be a terminal. Every guard exists to avoid blocking, not to avoid leaking.

## Was the secret readable after the handover

| Surface | Exposed | Why |
|---|---|---|
| `ps` argv | No | never in argv; `check.rs:1565` names the rule |
| `ps -Eww` environment, darwin | No | **measured**, darwin 27.0 — env not printed for a same-uid child |
| `/proc/<pid>/environ`, Linux | Owner only | v1 never ran on Linux; untested |
| Disk | No | `tests/secrets.rs` greps every file under `.armada/` and `$HOME` |
| The `--json` envelope, the progress stream | No | same test, both asserted |
| Run record, failure journal, ring buffer | No | those carry `EnvDelta::names()` and never values |
| The pipe, to a third party | No | two processes hold the ends; the write end is already closed |
| Process memory | **Yes** | see below |

**The value lives in the heap for the process's lifetime, twice over.** `App::handoff` (`app.rs:86`) holds the raw JSON,
and `Vault` holds the parsed values. Neither is zeroized, neither is `mlock`ed. A core dump, a
`sample`, or a debugger with `task_for_pid` carries both. v1 never claimed otherwise and never wrote
it down.

## What v1 got right, and it is most of it

| Mechanism | Verdict | Reason |
|---|---|---|
| Split pure planner from value-holding resolver | **Port the shape** | a function that never sees a value cannot leak one |
| No `Serialize`, hand-written `Debug` on the holder | **Port** | the derive is the leak; one deliberate exit, named `handoff` |
| Argv forbidden as a secret channel | **Port** | argv is world-readable through `ps`, measured above |
| Resolve before detaching | **Port the rule, not the code** | a `setsid` child has no controlling terminal and cannot prompt |
| `session(false)` for the provider itself | **Port** | `setsid` denies `/dev/tty`, so a provider spawned the ordinary way can never prompt |
| Provider stderr inherited, never captured | **Port** | a failing provider has no registered value to scrub against |
| Two redactors: exact-match plus shape | **Port both** | `hunter2` is a real password and no shape test will say so |
| Test fixture proven invisible to the shape redactor | **Port** | otherwise every grep passes against an implementation that does nothing |
| Piped stdin as the handoff channel | **Adapt** | correct for a payload; see the SIGPIPE defect below |
| Wholesale environment inheritance to every child | **Reject** | see below |

## The defects v1 left in this path

**The handoff write can kill the parent.** `main.rs:44` calls `posix::restore_sigpipe()`, setting
SIGPIPE to `SIG_DFL` process-wide, for the reason `traps.md:280` gives about `armada status | head`.
`process.rs:192` then writes the secret payload with `let _ = pipe.write_all(…)`. If the child exec'd
and exited before reading, that write raises SIGPIPE and the parent dies with exit 141, silently,
mid-spawn — after `offer_adoption` and after the pending owned row was written. `let _ =` cannot catch
it, because the signal is delivered before `write` returns. Never hit, never tested.

**A truncated payload reads as "no secrets".** `inherit` (`secrets.rs:196`) turns malformed JSON into
an empty `Vault` by design. Combined with a partial write, the run then fails on a grant it cannot
satisfy, naming the secret rather than the pipe. The message points at the config, and the config is
correct.

**The Drone's environment was inherited wholesale.** `env_clear` appears nowhere in v1's production
code — only at line 322 of `crates/helm/tests/support/mod.rs`. `Command::env` layers over the parent's
environment, so a token exported in the operator's shell reached every Drone `fleet::drone::start`
spawned. v1's own handoff table rejects "Armada's own environment" as a secret channel for exactly
this reason, and the Drone path inherits it anyway.

## What v2's spawn path must satisfy

Armada's baseline clause is *secrets are brokered and never held*
([Agent Prompt Contract](../contracts/agent-prompt.md), the baseline clauses). `crates/adapter-traits/src/lib.rs`,
lines 103-110, carries the `Secrets` trait returning a `Secret<String>` that cannot be printed. For the clause to be
true rather than aspirational, the spawn in **#26** must hold these.

| # | The spawn path must | Owner |
|---|---|---|
| 1 | Carry no secret material across the spawn boundary — not env, not argv, not stdin | the step |
| 2 | Build the Drone's environment from an explicit list, never inherit Fleet's | the step |
| 3 | Keep argv free of anything brokered, permanently | the step |
| 4 | Give the Drone `Stdio::null()` on stdin unless a payload is needed | the step |
| 5 | Handle `EPIPE` explicitly on any spawn-time pipe write, given SIGPIPE is `SIG_DFL` | the step |
| 6 | Keep `Secret<T>` free of `Debug`, `Display` and `Serialize`, with one named exit | `adapter-traits` |
| 7 | Redact twice — exact-match on what was brokered, shape-based on what was not | Fleet |

Requirement 2 is the one v1 evidence argues hardest for, because it is the only place v1's Drone spawn
was worse than its check spawn.

## Test cases this implies

The first two are the ones v1 could not have passed.

1. Export a planted value into Fleet's own environment, spawn a Drone, and read the Drone's
   environment from inside it. The planted value must be absent.
2. Assert the Drone's argv, captured by a stub that records what `execve` received, contains no
   brokered value. v1's integration suite already uses a recording stub for this.
3. Plant a value that is **first asserted invisible to the shape redactor**, run a Job that leaks it,
   and grep every file Fleet wrote. v1's `assert_absent` also asserts it searched something, so a walk
   that found nothing fails rather than passes.
4. A brokered response whose reader exits before the write completes must not kill Fleet. Spawn a
   child that exits immediately, write a payload, assert Fleet is alive and reports a named error.
5. A truncated or malformed brokered payload must fail naming the channel, not silently resolve to
   "no secrets".
6. A second Job running concurrently, granted nothing, must not see the first Job's secret. v1's
   `only_the_check_that_declared_the_grant_can_see_it` is the shape.

## What the harvest missed

`docs/v1-learnings/daemon-lifecycle.md` covers `setsid` in detail and says nothing about what crosses
it. What it would have needed: `setsid` touches no file descriptors, and `restore_sigpipe` makes
every unguarded pipe write a potential process death. Both are in the same
crate as the material it did cover.

`docs/v1-learnings/permissions.md` is unrelated and confirms only that v1's harvest never looked at
this subject.

## Honest limits

One reading pass, no execution of v1 code — v1 is deleted and was not rebuilt. The SIGPIPE defect is
derived from two verified lines and POSIX semantics, not reproduced. The `ps -Eww` result is darwin
27.0 and says nothing about Linux, where v1 also never ran.

## Artifacts

None. Every claim is a path and a line range at `v1-final`, checkable with
`git show v1-final:<path>`.
