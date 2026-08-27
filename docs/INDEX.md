# What is written down here

Every document under `docs/` is named below, and the gate refuses a document
that is not. That is the whole job of this file: not to describe anything, but
to make a document nobody indexed impossible to leave lying around.

A directory that carries its own `INDEX.md` is listed as a directory — its
contents are indexed there, by the same rule.

## Before you write here

- [`CLAUDE.md`](CLAUDE.md) — the rules for writing in this directory, and where
  the lexicon lives.

## Practices

Read the one that covers what you are about to write, before you write it.
Each ends with the questions it found and did not answer.

- [`practices/rust.md`](practices/rust.md) — crate boundaries, the
  type-system-first pattern, and why each rule exists rather than what it is.
- [`practices/bridge.md`](practices/bridge.md) — the three-process split, the
- [`practices/react.md`](practices/react.md) — how a component is written, and which React advice does not apply to a window that loads from disk
  security posture, and the v1 failure the desktop app exists to escape.
- [`practices/protocol.md`](practices/protocol.md) — the one seam between Rust
  and TypeScript: version skew, DTOs, and what survives when the two disagree.
- [`practices/acceptance-tests.md`](practices/acceptance-tests.md) — the one
  test per milestone that stands for its claim: why it is written first, what
  reconciling one costs, and what M1's proves.

## Contracts

Binding rules, carried out of the design workspace. Read the one covering what
you are about to build, before you build it. **Contract** is a rule others may
not override, **spec** is a designed artifact, **reference** is lookup material.

- [`contracts/system-architecture.md`](contracts/system-architecture.md) —
  *reference.* Topology, trust boundaries and the component taxonomy. The map
  with the why attached; `ARCHITECTURE.md` is the map with the where. The
  operation inventory behind its protocol surface is
  `crates/ipc/operations.toml`.
- [`contracts/technical-writing.md`](contracts/technical-writing.md) —
  *contract.* The shape of every document here: one mode per page, how a rule
  is phrased, when prose becomes a table. Read before writing in `docs/`.
- [`contracts/design-system.md`](contracts/design-system.md) — *contract.*
  Static UI chrome, the tokens, and the Voice and Copy contract. The parent
  nothing else may contradict.
- [`contracts/error-contract.md`](contracts/error-contract.md) — *contract.*
  What a failure carries, how it crosses the Rust and TypeScript halves, and
  what a person sees when one reaches them.
- [`contracts/configuration.md`](contracts/configuration.md) — *contract.* How
  a setting is classified, how two layers combine, and what a Job records about
  the configuration it ran under. The settings are
  `crates/config/settings.toml`.
- [`contracts/agent-prompt.md`](contracts/agent-prompt.md) — *contract.* Every
  prompt Armada assembles and puts in front of a model.
- [`contracts/agent-copy.md`](contracts/agent-copy.md) — *contract.* Text
  written at runtime by Drones, the Judge and Helm. Its sibling, going the
  other way.
- [`contracts/workflow-design-system.md`](contracts/workflow-design-system.md)
  — *spec.* How a workflow is structured and how evidence moves through one.
- [`contracts/adapters.md`](contracts/adapters.md) — *spec.* The five adapter
  boundaries, what each may expose, and where adapter configuration lives.
- [`contracts/iconography.md`](contracts/iconography.md) — *spec.* Which glyph
  means what, and what each may never be reused for. The roster is
  `packages/icons/icons.toml`.
- [`contracts/voice-engineering.md`](contracts/voice-engineering.md) — *spec.*
  The engineering behind the Voice contract — the enum-to-verb map, the lint,
  the rename sweeps.
- [`contracts/testkit-fixtures.md`](contracts/testkit-fixtures.md) — *spec.*
  The fixtures reproducing each known Drone failure mode, read by whoever
  builds verification.

## Pasted elsewhere

Two surfaces are not this repository and neither reads a file from it. The
canonical text is here so the three sets say the same thing rather than three
approximations.

- [`instructions/README.md`](instructions/README.md) — what all three must
  agree on, and where each is pasted.
- [`instructions/claude-desktop.md`](instructions/claude-desktop.md) — the
  desktop app's project instructions.
- [`instructions/claude-design.md`](instructions/claude-design.md) — the design
  project's session contract.

## Concepts

- [`concepts/`](concepts/) — what each thing in Armada is: Job, Drone, Fleet,
  Bridge, Kit, Machine, Manifest, Workflow, Judge, Doctor and the rest, indexed
  in its own `INDEX.md`. The state machine and the schemas they describe are
  data in `crates/core-model/domain/`.

## Journeys

- [`journeys/`](journeys/) — how a person moves through Armada, one document
  per journey, indexed in its own `INDEX.md`. A numbered filename means the
  design project has drawn it; an unnumbered one has not been drawn yet.

## Measured

- [`spikes/003-does-headless-output-parse.md`](spikes/003-does-headless-output-parse.md)
  — whether a headless agent's output stream can be read reliably.
- [`spikes/004-can-fleet-inject-a-turn.md`](spikes/004-can-fleet-inject-a-turn.md)
  — whether a turn can be injected into a live session, and what it does to one
  mid-tool-call.
- [`spikes/005-what-does-a-job-cost.md`](spikes/005-what-does-a-job-cost.md) —
  what a real Job costs, and whether quota is readable. Partly a negative
  result, which is why it is written down.
- [`spikes/006-will-a-drone-use-the-evidence-tool.md`](spikes/006-will-a-drone-use-the-evidence-tool.md)
  — whether an agent uses a tool it was given without being told it must.
- [`spikes/007-will-a-drone-take-the-escape-hatch.md`](spikes/007-will-a-drone-take-the-escape-hatch.md)
  — whether a Drone hands a hopeless task back rather than guessing, and
  whether the word the option is offered through changes it.
- [`spikes/008-does-v1s-detach-leave-the-secrets-pipe-open.md`](spikes/008-does-v1s-detach-leave-the-secrets-pipe-open.md)
  — whether `setsid` leaves v1's piped-stdin secret handoff readable, and what
  a Drone spawn actually carried. Read rather than measured.

Raw transcripts sit beside each record. A negative result is a result and stays.

## Carried out of v1

- [`v1-learnings/`](v1-learnings/) — what the deleted first attempt taught,
  indexed in its own `INDEX.md`. Reference, not scope.
- [`v1-decommission.md`](v1-decommission.md) — what was removed from the machine
  when v1 was deleted, and how to tell it is gone.

## Generated

- [`OPEN.md`](OPEN.md) — every open question, collected from the document that
  blocks on it. Written by `cargo xtask verify-docs --write`. Editing it by hand
  fails the gate.

## Not documents — procedures an agent loads

Under `.claude/skills/`, not here, because they are read before doing a kind of
work rather than looked up. Listed so somebody browsing what is written down
can find them.

| Before | Skill |
|---|---|
| Writing any comment, module doc or header | `comments` |
| Adding a command, script or task | `runnable-things` |
| Working a milestone step end to end | `milestone-step` |
| Adding or editing a document | `armada-documents` |
| Filing, citing or answering an open question | `armada-open-questions` |
| Building or changing a component | `armada-components` |
| Writing a commit message | `commit-message` |
| Writing anything a person reads on a surface | `armada-voice` |
| Starting, checking or cleaning up a local Fleet | `armada-local` |
| Reporting something Armada got wrong | `armada-bug` |
