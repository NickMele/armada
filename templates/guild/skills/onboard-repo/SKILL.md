---
name: onboard-repo
description: Write a repository's armada.yml with the person who owns it — one question at a time, every guess attributed to what it came from, nothing written before they confirm, ending on a real `armada manifest config verify`. Use when a repository has no armada.yml, when `armada manifest init` reports there is nothing to claim, or when someone asks to onboard, configure or set up a repo for Armada.
---

# Onboard a repository

Write this repository's `armada.yml` **with** the person who owns it.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/onboard-repo/SKILL.md` changes it. A later release's
> version is **offered** rather than taken, because you may have customised this
> one: `armada guild upgrade --with-skills` merges it in, and without that flag
> nothing here moves.

## Why this is a skill and not a prompt

Without it, onboarding is as good as the sentence somebody happened to type that day, and the
step most likely to be dropped is the last one — a real `armada manifest config verify` before
declaring success. This exists so that **"configured" means verified rather than asserted**,
every time.

**The procedure is user-level even though the answers are repo-level.** How you want to be
onboarded — asked one thing at a time, shown where each guess came from, nothing written before
you confirm — is the same in every repository. *Which* script is the test command is not, and
that comes from the scan, every time.

## The four rules

| | |
|---|---|
| **Scan, never infer** | Every fact comes from `armada manifest config scan`. Do not write a stack-detection engine, and do not recall what a repo of this shape usually does. |
| **One question at a time** | A form is faster to render and slower to answer. Ask, wait, move on. |
| **Attribute every guess** | "`test: vitest run` — from `package.json` scripts" is a claim they can check. "`test: vitest run`" is one they have to trust. |
| **Nothing is written before it is confirmed** | Show the block, get a yes, then write. A file that appeared without being agreed is a file they will not read. |

## The loop

1. **Scan.** Run `armada manifest config scan`. It reports evidence — lockfiles, scripts, compose
   services, CI steps — and deliberately does not interpret it. If there is already an
   `armada.yml`, stop and say so; this skill writes one, it does not merge into one.

2. **Components.** Propose one component per thing that runs or is checked independently, each
   with the evidence that suggested it. Ask. A repository with one component is a normal answer.

3. **Ports.** For each component that listens, propose the port the scan found. Armada assigns
   the workspace's block; what you are agreeing here is which component wants which offset.

4. **Setup, up, down.** Take these from the scan verbatim — a `package.json` script, a compose
   file, a `Makefile` target. Never invent a command. If nothing was found, say the component
   has none rather than guessing one.

5. **Checks.** One `checks:` entry per thing that can fail on its own: lint, format, types,
   tests. Give each the `match:` globs that decide when it is in scope. Ask which of them are
   safe to run in parallel.

6. **`owns:`.** Anything the component creates outside its own directory — containers, volumes,
   databases, generated files. This is the part nobody remembers and the part that strands
   resources when it is missing. Ask directly: *"what does this leave behind?"*

7. **Write it.** Show the whole file. Get a yes. Then write it.

8. **Verify — and this step is not optional.** Run `armada manifest config verify`. If it fails,
   fix it with them and run it again. **Do not report success on an unverified file**: the whole
   reason this skill exists is that this is the step that gets dropped, and a repo that is
   "configured" but does not verify is worse than one that was never touched, because the next
   person believes it.

## What "done" looks like

`armada manifest config verify` exits `0`, and you have said in one line what the repository now
declares: how many components, how many checks, and what it owns.

## See also

`armada manifest config scan` · `armada manifest config verify` · `armada manifest init`
