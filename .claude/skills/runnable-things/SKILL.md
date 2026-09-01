---
name: runnable-things
description: A command, script or task somebody else may run is documented where they will look, in words they can act on. Load before adding one, renaming one, or changing what one does.
---

# Anything runnable is documented

**A command nobody can run is a command only its author has.** It works until
they are unavailable, and then it is a line in a file somebody is afraid to
execute.

This applies to a `cargo xtask` task, a `package.json` script, a standalone
`.mjs` or `.sh`, a hook, and a command a Manifest declares. It applies the same
whether a person types it or an agent does.

## Where it goes

| What it is | Documented in |
|---|---|
| A repository-wide task or check | `README.md` — the table under **The gate** |
| Something you run while working in one area | That area's practice doc under `docs/practices/` |
| Something one package owns | That package's `README.md` |
| A spike's harness | Beside the spike record, in the record |
| A hook | Its own header, and the practice doc for what it guards |

**One home each.** A command in two places drifts, and the copy is always the
one somebody reads.

## What documenting it means

Four things, and a line that gives fewer is a line that will be run wrong.

| | |
|---|---|
| **What it does** | In a sentence, from the reader's side. Not "runs the verifier" — *checks that the token outputs match the CSS they are generated from* |
| **When you run it** | Before a commit, after changing a registry, only in CI, never by hand |
| **What it needs** | A network, `gh`, a built workspace, a running daemon. **Say when it needs nothing** — that is worth as much |
| **What its output means** | Especially failure. `verify-foundations` goes red naming a subject that does not exist yet; a reader who does not know that reads a healthy repository as a broken one |

## Human readable is the requirement, not a preference

**Write for somebody who has never seen this repository.** They should be able
to run it and know whether what came back is good.

- **No bare flag lists.** `--strict-mcp-config --permission-mode dontAsk` says
  nothing about why either is there.
- **Name the failure, not the exit code.** "Fails when a document is not in the
  index" beats "exits non-zero".
- **Never state a count.** "The gate runs seventeen rules" is wrong the moment
  somebody adds one, and somebody will.
- **If it is destructive, say so first.** Before what it does, not after.

## The test

**Could a person who has never seen this repository run it, and know whether the
output is good?** If the answer needs you in the room, it is not documented.

## When you change one

**Renaming a command is renaming it everywhere.** A task, a script or a flag
that changes meaning is a documentation change in the same commit — not a
follow-up, because the follow-up is what does not happen.

**Deleting one deletes its documentation.** A documented command that does not
exist is worse than an undocumented one: somebody will try it and conclude the
repository is broken.

## What this repository looks like today

Written down so the next person does not have to measure it again.

**Documented:** every `cargo xtask` task, in the README's table.

**Not documented anywhere a reader would look:**

- Most `package.json` scripts. Only `pnpm install` reaches the README, while
  the gate, Storybook, the desktop build, its typecheck and its codegen do not.
- `apps/desktop/codegen/vocabulary.mjs`, which generates the vocabulary the
  renderer imports. It is named in no document.

**A flag that is not obvious and is not written down:** `pnpm storybook` prompts
when its port is taken, which makes it unusable from an agent. `--ci` is the
answer and nothing says so outside a skill.
