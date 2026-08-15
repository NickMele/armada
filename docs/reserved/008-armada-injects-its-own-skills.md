---
id: 008
title: Armada injects its own skills
status: RESERVED
module: cross-cutting
raised: real use — user request
---

# 008 — Armada injects its own skills

**The ask, and it generalises past where it started.** *"Armada should inject custom skills into
Helm and the subagents that are dispatched so that they can properly use Armada if needed,
including the manifest, and propose changes to the manifest or propose changes to the guild."*

**Why this is the missing half of the three-layer sandwich (PLAN.md §5).** Armada reports facts,
an agent authors, Armada verifies — and the middle layer is currently expected to know how to
hold the tools. A Drone that changes what a repository runs has learned something the
`armada.yml` does not say, and it has no instruction telling it that saying so is part of the
job.

The natural home is the projection Guild already performs (PLAN.md §4.8, and
[`PHASES.md`](../PHASES.md) §8.4): the same mechanism that puts a guild's skills where Claude
Code reads them can put Armada's own there too. The open questions are which skills exist,
whether a Drone may propose a guild change or only a manifest one, and how a proposal is
carried back — an inbox entry with an id is the obvious answer, which puts this downstream of
`005-inbox-label-not-identity.md` and `001-raised-items-need-identity.md`.
