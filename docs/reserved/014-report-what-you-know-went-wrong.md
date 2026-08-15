---
id: 014
title: Reporting what you know went wrong
status: BUILT
module: cross-cutting
raised: real use, 2026-08-15 — a dry-run printed CREATED for work it had correctly not done, and exited 0
---

# 014 — Reporting what you know went wrong

> **Built.** `armada report "<what happened>"`, `~/.armada/recent.jsonl`, and a `reported` line
> shape in the failure log. What follows is the design, recorded here because the three
> decisions in *How it knows what just happened*, *One store* and *Secrets* are the parts a
> later change is most likely to get wrong — and the third is the one that cannot be repaired
> afterwards.

**The complaint this exists to fix.** `arm fleet spawn … --dry-run` printed `CREATED worktree`,
`STARTED drone` and `QUEUED` for work it had correctly not done. The render lied. **The exit
code was 0**, nothing threw, and so no entry could ever exist in
[010](010-armada-records-its-own-failures.md)'s log for it. The only way to get it fixed was to
copy the terminal into a chat and describe it by hand.

**What was asked for**, in the words it was asked in: *"Similar to the failures idea, but in
this case, it's something that's not a failure, but I wanna report it. The thing that just
happened, I had to copy and paste the output and then bring it back to you … it would be sweet
if we just had a command like `armada report` that actually reported issues like this where it
included the logs and the output and any other diagnostics that would be helpful, but **don't
include secrets**. Right now it can use the same failure system that's local to a machine. And
then in the future we can explore if it creates GitHub issues."*

## Why this is not [010](010-armada-records-its-own-failures.md) under a second name

`armada failures` records what **Armada noticed**. Its recorder sits where an `ArmadaError`
reaches the terminal, which is exactly what makes it trustworthy — and exactly what makes it
blind to this. The set it cannot see is the set worth naming:

| What went wrong | Reaches the failure log |
|---|---|
| Armada could not answer | yes, and that is the whole of [010](010-armada-records-its-own-failures.md) |
| Armada answered, and the answer was wrong | **no** — it exited 0 |
| Output that was missing, misleading, or the wrong shape | **no** |
| Anything that looks fine to the program | **no** |

**`failures` is what Armada thinks went wrong; `report` is what you know went wrong.** Neither
set contains the other, so both are needed.

## How it knows what just happened, which was the open question

A description alone loses the output he would otherwise have pasted, and the output is what made
the complaint checkable. Three ways to get it, and only one of them works after the fact:

| Option | What it captures | Why not |
|---|---|---|
| `--last`, re-running the previous command | a **second** run | re-running a mutating verb to describe the first one is a bug report that spawns a Job |
| piping or pasting the output in | whatever was kept | it is the manual step the ask exists to remove, and the scrollback is already gone |
| **a ring buffer, written as each run ends** | the run already done | the only one that can attach a run nobody knew would matter |

**The ring buffer, `~/.armada/recent.jsonl`, `armada_core::recent`.** After the fact is the only
time anybody files a report; the other two require having known in advance.

**What it costs is a new file holding every command typed, and that is a real privacy surface.**
Four things bound it, and the first settles it:

- **Armada already writes this down.** The failure log records the argv and the directory of
  every failure it reports. This extends that to the runs that *succeeded and lied* — the set
  the failure log structurally cannot see. It is not a new kind of data.
- **Bounded**: ten runs, rewritten in place. A log of everything since installation would be a
  different thing needing a different argument.
- **Redacted on the way in, not on the way out**, so a credential is never at rest there even
  for the runs nobody ever reports. Scrubbing at report time would leave a window, and the
  window would be the file's whole lifetime.
- **`$HOME` never appears**, by the same chokepoint [010](010-armada-records-its-own-failures.md)
  states.

**The envelope is captured, not the terminal.** Armada renders every answer from one
([`PLAN.md`](../PLAN.md) §3.1.1), so the envelope is what the render was drawn *from* — which
makes *"it said CREATED and created nothing"* a question a reader can settle, and makes the
attachment useful to the agent reading `--json` rather than only to a person. Capturing the
drawn terminal would mean teeing every byte of every stream and storing ANSI escapes.

## One store, and the origin is a column

Asked for in those terms — *"it can use the same failure system"* — and it is right for
[001](001-raised-items-need-identity.md)'s reason: a thing needing attention is useless until it
has an id you can act on one at a time. A second store means a second id space, a second `show`,
a second listing to remember and a second promotion path, and **a person triaging on a Monday
morning does not care which half of the machine noticed.**

So `armada_core::failure::Origin` is a field on the record, `Line::Reported` is a fourth line
shape in the same append-only file, and every verb [010](010-armada-records-its-own-failures.md)
built takes a report unchanged: `armada failures` lists it, `show` opens it, `fix` promotes it
onto the `bug` workflow, `clear` discards it.

**A report carries no class, and that is not a gap.** The class is Armada's own attribution and
Armada attributed nothing — it did not notice. Inventing one would seed exactly the defect
[010](010-armada-records-its-own-failures.md) exists to guard against, since its central
argument is that *a wrong class is itself a symptom*. The DETAIL cell leads with the origin
instead, so the listing reads:

```
OPEN    a91f0c37  environment x4, `armada manifest clean` could not be found to run   9m
OPEN    6d40b1e9  reported, the dry-run said CREATED worktree and made nothing         4m
```

**And a report is never deduplicated.** A failure's id is a fingerprint precisely so eight
occurrences are one row — Armada writes those without being asked, so collapsing repeats is a
kindness. A person typing `armada report` twice has done two deliberate things about two
different runs, and merging them would discard the second's diagnostics, which are the part that
differs and the part worth having.

## What is attached, and why each

| Attached | Because |
|---|---|
| the last runs and the envelope each answered with | **the thing that just happened** |
| `armada doctor`'s findings, the ones that are not `ok` | the verb whose whole subject is what this machine is missing |
| Armada's version, `claude`'s, the OS and arch | the first three questions anybody asks about a bug |
| the workspace, and whether it has an `armada.yml` | half of Armada's behaviour turns on this and it is invisible from the report's text |
| the ids of failures still open | a report and a recorded failure are often one event from two sides |
| the Jobs in flight | the ask names Helm, and a report about a Job has to name it |

**Every gatherer is best effort, and the report is filed either way.** A report exists *because
something is wrong*, so each one runs on a machine that may be broken in the way being reported
— a diagnostic that could fail the report is a bad diagnostic. Each degrades to an absence, and
the absence is itself evidence: *"`claude --version` did not answer"* is a finding.

**It runs before every other verb's preconditions, including Fleet's boot id.** `armada fleet
ls` refuses without one, and *"Armada will not run on this machine"* is the most valuable report
there is. So the boot id is optional here and the Jobs diagnostic is what gives way.

## Secrets, which is the constraint that cannot be repaired later

*"But don't include secrets."* A report attaches command lines, environment-derived text and
captured output, all of which routinely carry tokens. This repository is public permanently and
the GitHub-issue path below makes that worse rather than better: **a record that has already
leaked a secret locally cannot be un-published once it is.**

- **One detector, and it is not a new one.** `armada_guild::secrets` already decides whether a
  value is credential-shaped. `armada_helm::redact` is a *tokeniser* over that predicate, not a
  second predicate: Guild walks a parsed JSON document where a value is a leaf and its key is in
  hand, and a report attaches prose and shell lines where the same value arrives as
  `NAME=value`, as `--flag value`, or as a bare word.
- **It lives in Helm because Helm is where both sites are** — the entrypoint that writes the
  ring buffer, and the `report` verb — and Helm is the only crate that may name Guild and the
  core ([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.9). Pushing the detector down into the core
  was declined: it would move a rule out from under the module that owns secrets to save passing
  a function pointer.
- **The walk is in the core and it is exhaustive.** `armada_core::failure::reported` takes the
  redactor as an argument and applies it, then `tilde`, to every field of the record and of every
  run it attaches. A field added to the diagnostics cannot be written without passing through
  it, which is what makes the guarantee structural rather than a rule somebody has to remember.
- **Redacted from, never with.** `GITHUB_TOKEN=[redacted]` and `--token [redacted]` keep the
  fact that the variable was set and the flag was given, which is often the diagnostic, and lose
  only the part nobody may see. A redactor that dropped the whole line would make every
  attachment useless, which is the failure mode that ends with the guard being turned off.
- **`crates/helm/tests/report.rs` is the deliverable**, not the claim. It plants a
  credential-shaped value in an environment variable, on a command line and in the captured
  output of an earlier run, and asserts none reaches either file — and asserts the other half
  too, that the flag and the verb around it survive. It was verified by disabling the scrubbing
  and watching it fail.

## GitHub issues are deliberately not built

*"In the future we can explore if it creates GitHub issues."* The local record is the whole of
this. What it owes that future is two things it already has: a **shape that renders into an
issue body** — `armada_core::failure::task` is a heading, what happened, what was run and the
machine it ran on — and the **redaction**, which is the half that cannot be added afterwards.

## What it is downstream of

[001](001-raised-items-need-identity.md) is the argument for one id space. [002](002-tasks.md)
is the shape promotion follows, and when the task system exists a filed report and a recorded
failure become rows of it together — which is the point of them being one store now.
[010](010-armada-records-its-own-failures.md) is the half of the problem this completes.
