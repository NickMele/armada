# Port allocation — what v1 did, and what it cost

**Reference, not a scope recommendation.** Notion decides what v2 builds. This
note says how v1 did it and what it cost, for whoever builds anything nearby.

Read from `v1-final`: `crates/core/src/ports.rs`, `crates/manifest/src/db.rs`
(`workspaces` and `leases` tables, `Db::claim_block`), `docs/PLAN.md` §3.1,
`docs/traps.md` (SQLite `BEGIN IMMEDIATE` section).

## What it actually solved — not Fleet's own listener

**v1's port blocks belong to a *workspace* (one worktree), not to the
daemon.** `ports.rs`'s own header: "A block is claimed once at `armada
manifest init` and kept until `armada manifest clean`." It exists so that
concurrent worktrees of the same repo, each running the repo's own declared
services (`ports:` under a `command`/`docker` driver in `armada.yml` —
`api`, `pg`, etc.), do not collide on the same port number when N worktrees
run those services at once. It has nothing to do with any port the Armada
tool itself binds — v1 had no listening service of its own. `manifest.db`'s
`workspaces` table stores one `(port_from, port_to)` range per workspace id;
`leases` is a separate table (heartbeat/boot-id/pid rows for held resources
generally, keyed `(kind, key)`) and is not itself port-specific.

**v2 answer, stated plainly.** v2 serves everything Fleet does on one axum
listener — one process, one control-plane port, chosen once at Fleet's own
startup and written to the runtime file the daemon-lifecycle note describes.
That is not a leasing problem: there is exactly one Fleet per machine, so
there is no second claimant to collide with, and no pool to allocate from.
**What this was for, stated plainly so it is not mistaken for something else:** the
blocks belonged to a worktree, not to the daemon. Fleet's own listener had nothing
to do with them. Anyone reading this while building port handling should start
from that, because the obvious wrong reading is that Armada was allocating ports
for itself.
concurrent, repo-declared services running per worktree** (the `command`/
`docker` driver use case `ports.rs` was actually built for). It is a workspace
concern layered on top of Fleet, never something Fleet's own listener needs —
which is the distinction to carry into any reading of this mechanism.

## Port, adapt or reject, by piece

| Piece | Verdict | Reason |
|---|---|---|
| `choose_block` (lowest-free-block-of-size-N search) | **Reject for now** | No allocation pool exists in v2 yet — Fleet is one listener, not N leased blocks. Revive only if per-worktree repo services return. |
| `PORT_BASE`/`PORT_CEILING` constants and the "stay below the ephemeral range" reasoning | **Adapt if revived** | The reasoning (avoid the kernel's own ephemeral client-socket range, `32768–60999` on Linux) is a real constraint that will recur verbatim if any port-block feature returns. |
| `workspaces` table keyed by workspace id, holding a claimed range | **Reject as designed, port the idea of one-row-per-claimant** | v1's row conflates "this workspace still exists" with "this workspace holds a port block" (`claim_block(size: None)` registers a row with no block, purely so the workspace is reclaimable later) — a coupling worth *not* repeating; if v2 needs claim tracking, it should not also be the existence record for the thing making the claim. |
| `BEGIN IMMEDIATE` + `INSERT OR IGNORE` race handling | **Port the pattern, not the code** | Directly reusable *if* v2 ever has two writers claiming from one pool concurrently — SQLite's documented behavior (`docs/traps.md`: `DEFERRED` read-then-write fails non-retryably under a concurrent committed writer; `IMMEDIATE` serializes and always eventually succeeds) is the correct primitive for any future claim table, Fleet's runtime file included if it is ever backed by SQLite rather than a single JSON file. |

## What it cost v1 to get right

- **The base had to be a parameter, never a constant read inside the pure
  function.** An early design would have hardcoded `PORT_BASE` inside
  `choose_block`; the shipped version threads `base` in from the caller,
  because a fixed default left a user whose port 5460 was already taken with
  no way to move — `ports.rs`'s own comment: "every Armada on it reached for
  the same first port, so two concurrent workspaces collided on a port
  neither of them owned and the failure looked like flakiness." The fix was
  making `port_base` a machine-level override (`~/.armada/machine.yml`), and
  keeping the constant as only the default the caller starts from.
- **A workspace declaring no ports must claim no block.** `needs_block`
  exists specifically so a workspace with nothing to collide over does not
  eat ten ports from a finite pool that a workspace which *does* need them
  then cannot get — a real defect class the tests were built to catch, not
  hypothetical.
- **The claim transaction had to be `BEGIN IMMEDIATE`, not the obvious
  default.** `docs/traps.md`'s SQLite section is explicit that a naive
  read-then-write inside a `DEFERRED` transaction passes every single-writer
  test and then fails non-retryably (`SQLITE_BUSY_SNAPSHOT`, not a
  transient `SQLITE_BUSY`) the moment two claims race — and `busy_timeout`
  does nothing for that failure mode, because the reader's snapshot is
  already stale by the time it tries to write. This is the single most
  expensive-to-rediscover fact in this whole subject if a claim table
  returns.

## Test cases implied, if a claim table returns

- Two workspaces claiming concurrently must get non-overlapping blocks
  (`two_workspaces_get_non_overlapping_blocks`, ports directly).
- A block released by `clean` must be reusable by a later claim, including
  filling a hole between two still-held blocks, not just appended at the end
  (`a_hole_left_by_a_released_block_is_reused`).
- A hole too small for the requested size must be skipped, not split
  (`a_hole_too_small_is_skipped`).
- Exhausting the range (nothing free below `PORT_CEILING`) must return "no
  block available," not panic or wrap (`blocks_never_reach_the_ephemeral_range`).
- A machine with a non-default `port_base` must not have its blocks pushed
  around by another machine's rows sitting below that base
  (`a_block_below_the_base_does_not_push_the_first_one_up`).
- A claim racing another claim under `BEGIN IMMEDIATE` must serialize rather
  than corrupt or double-allocate — this is the case `docs/traps.md` says a
  `DEFERRED` transaction gets silently wrong under real contention, not in a
  single-threaded test.

