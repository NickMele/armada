# What v1 taught

v1 shipped 586 commits, 2,181 passing tests and a working screen, and the Jobs it
completed did not do what they claimed. It is gone from the working tree and
lives at tag `v1-final`. These notes are what was worth carrying out of it.

**These are reference, not scope.** Notion decides what Armada v2 builds.
Everything here answers a different question: *this is what happened, and either
this is something you can use, or here is a trap we already hit.* Read the note
that covers what you are about to build, before you build it.

Every claim cites `v1-final`. Read the source with `git show v1-final:<path>`.

| Note | Read it before you |
|---|---|
| [worktrees.md](worktrees.md) | Create, reuse, carry files into, or remove a worktree |
| [daemon-lifecycle.md](daemon-lifecycle.md) | Detach a process, write a pidfile, or make anything survive a restart |
| [port-allocation.md](port-allocation.md) | Hand out a resource two concurrent things could claim at once |
| [permissions.md](permissions.md) | Look for v1's permission tuning — it is a dead end, and this says why |
| [writing-exemplars.md](writing-exemplars.md) | Write a commit message, a PR body, or a prompt that asks a model to |

## Two of the four subjects came back empty, and that is a finding

The step that commissioned this expected v1 to hold two things it does not.

**v1 never had a `permissions` block.** Its `.claude/settings.json` is `{}`, with
two commits behind it. The tuning this harvest was sent to collect does not exist.

**There is no pre-AI era in v1's published history.** It opens at
`46d8d70 charkit: initial public commit` on 2026-08-11 and runs twelve days; 438
of its 586 commits carry a `Co-Authored-By: Claude` trailer, and the earliest
without one are tool-generated. There are 7 merged PRs, not the ~10 expected.

**Both have the same cause:** the history was squashed before publication, so
everything written before that point is unrecoverable. Anyone else who goes
looking for v1's early record should stop here rather than repeat the search.

What the writing note found instead is better than what was asked for: a seam on
2026-08-16 where the commit trailer names a different model, and the voice drops
from an argued case to changelog bullets — on the same diff. It is model-written
prose curated for voice, not human-era writing, and it says so.
