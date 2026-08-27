# Armada's concepts

One document per concept, carried out of the design workspace. Each
opens with a one-sentence **What it is** and then states the concept in full —
its shape, its rules, and what is still undecided about it, filed under that
document's own `## Open questions`.

**These are reference for what the product is, not a build order.** A
milestone or a capability decides what gets built; these documents answer a
different question: *this is what the thing means, and what governs it.*
Where two concepts touch, each restates only what it owns and links to the
other rather than repeating it — read the one that is missing rather than
assuming it is restated here.

| Concept | What it is |
|---|---|
| [bridge.md](bridge.md) | The Electron command-center shell — Armada's only engineer-facing surface |
| [convoy.md](convoy.md) | The atomic multi-workspace Job shape — one Job, one Drone, one PR |
| [doctor.md](doctor.md) | The health-check surface — a passive per-module grid and the first-run hard gate |
| [drone.md](drone.md) | The execution runtime for a single Job — a confined Claude Code process with its own worktree |
| [fleet.md](fleet.md) | The Rust daemon — the only actor that writes a state transition on a Job or a Drone |
| [helm.md](helm.md) | The conversational orchestrator agent that reasons across a Fleet |
| [job-board.md](job-board.md) | The surface for not-yet-started Jobs on one Manifest |
| [job-proposer.md](job-proposer.md) | The model call that reads a dispatch request and proposes a Job |
| [job.md](job.md) | The unit of work Fleet dispatches to a Drone — data, not an actor |
| [judge.md](judge.md) | The semantic, veto-only tier of evidence verification |
| [kit.md](kit.md) | The tool set you bring — Skills, MCP, sub agents, Commands, the allowlist |
| [log-envelope.md](log-envelope.md) | The field contract every log line carries across Fleet, Bridge and Drone |
| [machine.md](machine.md) | How this installation behaves — resources, timing, budget, notification routing |
| [manifest.md](manifest.md) | Per-project config, backed by `armada.yml` |
| [observe.md](observe.md) | Watching a Drone work while it keeps working — read-only, taking nothing over |
| [pilot.md](pilot.md) | The escape hatch from a running Job into a human-driven Claude Code session |
| [workflow.md](workflow.md) | The template a Job runs against — ordered steps, gates and retry policy |
