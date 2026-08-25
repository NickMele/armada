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
  security posture, and the v1 failure the desktop app exists to escape.
- [`practices/protocol.md`](practices/protocol.md) — the one seam between Rust
  and TypeScript: version skew, DTOs, and what survives when the two disagree.

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
