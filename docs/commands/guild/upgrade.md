# `armada guild upgrade`

Take what Armada has learned since your guild was made.

> **Status: built.** The verb [`docs/reserved/006`](../../reserved/006-guild-has-no-way-to-learn.md)
> asks for.

## Synopsis

```sh
armada guild upgrade [--with-skills] [--json]
```

## Arguments

| Flag | What it does |
|---|---|
| `--with-skills` | also take `skills/onboard-repo/`, which is otherwise offered rather than taken |

At a terminal the offered file is a question. Without one it is this flag, because an
interactive-only verb is a bug ([`PLAN.md`](../../PLAN.md) §3.1.1) and a silent default that
overwrote a skill you had customised is the failure the whole verb is arranged around.

## What updates, and what never does

| File | Takes updates? | Why |
|---|---|---|
| `templates.yml` | always | it is the record of what an upgrade did |
| `workflows/workflow.schema.json` | always | it is Armada's — every workflow is checked against it |
| `subagents/helm.md` | yes | operating knowledge, not personal |
| `skills/onboard-repo/SKILL.md` | offered | it may have been customised |
| `voice.md`, `how-i-work.md`, `expectations.md` | **never** | these are you |
| the four starter workflows, `permissions.yml` | **never** | the interview wrote your ceilings and your posture into them |

**"Never" is enforced by absence, not by a check.** Those files are not on the branch Armada's
templates live on, so the merge has nothing to say about them. A rule that depends on a
conditional being right every time is a rule that is wrong once; this one cannot fire.

## How it works

Your guild is already a git repository, so **no merge engine is written**. Armada's templates
ship as a branch — `armada` — inside your guild's own repository. Your edits stay on `main`, each
release appends one commit to `armada`, and this verb is a `git merge`: git tracks the base,
merges an untouched file silently, and conflicts only where both sides changed the same lines.

1. **Refuses to go backwards.** A guild stamped by a newer Armada than the one running is not
   downgraded — that would take older template text over newer and conflict on every line the two
   releases both touched.
2. **Commits whatever is uncommitted**, exactly as [`push.md`](push.md) does, because git will
   not merge over a dirty tree and an edit you made outside `armada guild edit` is not this
   verb's to discard.
3. **Finds the base.** The remote's `armada` branch if there is one, then this machine's, and
   otherwise it **adopts** one — see below.
4. **Builds one commit on `armada`** holding the managed files at their current text, in a
   private index. Nothing is checked out and no file in your working tree is touched.
5. **Merges it.** Then publishes `armada` if there is a remote, and re-projects onto Claude
   Code's load path ([`project.md`](project.md)) — but only on a clean merge, because projecting a
   file with conflict markers in it would put them in front of your next session.

### A guild made before provenance existed

Nothing in a guild said which template set its files came from, and without a base there is no
three-way merge — only overwrite or keep, and both are wrong. That was the blocker, not the
merging.

A guild with no `templates.yml` **adopts** a base: the newest commit whose subject is one
`armada guild init` writes (`guild: imported and interviewed`, `guild: re-initialised`), because
that is the last moment Armada wrote those files. Failing that, the oldest commit in the
history — and either way the commit is reported on the summary line, because a base nobody was
told about is a base nobody can dispute.

### Two machines

The `armada` branch is published to the same remote your guild syncs with, so the second machine
merges against the *same* base rather than adopting one of its own. It never touches `main`, so
an upgrade here cannot fight a `pull` from there.

| Both machines moved | What happens |
|---|---|
| you upgraded here, pulled there | the guild that arrives is already upgraded; `upgrade` there finds nothing to do |
| you upgraded here and edited there | `armada guild pull` reports the divergence and changes nothing, exactly as it always has |
| the other machine runs an older Armada | its upgrade is refused with `update armada, then retry` rather than taking older text over newer |
| the `armada` branch could not be published | a row says so; the next machine adopts its own base from history, which is the pre-provenance path and still correct |

## Output

```
  STATUS     ITEM                            DETAIL
  ADDED      templates.yml                   Armada's
  CHANGED    subagents/helm.md               operating knowledge
  UNCHANGED  workflows/workflow.schema.json  already what Armada ships
  UNCHANGED  skills/onboard-repo/SKILL.md    offered; --with-skills takes it
  UNCHANGED  voice.md                        yours — no release ever updates it
  UNCHANGED  expectations.md                 yours — no release ever updates it
  UNCHANGED  how-i-work.md                   yours — no release ever updates it

READY  ~/.armada/guild, no stamp -> 0.1.0 8f3a1c0d9e21, base adopted from c401123abcde
```

**The three rows that say nothing happened are the point.** A person watching a verb rewrite
their guild deserves to be told in words that their voice was not touched, rather than to infer
it from an absence.

A conflict is **reported and left**, with git's markers in the file, for you to resolve:

```
NEEDS ATTENTION  ~/.armada/guild, 1 file the merge could not resolve, resolve in ~/.armada/guild, then git commit there
```

Nothing else landed. Both texts are in the file, which is what lets you choose.

`armada guild ls` prints what the guild is stamped with, because a version nobody can see is one
nobody trusts.

## Dependencies

`git`, an initialised guild. Network only when the guild has a remote.

## Exit codes

`0` upgraded, or already at these templates · `1` `tool_failed` — the merge could not resolve, and **it is left for you** · `2` `bad_invocation` — no guild, or this guild was stamped by a newer Armada · `6` `environment` — no `git`.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`pull.md`](pull.md) · [`project.md`](project.md) · [`ls.md`](ls.md) ·
[`006`](../../reserved/006-guild-has-no-way-to-learn.md)
