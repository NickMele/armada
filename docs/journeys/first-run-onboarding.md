# First-Run Onboarding

**What it is:** "I'm brand new to the whole app" — the first walkthrough after installing Armada, chaining four already-designed journeys into one hard-gated sequence.

Design fidelity: not set. Analysis: Complete. UI/UX design: Not started.

---

*Numbering note: the design project has not yet drawn this journey and names no `Journey N - ...` file for it. Numbered here only to give the file set a stable order — see the note on Guild Setup & Configuration for how the sequence after Journey 9 was assigned.*

**Trigger:** First launch after install.

**Concepts touched:** Kit, Machine, Manifest, Fleet (via Doctor), Job Board.

**Milestone:** Ship. Design note: chains Guild Setup, Set Up a Project, Check System Health and Dispatch a Job into one hard-gated sequence — the only place in Armada where step order is enforced. Cannot complete until its four constituent journeys land, so it is finished last even though the user meets it first. That also makes it the journey most likely to expose seams between the others, and it will expose them late.

**Prototype discarded (Aug 2026).** An earlier prototyping pass in Bridge (Armada Mockups) produced surface-level designs for Steps 1 and 2. **That prototype was not completed and is not being used.** Its surface descriptions, field treatments, and button placements have been removed from this document.

**What was kept and why:** three findings from that pass stand independently of the prototype's quality — they are reasoning about the flow, not renderings of it. They are marked **Finding** below and are recorded as *open design questions*, not settled decisions. The five contract corrections are also kept, because they record drift that was caught; deleting them means the same drift returns unrecognized.

**UI/UX design for this journey is not started.** Nothing on this document should be treated as a specification of what a surface looks like.

## Flow — Hard-Gated, Linear

Each step must complete before the next unlocks. This is the one place in Armada where sequencing is enforced rather than left as a recommendation.

| # | Step | What happens |
| --- | --- | --- |
| 1 | Guild Init | Machine-level setup. **Finding — open:** should fork at entry between importing an existing Kit and starting fresh. See Step 1 below. This step's name, and whether Kit and Machine setup are one step here or two, is open — see Open questions. |
| 2 | Set Up a Project | Onboard your first repo into a working Manifest. **Finding — accepted and now built into the flow:** the sequence needs a **Locate** phase before Scan. See Step 2 below, and Set Up a Project (Manifest), which now carries Locate as its first phase. |
| 3 | Check System Health | Doctor's module grid confirms Fleet, Armada API, Kit, Machine, Manifest, SQLite, Git, Docker, Claude, Keychain and System stats all pass before you dispatch anything real. See Check System Health and Doctor. |
| 4 | Dispatch a Job | Approve your first real Job, closing the loop end to end. See Dispatch a Job. |

## Step 1 — Guild Init

What this step is called, and whether Kit setup and Machine setup are one step or two, is open — see Open questions.

> **Finding — open question, not a settled design.** Guild Init probably should not open as a guided tour through four groups in fixed order. It likely **forks at entry**, because the two populations arriving here need opposite things. The reasoning below survives the discarded prototype; the surfaces do not exist.

| Path | What the user does | Why |
| --- | --- | --- |
| **Import from another machine** | Asked only where the existing Kit lives. Armada performs the import. Done. | Someone who already has a Kit does not need to be walked through settings they have already decided. The import is the whole step. |
| **Start fresh** | All four settings groups available, defaults already filled in, freely reachable in any order. | Defaults are good enough to proceed. Nothing gated inside the step, so the user can inspect what interests them and skip the rest rather than being marched through four screens. |

Both paths land on Step 2.

### The four settings groups

Content-level notes only. **No surface design exists for any of these.**

| Group | Note |
| --- | --- |
| **AI Behavior** | **Finding — open:** this group should probably show the **actual Claude files found on the machine** — Agent files, Skills, MCP servers, Sub agents, Plugins, Models — rather than abstract settings, with each file carrying its state relative to Kit (in Kit / drifted / not in Kit). How that is presented is undesigned. |
| **Resources & Budget** | Defaults filled in. Unchanged from the original design. |
| **Safety** | Command allowlist and Destructive-op list. **Finding — open:** likely inline editors rather than links out. Push and Passive triggers as toggles with plain-language labels. |
| **Interface & Notifications** | Defaults filled in. Unchanged from the original design. |

**Moved out of onboarding:** Sync & Portability becomes its own top-level nav group, holding config-repo and pull-on-startup controls. It is a machine-lifecycle concern, not a first-run one. This one is a decision, not an open finding.

## Step 2 — Set Up a Project

> **Finding — the most substantive thing to survive the prototype.** The step needs **five** phases, not four. **Locate was missing from the original design**, which assumed Armada already knew which repo it was looking at. That gap is real independent of any prototype. This finding is no longer merely a finding: Set Up a Project (Manifest) now carries Locate as its own first phase.

| Phase | What happens |
| --- | --- |
| **Locate** | The user points Armada at a directory — a typed path or an OS directory picker — before anything is scanned. Undesigned. |
| **Scan** | One read-only pass over the repo. |
| **Proposal** | The proposed Manifest, for review. Iterative, not a one-shot gate. |
| **Write** | The Manifest is written. |
| **Verify** | Confirmation the written Manifest holds. |

Locate should stay reachable from later phases, so the user can change repos without restarting the journey.

**Rejected:** a "recently opened" list of repo paths. At first run there is nothing recent, and inventing plausible-looking entries only makes the surface look busier without helping anyone.

## Contract corrections — drift to avoid

The discarded prototype drifted from the design contract in five ways. **Recorded here so the same drift is not reintroduced by the next attempt** — this table is the most durable output of that pass.

| Drift | Correct behaviour | Rule |
| --- | --- | --- |
| Primary CTAs scaled up (larger type, taller, heavier padding) to signal importance | `--accent` fill at the standard 32px control height, `--text-sm` weight 500, one per view | Button spec + 4px grid. Emphasis is fill, not size |
| Primary buttons carried leading icons (magnifier on Scan, download on Import) | Label only | Iconography → Actions: text buttons carry no icon |
| Badges read `IN GUILD` / `DRIFTED` / `NOT IN GUILD` | Sentence case | ALL CAPS is legal only in `--text-2xs` table headers at 0.04em |
| Field labelled "Where is your project?" | A noun label, e.g. "Project location" | No Wh- sentence openers; they survive only as panel headings |
| Surfaces styled on the Nocturne palette (blurple accent, outlined primary, 7px radii) | Armada tokens: `--bg-*` slate ground, `--accent` #4A9EDB filled primary, `--radius-md` 5px | Hard rule 1. No value that is not a token |

## Still to reconcile

- **Kit-file state labels** (in Kit / drifted / not in Kit) are not Job states, so they sit outside the badge table on Iconography. Under that page's rule for anything unlisted, the default is no icon — see Open questions for the specific question this raises, tracked in the Iconography contract rather than duplicated here.

## Why Hard-Gated

First-run is the one moment where sequencing actively protects you — e.g. dispatching before Doctor has checked Fleet/Git/Docker would produce an opaque failure with no context on why. After this first pass, all four steps are freely revisitable independently: Kit and Machine edits, additional Manifests, Doctor checks, and dispatch have no gating between them anywhere else in the app. The hard-gate is unique to this journey.

Note that gating is **between** steps only. Inside a step, nothing is gated — Guild Init's four groups are freely ordered, and Locate remains reachable from later project phases.

## Exit Condition

Onboarding is complete once one Job has been successfully approved and dispatched through step 4.

## What is already decided and landed

- **What the status bar says for each of Fleet's runtime states, outside onboarding.** Three states, each named out loud with a mono clause carrying the fact that identifies it — running, not running, unreachable — decided in full and recorded on the design system contract's Status bar section. What the bar reads specifically *during* the gated onboarding sequence is a separate, still-open question — see Open questions.
- **The install and distribution strategy for the Fleet binary.** Bundled with Bridge, adopted by Fleet at a safe moment: one artefact, one signing and notarisation pipeline, no second updater. The running daemon restarts into the newly shipped binary when no Jobs are active, or on the engineer's explicit instruction. This does not remove version skew, it moves it from Bridge-version-versus-Fleet-version to installed-Fleet-versus-running-Fleet, which is exactly what the protocol handshake and its fallback window exist to cover. Left to decide, and smaller: what happens if a safe moment never arrives because Jobs are always running — either Fleet nags, or it adopts at some bound anyway.

## Open questions

- **[onboarding-reopen-after-complete]** Can First-Run Onboarding be reopened after it completes?
  Once it completes, all four surfaces are freely reachable with no gating anywhere. Whether the sequence itself can be re-entered is undecided. The case for: onboarding a second machine, or returning after a long gap, is the same walkthrough, and rebuilding it by hand from four separate surfaces is worse than replaying the guided path. The case against: the hard-gate exists to protect a *fresh* install, and a re-entered onboarding that blocks you from surfaces you already use would be actively worse than nothing on a working install. A middle answer exists: re-run it ungated, as a checklist rather than a sequence.

- **[onboarding-trigger-plain-language]** Do escalation trigger names get the same plain-language treatment as Safety's toggles?
  Whether trigger names (fan-out/rate, stalled, thrashing, gate-failure, evidence-suspect) stay as internal vocabulary in Alerts and escalation, or get the same plain-language treatment as Safety's toggles, is open — Safety's toggles have already been converted; Alerts has not.

- **[guild-init-fork-at-entry]** Should Guild Init fork at entry between an import path and a start-fresh path?
  Recorded under Step 1 above as a Finding. The two populations arriving at first run need opposite things — someone with an existing Kit needs only to be asked where it lives, while someone starting fresh needs the full four-group walkthrough with defaults pre-filled. This is reasoning that survived the discarded prototype; no surface exists for it.

- **[ai-behavior-group-shows-kit-files]** Does the AI Behavior settings group show the actual Claude files found on the machine, rather than abstract settings?
  Recorded under "The four settings groups" above. Each file would carry its state relative to Kit — in Kit, drifted, or not in Kit. How that is presented is undesigned.

- **[safety-group-inline-editors]** Are the command allowlist and destructive-op list edited inline during onboarding, or linked out to elsewhere?
  Recorded under "The four settings groups" above, alongside Push and Passive escalation triggers as toggles with plain-language labels.

Four further open items bear on this journey and are recorded, not duplicated, where each is the citable home. `kit-machine-setup-surface-naming` and `kit-import-missing-files`, both above on Guild Setup & Configuration, cover respectively what the Kit and Machine setup surfaces are called and whether first-run setup is one journey and one gated step or two, and what happens on the Import path when the imported Kit references Claude files that do not exist on this machine. `status-bar-onboarding`, in `../contracts/design-system.md`, covers whether the status bar reads the same three runtime states during the gated sequence as everywhere else, or gets its own reading before Fleet is reachable. `kit-file-icons`, in `../contracts/iconography.md`, covers whether the three Kit-file states (in Kit / drifted / not in Kit) get icons, or stay label-only.

## Related

Guild Setup & Configuration · Set Up a Project (Manifest) · Check System Health · Dispatch a Job.

This journey has no number because the design project has not drawn it. A number in a filename here means a `Journey N` drawing exists to match it; inventing one would assert a correspondence that does not.
