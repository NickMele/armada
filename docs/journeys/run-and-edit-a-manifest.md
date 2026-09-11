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

## Bridge gains a seventh surface

> **Rule.** Manifest is Bridge's seventh surface, in the rail beside Job Board and Doctor.
> Why: Bridge has been specified as six surfaces throughout, and Manifest is the addition.

Consequences propagated from this: the Bridge concept page, the Doctor concept page (which named "one of Bridge's six surfaces" twice), and Iconography's Navigation table, which needed a nav glyph for it. `file-cog` was already assigned to Manifest under Git and config, so the rail reuses it.

## Flow

Open Bridge → Manifest → read the current project's Checks and Commands → run one, or edit.

> **Rule.** One Manifest is open at a time, chosen by the project switcher already in the rail.
> Why: the switcher is how the rest of Bridge scopes itself, and a second selector inside the surface would answer the same question twice.

A monorepo's root `armada.yml` appears as its own entry in the switcher, since its Checks run only when the diff touches root paths and it is not just another workspace.

## Checks and Commands are separate groups

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

> **Rule.** Runs write their output to `./.armada`, where logs already go, so the panel is a view onto a file rather than a buffer that evaporates; Dismiss closes the panel, not the log.
> Why: retention is a garbage-collection concern, and there is already a retention window setting to hang it on.

> **Rule.** No run from this surface writes Evidence, and no Check gets a stored pass or fail against it.
> Why: a remembered per-Check verdict reads as Evidence the moment it sits beside a Job, and no copy saying "not evidence" survives being scanned; the tree can also change between the run and a dispatch, which is the whole reason Evidence ties a gate's answer to the diff it judged.

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
| Checks | Every Check, in declaration order | Its `run` line, and a sentence where `when` skips it for this Job |
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

The schema and the form move together — a field added to one is a field added to the other. The file stays reachable because pretending a committed file is not a file hides what lands in the repo, not because the form falls short.

> **Rule.** The toggle is never labelled with the file's format; it is named by its path.
> Why: the lexicon bans naming a Manifest by its format — it is a Manifest, never "the yaml."

> **Rule.** Editing during a running Job needs no warning and no lock.
> Why: a Job resolves its config from a snapshot, so it is judged against the Checks it started with and an edit on this surface cannot reach it — the same mechanism that stops a Drone granting itself a Command. The next Job picks up the new file; the running one does not.

> **Rule.** Save writes the file to disk and stops there — no staging, no commit.
> Why: the file is tracked, so the change appears in the working tree like any other edit and is committed when everything else is; Armada writing to git on your behalf would be a surprise in the one place a person is most sensitive to surprises.

> **Rule.** Write always commits whatever the proposal has iterated to — there is no separate Write-path mode for a single workspace, or for a monorepo batch.
> Why: iteration happens per workspace, at whatever pace suits each one's evidence, so there is nothing left for a Write-path mode to choose between.

> **Rule.** The snapshot a Job freezes at creation covers Checks as well as Commands.
> Why: a Job frozen at a gate would otherwise be judged against a Check that changed under it — the safety this claims is the safety of the yardstick, not only of the toolbelt.

> **Rule.** The snapshot is taken at Job creation, not at spawn.
> Why: a Job spawns one Drone per step, so a snapshot taken at spawn would be re-resolved at every step boundary — which has no person in it — and could let a Drone weaken a Check in one step and be measured against the weakened one in the next. The one re-snapshot is a scope revision, which a person approves. `../concepts/drone.md` carries the rule.

## Verify belongs here

Both halves belong here: **drift detection** (re-scan and flag) and the **live dry-run** (run everything once).

> **Rule.** This is the surface where you look at what drifted and act on it, not Doctor.
> Why: Doctor's Manifest module only reports *that* a manifest has drifted, and Doctor is a scan that changes nothing, so the re-scan cannot live there.

## Verify runs the file

> **Rule.** Verify runs setup and every Check once, and reports what happened — nothing more.
> Why: it exists for the moment a manifest is new or has just been edited, which is when a file that parses cleanly can still be wrong in every command it names.

> **Rule.** Verify and drift are two separate things, not two halves of one panel.
> Why: one panel with two verdict groups reads as an audit of the file rather than a rehearsal of it.

> **Rule.** Drift is a read — whether everything the file names still exists — and runs on opening, at no cost.

> **Rule.** Verify is an act — it runs the file, behind its own button, and asks where to run once for the whole set.

Running Verify straight after Write, as Set Up a Project (Manifest) does, is correct: a brand-new file is precisely the case Verify is for.

> **Rule.** Drift never reports a script the repo picked up that the file does not yet name.
> Why: the file carries no record of what it was written against, so nothing can say *new* or *changed* without storing a scan — existence needs no history, since the name is in the file and either the script is there or it is not.

> **Rule.** Drift says nothing about whether a listed command still does the right thing — a `test` script narrowed to one directory still reads as existing, and drift stays quiet.
> Why: keeping a script and its Check in step is an engineering job, not something Armada should guess at.

> **Rule.** The not-checked band is load-bearing: policy and permissions name nothing runnable, so a real portion of a mature manifest is outside what Verify can say anything about, and a clean result must not read as "all of this is still right."

> **Rule.** Drift detection and the live dry-run are two groups with two verdicts, not one score, answering different questions — *is this file still true*, and *does this file still work*. Drift's two verdicts: `gone` and `current`.

> **Rule.** The dry-run never runs automatically — not on opening the surface, and not as part of drift — and always asks where to run, once for the whole set rather than per Check.
> Why: it runs real Checks, and a real test suite is expensive; drift is a read and costs nothing, so it runs on opening.

> **Rule.** Verify reports, it does not fix — no Apply, no Accept all — and acting on a row means going to Edit, where the consequence is stated.
> Why: the Manifest concept page is explicit that Verify flags without changing the file, and Update is manual editing.

> **Rule.** A drifted `id` row reports and offers nothing at all.
> Why: `id` reads as drifted whenever a package is renamed, and changing it orphans every Job that references this Manifest.

> **Rule.** Drift is amber throughout, never red.
> Why: a drifted file is behind, not broken — and the dry-run below it proves the difference.

Verdicts render as words in the status colour, matching Doctor: `new`, `gone`, `diverged`, `current`.

> **Rule.** Neither half can see about a third of the file: policy and permissions have nothing in the repo to compare against, and the dry-run does not touch them.
> Why: the panel states that explicitly, rather than letting a clean result read as "all of this is still right."

> **Rule.** A dry-run failure is not a Doctor fail.
> Why: Doctor reports service health — things that are up or down, probed on demand and holding no state — while a dry-run is something you chose to run, on one project, and its result is a rehearsal like every other run on this surface. Routing it into Doctor would put a stored verdict in a surface that reports live facts, and make one project's broken Check read as a machine-level failure.

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
