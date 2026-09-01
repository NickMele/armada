# Journey 9 — Run and edit a Manifest

**What it is:** View a project's Manifest, run any single Check or Command against it without dispatching a Job, and edit it.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** "I want to run the lint here without spinning up a drone," or "this project's checks are wrong and I want to fix them."

**Concepts touched:** Bridge (Manifest — a new surface), Manifest, Fleet.

**Milestone:** Reach.

**Drawn against** `docs/contracts/configuration.md`.

New journey, added 2026-08-21 from a gap the other journeys did not cover.

## Why this journey did not exist

Most of it was already sanctioned and had no surface. The Manifest concept page states that Commands are "invoked by both a Drone (during a Job) and **you (directly via Bridge)** — same named registry either way," and that your own manual invocation needs no second approval because you are the one triggering it. It also specifies **Update** as manual editing, and **Verify** as drift detection plus a live dry-run. Three capabilities, no screen.

One part is genuinely new: **running a single Check on demand.** Checks are specified as invoked by Fleet as part of mechanical verification, and the only ad-hoc path was Verify's live dry-run, which runs all of them. Running one is new capability, not a missing screen.

## Bridge gains a seventh surface

Bridge has been specified as six surfaces throughout. Manifest is the seventh, in the rail beside Job Board and Doctor. Consequences to propagate: the Bridge concept page, the Doctor concept page ("one of Bridge's six surfaces," twice), and Iconography's Navigation table, which needs a nav glyph for it. `file-cog` is already assigned to Manifest in git-and-config, so the rail can reuse it.

## Flow

Open Bridge → Manifest → read the current project's Checks and Commands → run one, or edit.

**One Manifest at a time, chosen by the project switcher already in the rail.** Not a workspace list inside the surface, and not every manifest at once — the switcher is how the rest of Bridge scopes itself, and a second selector inside one surface would mean two ways to answer the same question. A monorepo's root `armada.yml` appears as its own entry in the switcher, since its Checks run only when the diff touches root paths and it is not just another workspace.

## Checks and Commands are separate groups

Two groups, not one list with a marker. They are separate registries in the Manifest and they differ in what they mean: a Check gates code and a Command does not. Presenting them as one list with a badge would ask a person to learn a distinction the layout could just state.

## Running one

**It asks where to run, in the run panel, before it starts.** Working tree as it is on disk, or a throwaway copy. Not remembered per command and not hidden behind a split button's caret: a migration and a lint want different answers, and the cost of asking is one click on the screen where you already chose to run something deliberately.

**Output goes to a panel on this surface** — exit code, duration, and the output — and it stays until dismissed. There is no Job, so no Job Board row and no Evidence, and a toast would throw away the thing you ran it to read.

### A passing Check leaves no verdict behind — but the output is a log

Two different things, and the first draft of this journey conflated them.

**The output persists as a log.** Runs write to `./.armada` where logs already go, so the panel is a view onto a file rather than a buffer that evaporates — Dismiss closes the panel, not the log. Retention is a garbage-collection concern and there is already a retention window setting to hang it on.

**What must not persist is a result a workflow can read.** No Evidence, and no stored pass or fail against the Check itself. Two rejected alternatives:

- *Remember the last result per Check with a timestamp* — a remembered verdict is indistinguishable from Evidence at a glance, and the moment it sits beside a Job someone reads it as gating. Copy saying "not evidence" does not survive being scanned; a log you have to open does.
- *It counts for a Job from the same tree* — rejected hardest. This is the path around verification that v1 proved becomes the default path. The tree can change between the run and the dispatch, which is the whole reason Evidence ties a gate's answer to the diff it judged.

**The transcript is keepable, the judgement is not.** This surface is for rehearsing; a Job is where a Check means something.

## Editing

**Forms by default, with the raw file behind a toggle.** Each section — Checks, Commands, allowlist, budget, freeze — as real controls, because those are the parts with known shapes and enumerated values. **The form expresses everything the schema has** — settled 2026-08-21: there is nothing the sections cannot say, so an edit in the file always round-trips back into them and nothing can be dropped. That makes the file view a preference rather than an escape hatch: it is there for people who would rather read the file than the form, not for what the form cannot reach. It also means the schema and the form move together — a field added to one is a field added to the other. The file stays reachable because pretending a committed file is not a file hides what lands in the repo, not because the form falls short.

**The toggle is never labelled with the file format.** The lexicon bans that word in the UI. It is the file, named by its path.

## Verify belongs here

Both halves: **drift detection** (re-scan and flag) and the **live dry-run** (run everything once). Doctor's Manifest module reports *that* a manifest has drifted; this is the surface where you look at what drifted and act on it. Doctor is a scan and changes nothing, so the re-scan cannot live there.

## Verify — does this file actually run, 2026-08-21

**Verify runs setup and every Check once and reports what happened.** That is the whole of it. It exists for the moment a manifest is new or has just been edited, which is when a file that parses cleanly can still be wrong in every command it names.

**Verify and drift are two things, not two halves of one.** An earlier draft of this journey had them in one panel with two verdict groups, which made Verify read as an audit of the file rather than a rehearsal of it. Drift asks whether everything the file names still exists — a read, shown on the surface. Verify runs the file — an act, with its own button, which asks where to run once for the whole set. That correction also resolves the apparent conflict with Set Up a Project: Verify running straight after Write is exactly right, because a brand-new file is precisely the case Verify is for.

**Drift is one question: does everything the file names still exist.** Nothing about what the repo gained. Two more ambitious drafts were cut — a `new` verdict for scripts the repo picked up, then a comparison of command strings against the evidence they were traced from. Both fail on the same fact: the file carries no record of what it was written against, so nothing can say *new* or *changed* without storing a scan. Existence needs no history — the name is in the file, and either the script is there or it is not. And nothing about what those commands *do*: a `test` script narrowed to one directory still exists and drift stays quiet, which is correct — keeping a script and its Check in step is an engineering job, not something Armada should guess at.

**The not-checked band is load-bearing.** Policy and permissions name nothing runnable, so about a third of a mature manifest is outside what Verify can say anything about, and a clean result must not read as "all of this is still right."

Drift detection and the live dry-run are two groups with two verdicts, not one score. They answer different questions: *is this file still true* and *does this file still work*. Drift's two verdicts: `gone` and `current`.

**The dry-run is on demand and asks first.** Settled 2026-08-21. It runs real Checks and a real test suite is expensive, so it is never part of opening this surface and never part of drift. Drift is a read and costs nothing, so it runs on opening; the dry-run is an act, so it is its own button and asks where to run — once for the whole set rather than per Check.

**Neither half can see about a third of the file.** Policy and permissions have nothing in the repo to compare against, and the dry-run does not touch them. The panel states that explicitly rather than letting a clean result read as "all of this is still right."

**Verify reports, it does not fix.** No Apply, no Accept all — the Manifest concept page is explicit that Verify flags without changing the file, and Update is manual editing. Acting on a row means going to Edit, where the consequence is stated. `id` reports and offers nothing at all: it reads as drifted whenever a package is renamed, and changing it orphans every Job that references this Manifest.

**Drift is amber throughout, never red.** A drifted file is behind, not broken — and the dry-run below it proves the difference. Verdicts are words in the status colour, matching Doctor: `new`, `gone`, `diverged`, `current`.

**A dry-run failure is not a Doctor fail.** Settled. Doctor reports service health — things that are up or down, probed on demand and holding no state. A dry-run is something you chose to run, on one project, and its result is a rehearsal like every other run on this surface. Routing it into Doctor would put a stored verdict in a surface that reports live facts, and make one project's broken Check read as a machine-level failure.

## What is already decided and landed

- **The Write-path choice is not made once per monorepo batch, or per workspace — it isn't made at all.** There is no separate Write-path mode for a single workspace or a monorepo batch; Write always commits whatever the proposal has iterated to. Iteration happens per-workspace, at whatever pace suits each one's evidence. (This closes the same question named from Set Up a Project's side.)
- **Save just saves the file.** Settled 2026-08-21: it writes to disk and stops there — no staging, no commit. The file is tracked, so the change appears in the working tree like any other edit and is committed when everything else is. Armada writing to git on your behalf would be a surprise in the one place a person is most sensitive to surprises.
- **Editing during a running Job is safe — no warning, no lock.** Settled 2026-08-21: a Job resolves its config from a snapshot, so it is judged against the Checks it started with and an edit on this surface cannot reach it. Same mechanism that stops a Drone granting itself a Command. The next Job picks up the new file; the running one does not.
- **The snapshot covers Checks as well as Commands, and it is taken at Job creation rather than at spawn.** Settled 2026-08-28, closing the question this page raised. Both halves matter here. *Checks too*, because a Job frozen at a gate would otherwise be judged against a Check that changed under it — the safety this page claims is the safety of the yardstick, not only of the toolbelt. *At Job creation*, because a Drone belongs to a workflow step, so a Job spawns one Drone per step and a snapshot taken at spawn would be re-resolved at every boundary — which would let a Drone weaken a Check in one step and be measured against the weakened one in the next. A step boundary has no person in it. The one re-snapshot is a scope revision, which a person approves. `../concepts/drone.md` carries the rule.
- **Ad-hoc run log retention is a Machine setting, default 30 days.** Settled 2026-08-21. Machine is where per-installation defaults live and this is machine-shaped rather than per-project — the logs are on your disk. 30 days rather than reusing the Job retention window, because the two age out for different reasons: a Job log is kept as long as its history is worth reading, a rehearsal only as long as you might still be debugging what you just ran. The logs live under `./.armada` with their own run directory, one per run, in the same tree as Job logs — one place logs live.

## Open questions

- **[runedit-adhoc-run-location]** Where does an ad-hoc run execute — a real worktree, or a simpler temporary checkout?
  A throwaway copy is a worktree by another name, and Fleet owns worktree lifecycle. Whether this reuses Fleet's worktree machinery or is a simpler temporary checkout is an implementation question with a UI consequence: how long "a throwaway copy" takes to prepare, and whether the panel needs a preparing state.

- **[runedit-destructive-command-warning]** Does a destructive Command warn on manual invocation against the working tree?
  The Manifest concept page says your own invocation needs no approval, which is right — but a destructive Command run against the working tree rather than a copy is the one case where the flag still carries information. The destructive flag itself is settled as the one field that is a judgement rather than an observation — Scan can only propose it by matching against Kit's destructive defaults, propose-and-confirm, and Verify cannot check it at all, so a person sets it and this surface's edit screen is the only place it can be set. Also unstated anywhere: where one Manifest flags a Command destructive and another does not across a Convoy, most-restrictive-wins is the obvious answer and no page says it.

- **[runedit-no-manifest-state]** What does this surface show when no Manifest exists for the selected project?
  Setup's territory, and this journey meets Set Up a Project (Manifest) here without either page claiming ownership of the empty state.

## Related

Set Up a Project (Manifest) — where a Manifest first comes to exist, and the surface this journey's empty state has to hand off to.
