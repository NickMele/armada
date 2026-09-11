# Journey 9 — Run and edit a Manifest

**What it is:** View a project's Manifest, run any single Check or Command against it — without dispatching a Job, or inside one Job's worktree — and edit it.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** "I want to run the lint here without spinning up a drone," "this project's checks are wrong and I want to fix them," or "the gate refused `test` on this Job and I want to run it myself."

**Concepts touched:** Bridge (Manifest — a new surface, and Job detail), Manifest, Fleet, Job.

**Milestone:** Reach.

**Drawn against** `docs/contracts/configuration.md`.

## Why this journey did not exist

Most of this capability was already sanctioned and had no surface. The Manifest concept page states that Commands are invoked by both a Drone during a Job and by you directly via Bridge, off the same named registry, and that your own manual invocation needs no second approval because you are the one triggering it. It also specifies **Update** as manual editing and **Verify** as drift detection plus a live dry-run.

**Running a single Check on demand is the one new capability.** Checks are specified as invoked by Fleet as part of mechanical verification, and the only ad-hoc path was Verify's live dry-run, which runs all of them. Running one is new capability, not a missing screen.

Manifest is Bridge's seventh surface, in the rail beside Job Board and Doctor, carrying the `file-cog` glyph already assigned to it under Git and config.

## Flow

Open Bridge → Manifest → read the current project's Checks and Commands → run one, or edit.

> **Rule.** One Manifest is open at a time, chosen by the project switcher already in the rail.
> Why: the switcher is how the rest of Bridge scopes itself, and a second selector inside the surface would answer the same question twice.

A monorepo's root `armada.yml` appears as its own entry in the switcher, since its Checks run only when the diff touches root paths and it is not just another workspace.

> **Rule.** Checks and Commands render as two separate groups, never one list with a badge.
> Why: they are separate registries that differ in what they mean — a Check gates code, a Command does not — and one list would ask a person to learn a distinction the layout could just state.

## Running one

> **Rule.** The run panel asks where to run — the working tree as it is on disk, or a throwaway copy — before every run, on the panel itself rather than behind a split-button caret, and never remembered per command.
> Why: a migration and a lint want different answers, and asking costs one click on a screen already reached deliberately.

> **Rule.** Output goes to a panel on this surface — exit code, duration, and the output — and stays until dismissed.
> Why: there is no Job, so no Job Board row and no Evidence, and a toast would throw away the thing you ran it to read.

A server started here runs in the working tree and holds a port claim of its own. It stops when you press Stop, when it exits, or when Fleet stops, and the claim goes with it.

### A passing Check leaves no verdict behind — but the output is a log

The output and the judgement are two different things.

Each run writes its output to `./.armada`, one run directory per run in the same tree as Job logs, so the panel is a view onto a file rather than a buffer that evaporates — Dismiss closes the panel, not the log.

> **Rule.** Ad-hoc run log retention is a Machine setting, default 30 days — shorter than a Job's own retention window.
> Why: a Job log is worth keeping as long as its history is worth reading, and a rehearsal is only worth keeping as long as you might still be debugging what you just ran.

> **Rule.** No run from this surface writes Evidence, and no Check gets a stored pass or fail against it.
> Why: a pass that counted for a Job from the same tree is the path around verification that v1 proved becomes the default path.

A remembered per-Check verdict reads as Evidence the moment it sits beside a Job, and no copy saying "not evidence" survives being scanned. The tree can also change between the run and a dispatch, which is the whole reason Evidence ties a gate's answer to the diff it judged.

The transcript is keepable, the judgement is not. This surface is for rehearsing; a Job is where a Check means something.

## Running one inside a Job

Job detail carries the same rehearsal for one Job, against the Manifest that Job froze and in that Job's worktree. It opens as a trailing sheet under the sheet rules in [Monitor Active Work](monitor-active-work.md).

> **Rule.** The run sheet lists and runs the Manifest the Job froze at creation.
> Why: the gate judges the Job against that snapshot, so rehearsing anything else answers a question the gate never asks.

> **Rule.** A run from the sheet writes no Evidence and no `check_runs` row, and moves nothing on the Job.
> Why: the reasoning under *A passing Check leaves no verdict behind* holds with more force beside a Job.

### Opening it

| From | Opens |
|---|---|
| **Run…** on the worktree row of *Where things are* | The sheet, nothing selected |
| `r` on Job detail | The sheet, nothing selected |
| **Run it here** on a refused Check's row | The sheet, that Check selected and narrowed |
| The command palette | One entry per Check and Command |

Where the Job's worktree no longer exists, **Run…** stays in place, disabled, and the row says why.

### What it lists

| Group | Holds | Each row carries |
|---|---|---|
| Setup | The Commands `setup.requires` names | Its `run` line; re-running one repairs a broken install |
| Checks | Every Check, in declaration order | Its `run` line |
| Commands | Every Command `setup.requires` does not name | Its `run` line, or its `serve` line for a server |

Setup rows show no time, because nothing records when setup finished in a worktree. The sheet's header shows when `armada.yml` was last edited before the Job froze it, which Fleet reads from git. Where the worktree's own `armada.yml` differs from the frozen one, a notice says so and offers the worktree's version.

### Running one

> **Rule.** A run from the sheet executes in the Job's own worktree.
> Why: a throwaway copy waits on `runedit-adhoc-run-location`, below.

| Case | Behaviour |
|---|---|
| A Check with `narrow` | Narrowed to what the Job changed; the whole tree is one press away |
| A Check with `requires` | Its prerequisites run first, as the Manifest concept states |
| Output | Streams live while the elapsed time counts up |
| Stop | Ends the run's process group; the log keeps what printed |
| A Drone working in the tree | Nothing locks; a notice says the run shares its tree and build directory |
| Approval | None, destructive or not; `runedit-destructive-command-warning` applies |

### A server

A Command with `serve` stays running, on ports from the Job's span. [Manifest](../concepts/manifest.md) defines its keys and [Fleet](../concepts/fleet.md) holds it, one instance per Job.

| Moment | What the sheet shows |
|---|---|
| Before `ready` passes | The row reads *starting*, with the log streaming |
| Serving | *serving*, how long it has been up, a button per link, and Stop |
| The sheet closed | Nothing stops; *Where things are* gains a row per server, with its links |
| Started by a Drone | The same row, with Stop |
| It exits on its own | Its exit code and log, as a failure |

> **Rule.** A link hands its address to the system browser.
> Why: no surface in Bridge navigates, per the design system's hard rules.

### What a run leaves

Each run writes its log under `./.armada` like every other ad-hoc run here, with the same retention. No Check row carries a result, and nothing from a run reaches the rail, the Job header or the Job Board.

> **Rule.** The sheet's run list shows each run's exit code, unhued, and opens its log.
> Why: a past rehearsal is worth scanning, and keeping it inside the sheet stops it reading as the gate's answer.

> **Rule.** After a run that changed the worktree, the sheet lists the changed files and offers Open the diff and Undo this run.
> Why: nothing in a Manifest says in advance which Commands write, and a Check's `requires` can name one that does.

> **Rule.** Undo restores a snapshot of the worktree taken just before the run, and is not offered while a Drone is working.
> Why: Fleet commits a Job's work only when it delivers, per `crates/fleet/src/landing.rs`, so a plain discard would take the Drone's uncommitted work with it.

## Editing

> **Rule.** Editing shows forms by default — one section each for Checks, Commands, allowlist, budget and freeze — with the raw file behind a toggle.
> Why: those are the parts of a Manifest with known shapes and enumerated values.

> **Rule.** The form expresses everything the schema has, so an edit made in the file always round-trips back into the form and nothing can be dropped.
> Why: the file view is there for people who would rather read the file than the form, not for what the form cannot reach.

The schema and the form move together — a field added to one is a field added to the other. The file stays reachable because hiding that a committed file is a file would hide what lands in the repo, not because the form falls short.

> **Rule.** The toggle is never labelled with the file's format; it is named by its path.
> Why: the lexicon bans naming a Manifest by its format — it is a Manifest, never "the yaml."

> **Rule.** Editing during a running Job needs no warning and no lock.
> Why: a Job is judged against the Checks and Commands it snapshotted at creation, so an edit here cannot reach it — the next Job picks up the new file, the running one does not.

> **Rule.** Save writes the file to disk and stops there — no staging, no commit.
> Why: the file is tracked, so the edit appears in the working tree and is committed along with everything else; Armada committing to git on your behalf would be a surprise in the one place a person is most sensitive to one.

> **Rule.** Write always commits whatever the proposal has iterated to — there is no separate Write-path mode for a single workspace versus a monorepo batch.
> Why: iteration happens per workspace, at whatever pace suits its own evidence, so there is nothing left for a Write-path mode to choose between.

## Verify

Both halves belong on this surface: **drift detection** (re-scan and flag) and the **live dry-run** (run everything once).

> **Rule.** This is the surface where you look at what drifted and act on it, not Doctor.
> Why: Doctor's Manifest module only reports *that* a manifest has drifted, and Doctor is a scan that changes nothing, so the re-scan and acting on it cannot live there.

> **Rule.** Verify runs setup and every Check once, and reports what happened — nothing more.
> Why: a manifest that parses cleanly can still be wrong in every command it names, which is most likely right after it is written or edited.

> **Rule.** Verify and drift are two separate things, not two halves of one panel.
> Why: one panel with two verdict groups reads as an audit of the file rather than a rehearsal of it.

> **Rule.** Drift is a read — whether everything the file names still exists — and runs on opening, at no cost.

> **Rule.** Verify is an act — it runs the file, behind its own button, and asks where to run once for the whole set.

Running Verify straight after Write, as [Set Up a Project (Manifest)](set-up-a-project-manifest.md) does, is correct: a brand-new file is exactly the case Verify exists for.

> **Rule.** Drift never reports a script the repo picked up that the file does not yet name.
> Why: the file carries no record of what it was written against, so nothing can call a script new or changed without storing a scan — existence alone needs no history.

> **Rule.** Drift says nothing about whether a listed command still does the right thing — a `test` script narrowed to one directory still reads as existing.
> Why: keeping a script and its Check in step is an engineering judgement, not something Armada checks for.

> **Rule.** Verify and drift say nothing about policy or permissions, because neither names anything runnable in the repo; the panel states that explicitly.
> Why: about a third of a mature Manifest names nothing runnable, and a clean result must not read as "all of this is still right" when that much carries no verdict.

> **Rule.** Drift's two verdicts are `gone` and `current`, answering *is this file still true* — a different question from the dry-run's *does this file still work*.

> **Rule.** The dry-run never runs automatically — not on opening the surface, and not as part of drift — and always asks where to run, once for the whole set.
> Why: it runs real Checks and a real test suite, which is expensive, while drift is a free read that can run on opening.

> **Rule.** Verify reports and never fixes — no Apply, no Accept all — and acting on a row means going to Edit, where the consequence is stated.
> Why: the Manifest concept page specifies Verify as flagging without changing the file, with Update as the only manual edit path.

> **Rule.** A drifted `id` row offers no fix action at all.
> Why: `id` reads as drifted whenever a package is renamed, and changing it orphans every Job that references this Manifest.

> **Rule.** Drift renders amber throughout, never red.
> Why: a drifted file is behind, not broken — the dry-run beside it is what would prove broken.

Verdicts render as words in the status colour, matching Doctor: `gone` and `current`.

> **Rule.** A dry-run failure never becomes a Doctor fail.
> Why: Doctor reports live, stateless service health, while a dry-run is a rehearsal chosen on one project — routing it into Doctor would store a verdict there and read one project's broken Check as a machine-level failure.

## Open questions

- **[runedit-adhoc-run-location]** Where does an ad-hoc run execute — a real worktree, or a simpler temporary checkout?
  A throwaway copy is a worktree by another name, and Fleet owns worktree lifecycle. Whether this reuses Fleet's worktree machinery or is a simpler temporary checkout is an implementation question with a UI consequence: how long "a throwaway copy" takes to prepare, and whether the panel needs a preparing state.

- **[runedit-destructive-command-warning]** Does a destructive Command warn on manual invocation against the working tree?
  The Manifest concept page says your own invocation needs no approval, which is right — but a destructive Command run against the working tree rather than a copy is the one case where the flag still carries information. The destructive flag itself is settled as the one field that is a judgement rather than an observation — Scan can only propose it by matching against Kit's destructive defaults, propose-and-confirm, and Verify cannot check it at all, so a person sets it and this surface's edit screen is the only place it can be set. Also unstated anywhere: where one Manifest flags a Command destructive and another does not across a Convoy, most-restrictive-wins is the obvious answer and no page says it.

- **[runedit-no-manifest-state]** What does this surface show when no Manifest exists for the selected project?
  Setup's territory, and this journey meets Set Up a Project (Manifest) here without either page claiming ownership of the empty state.

## Related

- [Set Up a Project (Manifest)](set-up-a-project-manifest.md) — where a Manifest first comes to exist, and the surface this journey's empty state hands off to.
- [Monitor Active Work](monitor-active-work.md) — Job detail, where the run sheet opens.
