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

**Armada does not work yet. Nothing here runs.**

This repository is the second attempt, started from scratch in August 2026. It
currently contains a build gate, a design-token pipeline, a failing acceptance
test, and twelve mostly-empty crates that fix a dependency shape before there is
anything to put inside them.

The acceptance test **fails on purpose**, and a hook enforces that it keeps
failing. It describes a Job that runs end to end, and it is the definition of
done for the first milestone — a test that passes early would mean the
definition had been quietly narrowed.

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
├── packages/        shared: design tokens, and what more than one surface reads
├── xtask/           the build gate
└── docs/            practices, spikes, and what v1 taught
```

## Building it

You will get a gate that reports red and a test that fails. That is the
expected output today.

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

Some of what the gate refuses: a source file missing from the manifest,
untyped JSON outside the two crates allowed to parse it, a vendor's name
outside the adapter layer, a design value that is not a token, a file over 900
lines, and anything that names a person or a machine.

### Tests

```sh
cargo nextest run --workspace     # not `cargo test`
cargo test -p acceptance          # fails on purpose. Do not fix it
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

Not yet. Armada is one person's project and the foundations are still moving
underneath it — an outside change today would be built on something that
changes next week.

Issues and questions are welcome. If something here is interesting, say so and
that is useful; there is nothing to review yet.

## Prior art, and a deleted first attempt

There was a v1. It worked well enough to be used and badly enough to be
replaced, and it was deleted rather than refactored. What it taught is written
down in [`docs/v1-learnings/`](docs/v1-learnings/) — including the things that
went wrong, which is most of it.

Its history is still here, on the `v1-archive` branch and the `v1-final` tag.
A bare file path in a document means `git show v1-final:<path>`.

## License

[Apache 2.0](LICENSE).
