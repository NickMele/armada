---
id: 011
title: What a Drone may do unattended
status: BUILT
module: fleet
raised: real use, 2026-08-15 — every Job stalled, and nothing had ever granted a Drone permission
---

# 011 — What a Drone may do unattended

> **Built.** `armada_core::fleet::drone::Posture`, `armada_guild::permissions`, and
> `~/.armada/guild/permissions.yml`. What follows is the design, recorded here because the two
> decisions in *The mode is the fix* and *Which side each thing is on* are the parts a later
> change is most likely to get wrong.

**The complaint this exists to fix.** Every Job spawned, recorded a worktree and a port block,
and then sat at `STALLED` until its wall-clock ceiling filed an inbox entry. The question asked
was narrower than the answer: *why can a Drone not run `git commit`?*

**Nothing ever granted it permission.** The Drone's argv carried nine flags. `--session-id`,
`--resume`, `--print`, `--output-format`, `--verbose`, `--model` and `--input-format` name the
session or describe the output. `--strict-mcp-config` and `--disable-slash-commands`
*withhold* capability. **Not one of them granted any.** So a Drone running headless under
`--print` reached its first state-mutating tool call, Claude Code asked a person for permission,
and there was no person to ask.

> **A missing capability does not fail. It waits.** A rejected argv dies in a second and leaves
> a usage error in the log. This burned the whole ceiling and then reported a timeout, which is
> a symptom of nothing — which is why it survived a day of Jobs and an argv suite that passed.

## The mode is the fix, and the lists are the posture

Three flags, and they are not equal partners.

| Flag | What it decides |
|---|---|
| `--permission-mode` | what happens to a tool call **the lists do not cover** |
| `--allowedTools` | what a Drone may use |
| `--disallowedTools` | what is refused however broadly the allow list grants |

**`dontAsk`, and this is the decision to protect.** The obvious reading of the bug is *"grant
edits"*, and the obvious flag for that is `acceptEdits` — which auto-approves `Edit` and `Write`
and then **prompts for the `cargo test` after them**. That is the identical stall, one flag
later, discovered one day later. `manual` prompts for everything. `bypassPermissions` and
`--dangerously-skip-permissions` hand an unattended model the caller's whole toolbelt, which is
precisely what `--strict-mcp-config` and `--disable-slash-commands` were added to prevent —
undoing that one argv over would be the change this repository has already decided against.
`auto` delegates the decision to a classifier, which means the posture is no longer written
anywhere a person can read.

**`dontAsk` denies what it does not cover and carries on.** That is what converts the failure
from a stall into a refusal — and a refusal is a thing a Job reports, a stall is a thing nobody
learns about until a ceiling elapses.

> **The lists say what a Drone may do. The mode says what happens when they are wrong.** Get the
> lists wrong and a Job is refused something and says so. Get the mode wrong and it goes quiet.

## Which side each thing is on, and why

**The worktree is the argument.** A Drone works in `~/.armada/workspaces/<repo>/<job>` on branch
`armada/<job>`. It cannot reach the user's checkout, and a bad attempt is one branch deleted.
That isolation is what makes granting write access reasonable at all — and it is also what
decides the list, because the only capabilities worth naming are the ones whose effect is *not*
confined to a directory.

**Allowed — `Read`, `Glob`, `Grep`, `Edit`, `Write`, `NotebookEdit`, `TodoWrite`, `Bash`.**

`Bash` is granted **whole**, and that is the deliberate one. The commands a repository's checks
are spelled with are unbounded — `cargo test`, `npm run check`, `make`, `./scripts/ci`,
`bin/rails test` — and an allowlist of them would be an enumeration of every build system there
is. Every one missed is a Job that edits code and cannot then verify it, which is worse than a
Job that does nothing: it produces a change nobody checked.

**Refused — sixteen rules, all of them `Bash(...)`**, because the other tools are already
confined to the session's own directory and there is nothing to subtract from them.

| Refused | Because |
|---|---|
| `git push`, `git remote` | publishes to the shared remote; repointing it makes every later push escape somewhere new |
| `git config` | `--global` writes the user's own git identity, which is in no worktree |
| `git worktree` | the other Jobs' worktrees are this one's siblings; removing one kills a Drone still working |
| `git checkout`, `git switch`, `git branch` | leaving `armada/<job>` lands this Job's commits on somebody else's branch |
| `sudo` | root is the definition of escaping a directory |
| `gh` | opens pull requests, pushes, and deletes repositories |
| `armada` | writes the user's **real** `~/.armada/` — other Jobs, other worktrees, the guild |
| `claude` | a Drone spawning its own sessions is spend no budget counts, in the real `~/.claude/` |
| `npm`/`pnpm`/`yarn`/`cargo publish`, `docker push` | publishing is irreversible and public |

**What is deliberately *not* refused, and the reason is the same one.** `rm`, `git reset --hard`
and `git commit --amend` all destroy work — all of it the Drone's own, in the Drone's own
worktree, on the Drone's own branch. Forbidding them would be protecting the Job from itself,
which is what the worktree already does, and it would cost a Drone the ability to clean up after
a bad attempt.

**`Bash(armada:*)` is the one worth reading twice.** [008](008-armada-injects-its-own-skills.md)
reserved giving Drones Armada's own tools deliberately, and said it should arrive as MCP rather
than as a shell command. Denying the CLI here is what kept that decision open instead of making
it by accident, badly, through a shell.

> **`008` is now built and the decision went the way this rule held it open for.** A Drone's
> Armada is `fleet.*` on its MCP belt, and the fourth tool — `fleet.propose` — is how it raises a
> change to `armada.yml` rather than making one. **This deny rule does not move**: the skill
> Armada injects tells an agent that a tool it has not been given is a tool it was not meant to
> use, which is a sentence that only means anything while the shell stays shut.

## The residual, stated rather than papered over

A deny rule is matched against each subcommand of a compound command, but `bash -c "git push"`
is one command whose text is an argument — the rule matches `bash`, not what is inside the
quotes. **The posture narrows the blast radius; it is not a sandbox.** The thing that actually
bounds a Drone is the worktree it is confined to, and the list is defence in depth on top of
that. This is the same shape of admission as the `--verbose` residual in
[`traps.md`](../traps.md): the check narrows the class, it does not close it.

## Where it lives: the guild, and not the three alternatives

**What you are willing to let an unattended agent do on your machine is a preference, and a
preference travels.** You decide it once and want it on every machine and in every repository —
the same argument that put the model-tier policy in the guild persona. So the file is
`~/.armada/guild/permissions.yml`, it syncs, and `armada guild ls` lists it.

| Not there | Because |
|---|---|
| **A compiled-in constant** | then it is Armada's decision rather than yours, and changing it means a release |
| **A workflow field** | budget ceilings differ per workflow and this does not — a bug Job and a feature Job spend differently and are allowed exactly the same things. Four copies would be four chances for three to disagree, answering a question nobody asked per workflow |
| **The persona** | `subagents/helm.md` is prose read by a model; this is argv read by Armada. A posture written as prose is a *description* of a posture — Helm would have to translate it into flags correctly on every spawn, and a model that got it wrong would produce exactly the silent failure this change exists to end |

The persona is told the file exists so that Helm can point at it when a Job reports being
refused something. The file is what decides.

**Absent is the default; broken is a refusal.** A guild with no `permissions.yml` gets the
shipped posture, because a user who has never thought about this must still get working Jobs. A
guild with one that does not parse — or names a mode Claude Code does not have — **refuses the
spawn** rather than falling back. Falling back would run a Drone under a posture the user did
not write, immediately after they narrowed one on purpose, and the Job would look like it had
worked.

**The guild's lists replace the shipped ones rather than extending them.** A list you can only
add to is a posture whose real contents are written down nowhere — you would have to read the
file *and* Armada's source to know what a Drone may do. `guild init` writes the whole default
out with its reasoning in the comments, so the file always answers the question by itself.

## Why the tests are where they are

**Asserting on argv proves you built the string you intended, not that it works.** That is
[`traps.md`](../traps.md)'s rule, and it was earned by the `--verbose` bug, which shipped with
every argv assertion passing because no Drone had ever run.

So the coverage is in three places and each answers a different question:

| Where | Answers |
|---|---|
| `fleet::drone` unit tests | is the vector the one Armada meant to build, for every shape of posture |
| `crates/helm/tests/fleet.rs` | did `execve` actually receive it, and does a guild's own file reach it unchanged |
| `armada doctor` | does the installed `claude` still accept all twelve flags — free, and no token |

`doctor` is the one that catches the next version of Claude Code rather than this one. It reads
`claude --help`, so it costs nothing, and the three new flags went into `drone::FLAGS` where it
already looks. Measured on the machine this was built on: **`drone argv OK — 12 flags
accepted`**.

## An existing guild does not pick this up

`guild init` writes `permissions.yml`; **an existing guild has no `permissions.yml` and will
never grow one from a template change.** It gets the compiled-in default instead, which is the
same posture — so Jobs work — but the file is not there to read or edit until it is created by
hand.

That gap is [006](006-guild-has-no-way-to-learn.md), it is designed there, and it is not this
item's to solve. It is worth noting that this is the second concrete thing waiting on it, and
the first one an existing guild would actively want.
