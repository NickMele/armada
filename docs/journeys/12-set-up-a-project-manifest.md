# Journey 12 — Set Up a Project (Manifest)

**What it is:** "I'm onboarding a new repo" — turn an unconfigured codebase into a working Manifest Armada can dispatch Jobs against.

Design fidelity: not set. Analysis: Complete. UI/UX design: Not started.

---

*Numbering note: the design project has not yet drawn this journey and names no `Journey N - ...` file for it. Numbered here only to give the file set a stable order — see the note on Guild Setup & Configuration for how the sequence after Journey 9 was assigned.*

**Trigger:** You point Armada at a repo/workspace it hasn't seen before.

**Concepts touched:** Manifest, Kit, Job Board.

**Milestone:** Reach. Design note: the only journey whose UI is built in the same phase as its logic — a design decision here has an immediate build consequence rather than a deferred one. Needed a Locate phase added before Scan: the original design assumed Armada already knew which repo it was looking at.

## Flow

1. **Locate** — point Armada at the repo. Missing from the original flow, which assumed Armada already knew which repo it was looking at.
2. **Scan** runs — read-only, discovers **every workspace in one pass** (monorepo-aware): lockfiles, package scripts, CI config, docker-compose services, workspace globs.
3. **Proposal** — one candidate `armada.yml` per discovered workspace, presented together as a batch, each strictly evidence-backed (no inferred/guessed lines). **This is the iterable stage**: you edit it yourself and/or work it through with an agent, for as many rounds as it takes.
4. **Write** — commits the current state of the proposal. **Not a mode chosen in advance.** Corrected 2026-08-21: this step used to offer a three-way Write-path choice (mechanical+manual / agent-assisted / evidence-only printout), which the Manifest concept page has since replaced — there is no separate Write-path decision made up front, for a single workspace or for a whole monorepo batch. The iteration moved into Proposal, so Write writes whatever you arrived at, after zero rounds or several.
5. **Verify runs after Write, gated behind your approval** — not silent, not a separate action you have to find later. **Verify is the live dry-run: setup and every Check, run once, to confirm the file actually works.** That is the whole of it — a brand-new file that parses cleanly can still be wrong in every command it names, which is exactly what this step catches. Drift detection is a *different* thing and belongs to Run and edit a Manifest, not here: there is nothing to have drifted from on a file written a second ago.
6. Manifest(s) now exist — each workspace appears independently scoped on Job Board, ready for dispatch.

## Batch Behavior for Monorepos

| Stage | Behavior |
| --- | --- |
| Scan | Discovers every workspace in the repo in one pass — not run separately per workspace |
| Proposal | One Manifest draft per workspace, shown together as a batch, not sequentially one at a time |
| Write | Commits whatever each proposal has been iterated to. No mode chosen up front — see Flow step 4 |
| Verify | The live dry-run, per Manifest as each is written, gated behind your approval per Manifest |

## Three items closed 2026-08-22

**Setup asks for policy.** A Policy band sits in the proposal — `auto_merge`, `review_gate`, `budget`, `dispatch_freeze` — with Kit's values shown and editable. Nothing in a repo corresponds to any of them, so every row reads *from Kit* in sans rather than citing a file: the one band where absent evidence is the normal case. Shown rather than applied silently, unlike Skills and the allowlist, because these are decisions with consequences at merge time rather than defaults.

**Editing happens in the cells — settled 2026-08-22.** No pencil, no per-section form. A value you can change is text with a hairline dashed rule under it; clicking turns it into a field at the same height, so no row moves. Three treatments, one row height: free text becomes an input, an enumerated value shows its options inline (a 24px segmented control costs the same room as the word it replaces), and a **set** opens a list anchored to the value. Not editable: a name, since renaming `test` breaks every workflow step naming it, and provenance, which is a record rather than a value. Save is unchanged — one action at the foot of the sheet; per-cell saving would make a half-edited manifest committable.

**Three sets, one popover.** Prerequisites, the destructive flag, and inserting a port variable are the only values a cell cannot hold. All three open the same way — click the value, pick from a list anchored to it — so there is one interaction to learn rather than a form per section. The prerequisites popover carries ticks for membership and a number plus grip for order, because two Commands where one seeds what the other migrated are not interchangeable. **Only Commands appear in it:** a Check cannot be another Check's prerequisite, since a gate that runs to set up a gate is no longer a gate. Port variables are the exception that stays inline — typing `$` in a command cell completes from the declared ports, because you are already typing there. The standalone Check editor was drawn and then cut by this decision.

**Editing policy is a choice among enumerated values, drawn 2026-08-22.** Every policy value comes from a fixed set, so the control shows all options at once — no text fields; a command string is free text and gets an input, a policy value never is. Three rules the drawing settles: each control carries a plain-words line naming the *consequence* of the selected value, because `tests-pass` means nothing on its own; a row reads *from Kit* until touched and *set here* after, which is the whole inheritance story on this surface; and **an inherited value is an absent key**, so leaving a row at Kit's value writes nothing and the project keeps following Kit if Kit later changes, while writing the same value explicitly silently pins it — two identical-looking rows with two different futures, which is why the source column is not decoration. Budget is the only row that can be *wrong* rather than merely different (a cap below one job's cost makes the workspace undispatchable); a warning for that is not drawn.

**The proposal conversation happens in Helm.** The button reads **Open in Helm**, not a panel on this screen — Helm is the conversational surface, and a second one here would be a second Helm. The session is scoped to this proposal and returns row edits, which land with their provenance changed to reflect that a person decided them. What the Helm side looks like belongs to Consult Helm, not this journey.

**Check names are shown as a batch grid.** Common Checks are cross-workspace by nature — almost every workspace has a `test` — so the names are a shared vocabulary whether or not anyone agreed one. A step naming `lint` on a workspace that declares none records *did not run* and renders as a clean pass, and a gap is a fact about the *set*, so the per-workspace sheet cannot show it. A grid of names against the batch's workspaces does. **Marked narrowly:** only where every sibling declares a name and one does not — a dash is not a warning, since `typecheck` legitimately absent from two workspaces is not a gap, and marking every absence would make the grid mostly amber and mean nothing. Reported, never filled in: adding a `lint` Check needs a command only the engineer knows.

**A Check may require Commands to run first — found by drawing, 2026-08-22.** An e2e Check needs `migrate` and `seed` to have run before it means anything. The schema had no way to say that: `setup.requires` is per worktree and runs once, which cannot express per-Check ordering, and nothing else in the file relates a Check to a Command. Added as `checks.<name>.requires` — an ordered list of **Command names**, never command strings, so what runs is written in exactly one place. Two consequences for the UI: a Check row shows its prerequisites in the same cell as its command, in order (`migrate, seed → pnpm playwright test`), because a separate column would let a reader scan the list and miss that two rows do four things; and a Check that fails because its prerequisite failed reports the prerequisite, not the Check. This also gives the Commands registry a second reason to exist — a Command earns its place by being something a Check depends on, not only something you run by hand. **Two follow-ups settled Aug 22 2026:** a prerequisite may name a root Command, the same as `setup.requires`; and one already run is skipped — **scoped to the execution context, not the worktree**. Checks share the worktree today, so two Checks naming `migrate` run it once. A Check running in its own container is a different context and its prerequisite runs again, since the effect is not there to inherit; keying the rule on context means per-container Checks need no exception later. That makes `requires` a guarantee of *has run*, not *has just run* — a Check needing fresh state resets it in its own command. The proposal row states the skip rather than leaving a reader to wonder what a second Check re-ran.

**The picker is the surface; proposals open over it — settled 2026-08-22.** The workspace list is where you stand for the whole journey, and each row carries two controls doing different jobs: a **tick** deciding whether the workspace is in this batch, and **Open** bringing up its proposal as a sheet. Both are needed because deciding to onboard something and deciding what it should check are different questions, and a person almost always wants to look before committing to the second. Consequences: Setup is not a wizard marching through four proposals in order — you open the ones needing attention, in any order, and close back to the picker; the "1 of 4" pagination framing is gone; and each row carries a state (`ready to write`, `open, being edited`, `no checks proposed`) with no hue, since none is a Job state. `no checks proposed` is the useful one — a workspace whose scripts include nothing conventional will produce a manifest that gates nothing, and the picker is where that is noticeable.

**Checks and Commands are two registries on the proposal, never one ticked list — settled 2026-08-22.** A Check verifies the work is right and gates code; a Command is a script the workspace uses and gates nothing. An earlier draft collapsed them into one list with a tick column, which made every Command read as a declined Check. Each row instead carries a **move** action beside edit and delete, because Scan cannot know which group a script belongs in and moving has to be as cheap as ticking was. Setup rows carry no move — a setup entry is not a Check or a Command, it names one.

**An undeclared port variable in a command is warned about, never refused — settled 2026-08-22.** A `$NAME_PORT` matching no declared port gets a line under the command field ("No port named WEB_PROT in this manifest. Did you mean $WEB_PORT?"), amber, with Save still enabled. Anything else in the command passes without comment. Refusal belongs to the compose case because that one is structural — the Docker adapter parses the document, so Armada knows what it found; a command string is opaque, and refusing there would be Armada claiming to understand something it only pattern-matched. A warning claims exactly as much as it knows.

**A failed Verify is fixed in place — settled 2026-08-22.** The Manifest edit form is mounted inside Setup rather than linked to: one component, two mounts, which is reuse and not a second rendering. Three details: it opens at the *failing row* only, since a failing Check means one command string is wrong and the rest of the manifest is not what you are there to reconsider; re-verify is scoped to what failed ("Verify typecheck", not "Verify"), because the passing Checks cost real time and nothing you edited touched them; and a hand-edited line's provenance changes to *edited during setup*, since it no longer traces to a file and should not keep claiming evidence it lost. "Open in Manifest" stays as a ghost for a fix bigger than one string. The failing row takes `--step-stopped-bg` — a Check that stopped the flow is a dead stop, not a retry and not a wait.

## What is already decided and landed

- **The Write-path choice is not made once per monorepo batch, or per workspace — it isn't made at all.** There is no separate Write-path mode. See "Flow" step 4 above.

## Open questions

- **[kit-inheritance-review-at-setup]** Is Kit inheritance surfaced for review during Manifest Setup, or applied silently?
  A new Manifest inherits Skills, sub-agents, allowlist rules, MCP config, and Commands from Kit. Whether Setup shows you that inheritance or applies it silently is undecided. The case for surfacing: the allowlist is the one that matters, since a Kit-level rule set up months ago now applies to a repo you have just onboarded, and silent inheritance means the first time you learn about it is when a Drone does something you did not expect. The case against: Setup already asks two orientation questions and presents a per-line traceable proposal, and adding an inheritance review makes a five-minute onboarding longer when the defaults are usually right — which is the whole point of having Kit defaults. A middle option: show inherited config as a collapsed summary with a count, expandable but not blocking.

- **[setup-checks-vs-scripts]** How does Setup ask which discovered scripts are Checks?
  Open, and forced. The schema pass is explicit that Scan **cannot** evidence whether a script gates code, and that a convention-based guess (`test`/`lint` → Checks) is inference in the exact sense the evidence rule forbids. So Setup has to ask — the shape of that question is undesigned.

- **[setup-locate-phase-shape]** What does the Locate phase look like?
  A path, a clone URL, a folder picker, or a list of what Armada has seen before — the design note flags the phase as missing from the original design, not its shape.

- **[setup-batch-at-scale]** How does a batch of twenty proposals read?
  Scan finds every workspace in one pass and proposals are shown together; a real monorepo could be twenty, and "together" stops being a screen at some size.

- **[drone-secrets-handling]** How are secrets handled for Drones?
  Shared with Guild Setup & Configuration, where the full item is recorded. Established principle: a Drone never holds secrets directly — Fleet brokers. Unresolved is the concrete mechanism, including unattended 1Password auth.

## Related

Manifest (full Setup/Verify/Update mechanics) · Kit (Skills/Agents/allowlist defaults a new Manifest inherits) · Job Board (where the result surfaces) · Run and edit a Manifest (drift detection and the live dry-run, once a Manifest already exists) · Consult Helm (where the proposal conversation happens).
