# Spike 13 — What does one gate's reading of the worktree cost?

**Between 14 and 25 milliseconds, and the size of the diff is not what decides it.** A worktree
with nothing changed reads in 15ms; one with 449 changed files reads in 18ms on the same quiet
machine. What the reading costs is the walk of the working tree, and the diff rides along on it.

Against a median step of 131s — spike 9, same repository — the reading is **0.011%** of a step,
and it is taken once, after the Checks have already spent seconds in subprocesses. The worst
single reading observed anywhere in 270 calls was 158ms, on a machine carrying a load average of
40, and that is 0.12% of a median step.

**So `#431` costs nothing anybody can perceive**, and the absolute tier is now answered on every
step of every workflow rather than only where the gate was already reading. Measured 2026-09-04,
before the change was made.

## What was measured

`WorkProduct::changed_files` as `crates/adapters` implements it: open the repository, walk to the
merge base between the Job's branch and the repository's own HEAD, and diff that tree against the
working directory with untracked files included. It is the exact call `fleet::gate::rule_on` makes
and the only new cost the change carries.

[`013-measure.rs`](013-measure.rs) is the harness — the workspace's own adapter, linked by path,
so what is timed is the shipped code and not a re-implementation of it.
[`013-run.sh`](013-run.sh) builds and drives it, [`013-samples.csv`](013-samples.csv) is every
call, and [`013-summaries.txt`](013-summaries.txt) is the harness's own per-batch summary.

| | |
|---|---|
| Repository | this one — 1,247 tracked files, 11,352 files on disk, a 1.4GB `target/` |
| Worktrees | one linked worktree in a Drone's shape, build directory and all, and the ordinary checkout |
| Build | `--release`, which is how Fleet ships |
| Calls | 270 — three scenarios, four batches, 20 or 30 calls each |
| Machine | one laptop, running three other agents' builds and test suites for three of the four batches |

## How long it takes

| worktree | changed files | min | median | p90 | max |
|---|---|---|---|---|---|
| **Quiet machine — load 10** | | | | | |
| nothing changed | 0 | 13.8ms | **15.0ms** | 17.5ms | 31.9ms |
| a step's diff | 12 | 16.3ms | **19.5ms** | 26.3ms | 28.1ms |
| far more than a step's | 412 | 15.8ms | **18.0ms** | 22.7ms | 25.1ms |
| **Busy machine — load 37 to 44** | | | | | |
| nothing changed | 0 | 13.7ms | 24.7ms | 46.2ms | 103.0ms |
| a step's diff | 49 | 15.1ms | 26.9ms | 44.3ms | 96.6ms |
| far more than a step's | 449 | 19.8ms | 39.5ms | 71.8ms | 158.4ms |

**The load average moves the answer more than the diff does**, which is the one result here worth
carrying forward. Thirty-four times as many changed files cost 3ms on a quiet machine; four other
Rust builds cost 10 to 20. Fleet runs several Drones at once by design, so the busy rows are the
production rows and the quiet ones are the floor.

**The first call in a fresh process costs 22 to 36ms**, roughly one extra reading's worth, which
is libgit2 opening the repository and its object database caches filling. Fleet is long-lived and pays it
once per daemon, not once per gate.

## The scan itself is not the cost

The absolute tier — every changed path against the four compiled-in boundaries — is a string walk
over a list that is already in memory:

| paths | median | max |
|---|---|---|
| 12 to 49 | 10µs | 19µs |
| 412 to 449 | 45µs | 181µs |

**Three parts in ten thousand of the reading it is answered over.** Nothing here would be saved by
making the check conditional; the whole cost is the reading, and the reading is what `#431` had to
decide about.

## The reading was already being taken

`fleet::settling` calls `changed_files` on **every ruling of every step**, unconditionally, to
write the step's own `Produced` row into its transcript. So the shape of this cost was not new
when the gate stopped being conditional — what changed is that the same reading is now taken
twice per ruling rather than once.

Not folded together here. Reusing the gate's reading in `settling` would mean the `Ruling` type
carrying a diff from the gate out to its caller, and a second 20ms is not worth a field on the
value every module in `fleet` matches on. It is written down rather than hidden: if a gate's cost
is ever measured again, this is where the other half of it is.

## What was not measured, and would change the answer

- **A cold filesystem cache.** `purge` needs a password this run did not have, so every number
  here is against a page cache that had the worktree in it. A machine that has just booted, or a
  repository that has not been touched in a day, pays for reading the tree from disk, and nothing
  here bounds that.
- **A repository larger than this one.** 11,352 files on disk is small. The cost is a walk of the
  working tree, so it scales with the tree rather than with the diff — a repository ten times the
  size should be assumed to cost roughly ten times this, and that assumption is untested.
- **A repository with a slow filesystem.** One local APFS volume. A network mount would be a
  different measurement entirely.
- **Whether the walk can be made cheaper.** `include_untracked` with `recurse_untracked_dirs` is
  what the adapter asks for and it is what makes the walk complete; whether libgit2 has a cheaper
  route to the same file list was not investigated, because at 15ms nothing needed one.

## What this contradicts

`crates/adapter-traits/src/work_product.rs` says of `changed_files` that it is *"a delta walk
costing under a microsecond"* over the same three diff sizes it quotes for `counted_files`. That
is off by four orders of magnitude against this repository: the cheapest reading in 270 calls was
13.7ms. The sentence's argument is unaffected — `counted_files` is 90ms where `changed_files` is
15ms, so it is still the expensive one and still belongs off the live seam — but the number
beside it is wrong and nothing was ever measured to produce it. Not corrected here, because that
file is outside this change's scope.
