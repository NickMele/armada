# Job Board

**What it is:** Not-yet-started Jobs for one Manifest — queued, blocked, waiting on resources, or waiting on your approval.

---

**Kind:** Surface.

Defines the Job Board — the engineer-facing surface for browsing and understanding not-yet-started work. Companion to the main Armada brief and the Monitoring & Review user journeys.

## What it's for

Your primary lens on **"what's queued and why"** — two equally weighted purposes: browsing/picking work to dispatch, and understanding dependencies (why something hasn't started).

## Scope

Not-yet-started Jobs only — queued, blocked, waiting-on-resources, or waiting-on-approval. Once a Drone picks a Job up, it moves to Active Jobs and comes off the Board. The Board is not a full-lifecycle view; Activity Feed and Active Jobs cover running/completed state.

**Manifest scope:** per-Manifest — you pick a Manifest first and the Board scopes to that Manifest's Workspace. Not a unified cross-Manifest view.

**Settled Aug 2026 — the Board scopes by `owner_manifest_id`.** A Job's old `manifest_id` / `manifest_ids` field split into an owner (exactly one, always present), a gate list (Manifests whose Checks must pass, possibly empty) and a write list. The Board keys off the owner, so every Job — [Convoy](convoy.md) included — has exactly one Board and appears on it. The three candidate answers this section previously carried (render under each declared Manifest, under one nominated Manifest, or not at all) are closed by the split.

**A Board still exists where the Workspace does not.** A Manifest's record outlives its `armada.yml`, so a Job whose Workspace has been deleted still has exactly one Board and renders under its last known name, marked as no longer present. Approving such a Job is refused at that moment, naming the missing Manifest — there is no configuration left to run it against. It carries no status or reason of its own for this: the gate a person is already standing at is where the refusal belongs. See [Manifest](manifest.md), Deletion.

**The gate side resolves along safety versus resource (Aug 2026).** Dispatch freeze, `auto_merge` and `review_gate` are most-restrictive-wins across the gating Manifests: any frozen Manifest freezes the Job, `never` beats `tests-pass` beats `always`, `human_always` beats `auto_if_judge_passes`. The budget cap follows `owner_manifest_id` rather than the minimum, so a small Workspace's cap cannot kill a Convoy several times larger. Still uncovered: what a freeze does to a Convoy already running, since freeze is enforced live rather than only at dispatch — see [Convoy](convoy.md). (Convoy also establishes that a repo is not a Workspace, which is why this section no longer says "project/repo".)

**Sub-dispatched Jobs are almost never here (decided Aug 2026).** A Job carrying `dispatched_by` is created and dispatched inside an already-approved Job — it does not queue and never needs approval, so it is running before anything could render it. The one exception is `waiting_on_resources`, a not-started reason unrelated to approval: a sub-dispatched Job with no headroom sits in exactly the state this Board is scoped to show. It is rendered, **read-only** — see Dispatch Flow. `blocked_by_dependency` is narrower than a second exception rather than being one: sub-dispatch **by itself** creates no `depends_on`, because `dispatched_by` is provenance, not a `dependencies` link — so being sub-dispatched is never on its own a reason to be blocked. A sub-dispatched Job may still carry a `depends_on` of its own: one step spawning two sub-Jobs where the second depends on the first is ordinary. Where it does, it is blocked like any other Job and belongs on the Board. See [Job](job.md).

## Layout

| View | Role |
| --- | --- |
| Flat list | Default landing view — sortable/filterable |
| DAG / graph view | Opt-in toggle, for inspecting dependencies — not the default |

Default view (list vs. graph) is user-configurable, not fixed — see Configuration below.

**The graph view has no Convoy case (see Open questions).** The graph view renders `dependencies`. A [Convoy](convoy.md) has none of its own by construction — but it **may be a peer node** in someone else's, decided Aug 2026, and the Job proposer may emit a graph containing one. So a Convoy in the graph is expected rather than exceptional, and whether it renders as an ordinary node, is visually distinguished, or expands its declared Workspaces is unstated.

## Job status on the Board

A not-yet-running Job is in one of two statuses, and only `queued` carries a reason.

| Status | Reason | Meaning |
| --- | --- | --- |
| `awaiting_approval` | none | Waiting on your explicit Job-level dispatch approval |
| `queued` | none | Approved. Nothing blocking but its turn |
| `queued` | `blocked_by_dependency` | Waiting on another Job in the DAG |
| `queued` | `waiting_on_resources` | CPU or memory headroom, or a port span, not currently available |

**Approval is a status, not an axis.** It was one half of a pair on `not_started`, and the pair existed because a Job can be approved and out of resources at once while one four-value field could say only one of them. Promoting approval to a status removes the tension rather than arbitrating it, and `ready` becomes the absence of a reason rather than a value. A sub-dispatched Job is created at `queued` with its reason naming the resource, so the combination that used to compute as `pre_approved_queued` and never render cannot arise. `queued`'s reason is recomputed rather than stored, so a held port span keeps saying so instead of going stale. States and reasons live on [Job](job.md).

**The original split is what made this reachable**, and its symptom is the diagnosis worth keeping. The symptom that forced the split: a sub-dispatched Job is always approved, so one out of headroom computed as `pre_approved_queued` and — by the visibility rule in Scope — never rendered at all. A Job made invisible because two labels collided.

The `snake_case` forms are the stored values defined by [Job](job.md); Title Case is the rendered label.

Blocked Jobs are shown (not hidden), rendered in a visually distinct state (e.g. greyed out).

## Origin tagging

Every Job shows a visible tag for where it came from:

| Origin | Source |
| --- | --- |
| Auto-detected | Failure or issue report surfaced the Job automatically |
| Manual | You created it directly |
| Helm-drafted | Proposed by a [Helm](helm.md) session, still requires your explicit approval like any other Job dispatch |
| Workflow-triggered | Created by a finished Job's `on_complete`. **Not already approved** — the Job that named it is over, so there is no approved parent for it to sit inside and it is created at `awaiting_approval` like any Job-level dispatch. It appears here and takes the ordinary gate. It carries no `dispatched_by`, linking back through `subject` instead, which is why it is the one origin Fleet does not write from that field |
| Sub-dispatched | Spawned by a step of another Job, carrying `dispatched_by`. **Already approved as part of its parent** — so it is created at `queued` rather than `awaiting_approval`, and its reason determines whether it renders here: it appears only while `waiting_on_resources`, and read-only. Under the old single-field model this was the case that broke, since a sub-dispatched Job out of headroom computed as `pre_approved_queued` and never rendered. See Job status on the Board above |

The `snake_case` form (`auto_detected`, `manual`, `helm_drafted`, `sub_dispatched`, `workflow_triggered`) is the stored value defined by [Job](job.md); the Title Case form above is the rendered label.

**No origin value or shape field distinguishes a Convoy on this Board (see Open questions).** Where Job records the Job proposer's choice is now settled: **nowhere**. Shape is derived from a Job's `write_targets` and its `atomic` flag rather than stored, so `origin` was never going to carry it and no shape field exists to read. The consequence for this surface stands unchanged — a Convoy is visually identical to a single-workspace Job on the very surface whose stated purpose is "what's queued and why" — and the Board must now compute the distinction from those two fields rather than render a stored label. What surface approves or overrides the Job proposer's proposal is tracked separately — see [Convoy](convoy.md) — and is not resolved here.

## Dispatch flow

Selecting a ready Job opens a **Job detail view** first — approval is a deliberate, separate action from there, not an immediate action on click. This keeps the one-by-one approval discipline (see the Armada brief on Drone dispatch control) intact even when browsing quickly.

**The card names a write-scope overlap (decided Aug 2026).** Where this Job's `write_targets` intersect those of a Job already running under the same root, the detail view says so, names the other Job, and offers to make this one `depends_on` it rather than run alongside. Approving anyway is allowed and is the ordinary case — the overlap is a fact, not a verdict, and two Jobs touching one package usually merge fine. Taking the offer creates an ordinary dependency edge, so the Job returns here at `queued` with `blocked_by_dependency` and no new reason exists for this. See [Fleet](fleet.md), Scheduling and gating.

**A sub-dispatched Job's row is read-only (decided Aug 2026).** No Approve, no Dispatch — the approval already happened at its parent, so the controls would offer a decision that has no content. It is rendered visually distinct, the same treatment blocked Jobs get. **Kill stays available**, because unlike Approve it is not meaningless: killing a sub-dispatched Job has a defined effect on the dispatching step, and a human who can see it stalled on resources should be able to stop it without first finding the parent.

**This card is where the [Job proposer](job-proposer.md)'s proposal is approved or overridden.** A Job dispatched from a prompt or a ticket link arrives here with its workflow and its scope already proposed, and the card is where either is changed. Why this gate and not one of its own, and what approving a proposal does and does not dispatch, are on that document. What this surface looks like is deliberately undecided; no journey has UI design started and this one is design order 1 — see [Convoy](convoy.md), Open questions.

**Two** distinct things are called approval on a Job's path, not three: the dispatch gate here — which covers the Job proposer's proposal, the Job's own dispatch, and any mid-flight scope widening — and a workflow's own human gate over finished work.

## Configuration

The following settings (see `../contracts/configuration.md`) directly affect this concept:

| Setting | Scope |
| --- | --- |
| Job Board default view (list vs. graph) | Machine |
| Job Board default sort/filter | Kit → Manifest |
| Landing Manifest / last-opened project | Machine |

## Open questions

- **[job-board-convoy-graph-rendering]** Given the graph view exists, how does a Convoy — which may be a peer node in the dependency graph but has no children of its own — actually render there? As an ordinary node, visually distinguished, or expanded into its declared Workspaces is unstated.
- **[job-board-convoy-list-distinction]** How is a Convoy visually distinguished from a single-workspace Job in the default flat-list view? No origin value or shape field carries the distinction — the Board must compute it from `write_targets` and `atomic` rather than render a stored label, and no visual treatment exists yet.
