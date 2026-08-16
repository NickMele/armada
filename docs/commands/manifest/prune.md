# `armada manifest prune`

Reclaim docker disk, including disk that is not Armada's.

> **Status: shipped.**

**The property no other Armada verb has:** every other reclaiming verb acts only on resources it
created and can prove it owns. This one can reach further — and that is the whole reason it is a
separate verb rather than a flag on [`clean.md`](clean.md). A flag that could delete somebody's
database is a flag that eventually does.

**Volumes only, and deliberately.** A named volume outlives `down` and outlives its container by
design, which is what makes it the leak. Images and build cache are `docker image prune` and
`docker builder prune` — they already do the right thing, and Armada has no ownership story for
them anyway: a pulled `postgres:16` is shared with everything else on the machine and was never
Armada's to offer.

## Synopsis

```sh
armada manifest prune [--json]
armada manifest prune --yes
armada manifest prune --dry-run
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--dry-run` | flag | off | List what could go. Removes nothing. |
| `--yes` | flag | off | Skip the list and remove Armada's own **idle** volumes only. |

There is no flag that removes an unlabelled volume. That is rule 3 below, and it is a property of
the verb rather than an omission from this table.

## The three rules, and none of them is a default

| # | Rule | The failure it prevents |
|---|---|---|
| 1 | **A preview is mandatory.** No invocation goes straight to removing. Rows toggle, enter confirms, esc touches nothing. | A one-line command that empties a machine before anybody has read what is on it. |
| 2 | **Armada's own *idle* volumes open ticked. Nothing else does** — not unlabelled volumes, and not Armada's own volumes a live workspace is still using. | Enter on an unread screen being the destructive keypress; and a background tick stopping a colleague's stack mid-run. |
| 3 | **An unlabelled volume is removed only on a per-run confirmation from a person at a terminal.** No flag can authorise it, so `--json` and a pipe never remove one. | A flag lives in a shell script or an agent's argv for ever; consent to delete unrecoverable data has to be given on the run that does it. |

Rule 1 reuses `armada fleet reap`'s selector rather than growing a second one. Rule 2 lives in
one function (`armada_core::disk::default_ticks`) so it is one rule in one place rather than a
habit two call sites share; rule 3 lives in `armada_core::disk::permitted`, which is the single
gate every removal passes through — a ticked row that is not permitted is still not removed.

**Owning a thing is not being finished with it.** A worktree somebody is working in right now has
volumes that are labelled, reclaimable in principle, and in use in fact. Liveness is decided the
way the reaper decides it: the workspace path is `stat`-ed and only `ENOENT` counts as gone.
`EACCES`, a hung mount, anything else — *in use*, because the cost of guessing wrong here is
somebody's running stack.

Rule 3 makes the verb less useful in automation. That is the correct trade: what is traded away
is an agent's ability to delete data it did not create and cannot identify.

## What each audience can do

| | List everything | Remove Armada's own idle | Remove unlabelled |
|---|---|---|---|
| At a terminal | yes | yes — tick and confirm, or `--yes` | **yes, and only here** — ticked by a person, on that run |
| Piped / no terminal | yes | yes, on `--yes` | never |
| `--json` | yes | yes, on `--yes` | never |

**Listing is the half that is always safe, and it is most of the value.** The reader asked what
is using the disk; on the measured machine the honest answer is *"almost none of it is mine"*.
Refusing to say so because Armada may not delete it would answer a question nobody asked.

## How it works

1. **The daemon is checked first.** `docker system df` is not client-side, so a dead daemon is an
   `environment` failure asked up front rather than discovered after the whole survey.
2. **Sizes** come from `docker system df -v`, matched by name.
3. **Ownership** comes from `docker volume ls --filter label=…` — the daemon's own answer. Both
   label namespaces are asked, one call each, so a volume stamped before the rename is still
   attributable for one release.
4. **Ordering**: Armada's own first, then everything else, biggest first within each group. The
   rows a reader has to judge one at a time are the ones worth putting at the top.
5. **Two gates on every removal**: the tick is the person's intent, and `permitted` is the rule
   that intent cannot override from the wrong place.

**Ownership is never read out of `df -v`'s `Labels` field.** It arrives as a single comma-joined
`k=v` string, and a label value may legally contain the delimiter ([`traps.md`](../../traps.md)) —
so a volume would eventually be attributed to the wrong workspace, which is the exact failure the
label exists to prevent.

An enumeration that fails is carried in `skipped` rather than raised: a size Armada could not read
is a row it can still offer, and a list that is empty because nothing could be listed must not be
indistinguishable from a tidy machine.

## Output

```
  STATUS   VOLUME                  DETAIL
  CLEAN    armada-a3f91c02_pgdata  79.0 MB
  SKIPPED  armada-b7c14e90_pgdata  48.2 MB — armada's, in use
  SKIPPED  someone-elses_pgdata    12.0 GB — not armada's

PARTIAL  79.0 MB freed, 2 of 3 armada's
```

A preview is the same table with every row `SKIPPED`:

```
  STATUS   VOLUME                  DETAIL
  SKIPPED  armada-a3f91c02_pgdata  79.0 MB — would go
  SKIPPED  armada-b7c14e90_pgdata  48.2 MB — armada's, in use
  SKIPPED  someone-elses_pgdata    12.0 GB — not armada's

  NOTE  no terminal to ask at; `--yes` removes armada's own
  NOTE  1 of these is not armada's; only a person can remove it

SKIPPED  nothing was removed, 2 of 3 armada's
```

**There is no `OFFERED` status.** A preview is a run in which every row is `SKIPPED` and the
detail says `would go` — so `--dry-run` needs no separate code path and no separate word, and a
reader who knows the status vocabulary already knows this one ([`render.md`](../render.md)).

**The rule is stated once, where the column cannot cut it.** Row details are short — `would go`,
`not armada's`, `armada's, in use` — because the DETAIL column truncates and the owner is the half
worth keeping. The full sentence goes in `withheld`, once, so a screen of untouched rows reads as
a rule rather than an oversight.

`--json` returns one result per candidate with `status`, `reference`, `kind`, `owner`, `bytes` and
`detail`, plus `freed`, `withheld` and `skipped`. **A size Armada could not read makes `freed`
`null` rather than a smaller number** — a total quietly missing a contributor reads as a complete
answer.

### Statuses

| Status | Means |
|---|---|
| `CLEAN` | The volume is gone. |
| `SKIPPED` | The volume is still there — not ticked, not permitted, or this was a preview. |
| `FAILED` | Docker refused to remove it. Carried on the row; the rest of the run continues. |

**`SKIPPED` is the ordinary outcome and is not a failure.** On the measured machine most rows are
somebody else's, and a verb that treated that as an error would be reporting the machine's normal
state as a fault every time it ran.

## Dependencies

| On | Why |
|---|---|
| A running docker daemon | `docker system df` has no client-side answer. Absent or wedged is `environment`. |
| `~/.armada/manifest.db` | **Not required.** Ownership is read from labels on the daemon, not from the store. |
| `armada.yml` | **Not required.** Prune is about the machine, not about a workspace. |

## Exit codes

**The code follows `error.class`, never the terminal state** — the one rule, and this verb is no
exception to it.

| | |
|---|---|
| `0` | the survey ran and nothing failed. **A run that removed nothing exits `0`**: a refusal rule 3 made is the verb working, and `SKIPPED` is the ordinary outcome on a machine whose volumes are mostly not Armada's |
| `1` | `tool_failed` — docker refused to remove a volume this run had confirmed. That is a leak Armada is about to stop looking at, so it reaches the exit code and a script can gate on it |
| `2` | `bad_invocation` — a flag this verb does not take |
| `6` | `environment` — the daemon will not answer |

The state and the code are different axes: a run that removed three of five volumes and failed on
one is `PARTIAL` and exits `1`, because `PARTIAL` and `FAILED` demand different actions from a
reader while both mean the same thing to a gate.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## Why this is a separate verb rather than a flag on `clean`

**`clean` is defined by what it can prove.** It releases what a workspace owns, found by label,
and it works after the directory is gone because ownership is recorded machine-globally. Every
resource it touches is one Armada stamped. That property is what makes `clean --all` safe to
suggest, safe to script and safe for an agent to run.

A `--everything` flag on `clean` would put a reach past that boundary one word away from a verb
people already run without reading. Two consequences follow, and each is enough on its own:

- **The confirmation model is different.** `clean` needs no per-run consent, because it cannot
  destroy anything it did not create. `prune` needs consent from a person at a terminal for
  exactly the rows `clean` would never see. Two consent models under one verb name is one verb
  whose safety depends on which flags were passed.
- **The audiences are different.** An agent may run `clean --all` unattended and should. The only
  thing an agent may do with `prune` is *look* — and that difference should be visible in the
  command that was typed, not buried in a flag table.

## What motivated it

A macOS storage warning, on a machine holding **171 local volumes and 12.0 GB, 100% reclaimable —
and not one of them carrying an Armada label.**

So the useful verb is one that can offer to remove things Armada did not make. And the dangerous
verb is the same verb: **"reclaimable" is docker's word for "no container is currently using it"**,
which is also true of a database volume between runs. That is exactly why the verb cannot be
trusted to decide alone — the number that makes prune worth building is the same number that makes
its every default a refusal.

`armada doctor` reports the same measurement as two rows that are never summed — the machine's
reclaimable and Armada's own share — because the remedies differ
([`doctor.md`](../doctor.md)).

## See also

[`clean.md`](clean.md) · [`down.md`](down.md) · [`status.md`](status.md) ·
[`doctor.md`](../doctor.md) · [`022-docker-hygiene.md`](../../reserved/022-docker-hygiene.md)
