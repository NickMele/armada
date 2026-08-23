# Daemon lifecycle — what v1 solved and what it left unresolved

Read from `v1-final`: `crates/fleet/src/daemon.rs` (548 lines — the prior
audit's line numbers, 84-121/239-429/59-68/85-87, do not match this file as it
stands; the mechanisms they describe do exist, just at different offsets),
`crates/manifest/src/posix.rs`, `crates/manifest/src/process.rs`,
`crates/fleet/src/daemon_log.rs`, `docs/traps.md`.

## The tension, stated first

**v1 shipped two lifecycle models, not one.** `daemon.rs`'s own module doc
says so directly: `armada daemon enable` on macOS installs a launchd job and
never calls `start()`; `start`/`stop` exist for "the machine that has no
launchd at all... without going through `armada daemon enable`." Two
supervisors were deliberately kept apart (calling both was rejected by name,
in the doc, as "hand[ing] the same job to two supervisors"), but that is
segregation, not resolution — v1 never had to choose because it never had a
Bridge process depending on the daemon being *findable*, only on it being
*running*. v2's requirement — Fleet outliving Bridge, found through a runtime
file, pid-verified before connecting — is a third model neither of v1's two
was built to answer. Don't inherit the framing that setsid-detach and launchd
are alternatives to pick between rather than layers. Worth knowing before
reading either: they answer different questions. setsid decides whether a child
survives its parent; launchd decides whether a process comes back after it
dies. v1 never wrote down which it was relying on, and that is the confusion
this note exists to hand over.

## 1. setsid detach — port, and required unconditionally

**What it is.** `posix::new_session` (`crates/manifest/src/posix.rs`) calls
`libc::setsid()` in `pre_exec`, between fork and exec. `process::ProcessGroup::spawn`
(`crates/manifest/src/process.rs`) is the one function that constructs every
child; `RunRequest::new_session` defaults to `true` (`crates/core/src/ctx.rs`),
so every spawn is detached unless a caller explicitly opts out.

**Port.** Directly. This is exactly what a Drone needs: launchd (once wired)
signals a job's whole process tree, and an undetached Drone dies at every
Fleet restart — the task's own framing, and v1's code already treats this as
non-negotiable for anything meant to outlive its parent.

**What it cost.** Two measured traps, both in `docs/traps.md`:
- `setsid()` and `Command::process_group(0)` are mutually exclusive — setting
  both fails `Operation not permitted (os error 1)`, because `setsid` refuses
  when the caller is already a process-group leader, which `process_group(0)`
  just made it. One flag, not two.
- `killpg` against a group holding only a zombie diverges by platform:
  `EPERM` on darwin, succeeds on Linux. Fixed three times before landing on
  the real rule (docs/traps.md: "the zombie-group entry was wrong a third
  time") — reap before reading the probe as empty, in the probe's own path
  (`posix::stop_group` calls `posix::reap_group` first), not in three patched
  callers.

**Test cases implied.**
- A liveness/group-alive check against a process group whose leader has
  already exited but is unreaped (zombie) must not report "alive" or error —
  run on both darwin and Linux; the two diverge.
- `setsid` + `process_group(0)` on the same spawn request must be rejected at
  construction, not discovered at spawn time.
- A Drone spawned before Fleet restarts must still be alive and reachable
  after the restart, whether or not Fleet itself is launchd-managed — the
  test v1 never had reason to write, because v1 had no long-lived Fleet a
  Drone needed to survive independently of.

## 2. Pidfile-based liveness + idempotent start — port, extend the schema

**What it is.** `daemon::is_running` reads `~/.armada/daemon.pid`, parses it,
and calls `posix::group_alive` — a stale, corrupt, or zero pidfile all read as
`None`, never an error (fail-safe by construction). `daemon::start` is
idempotent: if `is_running` finds a pid, it returns that pid rather than
spawning a second daemon.

**Port, with a required extension.** The pidfile-and-liveness shape ports
directly, but v1's file carries only a pid. v2 needs Bridge to find Fleet
through a file carrying **port, pid, and protocol version**, and to verify
the pid before treating the daemon as reachable — a stronger claim than v1
ever had to defend, because v1 had no external client asking "is this
actually you." `group_alive` alone proves *a* process holds that pid; it does
not prove it is *this Fleet's* boot, the same gap `armada_core::reap::pgid_is_ours`
exists to close for a Drone (boot id + process start time, guarding against
pid recycling). The daemon module's own header argues this gap is
"acceptable, and cheap" for v1 because there is only ever one daemon and it
checks its own pidfile. That argument does not survive Bridge becoming a
second, independent reader of the same fact.

**What it cost.** Nothing dramatic — the design note in the module header
(the pid-recycling window "nothing but the OS's own pid-reuse policy
bounds") was accepted as a known, low-probability gap rather than closed.

**Test cases implied.**
- Runtime file names a pid that is alive but belongs to an unrelated process
  (recycled pid) — Bridge must not treat this as Fleet running. v1 has no
  test for this because v1 never needed one.
- Stale runtime file (Fleet crashed, file not cleaned up) reads as "not
  running," not as an error — v1's own three tests for this
  (`a_stale_pidfile_is_not_running`, `a_pidfile_that_does_not_parse_is_not_running`,
  `a_zero_pid_is_never_alive`) port directly.
- `start` called twice returns the same pid, second call is a no-op on the
  file (`starting_writes_the_pidfile_and_the_log_and_is_idempotent`) — ports
  directly.
- `stop` against a missing runtime file is not an error; against a file
  naming an already-dead pid, it still cleans up and still logs
  (`stopping_with_no_pidfile_is_not_an_error_and_logs_nothing`,
  `stopping_a_stale_pidfile_still_cleans_up_and_logs`) — port directly.

## 3. JSONL audit log — port

**What it is.** `daemon_log.rs`: `~/.armada/daemon.jsonl`, append-only,
three-variant enum (`Started`/`Stopped`/a `gh`-unreachable machine fact),
folded on read, a torn last line skipped rather than fatal. No lock — the
daemon is documented as the file's only writer, so `OpenOptions::append` is
enough.

**Port.** Directly, no changes implied by the task's requirements.

**What it cost.** Nothing found in history beyond the design decision itself
(single-writer argument, reused from `crate::inbox`'s own doc comment).

**Test cases implied.** A torn last line (process died mid-write) must not
make earlier entries unreadable — implied by the module's own doc comment,
not exercised by a test in this file; worth adding explicitly in v2 rather
than inheriting the gap.

## 4. launchd plist install and verify — the verification step is the transferable part

**What it is.** `daemon::launchd` (macOS only): writes
`~/Library/LaunchAgents/com.armada.daemon.plist` with `RunAtLoad` and
`KeepAlive` both true, `launchctl load -w` to install, `launchctl unload -w`
to remove.

**Port — the verification step, not the mechanism itself, and not yet.**
`launchd::install` does not just write the plist and load it; it then calls
`loaded()`, which checks `launchctl list <LABEL>`'s own exit code (0 =
launchd holds the job, 113 = it does not). This exists because of a real bug,
commit `c244806`: **`launchctl load -w` exits `0` whether it worked or not.**
Measured on the author's own machine — it printed `Load failed: 5:
Input/output error` and exited `0`, and the naive `status.success()` read
that as success, so `armada daemon enable` reported the daemon on when
launchd held nothing. Worse, the same "Load failed: 5" string is also what an
*already-loaded* job prints on an ordinary second `enable` — so stderr
sniffing could not be the fix either, without breaking idempotence. The fix
asks launchd a direct question (`launchctl list`) instead of inferring from a
stream. This is the transferable lesson, independent of whether v2 uses
launchd on this timeline: **never trust a supervisor CLI's own exit code as
proof of the state it claims to have created — always read the state back.**

**What it cost — the load-bearing trap.** The same commit fixed a second,
worse bug: the plist was written to `~/.armada/Library/LaunchAgents/`
instead of `~/Library/LaunchAgents/`, because a call site passed
`armada_home` where the function's own parameter and doc comment named
`$HOME`. Both are `&Path`, so the type system did not catch it. `launchctl
load` takes an explicit path and loaded the job fine from the wrong
location — pid visible in `launchctl list`, everything looked correct — but
launchd only *auto-loads* from its standard directories, so the daemon whose
entire purpose was surviving a login was going to silently vanish at the next
one. The existing unit test for `plist_path` stayed green the whole time
because it asserted the function's behavior with a synthetic `$HOME`, which
was never in question — the bug was at the call site, three of them. It took
a test that runs the real binary against a scratch `$HOME` and inspects which
directory the file actually lands in to catch it.

**Test cases implied.**
- Any code path that installs a supervisor-managed job must assert the
  installed file's location by running the real entrypoint against a scratch
  home directory, not by unit-testing the path-construction function alone
  with a synthetic input — the exact gap that hid this bug.
- `launchctl load -w` succeeding by exit code must not be trusted; the
  install path must independently confirm via `launchctl list <label>`.
- A second `enable` after a `disable` (job present but `Disabled` in
  launchd's own database) must still succeed — this is what `-w` is for, and
  what makes the "Load failed: 5" string ambiguous between failure and
  ordinary idempotent reload.
