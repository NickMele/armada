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
| `armada clean [--all] [--force]` | Gives this repository's worktrees, branches and Jobs back, keeping any branch whose work is not merged |

[Running it locally](#running-it-locally) is how to start.

This repository is the second attempt, started from scratch in August 2026.

**A Job runs end to end.** Approve one and Armada cuts a worktree on its own
branch, spawns a confined agent into it, runs the repository's own Checks
against what comes back, advances the step only if they pass, commits, rebases
onto the branch it merges into, pushes, and opens a pull request whose body is
assembled from what it checked — not from anything the agent said about itself.
Several have; one of the documents in this repository was written that way.

**And it argues back.** A Judge reads a step's work against the criteria and can
refuse a step whose Checks all passed. Evidence that satisfies a Check by gutting
it — a test deleted, a skip added, the config a frozen command resolves through —
is caught, three of those patterns without a model call at all. A step that
edits outside what it declared is caught the same way. An agent that loops
without converging is told to report, and stopped if it does not.

Any of those stops the Job at the step it happened on, so you redirect that step
or restart it rather than starting over.

**The acceptance test passes.** It was written before the code it tests and
could not compile for the whole of the first milestone — deliberately, so the
compiler's error list was the list of what the remaining crates had to provide.
It describes a Job that runs end to end, and it is how a milestone proves its
claim. `docs/practices/acceptance-tests.md` is the arrangement.

**What it does not prove is the interesting part.** It is hermetic — no process
spawned, no repository touched, no network opened — so it proves the machinery
and not the merge. The merge is proved by doing it: a real agent fixed a real
defect in this repository, and the commit is in the history.

Watch the [milestones](https://github.com/NickMele/armada/milestones) for what
is next.

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
  a mechanical check decides, and a Judge can refuse a step whose checks passed.
- **An agent cannot reach the network.** Every Job works in its own git worktree
  on its own branch, in an environment built from nothing that carries no
  credential — so it cannot push. Armada pushes on its behalf once the checks
  have passed, and opens a pull request for a person to merge.
- **An agent inherits no MCP server.** Without that, a spawned agent comes up
  holding every server the operator has connected — measured at seven servers,
  ninety-five tools, personal accounts, which is the defect that made the first
  attempt unusable.

**What that is not is a sandbox**, and the difference is written down rather
than glossed: an agent can still run a shell, because a tool allowlist is a
permission list and not a toolset — measured. What bounds it is the worktree and
the empty environment. Real confinement means containers, and that is a
different system. `docs/scope.md` carries the reasoning.

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
a token, a file over 900 lines, a Storybook story whose title and directory
disagree, a domain registry row that disagrees with the machine it describes,
and anything that names a person or a machine.

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

**Fleet and Bridge ship as a pair and version together.** A major and a minor
in `protocol-version.toml` govern both. A major mismatch refuses the connection
in either direction; a minor one is additive-only, so a Fleet *ahead* of Bridge
connects and shows a banner and a Fleet *behind* it refuses — Bridge would be
reading fields that Fleet is too old to send. So they are rebuilt together or
not at all, and `docs/practices/protocol.md` says which number moves when.

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

### What a finished Job leaves behind

A Job that passes every Check ends with its work committed on its own branch,
that branch brought up to date with the branch it merges into, pushed, and a
pull request open against it. Fleet does all four — a Drone is denied `git`, and
a change nobody can merge is not a finished Job.

**The branch it merges into is `base:` in `armada.yml`.** Leave the key out and
Armada infers one: what `origin/HEAD` names, then `main`, then `master`. A
declared branch the repository has not got is refused by name rather than
replaced with a guess.

**Fleet rebases at every step boundary, not only at the end.** A Job that runs
for an hour is a Job `main` moves under, and finding that out at the end is
finding it out too late. At a boundary the Drone has just submitted and nothing
is in flight, so git can answer on its own — no question reaches the Drone.
Three outcomes:

| What git says | What happens |
|---|---|
| Not behind | Nothing at all, and nothing is announced |
| Behind, and it replays | The Drone is told what moved in the turn it gets for the next step |
| Behind, and it conflicts | The conflict is handed to the Drone as work, with every file named |

**Uncommitted work is never destroyed by this.** Fleet commits only at the last
step, so mid-Job the worktree is full of changes nothing has committed — the
rebase carries them across and puts them back. Where they will not go back
cleanly the files are left with conflict markers and git keeps its own copy in a
stash; where the branch's *own* commits will not replay, the branch is put back
exactly where it was and nothing is pushed.

**A pull request's body is assembled from the record, never written by an
agent.** It carries the brief, what the Job had to satisfy, every step with its
verdict, every Check with its outcome and a link to what it printed, and a
closing section naming what nothing checked. What the agent claimed is not in
it: a claim is a signal the gate ruled on, and the record is what Fleet
verified.

**A repository with no remote is ordinary.** The work is committed, nothing is
pushed, no pull request is invented, and the Job still completes — the Checks
passed either way. The branch is the whole of the work and you merge it where it
is.

Opening the pull request needs `gh` on your `PATH` and signed in. Without it the
branch is pushed and the pull request is yours to open; the Job does not fail
over it.

### Clearing up

**Destructive.** `armada clean` removes this repository's Armada worktrees,
deletes the branch each one is on, and forgets that Manifest's Jobs. `--all`
additionally removes the machine's store, its write-ahead files, the runtime
file and the MCP configuration.

It derives every branch it deletes from a Job it is deleting, and never matches
a name pattern — **a worktree with no Job behind it is reported and left where
it is.** It prints what it removed item by item, including the commit each
deleted branch pointed at, which is the only thing that makes one recoverable.

**A row the store cannot rebuild is cleared too**, by the id it still carries.
A migration can leave a Job the current build no longer folds — Fleet reports
those on start as *unreadable* — and clearing one needs no rebuild, so `clean`
removes its worktree, its branch and its row like any other, says why the row
would not rebuild while the row is still there to say it, and keeps the branch
if it holds unmerged work. That is still deriving from a record and not from a
pattern: the id came out of this store, and a row belonging to another Manifest
is counted and left for the repository that owns it.

**It will not delete a branch whose commits nobody has taken.** Fleet commits a
finished Job's work, so that branch is the only copy of it. A branch the base
branch cannot reach is named, counted — *2 commit(s) of its own are not on
`main`* — and left standing, while its worktree still goes, because a checkout
can be made again and a commit cannot. The base branch is `base:` in
`armada.yml`, or — with no such key — the one `origin/HEAD` names, or `main`, or
`master`, whichever is found first; where nothing answers, nothing can say what
merged means, so every branch is kept and the line says so.

**What to do about a branch it left:** merge it, then `git branch -d
armada/<job-id>` — git refuses that itself while the branch is unmerged, so the
two checks agree. `armada clean --force` deletes them instead, and the commits
with them. `--force` and `--all` are separate questions and separate flags:
one is *delete work nobody has taken*, the other is *clear this machine's
store too*.

Both forms refuse while Fleet is running and name the pid: the Jobs being
forgotten are the ones it is holding.

```sh
armada clean            # worktrees, branches and Jobs; unmerged branches stay
armada clean --all      # and the machine's store beside them
armada clean --force    # and the unmerged branches, and their commits
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
