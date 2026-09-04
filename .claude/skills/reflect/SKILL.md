---
name: reflect
description: >-
  Use when the user wants to end, wrap up, or close out a session working in this repo —
  "reflect", "wrap up", "wrap this up", "close out this session", "are we good to close
  out?", "anything left to update?", "/reflect". Drives every branch through the gate to
  merged, gives back the worktrees, fixes the prose the work just falsified, and checks
  that follow-up work has a home outside the transcript — so the session can be closed
  knowing everything already landed.
disable-model-invocation: true
---

# Reflecting before a session closes

A session is safe to close only when six things are true: **every branch is
through the gate and merged**, **every worktree this session cut is gone**, **no
prose still asserts what the work just made false**, **`docs/OPEN.md` and the
milestone match reality**, **every follow-up has a durable home**, and **the
owner has been told if a running Fleet is now stale**.

This skill checks all six and **drives the first four to completion itself** —
merge and cleanup and doc-fixes, not detection handed back as a to-do list. Then
it reports a verdict.

**That self-merge authority is scoped to this skill.** It is not a standing
"merge whatever is green"; invoke it only when actually wrapping up.

Announce: *"Using the reflect skill to check whether this session is ready to
close."*

## The one rule under all of it

**Code that moves falsifies prose that did not.** Everything below is a place
that prose lives. Work through them; do not reason about which are likely.

## Every edit this skill makes goes in a worktree

**This skill writes, and what it writes is not exempt.** Steps 3, 4 and 7 fix
prose, and step 7 edits this file. All of it happens on a branch in a worktree —
`git worktree add .claude/worktrees/<name> -b <branch> main` — and lands as a
pull request like any other work. Never on the checkout at `main`.

Two reasons, and the second is the one that bites:

**A reflect edit is ordinary work and takes the ordinary bar.** A doc correction
that goes straight onto `main` skips the gate, the diff and the review that every
other line in this repository passes through. The fix that reconciles prose can
be wrong.

**Editing `main` while committing from a worktree strands the edit.** Confirmed
4 Sep 2026, earlier in the very session that then ran this skill: a doc comment
in `crates/fleet/src/proposal.rs` was fixed in the main checkout while the commit
was taken from a worktree, so the pull request's message described a change its
own diff did not carry. It went in green, and the owner found the loose file on
`main` afterwards — *"There is an uncommitted change on main, is that yours?"* It
cost a second pull request to land what the first one claimed.

So: **check `git status --porcelain` on the main checkout before declaring
anything merged**, and take every edit inside the worktree the commit comes from.

## 1. Through the gate, merged, and `main` synced

For every branch this session committed to. The gate is all of it, every time:

| | |
|---|---|
| `cargo build --workspace --locked` | clean |
| `cargo nextest run --workspace --exclude acceptance` | the count, against the count on `main` |
| `cargo test -p acceptance` | separately; a milestone's own claim may be red while it is in flight |
| `cargo fmt --all --check` | clean |
| `cargo xtask verify-docs` | green — a stale `docs/OPEN.md` fails it |
| `cargo xtask verify-foundations` | **the delta, never the colour.** A `missing:` line this session added is a regression |
| Bridge: `typecheck`, `build`, `build-storybook` | if `apps/` or `packages/` was touched |

**Take the baseline from `main` in the same pass**, not from a brief written
hours ago. Counts moved 1249 → 1325 in one evening here; a stale baseline turns
a correct report into a wrong one.

**Rebase before merging, never merge a stale branch.** An agent that started
before three other merges is not describing today's `main`. `git rebase main`,
re-run the gate, then `git merge --ff-only`.

**Drive it to merged.** Do not stop and hand the owner a branch to merge — that
is the step this skill owns. Never force a merge past a failing gate, a real
conflict, or an unresolved question, and never merge a branch this session did
not produce.

**A Job's work is a pull request.** Fleet commits, pushes and opens one; `gh pr
merge <n> --merge` once `gh pr view <n>` reads `MERGEABLE`. Do not squash.

## 2. Worktrees given back

`agent-worktrees` owns the rules. **The merge is the moment**, and it still gets
missed — including by the agent that wrote that skill four hours earlier.

```
git worktree list | grep .claude/worktrees
git branch --list "armada/*"
```

Remove merged-and-clean ones yourself. **Worktrees this session did not create
are out of scope** — other agents' work lives in the same listing, and a dirty
tree is indistinguishable from an idle one from outside. Name those to the owner
rather than removing them.

## 3. Prose the work made untrue

Grep for the claim, not for the file. A sentence asserting Armada *cannot* do a
thing is the one that rots, and it rots silently because nothing tests prose.

- **Bridge copy** — `NOT_SERVED`, absent-state sentences, handover notes.
- **Enum verbs and registry notes** — `enum-verbs.toml` carries a note per row
  saying why the verb is safe. The note outlives the reason.
- **Manifest and config comments** — `armada.yml`, `settings.toml`.
- **Doc comments naming a gap** — closing the gap is what makes them false.
- **`CLAUDE.md`** — 50 lines is refused, 30 asks. It routes; it does not explain.
- **`docs/INDEX.md`** — the gate refuses a document that is not in it, including
  a new skill.

## 4. `docs/OPEN.md`, issues, milestones

- **An open question the work answered is a stale entry**, and `verify-docs`
  fails on one. Ask it of every area you touched.
- **Close what merged**, with what was decided and why — not "done". That comment
  is what the next reader gets instead of the diff.
- **Confirm the milestone is empty** before saying it is:
  `gh issue list --milestone <name> --state open`.

## 5. Follow-up work

Scan the session for anything flagged and not finished — a deferred fix, a known
gap, an aside that deserved an issue, a `TODO` left in code.

**Filing is the owner's call and is never done on your own initiative** —
`armada-bug`'s standing rule. For each item, use `AskUserQuestion`: file it now,
file it and fix it, fix it without an issue, or drop it. A follow-up that exists
only as a sentence in the transcript counts as **not documented**.

**The history file is not a home, and writing one does not discharge this
step.** It is a record of a session, read by nobody looking for open work. A
deferred item needs a durable home in the repository — an **open question** in
the document that blocks on it (`armada-open-questions`), or a **GitHub
issue** — and the history file then points at that.

**"Talk it through later" is not an outcome; it is where the item goes while it
waits.** A decision the owner defers is exactly what an open question is for:
one lives in the document that blocks on it and carries a slug code can cite, so
answering it surfaces everything that was waiting. Offer that as an option
alongside filing an issue, and say which fits — a call to make is a question, a
thing to do is an issue.

Confirmed 2026-08-31: two design conversations were deferred, recorded in the
history file, and reported as "recorded as decisions, not loose ends." The owner
read that and said they would get lost. They were right — `docs/OPEN.md` is what
the repository reads, and neither was in it.

## 6. The running Fleet

**A running Fleet is a stale binary the moment you merge.** If
`protocol-version.toml` moved or a store migration landed, say so in the last
message — a major bump means Bridge refuses to connect until it is rebuilt.

## 7. Skills, and this file

If something went wrong that a skill would have prevented, the skill did not
exist or did not say it. Write it where the next agent will hit it.

**A skill earns its lines by naming what the mistake cost.** A rule with no
incident behind it is advice, and advice is skipped.

## Output

| Check | Status | Detail |
|---|---|---|
| Gate passed, merged, `main` synced | ✅ / ❌ | branches; merged or blocked and why; test and foundations delta |
| Worktrees given back | ✅ / ❌ | removed; which remain and whose they are |
| Prose reconciled | ✅ / N/A | what was corrected; N/A only if no code changed |
| `OPEN.md`, issues, milestones | ✅ / N/A | entries removed, issues closed, milestone confirmed empty |
| Follow-up documented | ✅ / ❌ | where each item now lives, or what is missing |
| Fleet restart needed | ✅ / N/A | protocol version, migrations |

**Ready to close** only if every non-N/A row is ✅. Otherwise **Not ready**, and
say exactly what blocks each ❌.

Label every line with who acts: already handled, waiting on the owner, or context
only. **One sentence when nothing needs them** — do not leave it to be inferred.

## Gotchas

- **A pipe masks a failed merge.** Confirmed 2026-08-31: `git merge --ff-only X
  2>&1 | tail -2` exits with `tail`'s status, so an `&&` chain continued past
  `fatal: Not possible to fast-forward` and deleted the branch. The commit was
  recoverable by sha, but only because the sha was in the output. **Never pipe a
  command whose exit code an `&&` depends on.**
- **Parallel agents collide in vocabulary, not only in files.** Confirmed
  2026-08-31: two agents independently added a type named `Keeping` on one
  evening — different subjects, one name, caught only at the merge build. Tell
  each agent the nouns the others are introducing.
- **`verify-foundations` reads one higher inside a worktree.**
  `docs/v1-decommission.md` names `.claude/worktrees/`, which does not exist
  inside one. That line is an artifact, not a regression — compare `FAIL` lines,
  not the count.
- **The report is not the work.** An agent's summary is written by the thing with
  an interest in it being right. Read the diff. A gaming flag was once relayed to
  the owner as a Drone cheating; the diff showed both flags were false positives.
- **Abandoned work is not unlanded work.** Confirmed 2026-08-31: a `stash@{0}`
  dated three days earlier was described to the owner as a whole `forget_job`
  operation existing nowhere else, and dropping it as irreversible — on the
  strength of its date and its contents. `forget_job` was already on `main`,
  landed after the stash was taken. **Grep the tree for what the artifact
  contains before saying what losing it would cost**; a stash, an old branch or
  a detached worktree is a claim about the past, and the present is what decides
  whether it matters. The wrong framing had already reached a decision — the
  owner chose "leave it" against a loss that could not happen.
- **A branch left unmerged reads exactly like a merged one** from the session
  transcript. This skill found one that had been reported as "verified and
  waiting" and then never merged. Check `git branch --list`, not your memory.

## Verification

Before ending: state **PASS** or **FAIL** plainly — PASS only if what this skill
exists to produce actually landed, not "I ran the steps."

Then write `.claude/history/<UTC-timestamp>-reflect.md`:

```
---
skill: reflect
verdict: PASS | FAIL
date: <UTC ISO-8601>
milestone: <name>    # if applicable, omit otherwise
---

<one line on what happened>

## Follow-ups
| Action | Owner | Detail |
|---|---|---|
```

Omit the table entirely if nothing is outstanding. Commit it with the session's
own changes, not as a separate step.
