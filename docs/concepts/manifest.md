# Manifest

**What it is:** Per-project config, backed by an `armada.yml`: the Checks a change must pass to land, the Commands available to run against the repo, which Skills, sub agents and secrets it may use, and whether Fleet may dispatch work here at all. Extends or restricts Kit defaults.

---

**Kind:** Entity.

Defines Manifest — the per-project/repo definition of how Fleet and Drones may interact with a given codebase. Companion to the main Armada brief and `../contracts/configuration.md`.

## What it is

A Manifest defines a project's shape and boundaries — checks it must pass to land code, commands available to run against it, which Skills/Agents/secrets it can use, and whether Fleet may dispatch work in it at all. It is backed by an `armada.yml` file.

It is not the same shape as either tier above it. [Kit](kit.md) is the tool set you bring and holds defaults a Manifest may extend or restrict; [Machine](machine.md) is how this installation behaves and has no project-level counterpart at all.

## Workspace mapping

**One Manifest per workspace.** Even inside a large monolith with multiple independent components, each workspace gets its own `armada.yml` and is independently scoped on the [Job Board](job-board.md), same as a standalone repo.

**The Board scopes by a Job's `owner_manifest_id`**, which is always exactly one. Every Job — Convoy included — therefore has exactly one Board it appears on.

Components typically share one lockfile at the repo root under pnpm/yarn/npm-style workspaces. Only the package manifest (e.g. `package.json`) is truly per-workspace; the shared lockfile is evidence common to all of them, not a per-workspace artifact.

### Root `armada.yml` (monorepos only)

A monorepo may additionally have a root-level `armada.yml`, separate from any workspace's own. It is created only when Setup detects multiple workspaces — a simple, single-workspace repo's Manifest *is* the root, and no separate file exists.

**The root holds commands** — bootstrap/install, for example — **and Checks for the artefacts the root itself owns.** Root Checks exist because the root owns things nothing else gates:

- A change to a root test or CI command touches no workspace, so no workspace's Checks apply and the highest-blast-radius change in the repo has an empty mechanical tier.
- The shared lockfile is written by any workspace's change and read by every workspace's Checks, with no owner.
- A repo-wide Check — *do all workspaces build together* — belongs to no workspace by construction.

**Root Checks run only when the diff touches root paths.** Why: without it the root becomes a place to put a Check that gates every Job in the repo, and one slow Check there taxes every workspace. A root Check is scoped to root-owned artefacts, never promoted to a repo-wide gate.

**Which paths the root owns — nearest ancestor wins.** The nearest `armada.yml` up the directory tree owns a path; where no workspace claims it, the root does. Root paths are therefore exactly the paths no workspace claims.

A `/tooling` directory belonging to no workspace is a root path, gated by root Checks, and ungated only where the root declares none — the same state as a workspace with no Checks.

**Ownership is not exclusivity.** The shared lockfile is owned by the root under this rule, but *owned* means only *gated by root Checks*. Nothing reserves write scope, so two concurrent Jobs can still both write it.

**Per-workspace Checks are unchanged.** Every workspace still declares its own in its own `armada.yml`; root Checks are additional, not a replacement, and a workspace's Checks are never hoisted to the root.

Still open: whether the root *owns* the lockfile as opposed to merely being able to gate it — narrowed rather than closed by the nearest-ancestor rule, which settles gating and not exclusivity (see Open questions).

**The root gives repo-wide build-orchestration evidence a place to be cited from** — a root `Makefile`, `turbo.json`-style task runners — without duplicating it into every workspace's file.

**Armada only records *how to invoke* a tool.** It never replaces or centralizes the tool's own config resolution: ESLint resolving its own root `.eslintrc` up the directory tree is invisible to, and irrelevant for, Armada.

**A workspace declares a dependency on a root command explicitly**, as `setup.requires: [bootstrap]` in its own `armada.yml`. It is a plural list, since a workspace may require more than one root command.

**Fleet tracks whether a required root command has already run for a given worktree**, and re-runs it when **either** the worktree is new, or the evidence backing the root command — the lockfile/manifest Scan traced it from — has drifted since it last ran. That is the same signal Verify's drift detection computes, applied at dispatch time instead of on-demand.

### Cross-Workspace Jobs

The Job shapes covering cross-workspace work are selected **per-Job**, not fixed by the repo's overall shape.

> **Not every cross-workspace Job is a Convoy.** "Cross-Workspace Jobs" remains the name of this broader category, covering every shape below. A Convoy is specifically the atomic case landing as one PR; linked-DAG sub-Jobs span workspaces but push separately and are **not** Convoys.

| Shape | When it applies |
| --- | --- |
| Single-workspace Job | Default — the change is contained to one workspace |
| Linked-DAG sub-Jobs | Sequenceable dependencies — the "PR train" case |
| **Convoy** (atomic multi-workspace Job) | No valid intermediate state — must land together in one commit |

**Single-workspace Job.** Unchanged from today's model.

**Linked-DAG sub-Jobs.** The PR train: land the API change first, update the consumer against what merged. Each sub-Job runs fully within its own workspace's Manifest — own Checks, own approval, own worktree, own PR.

The dependent Job stays `blocked_by_dependency` until the upstream one reaches `completed_success`, then becomes dispatchable but not auto-dispatched.

**[Convoy](convoy.md).** Tightly coupled workspaces that must land together — a parser/generator pair, or tightly-coupled services common in real monorepos. One Job with **no children**, carrying several `write_targets` with `atomic` set.

One Drone, one worktree spanning them; each gating workspace's Checks run independently against its own Manifest; one combined approval; one PR. The shape follows from those two fields, and nothing on the record carries a shape name. Full definition, open items and the naming decision are on [Convoy](convoy.md) — see also the `../contracts/design-system.md` lexicon.

**You don't hand-draft which shape applies.** A Job's shape follows from `write_targets` and its `atomic` flag — there is no shape value for anything to choose. What the [Job proposer](job-proposer.md) does propose, why scope is not among it, and what the workflow's scope step does and does not settle are on that document.

**The one input this file supplies is a root-Manifest-level default-posture setting** — "prefer atomic" against "prefer strict per-workspace boundaries" — giving the Job proposer a per-repo prior. A prototype repo might not care about workspace boundaries at all; a real work monorepo usually does. It is declared in the root `armada.yml`, which is why it is defined here.

**Across repos, the atomic shape does not apply.** A [Convoy](convoy.md) is root-Manifest-scoped — every declared Workspace descends from a single root `armada.yml` — so it cannot span repos by construction, which is exactly what lets one worktree span Workspaces as ordinary git. Single-workspace and linked-DAG sub-Jobs both work across repos.

How the Job proposer relates to [Helm](helm.md)'s planning assist is on [Job proposer](job-proposer.md).

## Registries

Separate registries, not the same thing tagged two ways:

| Registry | Purpose | Invoked by |
| --- | --- | --- |
| Checks | Mandatory — must pass to land or advance code | Fleet, as part of mechanical verification |
| Commands | Optional, general-purpose — migrations, doc generation, builds, formatting, servers | A Drone during a Job, and you directly via Bridge |
| Evidence | Declares the harness — how this repo shows what a change did | Fleet, on a step whose evidence type is `visual` |
| Ports | Names a port a workspace needs, so Armada can place it | Nothing invokes it — Fleet reads it at claim time |

Checks gate workflow advancement — see [Workflow](workflow.md).

Commands cover anything project-specific. Armada doesn't reimplement those tools; it gives Fleet, Drones and you one consistent way to invoke them, through the same named registry either way.

### Command approval

A Command can be flagged destructive. That flag gates **Drone** invocation — pauses for your approval, same as any other risky allowlisted op. Your own **manual** invocation via Bridge doesn't require a second approval step, since you're already the one directly triggering it.

**A declared teardown Command is the escape hatch** for a process handed to another supervisor — `launchctl`, a systemd unit, a deliberate double-fork. Everything Armada spawned that stayed in its own tree is killed by process group with no declaration needed, and containers are handled by the Docker adapter, so a declared Command covers only what neither reaches.

### Commands that keep running

A Command with `serve` is a server: it stays up until something stops it. The shape follows Tilt's [`local_resource`](https://docs.tilt.dev/api.html#api.local_resource), which draws the same line between a command expected to exit and one expected to stay up.

```yaml
ports:
  storybook: {}

commands:
  storybook:
    serve: pnpm -C packages/components exec storybook dev -p ${port.storybook} --no-open --ci
    ready: curl -sf http://localhost:${port.storybook}
    links:
      - url: http://localhost:${port.storybook}
        name: Storybook
```

| field | type | required | default | meaning |
|---|---|---|---|---|
| `run` | string | no | — | Runs first; the server starts only if it exits zero |
| `serve` | string | yes, for a server | — | Starts the server, held until stopped |
| `ready` | string | no | serving once started | Exits zero once the server answers |
| `links` | list | no | empty | Addresses offered as buttons, each a `url` and optional `name` |

`${port.NAME}` resolves in every field from the claim the server runs under — see Ports, below. [Fleet](fleet.md) holds each server: one instance per Job, stopped when the Job ends.

> **Rule.** A server that exits on its own has failed, whatever its exit code.
> Why: staying up is the whole of what a server is for.

> **Rule.** A Drone starts a server only through Fleet's MCP tool, never from its shell.
> Why: Fleet has to know a server is running to hand it to the next Drone and to stop it when the Job ends.

### Check prerequisites

**A Check may require Commands to run before it.** `checks.<name>.requires` is an ordered list of **Command names** from this Manifest's own Commands registry.

An end-to-end Check that needs `migrate` and `seed` to have run first has no other way to say so: `setup.requires` (see Root `armada.yml`) is per *worktree* and runs once, so it cannot express per-Check ordering.

Rules that follow:

- **Names, never command strings.** A prerequisite points at a Commands entry, so what actually runs is written in exactly one place and stays in step when the Command is edited.
- **Ordered.** The list runs in the order given; two Commands where one seeds what the other migrated are not interchangeable.
- **A failure is attributed to the prerequisite.** A Check whose prerequisite failed reports the prerequisite as the failure, not the Check — otherwise a broken `migrate` reads as a broken test suite.
- **A prerequisite may name a root Command**, the same as `setup.requires`.
- **A prerequisite that has already run in the same execution context is skipped.** Fleet tracks it, so two Checks naming `migrate` run it once.

**Consequence for the Commands registry.** A Command earns its place by being something a Check depends on, not only something a person runs by hand. That is also an argument against ever folding Commands into Checks.

**Per context, not per worktree.** Checks share the worktree today, so the two are the same thing and the skip holds. A Check running in its own container is a different context: the prerequisite's effect is not there to inherit, so it must run again.

Stating the rule as per-context means per-container Checks need no exception written later — the tracking key is the context, and an isolated Check simply never finds a hit.

**Either way, `requires` guarantees *has run*, not *has just run*.** A Check needing genuinely fresh state resets what it needs in its own command.

In the Set Up a Project (Manifest) journey's proposal panel, a Check row shows its prerequisites in the same cell as its command.

### What a passing run of a Check looks like

**A Check may declare `expect_exit_code`, and absent means `0`.**

```yaml
checks:
  repro:
    run: cargo test --test regression -- --exact known_bug
    expect_exit_code: 1
```

Decided 8 Sep 2026. **The code lives on the Check, not on the step in a
workflow**, and it moved here from there — the same decision `when` records one
heading down, for the same reason. A Check that passes by failing knows that
about itself; a step naming it does not, and the number restated in `bug`,
`feature`, `refactor` and `revert` drifts the day the command changes.

- **Absent means zero**, which is what every Check written before the key
  existed meant.
- **`after_merge` keeps it**, where it drops `when` and `narrow`. Those are a
  step's and a Drone's questions and there is no step after a merge; this one is
  the Check's, so a Check that passes by failing does not report as broken every
  time somebody merges.
- **A step may still write it, and must agree.** The step-level key still
  parses so that workflows written before the move go on loading, and a step
  that writes a *different* number is refused before dispatch. Reading either
  one silently would be Armada picking which file is right about a repository's
  own command.

### Which paths a Check covers

**A Check may declare `when`, a list of path patterns, and a Job that changed none of them does not run it.**

```yaml
checks:
  storybook:
    run: pnpm -C packages/components build-storybook
    when: ["packages/**", "apps/desktop/**"]
```

Decided 29 Aug 2026. **The pattern lives on the Check, not on the step in a workflow.** The repository declares once what a Check covers and every workflow inherits it — the same glob repeated across `bug`, `feature`, `refactor` and `revert` would drift, and a Check that unexpectedly did not run would have two places to look. There is no step-level override, and adding one is a change to this paragraph first.

Rules that follow:

- **Absent means always.** A Check with no `when` runs on every step that names it, which is every Manifest written before the key existed. An empty list is refused: `when: []` is a Check that can never run.
- **The dialect is one dialect**, and the Configuration contract states it in full. A pattern from another dialect is refused at load rather than matched literally.
- **The kind of change is not read.** A file deleted from `packages/` is a change to `packages/`, and a rename arrives as two paths — the old one deleted and the new one added — so either side covers the Check on its own.
- **Skipped is not passed, and not `did not run` either.** A step that advanced because every Check was skipped verified nothing, and the record says so: `check_runs` carries a `skipped` row per Check, naming the paths it covers, and the Drone is told which of the three happened rather than being told it passed.
- **It is frozen with the workflow.** A Job resolves `when` at creation alongside the Check's command, so editing `armada.yml` mid-Job changes the next Job rather than moving the gate under this one.

`armada check`, the dry run a Drone can ask for, skips exactly what the gate would. A rehearsal that ran a Check the gate will not run would tell a Drone its work failed something nobody is going to ask.

### How much of the tree a Check reads

**A Check may declare `narrow`, and a Drone asking about its own change gets it run that way instead.** The Configuration contract holds the syntax and every key in it.

```yaml
checks:
  test:
    run: cargo nextest run --workspace --exclude acceptance
    narrow:
      run: cargo nextest run
      each: "-p {}"
      under: crates
      except:
        - acceptance
```

Decided 8 Sep 2026, after a Drone spent a whole step polling the whole gate and submitted nothing. It is the second question about paths a Check answers and `when` is the first: `when` decides whether the Check runs, `narrow` decides what it reads once it does.

Rules that follow:

- **Absent means whole.** A Check with nothing narrower to run declares nothing, and runs exactly as it did before the key existed. That is most Checks and always will be.
- **The derivation from a path to an argument is the Manifest's.** `under: crates` is this repository saying where its packages live. Nothing in Fleet or the runner knows it, which is what keeps the same key right for a repository that keeps its packages somewhere else.
- **A narrowed run gates nothing.** The Drone asks for it mid-step, the gate reads the whole of every Check regardless, and the report says in its own words that a narrowed pass is the smaller claim. `after_merge` drops `narrow` for the reason it drops `when`.
- **A Check that narrows to nothing the change touched is skipped, not passed.** The third answer, the same one a `when` skip records.
- **`except` restates an exclusion the whole run already makes.** Dropping `--workspace` so `-p` means something drops `--exclude` with it, and a narrowed `test` that pulled `acceptance` back in would show a Drone a bar the gate deliberately does not apply — the one case where a narrowed run is misleading rather than merely smaller.
- **It is frozen with the workflow**, beside the Check's command.

### Proving what merged

**A Manifest may name Checks to run against the tree a merge left behind, and absent means none run.**

```yaml
after_merge:
  checks: [build, test]
```

Decided 7 Sep 2026, #474. A Job's gate proves the work in a worktree cut from a base that has since moved; Fleet then brings the repository up to what merged and, until this key, stopped there — so the last run that said anything about the code holding was a run against a tree that no longer existed.

Rules that follow:

- **Absent is off, and there is no value meaning *all of them*.** What this costs is a build and a test run on the machine somebody is working on, for an event nobody is waiting for. `../practices/rust.md` section 8 names a hook that rebuilt on merge as the cause of v1's four-minute cold build and says outright not to reintroduce one. So the repository names what is worth that, one Check at a time, or nothing runs.
- **The run belongs to the commit, not to the Job that noticed.** Two Jobs merging within a minute are two merges into one commit, and the result is keyed by the commit the repository ended up on. The second Job reads the first one's answer rather than running the suite again.
- **A Check that declares `requires` is refused here.** A prerequisite writes in the tree it runs in, and that tree is the repository a person is standing in. Fleet fast-forwards it only over a clean checkout; a run that then reformatted it would put back what those refusals exist to keep out.
- **A Check's own `when` is not consulted.** `when` answers *did this step touch anything I cover*, and after a merge there is no step to ask it of. The list is the filter.
- **Nothing depends on the answer.** The work is merged. A red cannot fail the Job, cannot reopen it — `completed_success` is terminal — and rolls nothing back; the response is a person filing a new Job pointing back through `subject`. See [Fleet](fleet.md).
- **A merge Fleet declined to fast-forward proves nothing.** A dirty worktree or a checkout on another branch leaves no updated tree, and a run against whatever was there would be reporting on somebody's uncommitted work.

### Check timeout

A configured bound applies to every Check, and an optional per-Check field narrows it.

Both exist because a Check ranges from a seconds-long lint to a half-hour suite. One number has to accommodate the slowest and so never fires on the fast ones that hang; a field alone means every repo writes it on every Check or gets no bound at all.

**A timeout escalates. It does not fail the gate.** The Check did not fail — it did not finish, and retrying re-runs the same hang.

Nor is it one of the did-not-run reasons, since those mean the Check was correctly skipped, and this one ran and got stuck.

It is a condition the Drone cannot resolve. Killing it is Fleet's: `checks-runner` holds the process group and kills it whole, so a timeout stops the Check and its children rather than orphaning them.

Whether the per-Check field may raise the configured bound or only lower it is open (see Open questions).

## Evidence — how this repository shows its work

On a change whose point is not the code — a panel that should collapse, a screen that should render differently — the diff is the least useful thing on the screen, and it used to be the only thing offered. Reviewing meant reading a patch to infer an outcome you could have been shown.

Checks say whether work advances and Commands say what a Drone may run. Neither says how this project demonstrates that a change did what was asked. `evidence:` is that third thing.

```yaml
evidence:
  serve: pnpm dev --port 6006
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test {}
  frames: .playwright/frames
  never:
    - /settings
```

`run` and `frames` are the common denominator — the command that reaches the state and the directory it leaves files in. A web application also starts a server first, which is what `serve` and `ready` are for; a repository with nothing to serve — a desktop app, a CLI, a library, a data pipeline — declares only the two required keys:

```yaml
evidence:
  run: node run-and-capture.js {}
  frames: .armada/frames
```

| Field | What it is |
| --- | --- |
| `serve` | Starts the thing being shown. Long-running: held for the run, ended after it. It names its own port — Armada assigns none. Optional, and goes with `ready` |
| `ready` | Exits zero once `serve` is up. Optional, paired with `serve` — one without the other is refused — and there is no sleep to fall back on |
| `run` | Runs one spec. `{}` is where the spec's path goes, and a template without it is refused. Required |
| `frames` | Where the harness writes, relative to the repo root. Fleet reads it after the run and keeps what it finds. Required |
| `never` | Paths a spec must never visit. Optional, and Armada does not enforce it |

**Command lines and a directory, and not one of them is Armada's.** Nothing in Fleet knows what a browser is or which framework wrote the frames, the same way nothing in the Checks runner knows cargo from pnpm. Reaching a state is what an end-to-end test already does, so the repository's existing harness is the mechanism and Armada gains no capture stack of its own. A repository needing a pipeline writes a script and names the script — there is no shell, so `run` cannot pipe or chain.

**`ready` is a command and not a duration, where it is declared.** A number would be a guess against a machine somebody else is using, and what it produces is a frame of a blank page that looks exactly like a frame of a broken one.

**Starting a server is one row's detail, not the shape.** `serve` and `ready` describe a web application; a desktop app's driver launches its own process and owns it directly, a CLI's `run` is the whole invocation, and neither needs a port — so a repository with nothing to serve declares `evidence:` with `run` and `frames` alone. How Armada shows its own Bridge is still open, at #606.

### One run, in the Job's own worktree

Fleet runs `run` once, against the Job's own worktree, and reads `frames` after — there is no comparison against a checkout of the base commit. An earlier design served the base and shot the same spec at it so a reviewer saw before and after, but the pairing only worked where Armada could serve the old code on a port, which ruled out exactly the repositories `serve` and `ready` are now optional for. That code stays on `main` — `crates/fleet/src/basing.rs`, and the base-run method `crates/fleet/src/showing.rs` no longer calls — reachable by nothing, so it returns once something can tell `run` which tree it is aimed at, which is a design of its own.

**A spec captures and never compares.** `run` writes frames into the directory `frames` names and Fleet is what keeps them. A spec that reaches instead for its own runner's snapshot comparison — Playwright's `toHaveScreenshot()` and its kin — keeps baselines of its own and fails on a first run and on any difference, which answers a different question than the one a step whose evidence is what it looks like asks.

**`never` is honest about being a comment.** Nothing here drives a browser, so there is no navigation to intercept. What reads it is the block a Drone writing the spec is given, and the reviewer reading the spec against it. That is weaker than an interception, and the alternative is a key that reads as a guarantee.

**A section this repository does not declare is not a default.** A workflow with a `shown` step resolved against a Manifest with no harness is refused before anything is dispatched — so it reaches a person as a sentence rather than as a Job that cut a worktree, spawned a Drone and captured nothing.

### A Drone attesting to its own work

Every other evidence type is checkable by something other than the Drone. A screenshot of the wrong state looks exactly like one of the right state.

**The spec is what makes it checkable.** The Drone names a spec and never hands over a file; Fleet runs it and owns the frames. The spec is code, it lands in the diff next to the change, and a wrong frame comes from a wrong spec somebody can read. Weaker than a test, much stronger than an image with no provenance — and it is why a frame carries no caption. A sentence saying what a frame shows would be the attestation put back in a field nothing can check.

## Ports

A fourth definition registry, alongside Checks, Commands and Evidence. It defines rather than narrows, which is why it sits at the top level and not under permissions.

**Ports union across a Convoy's declaring Manifests, qualified by Manifest id.** The direction is stated here because the section cannot carry it: the permissions and knowledge sections exist so a setting inherits its Convoy direction from where it sits, and Ports is a third direction on a two-direction boundary.

Why: a port is knowledge rather than authority. Injecting a port number grants no ability the Drone lacked, since it could already bind any port, and the allowlist is blast-radius reduction rather than a sandbox. Commands made the identical move once namespaced by Manifest id.

Each entry has a name and these fields:

| Field | What it is |
| --- | --- |
| the name | What `${port.NAME}` resolves, and what an assignment keys to |
| `container` | The container-side port — the join key Armada matches on |
| `env` | The variable name the repo's own stack already expects |

`container` is what Armada matches on when it rewrites a compose document, so it knows which published port is which.

**`env` exists because a command string covers one channel and there are more.** A `package.json` script Armada invokes but did not author, a compose stack reading a variable, and a Pilot shell all read the environment rather than a command line.

`ARMADA_PORT_<name>` is emitted alongside regardless, so a name with no `env` still gets a variable and anything generic has one guaranteed form. Requiring `env` would be a coupling the repo has to accept in the one channel whose whole purpose is working with a repo untouched.

**Colliding `env` names across a Job's Manifest set are rejected at claim time.** Not at config load, and not scoped to a root: two independent roots can live in one repo, and a root-scoped check never compares them. The same check already exists for Manifest ids that mangle to one environment token.

Exact key naming and nesting is tracked in `../contracts/configuration.md`.

**A span outlives an interrupted Job and is released by the Job ending, not by anything about ports.** A claim lasts as long as its worktree; an interrupted Job holds both until a person answers its escalation and it reaches a terminal, after which retention sweeps the worktree and the span goes. A server started with no Job holds a claim of its own, released when the server stops.

Nothing here asks a person to release a port — there is no such action, and adding one would offer a control for a decision they are already making elsewhere. Past a threshold Fleet surfaces the hold so accumulation is not discovered at exhaustion. [Fleet](fleet.md) owns the mechanism.

## Setup (Scan → Proposal → Write)

Carried forward from the v1 prototype, where the pattern worked well.

| Stage | What happens |
| --- | --- |
| Scan | Read-only evidence gathering across the workspace. Nothing is written yet |
| Proposal | A *possible* `armada.yml`, iterated on rather than approved or rejected |
| Write | Commits the current state of the proposal to `armada.yml` |

**Scan** reads lockfiles, package scripts, CI config, docker-compose services, pyproject tool sections and workspace globs.

**A dispatched Scan is not a [Job](job.md)**, even where it is agent work: it carries no worktree and no resolved toolset, and a Job's `owner_manifest_id` is always present, which a repo with no `armada.yml` cannot supply. It is agent work Fleet runs outside the Job model, the same category as the [Judge](judge.md) and the [Job proposer](job-proposer.md).

**Proposal is not a single approve-or-reject draft.** For a clean, well-evidenced workspace, the proposal is often already the final file. For a gappier one it is a starting point you iterate on — editing it yourself and/or working it through with an agent — before it is something you'd want committed.

**At every stage of iteration, every line stays strictly evidence-backed**: it traces to something an actual file already said. Nothing inferred or guessed.

**Write is not a mode chosen in advance.** There is no separate "Write path" decision made up front, for a single workspace or for a whole monorepo batch. Write always writes whatever you've arrived at through Proposal/iteration, whether that took zero rounds of iteration or several.

### Monorepo onboarding

For a repo with multiple workspaces, Setup asks orientation questions once, before Scan runs — informational, guiding the flow, not written to `armada.yml`:

- **Which discovered workspaces do you want to set up now?** Onboarding isn't all-or-nothing — scope this batch to whichever subset you want, come back for the rest later.
- **Do any checks run at the root of the repo, outside any single workspace?** Feeds the root `armada.yml` (see Workspace Mapping) if so — both its commands and its Checks.

Proposals for the workspaces selected for this batch are presented together, each line traceable to something Scan actually found.

Iteration on any individual workspace's proposal — and Write for it — happens at whatever pace makes sense. A workspace with rich evidence might Write immediately; one with thin evidence might sit as a partial proposal until you're actually working in it, same as reaching for Update later.

**Explicitly not asked at Setup:** whether workspaces are coupled or dependent on each other, or how you push code changes (batched vs. per-workspace). That is a property of the *specific change* being made, not a fixed property of the repo — see Cross-Workspace Jobs.

## Verify

Run against an existing Manifest. Does both:

- **Drift detection** — re-scans and flags if `armada.yml` is out of sync with the current repo state (new scripts added, old ones removed), without changing the file
- **Live dry-run** — actually runs Setup and Checks once, to confirm they still work

## Update

Manual editing only — no re-scan involved. You edit the Manifest directly, same as any other setting. Setup and Verify both involve scanning; Update does not.

## A Manifest a Job creates

A Job can author an `armada.yml` — adding a service, extracting a package. Two questions look like one and are not: *do the new Checks pass*, which is about this work and is answered now, and *who gates this path from here*, which is a durable policy statement.

**Its Checks run before the creating Job lands.** Fleet finds a newly created `armada.yml` in the diff and runs the Checks it declares once, at that Job's final mechanical verification, attributed to the new Manifest.

Nothing in a `WorkflowDef` declares this and no workflow opts in — the trigger is a property of the diff, so it applies to all of them. Without it a Job could add a service together with its test suite and land with that suite never having run.

**It does not gate the creating Job.** `gate_manifest_ids[]` is resolved at dispatch and does not gain a Manifest the Job authored; the new file starts gating from the next Job dispatched. That list says who gates a path, not what ran.

The same freeze holds everywhere else — the `WorkflowDef`, `acceptance_criteria` and Check definitions are all fixed before the work starts, precisely so the yardstick cannot move under it, and a Job adding its own file to its own gate list is that move.

**Nothing is unguarded in the meantime.** Nearest-ancestor owns those paths until the new file exists, so the root's or the parent Workspace's Checks gate the diff that creates it.

**The atomic case needs no special handling.** An extraction landing as a [Convoy](convoy.md) would otherwise have to add a gate mid-Job, and would need an `id` for a Manifest that does not exist yet. Neither arises: the gate list does not move, and the new `id` matters from the next Job onward.

**The adversarial version closes by construction rather than by approval.** A Drone weakening a Check that gates it is a live concern, and a Drone authoring one that would gate it is the same shape. Here it cannot, because nothing it writes joins the list mid-flight.

What its new Checks get instead is a single run, and a Judge reading those tests as part of the diff, where the gaming patterns already look for a tautological or weakened test.

## Deletion

A Workspace can be removed and its `armada.yml` with it. Nearest-ancestor reverts those paths to the root automatically, so the file side needs nothing.

**The record side needs nothing either, because the record was never tied to the file.** A Manifest's row in the store is not deleted when its file is. It stops resolving — nothing is found at its path — and keeps its `id` and its last known name.

Every historical [Job](job.md) carrying it as `owner_manifest_id` or in its gate list still shows what it ran against, and every Evidence row naming it stays readable, which is what append-only was for. A surface renders the name and marks it as no longer present; nothing renders a bare `id`.

**So there is no deletion event to observe.** Nothing watches for an `armada.yml` disappearing and nothing needs to. An unresolvable Manifest is a state rather than an occurrence, discovered the moment something tries to resolve it — Verify notices on a re-scan, and nothing else has to.

**A not-yet-started Job whose owner no longer resolves is refused at dispatch.** There is no configuration to resolve against, so it cannot run.

It needs no status of its own and no third `queued` reason: dispatch is human-gated, so the refusal reaches a person at the moment they approve it, naming the Manifest that is gone. Rejected: reassigning such a Job to the nearest surviving ancestor, which would make the record say the Job ran against something it did not.

**Re-creating an `armada.yml` at the same path does not restore the identity.** `id` is explicit so that a Workspace can move without dangling its history, and a new file carries a new `id` unless it is written with the old one. Carrying it deliberately is how a Workspace is restored rather than replaced.

## Secrets

**Drones never hold secrets directly.** That constraint is Fleet-side and belongs to [Fleet](fleet.md) and the Adapters spec, not to any config tier — a Drone never holds a callable secrets tool at all. Manifest only scopes *which* secrets and providers are available to this specific project; Fleet still brokers the access.

## Skills, Sub Agents, Allowlist

Extends or restricts [Kit](kit.md) defaults — Kit holds the default set, a Manifest declares its own, and nothing above the Manifest constrains it. Applies to the allowlist, Skills, and **Sub agents** (project-specific definitions alongside Kit's global ones).

**How they actually merge is not decided.** No merge strategy exists in the Configuration Settings registry for Skills, MCP or Sub agents, and Kit and [Drone](drone.md) disagree on Sub agents outright — Kit describes them as layered on top, Drone's Convoy table puts them under intersection.

All three are currently placed by inferring the inheritance axis from the peer axis, which is not a decision (see Open questions).

Kit's global **Agent file** ("how I work" — personal, cross-project) has **no Manifest-level counterpart**. A repo's own agent file just lives naturally in the repo itself, alongside this Manifest — Armada doesn't inject or manage it separately.

### What's frozen vs. live

Skills/MCP/Agent-files/Commands are frozen into a Drone at spawn time, a process boot-time constraint. Allowlist and dispatch freeze are enforced live at every gated checkpoint. The budget cap is enforced live too, and since 8 Sept 2026 a Manifest states it — the dollar half then, both halves from 9 Sept — live at every tier, and frozen at none, because a Job past its cap is refused again at every admission until a number moves. See below.

Checks split: a Check that existed at spawn is frozen for the life of the Job, while a Check added mid-Job gates immediately — additive-only, and for a different reason than the boot-time constraint above. Full detail on [Drone](drone.md) and [Fleet](fleet.md).

## Budget — a Manifest states both caps

**A Manifest may state `drone.cost_cap_micros_per_job`**, in millionths of a
dollar, **and `drone.turn_cap_per_job`**, in turns. Each sits between
[Machine](machine.md)'s number and the Job's own. Stating nothing defers upward;
stating zero says no Job here starts anything, which is how a repository is held
without stopping the Fleet the other repositories run under.

**This section said the opposite until 8 Sept 2026**, and the sentence it argued
for was that there is one cap, so no precedence rule and no override. What
falsified it was a Job refused its last step at $5.28 against a $5 compile-time
constant, with no lever anywhere. An earlier Manifest-only row had been deleted
for good reasons that still hold — it declared a *full override* of the Machine
cap with no stated precedence, which is two independent caps rather than a tier.
What is here now is a tier: one order, written once, in `fleet::Allowance::at`.

**The turn cap took the same tier a day later, and for the same reason.** It
stayed Machine's alone on the argument that a repository able to raise its own
would be opting out of the one signal reading steadily across cache warmth. Job
`01M22TYSAE0023MADDP5ZQEYGW` is what falsified that: it finished, passed every
Check, and stopped at 393 turns against a 300 constant with a cheap final step
unrun and its branch committed by hand. A ceiling nobody can move is not a
policy. What keeps a repository from opting out is not the absence of a lever:
it is that a ceiling somebody had to raise is a ceiling somebody read.

**Two things the deleted row was carrying that are still true.** Verification
spend counts against the cap, because a cap that excluded the Judge would
understate what the Job cost; and a cap is a resource setting rather than a
safety one, so nothing takes the minimum across a Job's gating Manifests — the
owning Manifest's number is the one that applies. Peer polarity records that a
lower cap is the stricter one, for whatever consults it later.

## Dispatch freeze

A Manifest-level toggle to pause/freeze **all** dispatch for this project — during a release freeze, for example. It is independent of, and layered on top of, the existing per-Job approval gate.

**Cross-workspace interaction:** for the **linked-DAG** shape, no special-case logic is needed. If a frozen Manifest's Job is blocking a dependent Job in another (unfrozen) workspace, that dependent Job simply stays `blocked_by_dependency` until the frozen Manifest unfreezes and its Job actually completes — falling out of the existing DAG dependency status_reason, not a new mechanism.

**That reasoning does not cover a [Convoy](convoy.md)**, which has no children and so no dependent Job to block.

**Most-restrictive-wins.** Any frozen Manifest in a Job's gate list freezes the whole Job. Why: a freeze means do not touch this project, and the Job would touch it.

That answers dispatch. **A freeze landing on a Convoy already running is still unresolved** — freeze is enforced live at every gated checkpoint, not only at dispatch — see [Convoy](convoy.md), Open questions.

## Auto-merge and review gate

Both are per-Manifest, not global, and both use the same override pattern one level apart in the pipeline.

| Setting | Values | Behavior |
| --- | --- | --- |
| `auto_merge` | `never` / `checks-pass` / `always` | Enforced by Fleet before merge |
| `review_gate` | `human_always` (default) / `auto_if_judge_passes` | Whether a workflow's final review step requires a human |

A false `auto_merge` result routes to Inbox > Job Reviews rather than merging.

`review_gate` decides whether the final review step can advance on a Judge pass alone — see [Workflow](workflow.md), the `advance_gate` field.

**Across a Job gated by several Manifests, most-restrictive-wins for both**: `never` beats `checks-pass` beats `always`, and `human_always` beats `auto_if_judge_passes`. There is one PR, so the most cautious gating Manifest holds.

**Both are read at the question, never frozen onto a step.** A step declares `manifest_rule:auto_merge` or `manifest_rule:review_gate` and the record keeps the key rather than the answer — the settings rows call both policies *Live*, so a value written onto a Job at creation would go on stating a decision the repository had since changed. Fleet resolves `review_gate` at the advance gate and `auto_merge` on the sweep over an open pull request.

**`checks-pass` is the forge's checks, not Armada's.** A Check named in `armada.yml` has already run at the gate the Job is holding at, and totalling the two would claim a gate had held that never ran. Only *every check passed* is a pass: a repository whose forge runs nothing has proved nothing, and a check that finished in a word Armada has no name for counts as not passed.

**`auto_merge` does not read an approval, and `always` means always.** Its three values are all about machines; a person approving on the forge is neither, and whether that becomes a fourth value or a policy of its own is undecided. A forge that requires a review refuses the merge, so branch protection is the backstop and it is the forge's.

### A judgeless step under `auto_if_judge_passes`

A step may declare `manifest_rule:review_gate` and no Judge criterion — Code Review's `deliver` step does, and it is legal. **Where the policy then resolves to `auto_if_judge_passes`, the step holds for a person anyway.**

The gate names a tier the step never declared, so advancing would advance on the mechanical tier alone while reading as judged. That is what a workflow file spelling `auto_if_judge_passes` outright is refused for at parse time; it cannot be refused there through a policy, because the file does not say what the policy is. So it is refused where the resolution happens, and refusing means holding rather than failing: the step passed every tier it declared, and nothing a Drone can do from a worktree would fix a configuration mismatch. Fleet writes a line into the Job's log naming the file to fix.

## Still open

The engineer-facing walk from "add a new repo" to a working Manifest is designed as the Set Up a Project (Manifest) journey — tracked there, not as an open item here. The full set of dials scoped to a Manifest is a row in the Configuration Settings registry; `../contracts/configuration.md` owns the tiering rule, which is why no list of them appears in prose here.

## Open questions

- **[manifest-root-lockfile-ownership]** Does the root `armada.yml` *own* a shared lockfile it can gate, as opposed to merely being able to gate it? The nearest-ancestor rule settles gating and not exclusivity.
- **[manifest-check-timeout-raise-or-lower]** May a per-Check timeout field raise the configured bound, or may it only lower it?
- **[manifest-kit-merge-rules]** What are the Kit→Manifest merge rules for Skills, MCP and Sub agents? No merge strategy exists in the Configuration Settings registry for any of the three, and Kit and Drone disagree on Sub agents outright — Kit describes them as layered on top, Drone's Convoy table puts them under intersection.
