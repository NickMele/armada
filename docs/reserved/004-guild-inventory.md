---
id: 004
title: Seeing what is in your guild
status: BUILT
module: guild
raised: real use — user request
---

# 004 — Seeing what is in your guild

> **Built.** `armada guild ls` and `armada guild show` now cover the inventory question this
> section reserved. What follows is the original reservation, kept for the reasoning behind the
> shape they took.

**The complaint.** *"Right now, I have the guild set up, but I don't really know what is in
it. Like, I wish there was a way to view easily through the guild command what the skills are
that are in the guild, what the Claude files are that are in the guild, basically anything, and
be able to view and edit it."*

**The diagnosis, and it is embarrassing in a useful way.** `armada guild` has seven verbs —
`init`, `project`, `pull`, `push`, `export`, `import`, and the unbuilt `edit` and `verify` —
and **not one of them shows you what you have.** Every verb moves the guild somewhere: onto
this machine, into a repo, into a bundle, to the remote. The guild is the one thing in Armada
that is supposed to *be* you, and it is the only thing with no way to look at it.

A real guild already holds workflows, skills, subagents, `voice.md`, `how-i-work.md`,
`expectations.md`, `settings.json` and hooks. `export` will write all of it to a bundle, which
is the current answer to "what do I have" and is not an answer.

**The shape of the fix.** A read verb that lists what the guild contains by kind, and can show
one item's content — the same `STATUS · NAME · DETAIL · TIME` table every other listing uses,
with `--json` for agents. `verify` (already reserved) is the *correctness* question; this is
the prior *inventory* question, and inventory is the one you need first when a guild has
drifted between machines.

**Design questions this leaves open:**

- **What a "kind" is.** Workflows and skills are structured and can be summarised. `voice.md`
  is prose and can only be shown. A listing that flattens both to a filename is `ls` with extra
  steps.
- **Whether viewing and editing are one verb or two.** `edit` is already reserved as
  open-validate-commit; a viewer that can also edit either absorbs it or duplicates it.
- **What it says about drift.** The guild is a git worktree synced between machines. Whether
  the inventory reports uncommitted or unpulled state is the difference between a listing and a
  status.

**Was the strongest candidate of the reserved items**: it was small, self-contained, and the
user hit it in ordinary use rather than in the abstract — which is why it shipped first, on
`feat/guild-browse`.
