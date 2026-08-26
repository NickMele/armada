---
name: commit-message
description: How an Armada commit message is written — say what the diff cannot. Load before writing any commit message or PR description.
---

# Commit messages

**Say what the diff cannot.**

The diff already shows what changed. The message carries what a reader would
otherwise have to reconstruct: why, what was rejected, and what is still wrong.

**The test.** Read the message with the diff open. If it tells you nothing the
diff did not, it is noise with a timestamp.

This is the exact inverse of a refusal record, which demands the observable. A
commit message demands what is *not* observable from the artifact — which is
why an agent writes them badly, since the diff is the part it is holding.

## Shape

| | |
|---|---|
| Subject | Imperative, lowercase after the type, no trailing period |
| Body | Prose. Why it changed, what it broke, what it did not fix |
| Length | **Tracks the change.** A one-line fix gets one line |

**A bulleted list is the wrong medium.** `git log` is narrow and read in a
terminal rather than scanned on a screen — and a list of changed functions is
the diff, retyped.

**Uniform medium-length messages are the tell.** A prompt produces a paragraph
whatever the diff, and real history is wildly uneven.

## The shape to avoid

```
fix: resolve worktree path collision issue

- Updated add() to accept job_id parameter
- Modified path construction logic in worktree.rs
- Updated three call sites in dispatch.rs
- Added test coverage for concurrent dispatch
```

Every line is in the diff. It contains no banned phrase, so no lint reaches it —
the tell is the shape, and the shape is what a blocklist cannot catch.

## The same change, written

```
Derive worktree path from job id

Two Jobs against the same repo resolved to the same path and
the second died inside libgit2, naming neither Job.

Cleanup still keys off the repo name, so a killed Job now
leaves a directory the sweeper does not match. Not fixed here.
```

The second paragraph is the whole value. **Carry what you changed that nobody
asked for, and what you left broken** — the next person to touch the sweeper
needs it, and nothing else will tell them.

## A small change

```
Fix typo in the dispatch error string
```

## Two rules that are easy to miss

**Never announce the machine.** A labelled block of fields in the body says an
agent wrote this, and the repository is the one place that should not be
obvious. Trailers a tool appends are not that; a summary block is.

**Never restate a decision.** Link it or name it. A commit message that argues
a settled point re-opens it in the one place nobody will look for the argument.

## Where the rest lives

`docs/contracts/agent-copy.md` governs every surface a Drone, the Judge or Helm
writes at runtime — PR bodies, Judge summaries, evidence summaries, Helm
replies. This skill covers the commit message only.
