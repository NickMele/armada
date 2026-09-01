# Set Up a Project (Manifest)

**What it is:** "I'm onboarding a new repo" — turn an unconfigured codebase into a working Manifest Armada can dispatch Jobs against.

Design fidelity: full on Locate, the picker, the proposal sheet and Verify. Analysis: Complete. UI/UX design: In progress.

---


**Trigger:** You point Armada at a repo or workspace it has not seen before.

**Concepts touched:** Manifest, Kit, Job Board.

**Milestone:** Reach. The one journey whose UI is built in the same phase as its logic, so a decision here has an immediate build consequence rather than a deferred one.

**Drawn against** `docs/contracts/configuration.md` and `docs/contracts/design-system.md`, and against the Manifest schema — including `ports`, which is a top-level registry.

**The rule this journey is built on:** every line in a proposal traces to something a file already said, and where it does not, it says so. Two of the screens look the way they do because of it.

## Flow

An accent edge marks a step that asks you something. Everything else reports.

| Step | You | Armada |
| --- | --- | --- |
| Locate | Give a path, or clone | Nothing yet |
| Scan | Wait | Reads files, writes nothing |
| Pick | Tick, then open one | Ticks by evidence strength |
| Proposal | Sort Checks, add ports | Opens over the picker |
| Write | Commit it | Writes one file per workspace |
| Verify | Approve the run | Runs setup and every Check once |
| Fix | Correct the failing line | Re-runs that Check, then back to Verify |

Fix exists only when a Check fails. Once every Manifest is written, each workspace appears independently scoped on the Job Board, ready for dispatch.

## Getting in

**Locate is a dialog on an empty Manifest rail.** It exists because the flow otherwise assumed Armada already knew which repo it was looking at.

**The field is labelled `Project location`.** No Wh- opener on a label, per the design system contract — a panel heading may ask a question, a label names the thing.

**Choose a folder, or clone from a URL.** Reading a folder and fetching a repo are different acts, and a folder picker alone would be defensible, since cloning is git's job. Clone is here because the trigger is onboarding a repo you have not worked in yet, and sending a person to a terminal and back is the seam this journey exists to remove. It is the quieter of the two, because most onboarding is of code already on disk.

**Scan runs read-only and finds every workspace in one pass** — lockfiles, package scripts, CI config, docker-compose services, workspace globs. Monorepo-aware, not run separately per workspace.

**The picker replaces Locate once Scan has run, and is the surface you stand on for the rest of the journey.** Everything else opens over it. It carries the workspace list and, below it, the batch-level check-names grid.

### The picker

**Each row carries two controls doing different jobs.** A tick decides whether the workspace is in the batch you are setting up now; Open brings up its proposal to work on. Both are needed, because deciding to onboard something and deciding what it should check are different questions, and a person almost always wants to look before committing to the second.

**The batch is not all or nothing.** Scope it to what you want and come back for the rest.

**Ticked by evidence strength.** Strong evidence ticked, thin evidence not, so the default carries information rather than convenience.

**The state column reports rather than instructs** — `ready to write`, `open, being edited`, `no checks proposed`. No hue on any of them: none is a Job state. `no checks proposed` is the useful signal; a workspace whose scripts include nothing conventional will produce a manifest that gates nothing, and the picker is where that is noticeable.

**The open row takes the accent edge and tint**, the pair that means *the row you are on* everywhere else. Legitimate here, because the panel really is showing that row.

**Setup is not a wizard.** You open the proposals that need attention, in any order, and close back to the picker. There is no pagination framing.

## Check names across the batch

A grid of Check names against the batch's workspaces, below the workspace list — the one place the per-workspace sheet cannot look.

**Common Checks are cross-workspace by nature.** Almost every workspace has a `test`. A workflow step names `test` once and it runs wherever it is declared, so the names are a shared vocabulary whether or not anyone agreed one.

**A missing name reads as a pass.** A step naming `lint` on a workspace that declares none records *did not run*, and nothing on a green result says a workspace was skipped. A gap is a fact about the set rather than about any one file, which is why only the batch can show it.

**Marked narrowly: a name every sibling declares and this one does not.** A dash is not a warning — `typecheck` legitimately absent from some workspaces is not a gap, and marking every absence would make the grid mostly amber and mean nothing.

**Reported, never filled in.** Adding a `lint` Check needs a command only you know, so the grid names the gap and offers to take you to the workspace.

**The marked cell takes `--step-stopped-bg` and the word `missing`.** No hue on the word and no chip: a workspace lacking a Check name is not a Job state. Cells carry ticks rather than words, because each answers yes or no and a column of `declared` would be unreadable — the same mark the picker uses, where a tick also means *this one is in*.

## The proposal sheet

A sheet over the picker, one workspace at a time, opened from a row and closed back to it. The registries, the forms that edit them, and policy are all inside it.

**You iterate here.** Edit it yourself or work it through with an agent, as many rounds as you like. Write comes after and commits what you arrived at, after no rounds or several.

**Identity is the header, not a row.** The id is what the file *is* — the name every Job is keyed to, fixed once written. A registry row would imply one entry among many sharing the same actions, and it has none.

**Ports are declared first, above the lists that read them**, so you meet a variable before you meet a command using it.

**Row actions are ghost icons.** No fills — a sheet of rows each offering a filled control would be as many competing decisions, and the accent belongs to Write.

**Every row is editable.** Edit or delete any row, and add one at the end of each band, so the whole file is reachable without leaving the proposal.

### Provenance

**Every line cites its source, and the typeface is the citation.** Mono for a file Scan read. Sans for a guess, an edit, or an addition. It is the visible form of the evidence rule, and every Setup screen needs it.

**Placement is a guess; ports are not.** A compose service proves a port exists. Nothing proves a script gates code, so which group Scan put a script in is a guess, and those rows read `convention`.

**Provenance moves on edit.** A hand-written or hand-corrected line reads `edited during setup` or `added during setup` in sans where a filename would sit in mono. Editing in place makes that column do more work, not less: it is the only thing distinguishing a line Scan found from one you typed over.

**Provenance is not editable.** It is a record, not a value.

### Checks and Commands are two registries, never one ticked list

**A Check verifies the work is right and gates code. A Command is a script the workspace uses and gates nothing.** An earlier draft collapsed them into one list with a tick column, which made every Command read as a declined Check.

**Every row carries a move action instead of a tick.** Scan cannot know which group a script belongs in, so moving has to be as cheap as ticking was. Setup rows carry no move — a setup entry is not a Check or a Command, it names one.

### A Check may require Commands to run first

**`checks.<name>.requires` is an ordered list of Command names, never command strings**, so what runs is written in exactly one place. Found by drawing: an e2e Check needs `migrate` and `seed` to have run before it means anything, and the schema had no way to say so — `setup.requires` is per worktree and runs once, which cannot express per-Check ordering.

**A prerequisite may name a root Command**, the same as `setup.requires`.

**A prerequisite already run is skipped, scoped to the execution context rather than the worktree.** Checks share the worktree today, so two Checks naming `migrate` run it once; a Check running in its own container is a different context and runs it again, since the effect is not there to inherit. Keying the rule on context means per-container Checks need no exception later. That makes `requires` a guarantee of *has run*, not *has just run* — a Check needing fresh state resets it in its own command.

**The Check row shows its prerequisites in the same cell as its command, in order** (`migrate, seed → pnpm playwright test`). A separate column would let a reader scan the list and miss that one row does several things. The row states the skip rather than leaving a reader to wonder what a second Check re-ran.

**A Check that fails because its prerequisite failed reports the prerequisite, not the Check.**

**This is the second reason Commands exist.** A Command earns its place by being something a Check depends on, not only something you run by hand.

### Policy

**A Policy band sits in the proposal with the Kit and Machine values shown and editable** — `auto_merge`, `review_gate`, `budget`, `dispatch_freeze`. Nothing in a repo corresponds to any of them, so every row reads *from Kit* or *from Machine* in sans rather than citing a file: the one band where absent evidence is the normal case.

**Shown rather than applied silently**, unlike Skills and the allowlist, because these are decisions with consequences at merge time rather than defaults.

**Every policy value comes from a fixed set, so the control shows all options at once.** No text fields — a command string is free text and gets an input, a policy value never is.

**Each control carries a plain-words line naming the consequence of the selected value**, because `tests-pass` means nothing on its own.

**A row reads *from Kit* until touched and *set here* after**, which is the whole inheritance story on this surface.

**An inherited value is an absent key.** Leaving a row at Kit's value writes nothing and the project keeps following Kit if Kit later changes; writing the same value explicitly pins it. Two identical-looking rows with two different futures, which is why the source column is not decoration.

**Budget is the only row that can be wrong rather than merely different** — a cap below one Job's cost makes the workspace undispatchable. A warning for that is not drawn.

**Inheritance is applied silently and stated once, without controls.** One line names what came from Kit and is not written to this file. A default should be quiet, but a quiet default still has to be findable: a person who later hits an allowlist they never saw needs somewhere to have seen it, and that line is the somewhere. The Manifest surface is where it is changed.

### The proposal conversation happens in Helm

**The button reads `Open in Helm`.** Helm is the conversational surface, and a second one on this screen would be a second Helm. The session is scoped to this proposal and returns row edits, which land with their provenance changed to reflect that a person decided them. What the Helm side looks like belongs to [Consult Helm](consult-helm.md).

## Editing in place

**The values are the fields.** At rest an editable value is text with a hairline dashed rule under it; a sheet of bordered inputs would read as a form, and this reads as a file you can type into.

**The field appears in place, at the same height**, so no row moves and nothing below it shifts. The row tints while you are in it.

**No pencil.** If the value is editable where it sits, a row action opening a form to edit that same value is a second door to one room. The row menu is the residue of the pencil and holds only what a cell cannot: if a value can be a field in the row, it is one.

**Three kinds of value, three treatments, one row height.**

| Value | Treatment |
| --- | --- |
| Free text | Dashed rule, becomes an input |
| Enumerated | Options shown inline |
| A set | Row menu, opening a list anchored to the value |

An enumerated value shows its options inline because a segmented control at 24px costs the same room as the word it replaces and removes a click. A set cannot be typed into a cell without inventing a syntax.

**A name is not editable.** Renaming `test` breaks every workflow step naming it, so it is a deliberate act elsewhere.

**Save is one action at the foot of the sheet, writing the whole file.** Per-cell saving would make a half-edited manifest committable; the whole file lands in one commit or not at all.

### Three sets, one popover

Prerequisites, the destructive flag, and inserting a port variable are the only values on the sheet a cell cannot hold. All three open the same way — click the value, pick from a list anchored to it — so there is one interaction to learn rather than a form per section.

**The prerequisites popover carries ticks for membership and a number plus grip for order.** Ticking says *which*, the number and the grip say *in what order*; two Commands where one seeds what the other migrated are not interchangeable.

**Only Commands appear in it.** A Check cannot be another Check's prerequisite, because a gate that runs to set up a gate is no longer a gate.

**The destructive flag is the degenerate case** — a set of one, so its popover is a single switch with the sentence naming what it changes.

**Port variables are the exception that stays inline.** Typing `$` in a command cell completes from the declared ports, because you are already typing there and a picker would interrupt.

**The popover is the only floating layer on the sheet, so it takes `--shadow-overlay`.** The row beneath it keeps the accent tint that marks where you are, which stops the popover reading as detached from the row that opened it.

**The standalone Check editor was drawn and then cut** by editing moving into the cells.

## Ports

**A port is declared once and read by every Check and Command below it.** Armada leases a free port per worktree and passes it in, so two Jobs can run the same Check at once without colliding. The row states the container port and the variable it arrives as.

**Scan finds a port only where a file declares one, and finding none is the common case.** Plenty of projects hard-code it in source, which Scan does not read. The empty band teaches: one line on what a port entry buys you, and the control to add one. No illustration. The sentence names the benefit before the mechanism, because a person who has not hit a port collision does not know they are being protected from one.

### Adding a port

**Two fields are asked and a third is filled in.** Service name and container port are both things you know; the variable name is derived from the name and stays editable, so it can match what your code already reads.

**Derive only when there is nothing to inherit.** A detected port keeps its file's variable name — renaming it would break every command already reading it. Derivation is the starting value, not the rule.

**A colliding variable name warns in the field, not elsewhere.** Two manifests can both map a port to `PORT`, which only breaks when one Job gates both workspaces. The line appears under the variable field naming the other workspace and the condition. Setup is the one moment renaming is free, and the field is where you are looking. Two batch-level grids were drawn and cut: a warning belongs where the name is being typed, not on a separate surface a person has to find.

**The variable field carries the one sentence that prevents the most likely mistake** — writing a literal host and port into a Check and wondering why it fails. It names both ways a Check may use the port: set in the environment, where code reads it off the process environment, or written into the command for a tool that takes a flag and ignores the environment.

**A compose service needs no variable.** Armada rewrites the document, so a container gets its port without the file mentioning one. The variable is for commands that run outside a container.

**A port added by hand lands in the same row shape as a detected one**, with `added during setup` in sans where a filename would sit in mono.

### What else is worth detecting

Cheap and worth doing, because each is a file and so each is evidence: a Dockerfile `EXPOSE`, a `PORT` or `*_PORT` line in `.env.example`, a `--port` flag inside a package script, a `devServer.port` in a bundler config.

**A port hard-coded in application source stays out of reach.** Reading source to infer intent is the line the evidence rule draws, and a wrong guess here writes a variable your commands then fail to use. Better to ask.

## Compose, and what Armada refuses

**Nothing is written to your compose file.** Armada resolves it, rewrites every published port in memory, and feeds it on stdin. The file on disk is untouched.

**Two numbers, one of them in the file.** The container port is fixed and declared; the published port is leased per worktree and appears in no manifest.

**An undeclared published port is refused, not warned about.** A port Armada did not place would collide silently, so the run stops with a config error and a one-line fix. The refused row takes `--step-stopped-bg` and reads *would stop the run* — future tense, because nothing has run yet and this is a file being written. Naming the fix in the same row is the point: a refusal that does not say what to add is a wall.

**Setup is where that error is cheap.** Finding it at dispatch means a Job failing on something a scan already knew.

**Refusal belongs to the compose case because that one is structural.** The Docker adapter parses the document, so Armada knows what it found. A command string is opaque, and refusing there would be Armada claiming to understand something it only pattern-matched.

**An undeclared port variable in a command is warned about, never refused.** A `$NAME_PORT` matching no declared port gets an amber line under the command field naming the miss and the nearest declared port, with Save still enabled. Anything else in the command passes without comment. A warning claims exactly as much as it knows.

## After Write

**Verify runs on the sheet that wrote the file, straight after Write, gated behind your approval per Manifest.** Setup and every Check, run once.

**A failing Check here means the file is wrong, not the code** — the distinction a person needs at minute one of onboarding.

**Verify is a rehearsal, not an audit.** Drift asks whether everything a file names still exists, and there is nothing to have drifted from on a file written a second ago, so drift has no meaning here; it belongs to [Run and edit a Manifest](run-and-edit-a-manifest.md). This is also the moment the journey pays off or does not: a proposal that parses cleanly and fails every command has told you nothing until this step runs.

### Fixing a failed Verify

**The failing Check's command is a cell like any other, corrected where it failed.** Leaving Setup at minute one to fix a file Setup just wrote is a seam. The Manifest edit form is mounted inside Setup rather than linked to — one component, two mounts.

**Nothing opens.** One command string is wrong and it is already on screen in the row that failed, so the error output sits beneath it. That is the one thing worth adding to a row.

**It opens at the failing row only.** A failing Check means one command string is wrong, and the rest of the manifest is not what you are there to reconsider.

**Re-verify is scoped to what failed** — `Verify typecheck`, not `Verify`. The passing Checks cost real time and nothing you edited touched them.

**The failing row takes `--step-stopped-bg`**, the one place step activity colours a row rather than a glyph: a Check that stopped the flow is a dead stop, not a retry and not a wait.

**`Open in Manifest` stays as a ghost** for the case where the fix is bigger than one string, so the deep edit has a door without being the default.

## Batch behaviour for monorepos

| Stage | Behaviour |
| --- | --- |
| Scan | Every workspace in one pass, not run per workspace |
| Proposal | One draft per workspace, opened over the picker in any order |
| Write | Commits whatever each proposal was iterated to |
| Verify | The live dry-run, per Manifest, on your approval |

## Settled here

- **A guess says it is a guess.** A script Scan placed by convention carries `convention` in the provenance column, so moving or keeping it makes the choice yours without the file pretending it was ever evidence.
- **The picker ticks by evidence**, so the default carries information.
- **The picker is the surface**, and every proposal opens over it.
- **Two registries, never one ticked list**, with a move action on every row.
- **Inheritance is silent but stated once.** One line, no controls, changed on the Manifest surface.
- **Values are the fields.** No pencil, no per-section form, and a row menu only where a set exists.
- **Adding a port asks the name and the container port and fills in the variable**, which stays editable.
- **Verify here is the dry-run only.**
- **A failed Verify is fixed in the row**, with re-verify scoped to that Check.
- **There is no Write-path mode.** An earlier flow offered a three-way choice — mechanical plus manual, agent-assisted, evidence-only printout — made up front for a workspace or for a whole batch. The iteration moved into Proposal, so Write writes whatever you arrived at.

## What this journey needs built

| Component | Note |
| --- | --- |
| Value popover | Prerequisites, the destructive flag, a port insert — one component |
| Compose service list | Which published ports are rewritten, and which are refused |
| Port add form | Wants a sheet row; the Manifest surface needs the same form |
| Provenance column | Wants a sheet row; every Setup screen needs it |
| Checkbox row | Tick, name, detail, state — for the picker and the grid |

**The checkbox row is not for the proposal**, whose rows lost their ticks when Checks and Commands became separate registries and carry move, edit and delete instead.

**The port add form's starting-value rule must match in both mounts** — uppercase, `_PORT` suffix — or the same port gets two names.

## What is undrawn

- **The compose refusals beyond an undeclared published port** — two services sharing a container-side port, an entry Armada cannot parse, and a container-side range. They are one class of error but differ in the fix, which is the part that matters, so one shared error row will not do.
- **A compose override that appends rather than replaces.** Armada handles it in memory, so there may be nothing to show. Worth confirming rather than assuming.
- **A warning for a budget cap below one Job's cost.**

## Open questions

- **[setup-batch-at-scale]** How does a batch of twenty proposals read?
  Scan finds every workspace in one pass and the picker holds them all; a real monorepo could be twenty, and one list stops being a screen at some size.

This journey also shares `drone-secrets-handling` with [Guild Setup & Configuration](guild-setup-and-configuration.md), where the full item is recorded: how secrets are handled for Drones, given the established principle that a Drone never holds secrets directly and Fleet brokers, with the concrete mechanism — including unattended 1Password auth — still unresolved.

## Related

[Run and edit a Manifest](run-and-edit-a-manifest.md) for drift detection once a Manifest already exists · [Consult Helm](consult-helm.md) for the proposal conversation · [First-Run Onboarding](first-run-onboarding.md), which chains this journey behind Guild Setup.
