<p align="center">
  <img src="docs/assets/armada-banner.png" alt="Armada — delegate without watching. Run five coding agents at once. Armada monitors them, verifies what they produce, and brings you only what needs a person." width="100%">
</p>

<p align="center">
  <a href="#status"><img src="https://img.shields.io/badge/status-pre--alpha-EE8450" alt="Status: pre-alpha"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-4A9EDB" alt="License: Apache 2.0"></a>
  <img src="https://img.shields.io/badge/rust-1.90-4FB8D9" alt="Rust 1.90">
  <img src="https://img.shields.io/badge/platform-macOS-8C97A6" alt="Platform: macOS">
</p>

---

## Status

**Armada is pre-alpha, and part of it runs.** There is one binary, `armada`,
with four verbs:

| | |
|---|---|
| `armada serve [<path>]` | The daemon. Binds a loopback port, publishes a runtime file, serves the API and turns Jobs until it is signalled |
| `armada check <name>` | Runs one Check the repository's `armada.yml` declares |
| `armada run <name>` | Runs one Command it declares. Checks gate advancement; Commands do not, and the two stay separate verbs |
| `armada clean [--all]` | Gives this repository's worktrees, branches and Jobs back |

[Running it locally](#running-it-locally) is how to start.

This repository is the second attempt, started from scratch in August 2026. It
has a build gate, a design-token pipeline, a component library, and a Rust
workspace that can drive a Job from approval to a finished branch.

**The acceptance test passes.** It was written before the code it tests and
could not compile for the whole of the first milestone — deliberately, so the
compiler's error list was the list of what the remaining crates had to provide.
It describes a Job that runs end to end, and it is how a milestone proves its
claim. `docs/practices/acceptance-tests.md` is the arrangement.

**What it does not prove is the interesting part.** It is hermetic — no process
spawned, no repository touched, no network opened — so it proves the machinery
and not the merge.

Watch the [milestones](https://github.com/NickMele/armada/milestones) if you
want to know when that changes.

## What it is

You can run several coding agents at once today. What you cannot do is stop
watching them.

Armada is a macOS application that dispatches coding agents against git
repositories and **verifies their work before advancing them**. An agent does
not decide it is finished. It submits evidence through a tool Armada gave it,
Armada runs the repository's own checks against the result, and only then does
the work move to the next step. Saying "done" in prose does nothing.

The problem it exists for is not throughput. It is that delegating work to
something that reports on itself means reading everything anyway.

**Three rules it is built around:**

- **An agent cannot mark its own work complete.** Evidence goes through a tool;
  a mechanical check decides.
- **An agent cannot push.** Every Job works in its own git worktree on its own
  branch. Work stays local until a person merges it.
- **An agent gets only the tools it is handed.** No inheritance from the
  machine, the shell, or anybody's credentials.

## How it is put together

Two processes that ship as a pair and version together.

| | | |
|---|---|---|
| **Fleet** | Rust daemon | Owns Jobs, spawns and supervises agents, runs checks, writes the record. Runs detached — quitting the app does not kill work in flight |
| **Bridge** | Electron app | The only way in. Holds one connection to Fleet and never talks to an agent directly |

They meet at exactly one seam: a versioned protocol over HTTP and a WebSocket.
Every other boundary in the system is a function call or a file. When the two
disagree about the protocol version they refuse to guess — a small set of
routes stays available so you can still see what is running and stop it.

```
armada/
├── crates/          the Rust workspace — Fleet, the domain, the adapters
├── apps/desktop/    Bridge
├── packages/        shared: design tokens, icons, and what surfaces both read
├── xtask/           the build gate
└── docs/            contracts, practices, spikes, and what v1 taught
```

**[`ARCHITECTURE.md`](ARCHITECTURE.md) is the map** — the process topology, the
crate graph, and the rules that hold everywhere, with diagrams.

## Building it

You will get a gate that reports red. **That is the expected output** — it
stays red until a milestone is finished, and each failing line names its own
subject. The tests pass.

**Requirements:** macOS, Rust 1.90 or later, Node 22 or later, and
[pnpm](https://pnpm.io).

```sh
git clone https://github.com/NickMele/armada.git
cd armada
cargo xtask verify-foundations   # red until a milestone finishes — read what it names
pnpm install
```

### The gate

Armada checks itself with a task rather than a CI config, so the same command
runs on a laptop and in CI. It has no dependencies and needs nothing built.

| Command | Checks |
|---|---|
| `cargo xtask verify-foundations` | Every rule below. **Red is the expected state** until a milestone is finished |
| `cargo xtask verify-tokens` | The design token outputs match the CSS they are generated from |
| `cargo xtask verify-docs` | Open questions are collected, and every citation of one resolves |
| `cargo xtask verify-roadmap` | Capabilities and their GitHub issues agree. Needs `gh` and a network — deliberately not part of the gate |

Some of what the gate refuses: untyped JSON outside the two crates allowed to
parse it, a vendor's name outside the adapter layer, a design value that is not
a token, a file over 900 lines, and anything that names a person or a machine.

### Tests

```sh
cargo nextest run --workspace --exclude acceptance   # not `cargo test`
cargo test -p acceptance                            # the milestone's own claim
```

## Running it locally

```sh
pnpm dev
```

That is the whole loop: it stops the Fleet that is running, reinstalls `armada`
from the working tree, starts Fleet, waits for it to publish, and starts Bridge
in the foreground. **Ctrl-C stops both** — which is a convenience of this script
and not how Armada behaves in earnest, where closing Bridge leaves Fleet running.

**Reinstalling every time is the point, not overhead.** `cargo install` copies
rather than links, so an edit does not reach the installed command until it is
run again — and a stale `armada` publishing an older protocol to a Bridge built
from the current tree reads as a version-skew screen rather than as the stale
binary it is. Rebuilding one side and not the other is the mistake this script
exists to make impossible. It installs `--debug` for the same reason: a release
build is a minute every time and is the same program.

`scripts/dev` is the script; `pnpm dev` is how you run it.

### Or by hand

Worth knowing, because the script is only the two halves in the right order.

```sh
cargo install --path crates/armada --debug   # armada, on your PATH
armada serve .                               # runs until Ctrl-C
pnpm --filter @armada/desktop dev            # in another terminal
```

**Fleet first, always.** Fleet binds a port and publishes a runtime file; Bridge
reads that file to find out where to connect. Bridge started first has nothing
to read.

**Fleet and Bridge ship as a pair and version together.** One number in
`protocol-version.toml` governs both, and a peer that sees a version it does not
know refuses the connection rather than guessing. So they are rebuilt together
or not at all.

What a healthy start prints: the repository and its workflow, the pid, port and
protocol version, what reconciliation found, the turn interval, and how many
operations are being served. It then goes quiet — that is a Fleet with nothing
to do, not a wedge.

**It refuses before it takes anything.** A missing or malformed `armada.yml`, a
`.armada/workflows/` with no definition or more than one, a step naming a Check
the Manifest does not declare, or an agent CLI a Drone would not find are all
reported before a port is bound, one fault per line. Started against a Fleet
that is already running it exits **0** and names the pid — the state you asked
for already holds.


### Running a Check or a Command by hand

`armada.yml` declares both, and these run them exactly as a Job's gate would —
same parser, same runner, no shell, so a `run` string that pipes or redirects
does not work here either.

```sh
armada check test    # the Checks this repo declares: build, test
armada run fmt       # the Commands it declares: fmt, gate
```

The command's own exit code comes back out, so `armada check build` failing
means the build failed. A name in the wrong registry is refused with the verb
that would have worked, and a name in neither is refused by listing what is
declared. **Output is captured and printed when the command ends**, not
streamed — a long Check prints nothing while it runs.

### Clearing up

**Destructive.** `armada clean` removes this repository's Armada worktrees,
deletes the branch each one is on, and forgets that Manifest's Jobs. `--all`
additionally removes the machine's store, its write-ahead files, the runtime
file and the MCP configuration.

It derives every branch it deletes from a Job it is deleting, and never matches
a name pattern — **a worktree with no Job behind it is reported and left where
it is.** It prints what it removed item by item, including the commit each
deleted branch pointed at, which is the only thing that makes one recoverable.

Both forms refuse while Fleet is running and name the pid: the Jobs being
forgotten are the ones it is holding.

```sh
armada clean          # this repository's worktrees, branches and Jobs
armada clean --all    # and the machine's store beside them
```

## Roadmap

Work is tracked as GitHub issues, grouped two ways.

- **[Milestones](https://github.com/NickMele/armada/milestones)** — each one
  carries a claim that is either true or not. *M1 — Dogfood* is "Armada does a
  small real task in the Armada repo, and I merge the branch it wrote."
- **[Capabilities](https://github.com/NickMele/armada/labels/capability)** —
  what the system can do, written from the outside. A capability names the
  steps that make it real, so its progress is computed rather than reported.

## Contributing

Not yet — see [`CONTRIBUTING.md`](CONTRIBUTING.md). Issues and questions are
welcome; there is nothing to review.

Security: [`SECURITY.md`](SECURITY.md). Armada brokers credentials and runs
commands a repository declares, so the interesting constraints are listed
there.

## Prior art, and a deleted first attempt

There was a v1. It worked well enough to be used and badly enough to be
replaced, and it was deleted rather than refactored. What it taught is written
down in [`docs/v1-learnings/`](docs/v1-learnings/) — including the things that
went wrong, which is most of it.

Its history is still here, on the `v1-archive` branch and the `v1-final` tag.
A bare file path in a document means `git show v1-final:<path>`.

## License

[Apache 2.0](LICENSE).
