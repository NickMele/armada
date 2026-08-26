# Manifest

**What it is:** Per-project config, backed by an `armada.yml`: the Checks a change must pass to land, the Commands available to run against the repo, which Skills, sub agents and secrets it may use, and whether Fleet may dispatch work here at all. Extends or restricts Kit defaults.

---

**Kind:** Entity.

Defines Manifest — the per-project/repo definition of how Fleet and Drones may interact with a given codebase. Companion to the main Armada brief and `../contracts/configuration.md`.

## What it is

Not the same shape as either tier above it. [Kit](kit.md) is the tool set you bring and holds defaults a Manifest may extend or restrict; [Machine](machine.md) is how this installation behaves and has no project-level counterpart at all. A Manifest defines a project's shape and boundaries — checks it must pass to land code, commands available to run against it, which Skills/Agents/secrets it can use, and whether Fleet may dispatch work in it at all. Backed by an `armada.yml` file.

## Workspace mapping

**One Manifest per workspace** — even inside a large monolith with multiple independent components, each workspace gets its own `armada.yml` and is independently scoped on the [Job Board](job-board.md), same as a standalone repo. **The Board scopes by a Job's `owner_manifest_id`**, which is always exactly one, so every Job — Convoy included — has exactly one Board it appears on. Note: components typically share one lockfile at the repo root under pnpm/yarn/npm-style workspaces — only the package manifest (e.g. `package.json`) is truly per-workspace; the shared lockfile is evidence common to all of them, not a per-workspace artifact.

### Root `armada.yml` (monorepos only)

A monorepo may additionally have a root-level `armada.yml`, separate from any workspace's own — created only when Setup detects multiple workspaces (a simple, single-workspace repo's Manifest *is* the root; no separate file exists).

- Holds **commands** — e.g. bootstrap/install — and **Checks for the artefacts the root itself owns**. The rule was once commands only, never checks, and was reversed because the root turned out to own things nothing gated — a change to a root test or CI command touches no workspace, so no workspace's Checks apply and the highest-blast-radius change in the repo has an empty mechanical tier; the shared lockfile is written by any workspace's change and read by every workspace's Checks with no owner; and a repo-wide Check (*do all workspaces build together*) belongs to no workspace by construction.
- **Root Checks run only when the diff touches root paths.** This restriction is load-bearing, not a detail. Without it the root becomes a place to put a Check that gates every Job in the repo, and one slow Check there taxes every workspace — which is the failure mode the original commands-only rule existed to prevent. A root Check is scoped to root-owned artefacts, never promoted to a repo-wide gate.
- **Which paths the root owns — nearest ancestor wins.** The nearest `armada.yml` up the directory tree owns a path; where no workspace claims it, the root does. So root paths are exactly the paths no workspace claims, which is what the touch condition above needed. A `/tooling` directory belonging to no workspace is a root path, gated by root Checks, and ungated only where the root declares none — the same state as a workspace with no Checks, already accepted.
- **Ownership is not exclusivity.** The shared lockfile is owned by the root under this rule, which is right, but *owned* means only *gated by root Checks*. Nothing reserves write scope, so two concurrent Jobs can still both write it.
- **Per-workspace Checks are unchanged.** Every workspace still declares its own in its own `armada.yml`; root Checks are additional, not a replacement, and a workspace's Checks are never hoisted to the root.
- Still open: whether the root *owns* the lockfile as opposed to merely being able to gate it — narrowed rather than closed by the nearest-ancestor rule, which settles gating and not exclusivity (see Open questions).
- Gives repo-wide build-orchestration evidence (root `Makefile`, `turbo.json`-style task runners) a place to be cited from, without duplicating it into every workspace's file. Armada only records *how to invoke* a tool — it never replaces or centralizes the tool's own config resolution (e.g. ESLint resolving its own root `.eslintrc` up the directory tree is invisible to, and irrelevant for, Armada).
- A workspace declares a dependency on a root command explicitly: `setup.requires: [bootstrap]` in its own `armada.yml` — a plural list, since a workspace may require more than one root command.
- Fleet tracks whether a required root command has already run for a given worktree, and re-runs it when **either**: (a) the worktree is new, or (b) the evidence backing the root command (the lockfile/manifest Scan traced it from) has drifted since it last ran — the same signal Verify's drift detection computes, applied at dispatch time instead of on-demand.

### Cross-Workspace Jobs

Three Job shapes cover cross-workspace work, selected **per-Job**, not fixed by the repo's overall shape:

> **Not every cross-workspace Job is a Convoy.** "Cross-Workspace Jobs" remains the name of this broader category, covering all three shapes. A Convoy is specifically the atomic case landing as one PR; linked-DAG sub-Jobs span workspaces but push separately and are **not** Convoys. Collapsing the two would destroy the distinction this table exists to draw. The Convoy shape is defined on [Convoy](convoy.md) — including its open items and the naming decision (see the `../contracts/design-system.md` lexicon).

| Shape | When it applies | Mechanics |
| --- | --- | --- |
| Single-workspace Job | Default — change is contained to one workspace | Unchanged from today's model |
| Linked-DAG sub-Jobs | Sequenceable dependencies — the "PR train" case (land the API change first, update the consumer against what merged) | Each sub-Job runs fully within its own workspace's Manifest — own Checks, own approval, own worktree, own PR. The dependent Job stays `blocked_by_dependency` until the upstream one reaches `completed_success`, then becomes dispatchable but not auto-dispatched |
| [**Convoy**](convoy.md) (atomic multi-workspace Job) | No valid intermediate state — tightly coupled workspaces that must land together in one commit (e.g. a parser/generator pair, or tightly-coupled services common in real monorepos) | One Job with **no children**, carrying several `write_targets` with `atomic` set. One Drone, one worktree spanning them; each gating workspace's Checks run independently against its own Manifest; one combined approval; one PR. The shape follows from those two fields; nothing on the record carries a shape name. Full definition and open items on [Convoy](convoy.md) |

**You don't hand-draft which shape applies.** The [Job proposer](job-proposer.md) emits a Job's `write_targets` and its `atomic` flag, and the shape follows from those two fields — there is no shape value for it to choose. What that call reads, when it runs and where its proposal is approved are on that document.

**The one input this file supplies is a root-Manifest-level default-posture setting** — "prefer atomic" against "prefer strict per-workspace boundaries" — giving the Job proposer a per-repo prior. A prototype repo might not care about workspace boundaries at all; a real work monorepo usually does. It is declared in the root `armada.yml`, which is why it is defined here.

**Across repos, only two of the three shapes apply.** A [Convoy](convoy.md) is root-Manifest-scoped — every declared Workspace descends from a single root `armada.yml` — so it cannot span repos by construction, which is exactly what lets one worktree span Workspaces as ordinary git. Single-workspace and linked-DAG sub-Jobs both work across repos; the atomic case does not.

How the Job proposer relates to [Helm](helm.md)'s planning assist is on [Job proposer](job-proposer.md).

## Registries

Separate registries, not the same thing tagged two ways:

| Registry | Purpose | Invoked by |
| --- | --- | --- |
| Checks | Mandatory — must pass to land/advance code. Gates workflow advancement (see [Workflow](workflow.md)) | Fleet, as part of mechanical verification |
| Commands | Optional, general-purpose — migrations, doc generation, builds, formatting, anything project-specific. Armada doesn't reimplement these tools, just gives Fleet/Drones/you one consistent way to invoke them | Both a Drone (during a Job) and you (directly via Bridge) — same named registry either way |
| Ports | Names a port a workspace needs, so Armada can place it | Nothing invokes it — Fleet reads it at claim time |

### Command approval

A Command can be flagged destructive. That flag gates **Drone** invocation — pauses for your approval, same as any other risky allowlisted op. Your own **manual** invocation via Bridge doesn't require a second approval step, since you're already the one directly triggering it.

**A declared teardown Command is the escape hatch** for a process handed to another supervisor — `launchctl`, a systemd unit, a deliberate double-fork. Everything Armada spawned that stayed in its own tree is killed by process group with no declaration needed, and containers are handled by the Docker adapter, so a declared Command covers only what neither reaches.

### Check prerequisites

**A Check may require Commands to run before it.** `checks.<name>.requires` is an ordered list of **Command names** from this Manifest's own Commands registry. An end-to-end Check that needs `migrate` and `seed` to have run first has no other way to say so: `setup.requires` (see Root `armada.yml`) is per *worktree* and runs once, so it cannot express per-Check ordering, and before this nothing in the file related a Check to a Command at all.

Rules that follow:

- **Names, never command strings.** A prerequisite points at a Commands entry, so what actually runs is written in exactly one place and stays in step when the Command is edited.
- **Ordered.** The list runs in the order given; two Commands where one seeds what the other migrated are not interchangeable.
- **A failure is attributed to the prerequisite.** A Check whose prerequisite failed reports the prerequisite as the failure, not the Check — otherwise a broken `migrate` reads as a broken test suite.
- **Consequence for the Commands registry.** A Command now earns its place by being something a Check depends on, not only something a person runs by hand. That is a second reason for the registry to exist and an argument against ever folding Commands into Checks.
- A prerequisite **may** name a root Command, the same as `setup.requires`. And a prerequisite that has already run **in the same execution context is skipped** — Fleet tracks it, so two Checks naming `migrate` run it once.

  **Per context, not per worktree.** Checks share the worktree today, so the two are the same thing and the skip holds. A Check running in its own container is a different context: the prerequisite's effect is not there to inherit, so it must run again. Stating the rule as per-context means per-container Checks need no exception written later — the tracking key is the context, and an isolated Check simply never finds a hit.

  Consequence either way: `requires` guarantees *has run*, not *has just run*. A Check needing genuinely fresh state resets what it needs in its own command.

Found while drawing the Set Up a Project (Manifest) journey's proposal panel, where a Check row shows its prerequisites in the same cell as its command.

### Check timeout

A configured bound applies to every Check, and an optional per-Check field narrows it. Both, because a Check ranges from a seconds-long lint to a half-hour suite: one number has to accommodate the slowest and so never fires on the fast ones that hang, and a field alone means every repo writes it on every Check or gets no bound at all.

**A timeout escalates. It does not fail the gate.** The Check did not fail — it did not finish, and retrying re-runs the same hang. Nor is it one of the did-not-run reasons, since those mean the Check was correctly skipped and this one ran and got stuck. It is a condition the Drone cannot resolve.

Killing it is Fleet's: `checks-runner` holds the process group and kills it whole, so a timeout stops the Check and its children rather than orphaning them.

Whether the per-Check field may raise the configured bound or only lower it is open (see Open questions).

## Ports

A third definition registry, alongside Checks and Commands. It defines rather than narrows, which is why it sits at the top level and not under permissions.

**The direction is stated here because the section cannot carry it.** The permissions and knowledge sections exist so a setting inherits its Convoy direction from where it sits. Ports is a third direction on a two-direction boundary: **ports union across a Convoy's declaring Manifests, qualified by Manifest id.** A port is knowledge rather than authority — injecting a port number grants no ability the Drone lacked, since it could already bind any port, and the allowlist is blast-radius reduction rather than a sandbox. Commands made the identical move once namespaced by Manifest id.

Each entry has a name and two fields.

| Field | What it is |
| --- | --- |
| the name | What `${port.NAME}` resolves, and what an assignment keys to |
| `container` | The container-side port. The join key Armada matches on when it rewrites a compose document, so it knows which published port is which |
| `env` | The variable name the repo's own stack already expects |

**`env` exists because a command string covers one channel and there are more.** A `package.json` script Armada invokes but did not author, a compose stack reading a variable, and a Pilot shell all read the environment rather than a command line. `ARMADA_PORT_<name>` is emitted alongside regardless, so a name with no `env` still gets a variable and anything generic has one guaranteed form — but requiring it would be a coupling the repo has to accept in the one channel whose whole purpose is working with a repo untouched.

**Colliding `env` names across a Job's Manifest set are rejected at claim time.** Not at config load, and not scoped to a root: two independent roots can live in one repo, and a root-scoped check never compares them. The same check already exists for Manifest ids that mangle to one environment token.

Exact key naming and nesting is tracked in `../contracts/configuration.md`.

**A span outlives an interrupted Job and is released by the Job ending, not by anything about ports.** A claim lasts as long as its worktree; an interrupted Job holds both until a person answers its escalation and it reaches a terminal, after which retention sweeps the worktree and the span goes. Nothing here asks a person to release a port — there is no such action, and adding one would offer a control for a decision they are already making elsewhere. Past a threshold Fleet surfaces the hold so accumulation is not discovered at exhaustion. [Fleet](fleet.md) owns the mechanism.

## Setup (Scan → Proposal → Write)

Carried forward from the v1 prototype — this pattern worked well and generalizes cleanly to v2.

| Stage | What happens |
| --- | --- |
| Scan | Read-only evidence gathering across the workspace — lockfiles, package scripts, CI config, docker-compose services, pyproject tool sections, workspace globs. Nothing is written yet. **A dispatched Scan is not a [Job](job.md)**, even where it is agent work: it carries no worktree and no resolved toolset, and a Job's `owner_manifest_id` is always present, which a repo with no `armada.yml` cannot supply. It is agent work Fleet runs outside the Job model, the same category as the [Judge](judge.md) and the [Job proposer](job-proposer.md). |
| Proposal | Not a single approve-or-reject draft — a *possible* `armada.yml`. For a clean, well-evidenced workspace, the proposal is often already the final file. For a gappier one, it's a starting point you iterate on — editing it yourself and/or working it through with an agent — before it's something you'd want committed. At every stage of iteration, every line stays strictly evidence-backed: traces to something an actual file already said. Nothing inferred or guessed. |
| Write | Commits the current state of the proposal to `armada.yml` once you're satisfied with it. Not a mode chosen in advance — there is no separate "Write path" decision made up front, for a single workspace or for a whole monorepo batch. Write always writes whatever you've arrived at through Proposal/iteration, whether that took zero rounds of iteration or several |

### Monorepo onboarding

For a repo with multiple workspaces, Setup asks two orientation questions once, before Scan runs — informational, guiding the flow, not written to `armada.yml`:

- **Which discovered workspaces do you want to set up now?** Onboarding isn't all-or-nothing — scope this batch to whichever subset you want, come back for the rest later.
- **Do any checks run at the root of the repo, outside any single workspace?** Feeds the root `armada.yml` (see Workspace Mapping) if so — both its commands and its Checks.

Proposals for the workspaces selected for this batch are presented together, each line traceable to something Scan actually found. Iteration on any individual workspace's proposal — and Write for it — happens at whatever pace makes sense: a workspace with rich evidence might Write immediately, one with thin evidence might sit as a partial proposal until you're actually working in it, same as reaching for Update later.

**Explicitly not asked at Setup:** whether workspaces are coupled or dependent on each other, or how you push code changes (batched vs. per-workspace). That turned out to be a property of the *specific change* being made, not a fixed property of the repo — see Cross-Workspace Jobs.

## Verify

Run against an existing Manifest. Does both:

- **Drift detection** — re-scans and flags if `armada.yml` is out of sync with the current repo state (new scripts added, old ones removed), without changing the file
- **Live dry-run** — actually runs Setup and Checks once, to confirm they still work

## Update

Manual editing only — no re-scan involved. You edit the Manifest directly, same as any other setting. (Contrast with Setup/Verify, which both involve scanning.)

## A Manifest a Job creates

A Job can author an `armada.yml` — adding a service, extracting a package. Two questions look like one and are not: *do the new Checks pass*, which is about this work and is answered now, and *who gates this path from here*, which is a durable policy statement.

**Its Checks run before the creating Job lands.** Fleet finds a newly created `armada.yml` in the diff and runs the Checks it declares once, at that Job's final mechanical verification, attributed to the new Manifest. Nothing in a `WorkflowDef` declares this and no workflow opts in — the trigger is a property of the diff, so it applies to all of them. Without it a Job could add a service together with its test suite and land with that suite never having run, which is what anyone would expect of the work regardless of how Armada models gates.

**It does not gate the creating Job.** `gate_manifest_ids[]` is resolved at dispatch and does not gain a Manifest the Job authored; the new file starts gating from the next Job dispatched. That list says who gates a path, not what ran. The same freeze holds everywhere else — the `WorkflowDef`, `acceptance_criteria` and Check definitions are all fixed before the work starts precisely so the yardstick cannot move under it, and a Job adding its own file to its own gate list is that move.

**Nothing is unguarded in the meantime.** Nearest-ancestor owns those paths until the new file exists, so the root's or the parent Workspace's Checks gate the diff that creates it.

**The atomic case needs no special handling.** An extraction landing as a [Convoy](convoy.md) would otherwise have to add a gate mid-Job, and would need an `id` for a Manifest that does not exist yet. Neither arises: the gate list does not move, and the new `id` matters from the next Job onward.

**The adversarial version closes by construction rather than by approval.** A Drone weakening a Check that gates it is a live concern; a Drone authoring one that would gate it is the same shape. Here it cannot, because nothing it writes joins the list mid-flight. What its new Checks get instead is a single run and a Judge reading those tests as part of the diff, where the gaming patterns already look for a tautological or weakened test.

## Deletion

A Workspace can be removed and its `armada.yml` with it. Nearest-ancestor reverts those paths to the root automatically, so the file side needs nothing.

**The record side needs nothing either, because the record was never tied to the file.** A Manifest's row in the store is not deleted when its file is. It stops resolving — nothing is found at its path — and keeps its `id` and its last known name. Every historical [Job](job.md) carrying it as `owner_manifest_id` or in its gate list still shows what it ran against, and every Evidence row naming it stays readable, which is what append-only was for. A surface renders the name and marks it as no longer present; nothing renders a bare `id`.

**So there is no deletion event to observe.** Nothing watches for an `armada.yml` disappearing and nothing needs to. An unresolvable Manifest is a state rather than an occurrence, discovered the moment something tries to resolve it — Verify notices on a re-scan, and nothing else has to.

**A not-yet-started Job whose owner no longer resolves is refused at dispatch.** There is no configuration to resolve against, so it cannot run. It needs no status of its own and no third `queued` reason: dispatch is human-gated, so the refusal reaches a person at the moment they approve it, naming the Manifest that is gone — which is where they already are. Reassigning such a Job to the nearest surviving ancestor was rejected, because it would make the record say the Job ran against something it did not.

**Re-creating an `armada.yml` at the same path does not restore the identity.** `id` is explicit so that a Workspace can move without dangling its history, and a new file carries a new `id` unless it is written with the old one. Carrying it deliberately is how a Workspace is restored rather than replaced.

## Secrets

**Drones never hold secrets directly.** That constraint is Fleet-side and belongs to [Fleet](fleet.md) and the Adapters spec, not to any config tier — a Drone never holds a callable secrets tool at all. Manifest only scopes *which* secrets and providers are available to this specific project; Fleet still brokers the access.

## Skills, Sub Agents, Allowlist

Extends or restricts [Kit](kit.md) defaults — Kit holds the default set, a Manifest declares its own, and nothing above the Manifest constrains it. Applies to the allowlist, Skills, and **Sub agents** (project-specific definitions alongside Kit's global ones).

**How they actually merge is not decided.** No merge strategy exists in the Configuration Settings registry for Skills, MCP or Sub agents, and Kit and [Drone](drone.md) disagree on Sub agents outright — Kit describes them as layered on top, Drone's Convoy table puts them under intersection. The `armada.yml` design pass placed all three by inferring the inheritance axis from the peer axis, which is not a decision (see Open questions).

Note: Kit's global **Agent file** ("how I work" — personal, cross-project) has **no Manifest-level counterpart**. A repo's own agent file just lives naturally in the repo itself, alongside this Manifest — Armada doesn't inject or manage it separately.

### What's frozen vs. live

Skills/MCP/Agent-files/Commands are frozen into a Drone at spawn time (process boot-time constraint). Allowlist, budget cap, and dispatch freeze are enforced live at every gated checkpoint. Checks split: a Check that existed at spawn is frozen for the life of the Job, while a Check added mid-Job gates immediately — additive-only, and for a different reason than the boot-time constraint above. Full detail on [Drone](drone.md) and [Fleet](fleet.md).

## Budget

A Manifest can set its own $ / quota limit, and this document has long said it **fully overrides** the [Machine](machine.md)-level cap — not additive, not a ceiling.

**That override is not encoded anywhere, and precedence is open** (see [Machine](machine.md), Open questions). The two are independent single-layer rows in the Configuration Settings registry — one scoped Machine, one scoped Manifest-only, both carrying `n/a — single layer` as their merge strategy. There is no two-tier merge between them to invoke. Budget is also the one place the Kit/Machine test gives an unclear answer: routing has no project-level version, but a Manifest cap demonstrably does.

**A Job gated by several Manifests takes its owner's cap, not the lowest.** The two-tier rule above says nothing about several peers, and the obvious answer — the minimum — is wrong here. A cap is a **resource** setting, not a safety one. Taking the minimum lets a small workspace's cap, sized for that workspace's own work, govern a [Convoy](convoy.md) several times larger and kill it for a reason unrelated to that workspace. Over-caution on a safety setting costs a manual step; over-caution here makes the work impossible. Known and accepted: this is sidesteppable by choosing which workspace owns the Job.

**Verification spend counts against the cap.** A Judge call bills against the same cap as the work it verifies — a cap that excluded it would understate what the Job cost. Whether verification carries a sub-limit of its own is [Judge](judge.md)'s to say.

## Dispatch freeze

A Manifest-level toggle to pause/freeze **all** dispatch for this project (e.g. during a release freeze) — independent of, and layered on top of, the existing per-Job approval gate.

**Cross-workspace interaction:** for the **linked-DAG** shape, no special-case logic is needed. If a frozen Manifest's Job is blocking a dependent Job in another (unfrozen) workspace, that dependent Job simply stays `blocked_by_dependency` until the frozen Manifest unfreezes and its Job actually completes — falls out of the existing DAG dependency status_reason, not a new mechanism.

**That reasoning does not cover a [Convoy](convoy.md)**, which has no children and so no dependent Job to block. **Most-restrictive-wins.** Any frozen Manifest in a Job's gate list freezes the whole Job. A freeze means do not touch this project, and the Job would touch it. This answers dispatch. **A freeze landing on a Convoy already running is still unresolved** — freeze is enforced live at every gated checkpoint, not only at dispatch — see [Convoy](convoy.md), Open questions.

## Auto-merge and review gate

Both are per-Manifest, not global, and both use the same override pattern one level apart in the pipeline.

| Setting | Values | Behavior |
| --- | --- | --- |
| `auto_merge` | `never` / `tests-pass` / `always` | Enforced by Fleet before merge. A false result routes to Inbox > Job Reviews rather than merging. **Across a Job gated by several Manifests: most-restrictive-wins** — `never` beats `tests-pass` beats `always`. One PR, so the most cautious gating Manifest holds |
| `review_gate` | `human_always` (default) / `auto_if_judge_passes` | Controls whether a workflow's final review step requires a human, or can advance on a Judge pass alone. See [Workflow](workflow.md) — the `advance_gate` field. **Across a Job gated by several Manifests: most-restrictive-wins** — `human_always` beats `auto_if_judge_passes` |

## Still open

The engineer-facing walk from "add a new repo" to a working Manifest is designed as the Set Up a Project (Manifest) journey — tracked there, not as an open item here. The full set of dials scoped to a Manifest is a row in the Configuration Settings registry; `../contracts/configuration.md` owns the tiering rule, which is why no list of them appears in prose here.

## Open questions

- **[manifest-root-lockfile-ownership]** Does the root `armada.yml` *own* a shared lockfile it can gate, as opposed to merely being able to gate it? The nearest-ancestor rule settles gating and not exclusivity.
- **[manifest-check-timeout-raise-or-lower]** May a per-Check timeout field raise the configured bound, or may it only lower it?
- **[manifest-kit-merge-rules]** What are the Kit→Manifest merge rules for Skills, MCP and Sub agents? No merge strategy exists in the Configuration Settings registry for any of the three, and Kit and Drone disagree on Sub agents outright — Kit describes them as layered on top, Drone's Convoy table puts them under intersection.
