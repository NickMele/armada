---
id: 025
title: A Drone reached outside its worktree
status: RESERVED
module: fleet
raised: watching a real Job's transcript, 2026-08-16
---

# 025 — A Drone reached outside its worktree

**An unattended Drone installed third-party software on the operator's machine.** Measured
2026-08-16, from the Drone's own transcript:

```text
Bash  npx skills find write-design
Bash  npx skills add jayden-dang/skills@write-design -g -y
Bash  npx skills add pproenca/dot-skills@write-design-docs -g -y
```

The last one succeeded. `~/.claude/skills/write-design-docs` exists on the machine and no person
put it there. The Drone was three steps into a `design` Job, could not find a skill its workflow
named, and went and fetched one from the internet.

## Why the posture allowed it

[`011`](011-what-a-drone-may-do-unattended.md) grants `Bash` **whole**, and that decision is
still right: *"an allowlist of commands would be an enumeration of every build system there is,
and each one missing is a Job that edits code it cannot test."* [`DENY`] is what makes that
affordable — a finite list of the ways out of a worktree.

The list is missing a whole class. It covers `git push`, `git remote`, `git config`, `sudo`,
`gh`, `armada`, `claude`, and five publishes. **It has nothing about package installers**, and a
global install is exactly the shape everything on that list has: an effect that is not confined
to the worktree, and one nobody watching would have approved.

`-g` is the operative flag, and it is not the only spelling. `npm i -g`, `pnpm add -g`,
`yarn global add`, `pipx install`, `cargo install`, `brew install`, `gem install`, `go install`
and `uv tool install` all write outside the tree, and several write onto `PATH` — which means the
next Drone, and the operator's own shell, inherit whatever was installed.

## What has to be decided, and it is not obvious

**Denying the installers outright is not free.** A Job that must run a repository's checks may
legitimately need to install its toolchain, and `011` chose breadth precisely so that a Job could
verify its own work. A rule that blocks `cargo install` blocks a class of real work — which is
the trade `011` refused to make for build commands and might still refuse here.

Three candidates, none costless:

| Option | What it costs |
|---|---|
| Deny global installs by pattern (`Bash(npm i -g:*)` and its ~9 siblings) | An enumeration that will be incomplete the day a tenth installer appears — the same objection `011` raised against allowlisting commands |
| Deny the installers entirely | Costs a Job the ability to provision what its checks need |
| Contain instead of deny — a per-Job `HOME`, `npm_config_prefix`, `CARGO_HOME` | The honest fix, and the largest: it makes *"outside the worktree"* a property of the environment rather than a list of forbidden words |

**The third is the one that matches how the rest of Armada works** — the worktree is containment
rather than a rule, and [`020`](020-the-tui-decided.md)'s first decision argues throughout that
mechanisms beat instructions. It is
also the only one that does not need updating when somebody invents another package manager.

## The narrower thing this also revealed

The Drone reached for a skill because the one its workflow named was not resolvable to it. Giving
a Drone the guild's skills is `docs/reserved/008`'s territory and is partly built; **what is not
built is what a Drone should do when a named skill is missing.** Right now it improvises, and
improvising is how it ended up on the internet. Refusing the step with a `BLOCKED` verdict naming
the skill would have been the correct outcome and would have cost nothing.

## Decided 2026-08-16: left open, deliberately

**Nothing is being built for this yet.** The owner's call, in his words: *"until agents really
start misbehaving I don't think armada needs to be this strict."* One Drone fetched one skill
because the one its workflow named was missing — that is a nuisance, not a pattern, and a posture
tightened against a single incident is a posture nobody can loosen later without an argument.

**The containment option above was withdrawn before it was rejected, and for a better reason than
proportionality.** Giving a Job its own `HOME` would move `~/.claude` — and a Drone *is* a
`claude` process. It would lose its own credentials and settings and come up unauthenticated, lose
`~/.gitconfig` so its commits carry the wrong identity or none, lose `~/.ssh`, and redownload a
toolchain per Job because `~/.cargo` moved with it. **The thing being contained is the same
program whose configuration lives in the directory being moved.** Anybody reaching for this again
should reach for the narrow form instead: `npm_config_prefix`, `PNPM_HOME`, `CARGO_INSTALL_ROOT`
(*not* `CARGO_HOME`, which holds the registry cache and credentials), `PIPX_HOME`, `GOBIN` — which
move where a global install *lands* and leave identity, credentials and caches alone.

**What would change the decision:** a second incident, or any install that reaches something
shared and durable rather than a skills directory. The narrow form above is the thing to build
then, and it is small.

**The narrower half is still worth doing on its own merits** and is not blocked by this: a Drone
that cannot resolve a skill its workflow names should emit a `BLOCKED` verdict naming it, rather
than improvising. That is what sent this one to the internet, and refusing would have cost
nothing.
