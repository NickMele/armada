---
name: onboard-repo
description: Write a repository's armada.yml with the user, from evidence rather than inference. Invoked when `armada manifest config scan` hands over, or whenever a repo has no armada.yml.
---

You are onboarding a repository to Armada. The result is one file, `armada.yml`, and it must
describe **how this repo is actually operated** — not how a repo of this shape usually is.

**Armada has already told you the facts and decided nothing.** `armada manifest config scan`
reports what exists: lockfiles, every script verbatim, compose services, CI steps. It picks none
of them, because "these 14 scripts exist" cannot be wrong and "your test command is `pnpm test`"
can. Picking is your job, and confirming is the user's.

## The loop

1. **Run `armada manifest config scan`** if you were not handed its output.
2. **Read what the scan cannot.** `README`, `CONTRIBUTING`, `AGENTS.md`/`CLAUDE.md`, the CI
   workflow. CI is the strongest evidence in the repo — it is what someone already decided must
   run before merge.
3. **Present a table, and ask.** One row per task, with where you got it:

   ```
   TASK      I THINK YOU RUN     FROM
   test      pnpm test           CI, and README "Testing"
   types     pnpm typecheck      scripts — not in CI, is that deliberate?
   ```

   **Ask per task, never per file.** "Is this how you run tests?" is answerable. "Review this
   YAML" is not, and produces a nod rather than a correction.
4. **Ask about what the evidence cannot settle.** Which suite is slow enough to deserve
   `cost:`. Which two cannot share a browser (`exclusive:`). What genuinely needs a database up.
   A script that exists but nobody runs.
5. **Write `armada.yml`.** Then `armada manifest config verify`.
6. **Iterate on pass 1.** It is static and takes seconds — a wrong `argv[0]`, a `needs:` that
   does not resolve, a `match:` glob hitting nothing. Fix and re-run; this is cheap.
7. **Do not stop at green.** Ask what they do in this repo by hand that is not yet in the file.
   Deploys, seeds, one-off scripts — those are `commands:` entries, and they are the reason the
   next agent will not have to ask.
8. **Finish on a real `armada manifest config verify`**, pass 2 included. Only then say it is
   done. **"Configured" means verified, never asserted.**

## Rules

- **Write nothing before step 3 is answered.** A generated config the user then reviews is a
  config the user skims.
- **Never invent a script that was not in the evidence.** If a task has no command, say so and
  ask; a plausible-looking `pnpm test:unit` that does not exist fails on the first real run, in
  a fresh worktree, at the worst moment.
- **Prefer fewer checks that are real** over a complete-looking set that nobody runs.
- Record *why* in comments where a choice was not obvious — the next reader is often you.

## When this repo is unusual

If its shape is not covered by anything you have seen — an unfamiliar build system, a monorepo
that is really several products — say so plainly rather than forcing it into a familiar shape.
That is also the signal that Armada may need a new fixture for that repository shape
(`ARCHITECTURE.md` §2.4 — add a fixture before adding a feature).
