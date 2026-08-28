# Convoy

**What it is:** Atomic multi-workspace Job shape — one Job, one Drone, one worktree spanning declared Workspaces, landing as one PR.

---

**Kind:** Shape.

Formalises Convoy — the atomic multi-workspace [Job](job.md) shape. This document is the citable source; [Manifest](manifest.md), [Job](job.md), [Fleet](fleet.md) and [Drone](drone.md) link here.

## What it is

A Convoy is **one Job**: one [Drone](drone.md), one worktree spanning a declared set of Workspaces. Each gating Workspace's Checks run and gate **independently against its own** [Manifest](manifest.md).

One combined human approval over the combined diff. One PR touching all declared Workspace paths.

### Derived, not stored

**Convoy is a derived shape, not a stored value.** Nothing on [Job](job.md) says "convoy". A Job **is** a Convoy when it has several `write_targets` and `atomic` set — several places to write, which must land as one unit.

`write_targets` holds paths; the declared set of Workspaces a Convoy spans is what those paths resolve to under nearest-ancestor ownership, not a second list. The name stays as vocabulary for people and for this document; the record carries the properties instead.

Several write targets **without** `atomic` is the wide-and-uncoupled case and is not a Convoy. **Counting Manifests does not identify a Convoy** — a root change gated by twelve Workspaces would count as one.

### No valid intermediate state

A Convoy exists for changes with **no valid intermediate state** — tightly coupled Workspaces that must land in a single commit. A parser/generator pair, coupled sibling packages, tightly coupled services. Confirmed as a real pattern from a live work monorepo, not a hypothetical.

### One root `armada.yml`

**Root-Manifest-scoped.** Every declared Workspace must descend from a single root `armada.yml`. A Convoy does not span repos, or two independent roots inside one repo — neither half of the definition is constructible there, since one worktree cannot span two repos and one PR cannot touch two.

Stated as one root rather than one repo because Armada knows roots and Workspaces, not repositories. Cross-root coupling falls to linked-DAG sub-Jobs. A dedicated multi-repo Job shape is tracked separately and is not designed.

### Revising the declared set

**The declared set is not fixed at dispatch.** It may be revised mid-Job by rescope-and-respawn: [Fleet](fleet.md) terminates the [Drone](drone.md), re-resolves configuration against the new Manifest set, and spawns a fresh Drone on the same worktree and branch. **A narrowing proceeds unchallenged; a widening returns to the dispatch approval gate first.**

**A revision bites hardest on a Convoy.** Permissions intersect across the gating Manifests, and intersection is monotone — widening a Convoy can only remove capability, so the respawned Drone may be less capable than the one that asked to widen. The revision must also stay inside the root bound above.

### A revision can change the shape

A revision can change the shape itself: dropping to one `write_target`, or clearing `atomic`, means the Job stops being a Convoy mid-flight. A revision records the change to `atomic` alongside the paths added and removed, so a Job's earlier shape stays reconstructable.

### Overlap with concurrent work

**A Convoy overlaps more concurrent work than a single-workspace Job, by construction.** Its declared scope is wider, so its `write_targets` intersect more of what else is running, and the approval card will name a collision more often for one.

The alternative to one Job spanning coupled Workspaces is several Jobs touching the same set, which collides identically and lands in pieces. Overlap is surfaced and never serialised; see [Fleet](fleet.md), Scheduling and gating.

### No children, a peer in the DAG

**A Convoy has no children.** It is a single Job with several `write_targets`, not a parent that decomposes into sub-Jobs. A Convoy **does not decompose** — with no children, nothing hangs below it.

**It may still be a peer node in Job's `dependencies` DAG:** `depends_on` / `blocks` sequence *peer* Jobs, not children, so having no children never ruled this out. A Convoy landing a coupled types/API pair with a UI Job downstream is the common feature shape, and it works because a Convoy completes as one unit — a dependent Job never observes a half-landed upstream.

### The fields a Job carries

**A Job carries `owner_manifest_id`** (exactly one, and the [Job Board](job-board.md)'s scoping key), **`gate_manifest_ids[]`** (may be empty) and **`write_targets[]`**. These replaced a single `manifest_ids` field, which answered four questions at once and whose entry count doubled as the shape discriminator.

## Three shapes, not two

**Job shape** is the category covering all three. **Cross-Workspace Jobs** is the narrower name for the two that actually span Workspaces: linked-DAG sub-Jobs and Convoy. A single-workspace Job is contained to one Workspace and belongs to neither.

Cross-Workspace Jobs is not a synonym for Convoy — linked-DAG sub-Jobs span Workspaces too and are not Convoys, and collapsing the two destroys the distinction the Job proposer exists to draw.

| Shape | Structure | How it lands |
| --- | --- | --- |
| Single-workspace Job | One `write_target`. Default — contained to one Workspace | Own worktree, own Checks, own approval, own PR |
| Linked-DAG sub-Jobs | **Separate Jobs**, one `write_target` each, linked by `dependencies` | Each sub-Job pushes **separately** — a PR train |
| **Convoy** | **One Job, no children.** Several `write_targets`, `atomic` set, one root `armada.yml` | One PR touching every declared Workspace path — **atomically** or not at all |

**Linked-DAG sub-Jobs span Workspaces, and are for sequenceable coupling** — land the API change, then update the consumer against what merged. The shape is recorded on `dependencies` alone; there is no `linked_dag` value anywhere, because a member is an ordinary single-target Job that carries a link.

The dependent Job stays `blocked_by_dependency` until the upstream one reaches `completed_success`, then becomes dispatchable but is not auto-dispatched. A Convoy is for coupling with no valid intermediate state.

## How a Convoy is chosen

You do not hand-draft the shape. A Convoy is what several `write_targets` with `atomic` set amount to, and both are settled by the workflow's scope step — so a Convoy is arrived at once the work has been read, not approved as a category at dispatch. The [Job proposer](job-proposer.md) owns why scope is not proposed earlier; none of it is restated here.

**The shape is never chosen directly.** There is no shape value to propose: it falls out of the pair, so a Convoy is arrived at by proposing coupling rather than by naming a category.

## What is already resolved

### One gating Workspace's Checks fail

**Whole-Job failure.** Any gating Workspace's Check failure fails the Job's single `workflow_status`, and the standard gate-failure retry flow applies to the whole Job. Why: Workspaces that cannot land independently do not fail independently either. See [Job](job.md).

### Retry accounting

**One shared `retry_count` per step**, across all gating Workspaces. There is no per-Workspace retry accounting.

### Schema impact on the nested workflow machine

**No change to the nested machine is needed for `workflow_status`.** Retry accounting and the `job_steps` rows are keyed per step rather than per Workspace, and that holds.

**Evidence did change.** Each entry carries a Manifest reference, and each gating Manifest carries an outcome of ran-and-passed, ran-and-failed, or did-not-run-and-why.

Without the first, this document's promise that each gating Workspace's Checks run independently had nowhere to record which one failed. Without the second, a root Check correctly skipped because the diff touched no root path is indistinguishable from one silently misconfigured.

### Which Manifest's config applies when the gating Manifests disagree

**Permissions intersect; knowledge unions.** The table lives on [Drone](drone.md).

Allowlist, secrets, **MCP and Sub agents** all resolve most-restrictive-wins (intersection): only ops allowed by *every* gating Manifest, only secrets *every* one grants, only servers and personas every one defines. A Drone unable to reach a secret because another gating Manifest withholds it is a visible, debuggable failure, not a silent scope violation.

**Commands are the exception:** they are namespaced by Manifest `id` and a Convoy Drone gets the **union** — `api:migrate` and `billing:migrate` are two commands, not one name with two meanings, so there is nothing to conflict.

**Skills and the Agent file union.** They are instructions, not permissions, so they grant no authority for the direction rule to protect, and intersecting them would be near-vacuous since Skills are repo-specific, leaving a Convoy Drone *less* capable than one working either Workspace alone.

**A Convoy Drone is spawnable because of these rules.** Four of the six items frozen at spawn had no rule governing them until this.

### One worktree, or one per Workspace with a coordinating layer

**One worktree spanning every declared Workspace** — forced by the Drone model, not chosen. A Drone is 1:1 with a Job and has one worktree on one branch; one worktree per Workspace means either multiple Drones or one Drone straddling branches, and neither can produce a single commit.

### Approval as one gate, or per Workspace with a final join

**One gate over the combined diff** — it falls out of whole-Job failure. If any Workspace's Check failure fails the whole Job on one shared `retry_count`, a per-Workspace approval gate offers a choice that cannot be acted on: rejecting one Workspace kills the whole Job anyway.

Grouping the diff per Workspace for review is presentation, not gate scope.

### Which Manifest's policy applies — freeze, auto-merge, review gate, budget cap

**Split along safety versus resource.** Dispatch freeze, `auto_merge` and `review_gate` are **most-restrictive-wins** across the gating Manifests: any frozen Manifest freezes the Job, `never` beats `tests-pass` beats `always`, `human_always` beats `auto_if_judge_passes`.

The **budget cap follows `owner_manifest_id`**, not the minimum. Why: taking the lowest lets a small Workspace's cap, sized for its own work, kill a Convoy several times larger for a reason unrelated to that Workspace. Over-caution on a safety setting costs a manual step; over-caution on a resource setting makes the work impossible.

**A freeze landing on a Convoy already running is not covered**, since freeze is enforced live rather than only at dispatch (see Open questions).

### Does the Judge evaluate each Workspace's slice, or the combined diff

**The combined diff, once.** The Judge follows the human approval gate rather than the mechanical tier. `acceptance_criteria` are written by the requester about the change, not about a Workspace, and nothing scopes a criterion to one — judging per Workspace would evaluate a criterion about consumers against a types package containing none.

A refusal must cite the specific evidence it refuses on, which is what sharper per-Workspace refusals would otherwise buy.

**The resulting asymmetry is deliberate:** Checks are per gating Manifest and attributed as such, while Judge and human approval are both combined. Cost accepted: a wide combined diff is where `max_context_size` bites first, and exceeding it escalates to a human rather than failing the step.

### Scope limit on the worktree and approval answers

Each holds inside one root `armada.yml`, where a worktree spanning declared Workspaces is ordinary git. Neither survives across roots in separate repos, which is why a Convoy is root-Manifest-scoped — see What it is.

## Open questions

- **[convoy-dispatch-approval-surface]** What does the dispatch-approval surface for a Convoy actually look like? The structural question — that a Convoy proposal merges into the dispatch approval gate, with each Job still taking its own one-by-one dispatch approval — was settled Aug 2026. The surface itself is deliberately still open, since Dispatch a Job is design order 1. One of the two related Board questions is closed on the design side (Aug 21 2026): a Convoy row shows its **first write target and a `+2`**, keeping an identifier in the identifier column rather than a count where every other row names a place; a Convoy takes **no chip and no hue**, since a bordered pill is a Job state and nothing else, so shape reads as plain text in the identifier column.
- **[convoy-board-graph-view]** Does the Job Board need a graph view for DAG dependencies, now that a Convoy may be a peer node in that graph? Related: the Board must **compute** the shape per row, since it is derived rather than stored.
- **[convoy-multi-repo-job-shape]** Does Armada need a multi-repo Job shape, and what is its workflow DAG? Carried forward from the root-scoping decision, which left it open, and no scenario currently pressures it.
- **[convoy-freeze-while-running]** What happens when a dispatch freeze lands on a Convoy that is already running? Named in the policy resolution above rather than filed as its own question until now: freeze is enforced live rather than only at dispatch, and the already-running case is not covered.
