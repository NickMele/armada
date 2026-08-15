---
id: 006
title: The guild has no way to learn
status: BUILT
module: guild
raised: review pass — noticed writing the guild template
---

# 006 — The guild has no way to learn

> **BUILT** — `armada guild upgrade`
> ([`commands/guild/upgrade.md`](../commands/guild/upgrade.md)). All three parts
> of the shape below shipped as written: `init` stamps, the templates ship as a
> branch inside the guild's own repository, and the upgrade is a `git merge`. No
> merge engine was written. **Two things below turned out to be wrong or
> incomplete** — see *What shipped, and where this document was wrong*.

**The problem.** `templates/guild/subagents/helm.md` says *"it is yours from that moment, and
nothing here is updated by a later Armada release."* That sentence is correct for `voice.md`
and **wrong for the persona**: operating knowledge Armada learns — how to delegate, what to
verify — is exactly what should reach an existing guild, and today nothing can.

**The blocker is more basic than merging: a guild records no provenance.** Nothing anywhere
says which Armada template version any file came from. Without a base there is no three-way
merge — only overwrite or keep, and both are wrong.

**The shape of the fix, in three parts:**

1. **Stamp the template version at `init`**, so a later upgrade has a base to merge from.
2. **Ship Armada's templates as an upstream branch inside the guild's own git repo.** The guild
   is already a git worktree. The user's edits live on `main`, releases advance `armada`, and
   `armada guild upgrade` is a `git merge` — git tracks the base, merges untouched files
   silently, and conflicts only where both sides changed the same lines. **No merge engine gets
   written**; the repository already contains one.
3. **A per-file update policy**, because not everything should update:

| File | Takes updates? |
|---|---|
| `workflows/workflow.schema.json` | Always — it is Armada's |
| `subagents/helm.md` | Yes — operating knowledge, not personal |
| `skills/onboard-repo/` | Offer — it may have been customised |
| `voice.md`, `how-i-work.md`, `expectations.md` | **Never** — these are the user |

**Cheap now, expensive later.** The guild had no remotes when this was found; once it is syncing
between machines, adding provenance means reconciling histories rather than writing one field.

## What shipped, and where this document was wrong

**1. "The guild had no remotes" had already expired.** It was checked rather than assumed before
building, and by then the guild had one, plus a `main` on it. That changed one thing: the branch
Armada's templates live on is published to the same remote, so a second machine merges against
the *same* base instead of adopting one of its own from history. Both paths work; only one of
them keeps two machines at different Armada versions out of a conflict.

**2. The table above is not the whole of what must never update.** It names the three fragments,
and it is right about them — but the four starter workflows and `permissions.yml` carry the
interview's answers, your iteration ceilings and your Drone posture. Carrying Armada's numbers
back over yours is the same failure as overwriting `voice.md`, one level less obvious. They are
**never** updated, for that reason and not by oversight.

**The gap that leaves, stated rather than closed.** `workflows/workflow.schema.json` always
updates, and the four starter workflows never do — so a release that adds a predicate to the
schema ships one no starter workflow uses, and an existing guild's `bug.yml` never learns the
step that would use it. That is a real hole and it is not this verb's to close by guessing: the
question is whether a workflow you have edited should take a structural change, and the honest
answer needs a shape nobody has designed. **Raised here rather than papered over.**

**What the stamp is.** `~/.armada/guild/templates.yml`, holding the Armada version and a
content digest of the managed set. The digest is what identifies a template set — the package
version carries no compatibility signal (`AGENTS.md`) — and the version is there for the one
decision content cannot make: refusing to upgrade a guild *backwards*.
