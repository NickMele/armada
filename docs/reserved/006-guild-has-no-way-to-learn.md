---
id: 006
title: The guild has no way to learn
status: RESERVED
module: guild
raised: review pass — noticed writing the guild template
---

# 006 — The guild has no way to learn

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
