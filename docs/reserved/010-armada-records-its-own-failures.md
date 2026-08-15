---
id: 010
title: Armada records its own failures
status: BUILT
module: cross-cutting
raised: real use, 2026-08-15
---

# 010 — Armada records its own failures

> **Built.** `armada failures`, `show`, `fix` and `clear`, with the log at
> `~/.armada/failures.jsonl`. What follows is the design, recorded here because the reasoning —
> especially the two decisions in *What gets recorded* and *Deduplication* — is the part that a
> later change is most likely to get wrong.

**The complaint this exists to fix.** `armada bridge` failed with `class: environment`,
blaming the binary's own path, and the real fault was Armada mishandling a missing worktree.
The report was on the screen for as long as the terminal scrollback kept it, and then it was
gone. There was nowhere for a failure of Armada's to go — so noticing one and doing something
about one were separated by however long it took to reproduce it from memory.

**What was asked for**, in the words it was asked in: *"If I am running something within the
Armada CLI and it fails … it would be crazy cool if that somehow filed an issue somewhere … It
can even just be local on my machine, specific to the repo. And then within Armada I can see
these bugs that have been reported by myself … then I could specifically go and try to trigger
one of those to become a Job that gets fixed."*

## The four parts

| Part | Verb | Where it lives |
|---|---|---|
| Recording | none — a side effect of failing | `crates/helm/src/main.rs`'s `record`, at the one place errors are rendered |
| Listing, and one entry whole | `armada failures`, `armada failures show <id>` | `crates/helm/src/verbs/failures.rs` |
| Navigating it, at a terminal | `armada failures` — the same verb | the same, over `crates/helm/src/ask/select.rs` |
| Promotion into a Job | `armada failures fix <id>` | the same, over `fleet spawn` |
| Discarding | `armada failures clear <id> \| --all` | the same |

The format and the fold are `armada_core::failure`; the file is `armada_manifest::failures`.
Parsing a string is pure and lives in the core, opening a file is not and does not
([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.5).

## What gets recorded: everything that reaches the report, minus a refusal Armada meant

**Every failure, and no filter on `class`.** Two arguments settled it and the second is the one
that matters.

A class filter would have discarded the report that prompted this. The failure was classified
`environment` and it was not an environment problem. **A wrong class is itself a symptom**, so a
filter that trusts the class throws away exactly the failures worth keeping, and throws them
away at the one moment they cannot be recovered.

**The site does the filtering that a class never could.** The recorder sits where the binary
reports an `ArmadaError` and exits by its class — the path taken when *Armada could not answer*.
A check whose tests fail is an *answer*: it comes back as an envelope with `FAILED` in it, exits
1, and never reaches the recorder. So "a failing test suite fills the log" cannot happen, and it
cannot happen structurally rather than by a rule somebody has to maintain.

What that does admit is the mistyped verb. `bad_invocation` is a person being human, not Armada
being broken. That is the price, and dedup is what makes it affordable: four typos are one row
saying `x4`, and one `armada failures clear` is the end of it. **Deciding at write time is
irreversible; deciding at read time is one keystroke.**

### The correction, made after a day of real use

The site was not enough, and the log said so: twelve entries in thirteen minutes and **not one
was a bug**. Every one was `bad_invocation` — two reserved flags refusing by name, three
reserved verbs refusing by name, and a verb somebody typed as `bogus`. The sharpest is
`--detach is not built yet`, which is a reserved name refusing by name and is a *feature*, asked
for deliberately. Recording it as a failure to be fixed is the log misreporting a success.

The class still cannot draw the line, for the reason above. What can is **whose mistake it
was**: *did Armada do something wrong, or did the caller ask for something that does not exist?*
That is `armada_core::failure::Fault`, and three things about it are the design:

- **Only a refusal site can answer it**, because only it knows whether it recognised what it was
  refusing. So the parser marks its own refusals and the marker travels out on `ParseFailure`.
  Nothing downstream re-derives it from the message, which would be a second grammar over
  Armada's own prose.
- **Unmarked is recorded.** `Fault::Armadas` is the default, so the failure mode of the seam is
  one noisy row — deduplicated, one `clear` away — rather than a bug that is never reported.
  That direction is not negotiable.
- **A site saying `unknown` has that claim audited.** Saying a name is unknown is a refusal
  Armada meant *only if it is true*, so the site checks the whole roster — `every_verb()` plus
  every reserved table — rather than the one table it happens to hold. `unknown command
  `guild verify``, about a verb the help page advertises under NOT BUILT YET, is Armada
  contradicting its own roster: it is a bug, and it stays recorded. That is the test the seam
  was placed to pass, because it is one line away in the parser from
  `unknown verb `armada guild bogus``, which is a typo and is not.

A verb's own refusals are untouched — see *Known rough edge* below, which is still deliberate.

### And a throwaway worktree is not the machine's to keep

The same log recorded failures from agents' worktrees under `.claude/worktrees/`. Those are not
kept either, and the reason is not that they are uninteresting: **the entry cannot survive its
own subject.** The row's `cwd` is what `armada failures fix` branches the Job from, and a
worktree under `.claude/worktrees/` or `~/.armada/workspaces/` is removed when the work that
made it ends — so by the time anybody reads the row it names a directory that is gone. The Job
cannot start there and the failure cannot be reproduced there. An agent that hits a real bug has
a better channel anyway: the report it hands back, which a person reads.

It is a substring test on the path rather than a `git` call, because the whole premise of this
code path is that something has already gone wrong.

## The listing is navigable at a terminal

**Asked for in these words:** *"It should be an interactive terminal, whatever that is called,
so that I can navigate the list and quickly dispatch a job rather than having to remember the ID
and copy it and then run fix with the ID."*

`armada guild ls` is the shape and the precedent: **at a terminal the listing is navigable;
through a pipe, and under `--json` even at a terminal, it is the same listing printed once.**
Same verb, and deliberately **no flag that opts into the interaction** — a terminal is the flag,
which is [`PLAN.md`](../PLAN.md) §3.1.1's three audiences working normally. An interactive-only
verb would be a bug rather than a feature, so the interaction is layered on an answer that
stands without it.

From a selected row, three things and a way out — **show it whole**, **spawn a Job on it**,
**discard it**. Each runs the function `show`, `fix` and `clear` already are, so there is one
implementation of each and one place a field can be added to it. **The selection carries the id
into `fix`**, which is the whole feature: nothing is retyped and nothing is mistyped.

Three things are reused rather than rebuilt: `crates/helm/src/ask/select.rs` is the selector, it
exists because a one-off was rejected once already; `guild::report` draws what just happened,
onto stderr where every mid-session report goes; and `render::failure_detail` writes the detail
cell, so the person navigating and the person reading a pipe cannot be shown two sentences.

The rows keep the table's shape — `STATUS · ID · DETAIL · TIME`, in that order, status first and
always a word. **No tick and no cross**: a row told apart by a glyph or a colour is a row a
monochrome terminal cannot tell apart at all.

**Discarding asks nothing first**, which is where this parts company with `guild ls`'s delete.
That removes a file from a guild that syncs to every machine; this appends a line to an
append-only log, keeps the id and the entry, and reopens on the next recurrence. There is
nothing to confirm because there is nothing to lose.

## Deduplication is what makes this observability rather than a log

The same failure eight times is a different fact from eight failures once each, and a list that
does not collapse repeats is unreadable after a day of real use. So a row carries a **count**
and a **last-seen**, and the fingerprint decides what "the same" means:

**class + `where` + a normalised message.** The message is normalised because two runs of one
bug differ in the things that are about the *run* rather than the bug — a pid, a duration, an
absolute path. The normaliser is `Scrub`, the same one `armada manifest explain`'s failure
signature uses ([`PLAN.md`](../PLAN.md) §3.4): "is this the same failure" is one question and it
should not have two answers in one binary.

Three consequences, each of which rules out an alternative:

- **The class is in the fingerprint**, even though a wrong class is a symptom. Two failures
  Armada classified differently are two things to look at, and collapsing them would hide the
  reclassification — which is itself the fix landing.
- **The directory is not in it.** The same bug in two repositories is one bug with more
  evidence, not two. The entry carries the most recent `cwd` so the Job that fixes it still has
  somewhere to start.
- **Clearing resets rather than deletes.** A cleared entry keeps its id and its place in the
  file; a later occurrence reopens it with a count of one, because a bug you dismissed and that
  came back is news, and a count running since the beginning of time is not.

## The constraints it is built under

- **Recording never changes an exit code and never swallows an error.** It is a side effect
  allowed to fail silently — the write returns `bool`, not `Result`. A logger that turns a bug
  into a different bug is worse than none. Only the reading verbs, which a person invoked
  deliberately, are allowed to complain.
- **No absolute `$HOME` is ever written.** Every string reaching the file goes through `tilde`
  first, so `/home/you/.cargo/bin/armada` is stored as `~/.cargo/bin/armada`. This repository is
  public permanently, and the log carries paths — so the rule is enforced at the one chokepoint
  rather than asked for at each call site. It also means a promoted Job's task, which leaves the
  machine, cannot carry a home path out of the log.
- **Append-only**, the same shape as the inbox and for the reason [`PLAN.md`](../PLAN.md) §15.3
  gives: it survives every kind of crash. That is not incidental here — the thing being recorded
  *is* the crash. A torn last line is skipped and the entries before it still read.
- **One file for the machine, not one per repository.** Every entry carries the directory it
  happened in, so "the failures from this repo" is a filter rather than a second store. Keying
  by project would mean a `git` call on the error path, in the one code path whose entire
  premise is that something has already gone wrong.
- **Promotion spends nothing.** `fix` names the `bug` workflow ([`PLAN.md`](../PLAN.md) §14.6)
  rather than classifying it. Every entry here is by construction a failure of Armada's, so
  `bug` is the answer before the question is asked — and a named workflow makes no model call,
  which is what makes the path testable.

## What it is downstream of

[001](001-raised-items-need-identity.md) is the argument for the three states: *done*, *not
doing it* and *being worked on* are different answers, and a tick loses the one that matters.
[002](002-tasks.md) is the shape promotion follows — a record that becomes a Job and **links the
two**, so the row shows the Job's name once one is on it.

**When the task system exists**, a recorded failure and a task are the same object from two
directions, exactly as [002](002-tasks.md) says a task and a raised item are. This should become
a source of task rows rather than a fourth list.

## Known rough edge

**The reading verbs record their own failures.** `armada failures show <a typo>` is a
`bad_invocation` and is written down like any other. That is the design working as stated — the
site is the filter, not the class — and it means browsing the log can add to it. Dedup bounds it
to one row and `clear` ends it. It was left as-is deliberately: `show` failing to resolve a
prefix could also be Armada mis-resolving one, which is a real bug shape, and an exemption would
be the first hole in "nothing fails unseen".

**`Fault` did not change that**, and the boundary is worth stating because it looks arbitrary
from a distance. The parser's refusals are marked because the parser *knows the whole space of
names* and can say for certain that one is not in it. A verb resolving a prefix against a log
knows only what the log happens to hold, and "I did not find it" is exactly what a resolution
bug looks like from the inside. The seam is where the certainty is.
