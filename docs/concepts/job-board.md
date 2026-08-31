# Job Board

**What it is:** Not-yet-started Jobs for one Manifest — queued, blocked, waiting on resources, or waiting on your approval.

---

**Kind:** Surface.

Defines the Job Board — the engineer-facing surface for browsing and understanding not-yet-started work. Companion to the main Armada brief and the Monitoring & Review user journeys.

## What it's for

Your lens on **every Job and what it is doing** — picking work to dispatch, watching what is running, reading what finished, and understanding why something has not started.

## Scope

**Every Job, at every status, with state as a filter.** A Job does not leave the Board when a Drone picks it up; the filter is what narrows the list, and the filter is a control rather than a routing decision.

**Reviews is a filter here, and a fast one.** A Job at `awaiting_review` is a Job, and a queue of them is this list filtered to that status. It is not a destination of its own: it was one, and folding it in is what makes "what needs me" a single gesture rather than a second place to check.

**A filter changes a row's field set, and that is not the row dropping a field.** A Job at review is at its last step with its cost settled, so a step bar and a spend figure say nothing there — the Reviews field run was drawn without them. Field sets already vary by status; the rule that no field is dropped is about width, never about state.

**One list rather than four surfaces.** Splitting not-yet-started, running, awaiting-review and finished across separate destinations divided one population four ways, made a person learn which surface held which state, and gave four lists of Jobs four chances to disagree about what a Job row looks like. What used to be Active Jobs is this list filtered to what is running, what used to be Reviews is it filtered to `awaiting_review`, and there is no Activity Feed: finished Jobs are this list filtered to what is over.

Why it matters that this is a filter: the state a Job is in changes constantly and is not where a person's attention lives. Scoping by Manifest is a place; scoping by state is a question, and a question belongs in a control.

### One Board per Manifest

**The Board is per-Manifest.** You pick a Manifest first and the Board scopes to that Manifest's Workspace. It is not a unified cross-Manifest view.

**The Board scopes by `owner_manifest_id`.** A Job's old `manifest_id` / `manifest_ids` field split into an owner (exactly one, always present), a gate list (Manifests whose Checks must pass, possibly empty) and a write list. Every Job — [Convoy](convoy.md) included — has exactly one Board and appears on it.

### A Board outlives its Workspace

**A Board still exists where the Workspace does not.** A Manifest's record outlives its `armada.yml`, so a Job whose Workspace has been deleted still has exactly one Board and renders under its last known name, marked as no longer present.

Approving such a Job is refused at that moment, naming the missing Manifest — there is no configuration left to run it against. It carries no status or reason of its own for this: the gate a person is already standing at is where the refusal belongs. See [Manifest](manifest.md), Deletion.

### Safety versus resource on the gate side

**The gate side resolves along safety versus resource.** Dispatch freeze, `auto_merge` and `review_gate` are most-restrictive-wins across the gating Manifests: any frozen Manifest freezes the Job, `never` beats `tests-pass` beats `always`, `human_always` beats `auto_if_judge_passes`. The budget cap follows `owner_manifest_id` rather than the minimum, so a small Workspace's cap cannot kill a Convoy several times larger.

Still uncovered: what a freeze does to a Convoy already running, since freeze is enforced live rather than only at dispatch — see [Convoy](convoy.md).

### A sub-dispatched Job is here like any other

**A Job carrying `dispatched_by` appears on the Board the same as one you dispatched.** It is created and dispatched inside an already-approved Job — it does not queue and never needs approval — so it is usually running by the time anything renders it, and running is a filter rather than an absence.

**Its row is read-only, and that is the only thing different about it.** No Approve and no Dispatch: the approval happened at its parent, so those controls would offer a decision with no content. Kill stays. See Dispatch flow.

`dispatched_by` is provenance rather than a `dependencies` link, so sub-dispatch **by itself** creates no `depends_on`. A sub-dispatched Job may still carry one — one step spawning two sub-Jobs where the second depends on the first is ordinary — and where it does it is blocked like any other Job. See [Job](job.md).

## Layout

| View | Role |
| --- | --- |
| Flat list | The Board. Sorted and filtered, state among the filters |
| DAG / graph view | Opt-in toggle, for inspecting dependencies — not the default, and not scheduled |

Default view (list vs. graph) is user-configurable, not fixed — see Configuration below. The graph view is unscheduled: the graph a person asked for is a Job's own workflow, which is drawn on job detail — see [Monitor Active Work](../journeys/4-monitor-active-work.md).

**The graph view has no Convoy case (see Open questions).** The graph view renders `dependencies`. A [Convoy](convoy.md) has none of its own by construction, but it **may be a peer node** in someone else's, and the Job proposer may emit a graph containing one.

A Convoy in the graph is expected rather than exceptional, and whether it renders as an ordinary node, is visually distinguished, or expands its declared Workspaces is unstated.

## Job status on the Board

### The default sort is Critical first

**Critical first: the needs-you cluster, then oldest within every group.**
Drawn 2026-08-31, answering `[job-board-sort-order]`, which had carried no body
since it was filed. Oldest-first remains in the control as the other option.

The sort agrees with the filter rather than cutting across it: the tab that
groups the statuses which stop until a person reads them is the same set the
default sort lifts to the top, so changing the tab reorders nothing a person had
learned. The setting's key is `job_board.default_sort`.

## Job status on the Board

**Every status a Job can hold appears here**, and the filter decides which are on screen. Two of them carry the Board's own reading, and only `queued` carries a reason.

| Status | Reason | Meaning |
| --- | --- | --- |
| `awaiting_approval` | none | Waiting on your explicit Job-level dispatch approval |
| `queued` | none | Approved. Nothing blocking but its turn |
| `queued` | `blocked_by_dependency` | Waiting on another Job in the DAG |
| `queued` | `waiting_on_resources` | CPU or memory headroom, or a port span, not currently available |

**Approval is a status, not an axis.** A Job can be approved and out of resources at once, and a status plus a reason says both. `ready` is the absence of a reason rather than a value.

**A sub-dispatched Job is created at `queued` with its reason naming the resource**, so the combination that used to compute as `pre_approved_queued` and never render cannot arise.

**`queued`'s reason is recomputed rather than stored**, so a held port span keeps saying so instead of going stale. States and reasons live on [Job](job.md).

The `snake_case` forms are the stored values defined by [Job](job.md); Title Case is the rendered label.

Blocked Jobs are shown rather than hidden, stepped down to `--border-subtle` and `--fg-subtle` — dimming is a token step, never an alpha.

**A filter that empties the list says which filter did it.** A Board with nothing on it under a filter is not the same as a Manifest with no Jobs, and the empty state has to tell those apart — see [Bridge](bridge.md).

## Origin tagging

Every Job shows a visible tag for where it came from.

| Origin | Source |
| --- | --- |
| Found by Fleet | Failure or issue report surfaced the Job automatically |
| Dispatched by you | You created it directly |
| Drafted in Helm | Proposed by a Helm session |
| Workflow-triggered | Created by a finished Job's `on_complete` |
| Sub-dispatched | Spawned by a step of another Job, carrying `dispatched_by` |

A Job drafted in Helm still requires your explicit approval, like any other Job dispatch — see [Helm](helm.md).

### A Workflow-triggered Job takes the ordinary gate

**A Workflow-triggered Job is not already approved.** The Job that named it is over, so there is no approved parent for it to sit inside, and it is created at `awaiting_approval` like any Job-level dispatch. It appears here and takes the ordinary gate.

It carries no `dispatched_by`, linking back through `subject` instead, which is why it is the one origin Fleet does not write from that field.

### A sub-dispatched Job is approved with its parent

**A sub-dispatched Job is already approved as part of its parent.** It is created at `queued` rather than `awaiting_approval`, never at a status that would put it in front of you for a decision, and it renders here at whatever status it holds. See Job status on the Board above.

### Origin is display-only

**A label on the row, never a filter axis.** Drawn 2026-08-31 as a filter and
rejected on the drawing, answering `[origin-tag-filterable-or-display]`. Both
readings were drawn side by side; the filter earned no question a person
actually asks. *What needs me*, *what is running* and *why has that not started*
are all state, and the Board's scope is already the Manifest — so an origin axis
adds a control that narrows by a fact nobody is narrowing on.

**And it names the condition that would reverse this.** One machine dispatches
almost everything on it, so origin is nearly constant today, and the one value
worth finding — sub-dispatched — is already legible because those rows name
their parent. It becomes an axis when agents draft Jobs in numbers.

It stays on the row because it answers a different question, asked after a row
is already found: where did this come from. A sub-dispatched row names its
parent Job there rather than only its origin.

### Stored value and rendered label

The `snake_case` form is the stored value, defined with the rest of the Job
schema in `crates/core-model/domain/job-fields.toml` — [Job](job.md) carries no
origin field of its own. The label above is what a row renders.

| Stored | Rendered |
| --- | --- |
| `auto_detected` | Found by Fleet |
| `manual` | Dispatched by you |
| `helm_drafted` | Drafted in Helm |
| `workflow_triggered` | Workflow-triggered |
| `sub_dispatched` | Sub-dispatched by `<parent job id>` |

**Three of the five read as sentences rather than labels**, settled on the
drawing and amended 2026-08-21 — see [Monitor active
work](../journeys/4-monitor-active-work.md). *Auto-detected*, *Manual* and
*Helm-drafted* were settings vocabulary naming modes the lexicon does not have;
*Found by Fleet*, *Dispatched by you* and *Drafted in Helm* say who acted. The
other two already named an act and are unchanged. Origin renders as plain sans
in `--fg-muted` and never as a chip, because a chip is a status.

### A Convoy's row names its first write target

**No origin value or shape field distinguishes a Convoy on this Board.** Shape is derived from a Job's `write_targets` and its `atomic` flag rather than stored — nothing on Job records a shape at all — so `origin` was never going to carry it and no shape field exists to read. **The Board computes the distinction from those two fields** rather than rendering a stored label.

**The row shows the first write target, then a count of the rest** — `+2` where three Workspaces are declared. Every other row names a place in that column, so a Convoy names one too. A bare count there was drawn and rejected: it puts a number where the column holds an identifier, and it gives the folder glyph a second meaning.

**A Convoy takes no chip and no hue.** A bordered pill is a Job state and nothing else, so shape reads as plain text.

How a Convoy renders in the graph view is separate and unsettled — see Open questions. What surface approves or overrides the Job proposer's proposal is tracked separately — see [Convoy](convoy.md) — and is not resolved here.

## Dispatch flow

**Selecting a ready Job opens a Job detail view first.** Approval is a deliberate, separate action from there, not an immediate action on click.

**So no row's control approves anything. A Job awaiting approval carries Review.** The built row shipped an Approve control, which contradicted this rule the whole time it was on screen; settled 2026-08-31 in favour of the rule. Review is the same word the `awaiting_review` row already carries, and it means the same thing in both places — *go read this* — because in both places the act itself happens on detail. The Board's keyboard model loses its Approve key entirely rather than moving it. Why: this keeps the one-by-one approval discipline intact even when browsing quickly — see the Armada brief on Drone dispatch control.

### Write-scope overlap on the card

**The card names a write-scope overlap.** Where this Job's `write_targets` intersect those of a Job already running under the same root, the detail view says so, names the other Job, and offers to make this one `depends_on` it rather than run alongside.

Approving anyway is allowed and is the ordinary case — the overlap is a fact, not a verdict, and two Jobs touching one package usually merge fine. Taking the offer creates an ordinary dependency edge, so the Job returns here at `queued` with `blocked_by_dependency` and no new reason exists for this. See [Fleet](fleet.md), Scheduling and gating.

### Controls on a sub-dispatched row

**A sub-dispatched Job's row is read-only.** No Approve, no Dispatch — the approval already happened at its parent, so the controls would offer a decision that has no content. Its origin tag is what says so; it takes no dimming of its own, because being sub-dispatched is not a reason a Job is not moving.

**Kill stays available.** Why: killing a sub-dispatched Job has a defined effect on the dispatching step, and a human who can see it stalled on resources should be able to stop it without first finding the parent.

### Where the proposal is approved or overridden

**This card is where the [Job proposer](job-proposer.md)'s proposal is approved or overridden.** A Job dispatched from a prompt or a ticket link arrives here with its workflow and its name already proposed, and the card is where either is changed. Why this gate and not one of its own, and what approving a proposal does and does not dispatch, are on that document.

What this surface looks like is deliberately undecided; no journey has UI design started and this one is design order 1 — see [Convoy](convoy.md), Open questions.

### What is called approval on a Job's path

**Two distinct things are called approval on a Job's path, not three:** the dispatch gate here — which covers the Job proposer's proposal, the Job's own dispatch, and any mid-flight scope widening — and a workflow's own human gate over finished work.

## Configuration

The following settings (see `../contracts/configuration.md`) directly affect this concept:

| Setting | Scope |
| --- | --- |
| Job Board default view (list vs. graph) | Machine |
| Job Board default sort/filter | Kit → Manifest |
| Landing Manifest / last-opened project | Machine |

## Open questions

- **[job-board-graph-view]** What is the Board's graph view, and how does a Convoy render inside it? The Layout section above states an opt-in toggle beside the flat list, defaulting per Machine, and that much is settled. What is not: whether the view is a surface of its own or dependency affordances on the rows themselves, and how a Convoy renders once inside it — as an ordinary node, visually distinguished, or expanded into its declared Workspaces. A Convoy has no `dependencies` of its own by construction, though it may be a peer in someone else's. **Unscheduled.** The graph the owner wants is a Job's own workflow, drawn on job detail, and this one answers a question nobody is currently asking.
