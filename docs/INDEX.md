# What is written down here

Every document under `docs/` is named below, and the gate refuses a document
that is not. That is the whole job of this file: not to describe anything, but
to make a document nobody indexed impossible to leave lying around.

A directory that carries its own `INDEX.md` is listed as a directory — its
contents are indexed there, by the same rule.

## Before you propose anything

- [`scope.md`](scope.md) — what Armada is for and what it is not, in the
  owner's words. A proposal that serves neither the seven steps nor the four
  pains is a rabbit hole, and one has already cost a night.

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
- [`practices/capture-window.md`](practices/capture-window.md) — the security
  review behind the one window that loads another repository's running web app:
  what it may load, what the injected layer may touch, what the page can reach
  back into, and what is refused outright.
- [`practices/protocol.md`](practices/protocol.md) — the one seam between Rust
  and TypeScript: version skew, DTOs, and what survives when the two disagree.
- [`practices/writing-an-issue.md`](practices/writing-an-issue.md) — an issue has
  two readers who need opposite things. Where the consequence goes, where the
  detail goes, and why the answer was in third position
- [`practices/half-built.md`](practices/half-built.md) — the defect this
  codebase produces most: a thing that exists and nothing reaches. Read before
  calling a change finished
- [`practices/acceptance-tests.md`](practices/acceptance-tests.md) — the one
  test per milestone that stands for its claim: why it is written first, what
  reconciling one costs, and what M1's proves.
- [`practices/running-locally.md`](practices/running-locally.md) — starting,
  checking and stopping a local Fleet: what it prints, what it refuses before it
  binds a port, what `armada clean` will not delete, and Bridge on a mock Fleet
  with `pnpm mock`
- [`practices/code-graph.md`](practices/code-graph.md) — what to ask GitNexus
  before an edit and before a commit, how to build its index with
  `pnpm gitnexus:index`, and what it cannot see

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

One surface is not this repository and does not read a file from it. The
canonical text is here so the two sets say the same thing rather than two
approximations.

- [`instructions/README.md`](instructions/README.md) — what both must agree on,
  and where it is pasted.
- [`instructions/claude-desktop.md`](instructions/claude-desktop.md) — the
  desktop app's project instructions.

## Concepts

- [`concepts/`](concepts/) — what each thing in Armada is: Job, Drone, Fleet,
  Bridge, Kit, Machine, Manifest, Workflow, Judge, Doctor and the rest, indexed
  in its own `INDEX.md`. The state machine and the schemas they describe are
  data in `crates/core-model/domain/`.

## Capabilities

- [`capabilities/`](capabilities/) — one file per capability that has prose
  worth keeping, each bound by its frontmatter to the issue that tracks it,
  indexed in its own `INDEX.md`. The roadmap itself is GitHub issues; these hold
  the reasoning an issue body buries when it closes.
  - [`capabilities/merge-line.md`](capabilities/merge-line.md) — merges taking
    turns onto `main`, `scripts/land`, and where each part goes in Fleet.

## Journeys

- [`journeys/`](journeys/) — how a person moves through Armada, one document
  per journey, indexed in its own `INDEX.md`.

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
- [`spikes/009-how-long-does-a-step-take.md`](spikes/009-how-long-does-a-step-take.md)
  — what a step's wall clock and tool-call count actually are, against the Jobs
  this repository has run, and why elapsed time does not separate a stuck step
  from a slow one.
- [`spikes/010-can-a-drone-be-identified.md`](spikes/010-can-a-drone-be-identified.md)
  — which MCP transports the agent CLI accepts, and whether any of them carries
  an identity Fleet does not have to trust the Drone about. A negative on unix
  sockets and a positive on the peer process.
- [`spikes/011-what-can-one-drone-reach.md`](spikes/011-what-can-one-drone-reach.md)
  — if every Drone gets its own endpoint, what an adversarial Drone can reach of
  another's: what the allowlist stops, what it does not, and why no per-Drone
  secret placement survives a Drone with the Write grant.
- [`spikes/012-peer-identity-under-concurrency.md`](spikes/012-peer-identity-under-concurrency.md)
  — whether the peer-process identity of spike 10 holds at five Drones and under
  concurrent connections, how wide the lookup's window is and which way it fails
  when it closes, and what the same answer costs without `lsof`.
- [`spikes/013-what-does-reading-the-diff-cost.md`](spikes/013-what-does-reading-the-diff-cost.md)
  — what one gate's reading of a Job's worktree costs, and whether the size of
  the diff or the load on the machine is what decides it. The measurement `#431`
  waited on.
- [`spikes/014-how-much-room-does-a-brief-leave-for-comments.md`](spikes/014-how-much-room-does-a-brief-leave-for-comments.md)
  — how much of an opening brief a merge press's chosen comments could take
  before they are most of what the Drone reads, read cold against `assemble`
  in `crates/fleet/src/briefing.rs`, and the character bound on the whole set
  that follows from it. Not measured in this session; handed over.
- [`spikes/015-can-a-person-answer-a-blocked-command.md`](spikes/015-can-a-person-answer-a-blocked-command.md)
  — whether a Drone refused a command can wait on a person's answer through a
  permission tool, how long the agent CLI waits over stdio and over HTTP, and
  what a tool that fails does to the call.
- [`spikes/016-can-fleet-resume-a-headless-session.md`](spikes/016-can-fleet-resume-a-headless-session.md)
  — whether `--resume` recovers a headless session after it exits, whether it
  needs the original working directory, how an unknown id fails, and what
  `cleanupPeriodDays` governs.
- [`spikes/017-what-confines-a-scout-and-what-a-stop-leaves.md`](spikes/017-what-confines-a-scout-and-what-a-stop-leaves.md)
  — whether `--tools` and `--restricted` hold a read-only agent to one checkout,
  and what `SIGINT` and `SIGTERM` each leave of a run's cost.
- [`spikes/018-what-can-a-helm-session-do-in-each-permission-mode.md`](spikes/018-what-can-a-helm-session-do-in-each-permission-mode.md)
- [`spikes/019-is-auto-mode-reachable-for-a-helm-session.md`](spikes/019-is-auto-mode-reachable-for-a-helm-session.md)
  — what a session resolves once `--strict-mcp-config` comes off, what each
  permission mode lets it call, and what a refusal leaves in the stream.
- [`spikes/019-what-raises-the-resize-loop-notice-on-a-studio.md`](spikes/019-what-raises-the-resize-loop-notice-on-a-studio.md)
  — why a whiteboard card that resizes with its own words raises Chromium's
  `ResizeObserver` loop notice where Armada's own observers never do, what the
  card's box was measured at before and after, and what on the board was ruled
  out.

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
| Starting a Fleet of your own, to read or record a Job without the owner's | `dev-fleet` |
| Finding out what a Job did and why it stopped | `what-happened-to-a-job` |
| Reporting something Armada got wrong | `armada-bug` |
| Filing what the owner finds while he walks the app | `app-walkthrough` |
| Working through the notes the owner left on Bridge with the annotation layer | `annotations` |
| Working one issue end to end — worktree, plan, implement, test, commit, merge | `work-issue` |
| Cutting an agent worktree, and giving it back when its branch merges | `agent-worktrees` |
| Running a whole milestone by dispatching agents at its issues | `orchestrate-milestone` |
| Closing a session — what the work just made untrue, and what to update | `reflect` |
| Putting a decision to a person, and writing the options | `asking-a-person` |
| Running a whole milestone from one approval, wave by wave | `epic-as-one-job` |
| Moving the owner's Fleet and Bridge onto a merged fix | `restart-app` |
| Walking the owner through what this session left waiting on him, one question at a time | `review-open-items` |
| Asking the code graph how something works | `gitnexus-exploring` |
| Asking the code graph what a change breaks | `gitnexus-impact-analysis` |
| Tracing a bug through the code graph | `gitnexus-debugging` |
| Renaming, extracting or moving code through the code graph | `gitnexus-refactoring` |
| Looking up a GitNexus tool, resource or schema | `gitnexus-guide` |
| Building, checking or cleaning the GitNexus index | `gitnexus-cli` |
