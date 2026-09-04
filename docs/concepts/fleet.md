# Fleet

**What it is:** The Rust daemon that schedules, gates and verifies Jobs, and the only actor that writes a state transition on either a Job or a Drone. Everything else reports; Fleet decides. launchd-supervised, api in-process, no engineer-facing surface of its own.

---

**Kind:** Process.

Formalizes Fleet — the Rust daemon that manages the set of Drones working on Jobs. Companion to the main Armada brief.

## What it is

The actual execution engine underlying everything else in Armada. Fleet schedules, gates, verifies, and drives every Job and Drone state transition (see [Job](job.md) — Ownership Split, [Drone](drone.md) — Core Principle).

Fleet has **no engineer-facing surface** beyond Doctor's module status. It is pure infrastructure; you never "go to Fleet" the way you go to Job Board or Helm. Its behaviour is specified across nearly every other concept document — what this document owns is the daemon lifecycle, and a map of where the rest lives.

## Daemon lifecycle

Every number below was measured on macOS 27.0 / 26A5406e, launchd 7.0.0.

**Fleet and the API are one process.** `api` runs in-process with `fleet`; there is no second daemon. Doctor stays readable when Fleet dies through `armada doctor --json`, a short-lived probe process Bridge spawns.

- **Startup.** Bridge bootstraps a **launchd job**, plist kept at `~/Library/Application Support/Armada/` — deliberately **outside** `~/Library/LaunchAgents`. launchd only auto-loads from the LaunchAgents directories, so a plist elsewhere is never seen at login: **"does not start at boot" becomes structural rather than configured off**. `KeepAlive={SuccessfulExit:false}`, `ThrottleInterval` 2, mode 644. launchd parents Fleet to PID 1 at spawn.
- **Quitting the app.** Does **not** kill Fleet. Jobs keep progressing, independent of Bridge, because Fleet is not Electron's descendant.
- **Reopening Bridge.** **Reconnects** to the running Fleet rather than spawning a duplicate. Bootstrapping is per-login-session, so Bridge bootstraps every login, idempotently — with a `launchctl print` pre-check, because bootstrapping an already-loaded job and bootstrapping a world-writable plist fail with the **identical** error, `Bootstrap failed: 5: Input/output error`.
- **Fleet crashes — signal or non-zero exit.** launchd restarts it automatically. Doctor's row flips fail → pass with no user action. **There is no cap and no backoff curve** — a flat `ThrottleInterval`, forever.
- **Fleet is wedged — alive, not answering.** `launchctl kickstart -k gui/$UID/com.armada.fleet`. **26 ms** to a new PID. This is what the "Restart Fleet" button does, and it means **skip the throttle wait**, not *recover*.
- **Fleet exits 0 deliberately.** launchd leaves it down by design. Kickstarting just makes it exit 0 again. Doctor must show the **reason**, not a restart button.
- **On any restart.** Reconciles SQLite job state against live OS processes. A Job marked running with no matching process is flagged `interrupted`. **A Job whose Drone is still alive and orphaned is not yet specified** — tracked in `../contracts/system-architecture.md`, and see the `setsid` constraint below. Also sweeps worktrees for terminal Jobs past retention.
- **Uninstall.** Must `launchctl bootout`, not merely delete the plist. A loaded job survives deletion of its own plist — verified.

Two plist keys do not mean what they read. **`RunAtLoad=false` is a lie in the presence of `KeepAlive`** — both `true` and `{SuccessfulExit:false}` started the job the moment it was bootstrapped. **`Crashed:true` means signal-terminated, not failed** — `exit 1` left the job down, and a Rust panic exits 101.

### Two design constraints that follow, not preferences

**Fleet must `exit 0` on a permanent refusal.** A schema newer than the binary, an unparseable Machine config file — anything that will fail identically on the next attempt. Why: `KeepAlive={SuccessfulExit:false}` is what makes this work, and without it launchd crash-loops Fleet every `ThrottleInterval` until a human notices.

The Configuration setting *Fleet self-restart attempt limit/backoff* describes a cap and a backoff curve launchd does not offer, and is **not implementable as written** (see Open questions).

**Every Drone is spawned with `setsid`.** launchd signals a job's whole process tree, so a Drone spawned as a plain child of Fleet is **killed every time Fleet restarts** — measured, at both `kickstart -k` and `bootout`; a `setsid` child survived both. Silently, mid-Job, burning tokens against a real repo — a Drone killed by a restart it had nothing to do with, which is the opposite of the one killing Fleet does do: a cap, deliberately, on a Drone that is burning without converging.

`setsid` buys protection against exactly one thing, a group-directed signal. Fleet itself needs none, since launchd parents it to PID 1 at birth and PID-1-parented children are immune. macOS ships no `/usr/bin/setsid`, so this is a `libc::setsid()` call in `Command::pre_exec`.

**These two ship together.** Detaching Drones is what makes restart safe, and it is also what creates the orphaned-but-alive reconciliation case.

### Process groups and cleanup

**Whoever spawns, owns.** `checks-runner` holds the groups it spawns during a Job; Fleet holds the ones it spawns during the startup sweep, when no Check is running and no runner is involved. Why: the thing that started a process is the only thing that can be sure it is the one to stop it.

Neither reaches into the other, so make-a-group and kill-a-group is a utility both use rather than a service one owns.

#### `checks-runner` takes no live handle

**`checks-runner` takes injected data, never a live handle into Fleet.** It is a separate crate precisely for that, and a group handle is a live handle wearing a different name, so Fleet does not spawn on its behalf. Why: that coupling is what made v1's runner unportable.

#### The group id is persisted

**The group id is persisted against the Job**, alongside the port claim and the Drone PID. A live handle cannot be the only record: if `checks-runner` dies with children still running, the group outlives its owner and nothing holds it. With the id stored, Fleet's restart reconciliation of dead processes against live Jobs can kill an orphaned group in the pass it already runs.

If Fleet stays up while `checks-runner` dies, nothing sweeps until the next restart. Whether that sweep runs at start, on a timer, or both is open (see Open questions).

#### Where the platform difference belongs

Process-group semantics differ across platforms, and where that difference belongs is tracked in `../contracts/adapters.md`.

## Scheduling and gating

### Drone dispatch control

**Every Job-level Drone dispatch requires explicit human approval, strictly one by one.** This is the primary control on Fleet's autonomy, not a resource question. **Nothing weakens it, including an incident.**

A Job marked `urgency: incident` has its approval surfaced sooner — it interrupts rather than queues and takes the scheduling tiebreak — and is approved exactly like any other. The Production Support Incident variant varies thresholds, not the approval path.

**The configurable concurrency cap bounds how many Drones run at once**, and admission refuses past it: a Job approved while the cap is spent stays `queued` and the Board says `waiting_on_resources`. It was informational and display-only until Throughput, when the second working slot arrived and a number that bounded nothing became the number that bounds this.

**Admission is the only thing that starts a Drone**, which is what makes the bound a bound rather than a default. Every act that puts a Job back to work goes through it — approving at a human gate, asking for changes, restarting a step, overruling a verdict — so a person cannot push Fleet past its cap by pressing a button on a Job that is already approved. None of those acts is refused when the cap is spent: the decision lands, and the Job waits at `queued` saying why. [Job](job.md), Recovering an escalated Job, has the two that were exceptions until they were not.

**It bounds Drones and never approvals**, which is the whole of how the two sit together. The cap decides how many *approved* Jobs run at the same time; it never decides that a Job is approved, never batches approvals, and is not a way to approve several at once. The gate above is untouched by it.

Sub-dispatches inside an already-approved Job need no separate approval — see [Workflow](workflow.md), Dispatch Approval, Two Levels. Away-from-desk pre-authorized batches are supported.

#### Sub-dispatch and the Job proposer are two routes, and they do not converge

Both turn one thing into several Jobs, and the difference between them is the approval gate — which is why folding them together was rejected rather than deferred.

| | Job proposer | Sub-dispatch |
|---|---|---|
| Who asks | A person describing work | The Drone of an approved Job, on its dispatching step |
| What comes back | Top-level Jobs at `awaiting_approval` | Children at `queued` |
| Who approves | A person, **each Job in turn** | A person, **once, on the plan** — before any child exists |
| What was read | The request, as it was typed | An artifact under `.armada/artifacts/`, written by a Drone that read the epic |

**One path is the gate and the other is the exemption from it.** The approval gate is the primary control on Fleet's autonomy; the sub-dispatched case is the one thing that gets past it, and it gets past because a person already approved the split the children came out of. A shared path would put the exemption one refactor away from the rule, and the thing that keeps them apart is that they reach different constructors — `create_top_level` enters at `awaiting_approval` and `create_sub_dispatched` enters at `queued`, and neither takes a status.

**The saving that would have justified converging them is not there.** What the two share is drafting a proposal into a Job, and they already share it: both go through the same refusals for a blank title, a workflow nothing holds and a Manifest that is not this one. What differs is everything about who decided and what they read.

### Concurrency gating (resources)

**Fleet reads CPU, memory and disk headroom before spawning each Drone.** If capacity is unavailable the Job queues and shows "waiting" on the [Job Board](job-board.md). This determines resource eligibility only; the approval gate above determines whether a Drone starts at all.

**It is the same predicate the concurrency cap is asked through**, so a Board cannot say a Job is blocked while Fleet is starting it, and the reason is recomputed at every read rather than stored — headroom frees on its own, so a written-down reason is wrong from the moment it is written.

**Disk is the third signal and it earns its place from a measured failure**, not from symmetry with the other two: a volume filled during a parallel agent run and agents died at zero bytes free holding uncommitted work, with no warning of any kind. It is also the one held against an absolute floor rather than a share, because what a Job costs in disk — a worktree plus a build — is a number of gigabytes rather than a fraction of whatever volume it landed on, which is why it is a settings row of its own rather than sharing the CPU and memory threshold.

**Quota is not a fourth.** The agent's rate-limit event carries a window and a status and no quantity, so there is no number to hold a Job back against. See the spike on what a Job costs.

**A machine that cannot be read admits.** A failed reading holds nothing back: a Fleet that queues every Job for ever because a command did not answer is a Fleet that looks dead, and the concurrency cap is still the cap.

**A person's act is never refused for a machine.** An approval, a restart, an override and a request for changes all leave the Job at `queued` whatever the machine holds; admission is the only thing that starts a Drone, so a Job a person just re-queued waits exactly as any other queued Job does.

**Which of the four reasons is holding a Job does not reach anybody yet.** The Board has one label for all of them, and nothing else reads the answer.

**That poll only covers Jobs that have not started.** A Job that exhausts CPU or memory while already running has nowhere to queue back to and escalates as `resource_exhausted`.

### Budget gating (what the Job has spent)

**Fleet reads what a Job's Drones have already cost before starting another one on it.** Over its cap, the Job stays `queued` and reads `over_budget`. It is the third reason a Job waits and it is asked through the same one-answer arrangement the other two are, so a Board cannot say a Job is over budget while Fleet is starting it.

**It reads before "waiting on resources" when both hold.** Headroom frees on its own and a spent budget does not; a person told their Job is waiting for the machine would go and watch something that is already on its way, while the thing actually holding it needs them.

**It refuses the next dispatch. It does not stop a Drone that is spending.** What a run cost arrives on the final line of a Drone's session and nowhere else, so there is no mid-session figure to interrupt on. What that catches is a runaway *sequence* of Jobs, which is the shape a runaway has.

**The cap is per Job and the spend is per Drone.** A Drone belongs to a step, so a four-step Job is four Drones that never meet, and the sum lives in the record because nothing else outlives all of them. The dollars are notional — what the run would have cost at list price — and the turn count is the steadier signal beside them; [Machine](machine.md) carries both numbers and the measurement behind them.

A **port span that cannot be re-claimed during a scope revision** is the case with a graceful path and does not escalate: the revision fails rather than the Job, which continues on its pre-revision claim and never leaves `running`.

### DAG scheduling

**Fleet schedules by dependency graph in topological order**, on top of the approval and resource gates. **The tiebreak between ready peers reads a Job's `urgency` field**, which is what the concurrency/priority tiebreak setting names.

The dependency model itself is a property of a Job — see [Job](job.md). Only the linked-DAG Job shape decomposes into sub-Jobs; a Convoy has no children.

**An edge releases on the upstream's terminal status, and not every terminal releases it.** `completed_success` makes the dependent dispatchable; `superseded` unblocks it and surfaces it with the dependency marked unsatisfied, since the work landed outside the Job rather than not landing; any other terminal escalates it as `dependency_failed`, so a person decides rather than one failure terminating a chain unattended.

An edge carries no strength of its own — every edge gates identically and the variation is in that terminal status.

**A cycle is unstatable rather than detected.** An edge may only name a Job that already exists, and a Job's edges are written once, at creation — so every edge points at a strictly older Job and there is no acyclicity check to keep in step. A proposal naming a peer Fleet does not hold is refused where it enters, beside the workflow id and the Manifest id it is refused with.

**Escalating stops at the first dependent.** Nothing below it is cancelled or moved: `escalated` is not terminal, so a Job waiting on the one that was just escalated is still waiting on a Job that may yet run.

### Scope revision mid-Job

**Rescope-and-respawn.** Fleet terminates the Drone, re-resolves configuration against the new declared set, and spawns a fresh Drone **on the same worktree and branch** — work survives, session context does not, since Facts and Evidence live on the Job rather than on the Drone.

**Narrowing proceeds unchallenged. A person's widening returns to the dispatch approval gate; a Drone asking for one is answered by the [Judge](judge.md), and the Job never leaves `running`.** Why: scope is not a permission system — a declaration never bound writes, and it exists so that drift is detectable — so *does this belong to the step this Drone was given* is a Judge's question. A refusal escalates, which is where a person comes in. [Change a Job's scope](../journeys/change-a-jobs-scope.md).

Known cost: permissions intersect across the declared set, so a respawned Drone can come back less capable than the one that asked to widen. A judged widening respawns nothing, so the Drone that asked keeps its own toolset and the intersection reaches the next one.

### Job proposal

**Fleet makes the [Job proposer](job-proposer.md) call on every dispatch path**, single-Workspace repos included, and surfaces what comes back at the dispatch approval gate above rather than acting on it. What it proposes, and why it runs uniformly, are on that document.

What Fleet takes from it: a workflow, a title, and how many Jobs the request is. **Not `write_targets`** — the proposer does not propose them, so a Job reaches the gate with `write_targets` null, and null is not empty. Which paths the work touches is the first step's, declared by a Drone that has read the code.

This sentence used to say the opposite, and the overlap warning below was written on it. It was wrong from the day the proposer shipped.

### Write-scope overlap

**Surfaced, never serialised.** Where two unfinished Jobs claim the same paths, Fleet says so on each one's detail — naming the other Job, its status, and the paths both reach. Nothing is held back and nothing is refused: approving anyway is allowed and is the common case.

**It is deliberately not a lease.** Why: `write_targets` is a declaration and a Drone's worktree is a whole-repo checkout, so a hold over declared paths would serialise the Jobs that declared honestly and miss the one that wrote somewhere it never named — which is the collision nobody saw coming.

**It compares what two Jobs claimed, and a claim is not a write.** The Job that never names a path and writes there anyway produces nothing here, and cannot: that is the same whole-repo checkout the paragraph above turns on. The check that reads a real diff is the per-step drift check, and it measures one step against its own plan.

**A Job's claim has two possible authors, and the warning says which.** `write_targets` is what the requester stated before anything ran, and it is null on every Job the proposer drafted. A step's declared plan is what the Drone working it said, having read the code, and it is what a running Job actually has. Both are compared; each named path says which author it came from on each side. The latest run of a step replaces its earlier runs, because calling the scope tool again is how a Drone corrects its plan. Neither list is authoritative over the other; [Change a Job's scope](../journeys/change-a-jobs-scope.md) holds what each is for.

**A Job that has claimed nothing is not compared at all**, and that answer is distinct from "compared and found nobody". Every proposer-drafted Job is in the first state at its approval gate, so **the overlap is ordinarily first visible once both Jobs are running** rather than on the card of the second one to be approved. Naming an overlap before the second Job's paths are known would need the proposer to guess them, which was measured and rejected — see [Job proposer](job-proposer.md).

**Every unfinished Job, not only the running ones.** The pair is one fact and it has to read the same from either side; naming only the running peers would have made two Jobs' detail views disagree about whether there is a collision. The other Job's status travels with the warning, so a person can see which of the two is already writing.

The remedy needs no new state: `depends_on` already sequences Jobs and already parks the waiting one at `blocked_by_dependency`. **Taking it is not built** — there is no operation that writes an edge onto a Job that already exists, and this page says above that a Job's edges are written once, at creation. That write-once property is what lets DAG scheduling above skip a topological sort, so an operation that breaks it is not a small one; `#231` is where that is settled. What a person has today is the two gate answers they already had.

### Catching a branch up

**Rebasing a Job's branch is Fleet's, always.** Never the Drone — it has no git and `docs/concepts/drone.md` says outright it cannot be trusted to manage its own state — and never nobody, which is what "the base moved and the step never noticed" is.

| | |
|---|---|
| Clean rebase | **the worktree the Drone is in is updated in place** — same path, same branch, same work, the base moved underneath it |
| Conflicted rebase | **the Drone is asked to resolve the conflicts** before it continues |
| A rebase that will not replay | the branch is left exactly where it was, and the Drone is told so and told it is not theirs to fix |

**Nothing is created and nothing is discarded.** A rebase is not a new worktree: `Vcs` has no removal at all, a Job's earlier steps' work lives on the worktree it is on, and a restart exists precisely so that work survives. "Bring the branch up to date" and "keep the worktree" are the same sentence, not two competing ones.

**Every moment that starts, resumes or advances a step catches up first.** There are seven, and **six of them are a spawn**:

| | who is told, and how |
|---|---|
| A mechanical step boundary | the Drone being spawned for the next step, in its opening brief |
| A boundary a person approved | the same |
| A boundary a person overruled | the same |
| A first dispatch | the Drone being spawned, in its opening brief |
| A restart of a stopped step | the Drone being spawned, in its opening brief |
| An override where the step's Drone has gone | the Drone being spawned, in its opening brief |
| A finished Job | nobody — the branch is caught up, and a conflicted one is not pushed and gets no pull request |

**This table used to divide by whether a Drone was there to be told**, and the first three rows read "the live Drone, in the turn carrying the verdict". A Drone belongs to a workflow step ([Drone](drone.md)), so a step boundary ends one and starts another — there is no live session at a boundary and nothing to inject a turn into. The division collapsed, and what is left is one shape: **everything but a finished Job rides an opening brief.**

**The conflict is therefore always the new Drone's opening work.** Refusing the act instead would put a person at a merge conflict inside a Drone's worktree, which is the one job the Drone is already in the right place to do.

**The step's baseline is read after the rebase, never before.** A rebase writes content: a clean one replays the branch onto a base that itself moved, and a conflicted one leaves markers in the files it could not merge. A baseline taken before it credits the step with git's output, and a Drone that resolved nothing then passes `diff_nonempty` on what it was handed.

### Network loss mid-Job

**The Drone and Job auto-retry on reconnect** and resume where they left off. Process-crash recovery flags `interrupted` instead of resuming — see the daemon lifecycle above.

### What Fleet knows after the merge, and what it does not

**Armada opens a pull request and a person merges it**, so everything after that moment is only ever knowable by asking. Fleet asks about **one** pull request per sweep and rotates, because the turn interval is 250ms and asking the forge is a process — an open pull request needs asking rarely and a merged one never again.

| What one ask answers | What follows |
|---|---|
| Somebody merged it | The Job's record says so, the row says so, and the repository every worktree is cut from is brought up to what merged |
| It was closed and never merged | The record says so, and nothing else moves — nothing arrived on the base |
| It is still open, against a base that has since moved | The forge is asked to compare it afresh, **once**, because it pins the comparison at the commit the pull request was opened from and renders other people's commits as this Job's work until something moves it |
| Nothing on this machine could say | Nothing is written down and it is asked again later |

**The repository is fast-forwarded or left alone, and never anything in between.** A checkout on some other branch, a working tree carrying somebody's uncommitted change, a repository with no remote, and a history that will not fast-forward are all refusals — this is the one thing Fleet writes into the repository a person is standing in, and `--autostash` is not on offer because nobody asked for a rebase.

**Nothing watches main for breakage after a merge.** Under `auto_merge: never` a person merges and Armada is not party to it, so a post-merge signal would cover only the subset Fleet merged itself. A detector that fires on some breakages and not others is worse than none, because its silence reads as an all-clear. Running the Checks once against the updated tree is the piece that would answer it, and it is not built — #474, where the reason it is its own issue is that the run belongs to the commit rather than to any one Job.

A merge that breaks main is raised by a person, and the response is a new Job pointing back through `subject` rather than anything reopening: `completed_success` is terminal and stays true. See [Job](job.md). Whether Fleet should watch the merges it did perform is open (see Open questions).

### Restarting Fleet

**Restarting Fleet is a `launchctl` call from Bridge, not an API command.** `restart_fleet` cannot be served by the process being restarted. Bridge already owns bootstrapping the launchd job, so it owns restarting it, and the operation is a `child_process` call rather than a protocol operation.

**`kickstart -k` does not bypass the throttle.** Issued inside a live crash-restart window it returned immediately, but the new instance took 19.0 s to appear at the 10 s default. `ThrottleInterval` 2 brings that to 2.6 s.

## Ports

A Job claims a contiguous span of ports for the life of its worktree.

### The range

`Port range base` and `Port range ceiling` are settings. The ceiling is **detected at daemon start from the platform's ephemeral port floor, minus one** — `net.inet.ip.portrange.first` via sysctl on macOS, `net.ipv4.ip_local_port_range` in `/proc` on Linux — with `32767` as a fallback if the read fails rather than as the value.

A constant stores the answer instead of the rule. `32767` is one below Linux's ephemeral floor; on macOS, whose floor is 49152, it reserves roughly sixteen thousand ports the only supported platform will never use.

**Never guess high.** A ceiling above the real floor hands out spans the kernel will also assign, which is the collision class the ceiling exists to prevent. Detection supplies the default; an explicit setting still wins, so a lower cap survives and Doctor has a value to show.

This is the first place Armada reads a kernel parameter. Where that dependency belongs is tracked in `../contracts/adapters.md`.

### Claiming

A claim carries a `job_id`, so which Jobs hold which spans is a query rather than an inspection of directories.

**A bind-and-connect probe gates every hand-out**, and it is load-bearing rather than defensive. Why: teardown that silently failed, teardown never declared, and a process outside every tree are indistinguishable to the store.

### Compose

Armada resolves the repo's compose files, rewrites every published port into the claimed span, and feeds the whole document on stdin, never to disk. The Docker adapter does the rewrite.

**Refuse any published port Armada did not place.** Rewriting is what makes refusal possible. Under environment interpolation Armada never parses the document and so can refuse nothing.

Two failures follow — a service nobody parameterised binds its base port in every worktree and reads as flakiness rather than as a missing line, and a correctly parameterised file still collides, because the fallback value *is* the collision port and fires whenever the variable is absent.

Traps, all measured: compose overrides append rather than replace, and the fix is silently ignored below Compose 2.24.4, which is why the resolved document is transformed in memory. Container-side ports must be distinct across services. Unknown, unparseable and range-valued published entries are all `bad_config`.

### Teardown, then release

Teardown is partitioned by process tree rather than offered as alternatives.

| Covers | Mechanism |
| --- | --- |
| Anything Armada spawned that stayed in its tree | Process-group kill. No declaration, no adapter |
| Containers | The Docker adapter |
| Anything handed to another supervisor | A declared Command, on the Manifest |

In-tree means a `setup.run` that backgrounds a dev server, a Check that leaves a child, a Command that forks. Another supervisor means `launchctl`, a systemd unit, or a deliberate double-fork.

A group kill cannot reach containers: `docker compose up -d` returns immediately and the containers are children of the Docker daemon, a different tree.

**Release is gated on teardown.** An undeclared supervisor-held process holds its port past release, the probe refuses that span, and the range loses it. How a person learns the range is degrading is open (see Open questions).

### An interrupted Job holds its span

**Nothing releases automatically.** An interrupted Job's worktree is never swept, and a claim's lifetime is its worktree's lifetime.

**Nothing releases and reclaims the span either.** A Drone is `setsid`, so a Drone whose Fleet died may have survived and may still be bound to those ports — releasing a span a live process holds is the double-booking the mechanism exists to prevent. It also breaks number stability, and puts Fleet in the business of deciding an interrupted Job is over.

The exhaustion message names how many spans interrupted Jobs hold and points at Alerts. **Spans accumulate rather than run concurrently**, so headroom against slow accumulation is the headroom that matters.

#### Answering the escalation is what releases it

**What ends the hold is the Job reaching a terminal.** A claim's lifetime is its worktree's lifetime, an interrupted Job sits at `escalated`, and its worktree is not swept while it stays there. A person answering that escalation — kill, redispatch or Pilot — takes the Job terminal; retention then sweeps the worktree and the span goes with it.

The act is answering an escalation rather than anything about ports, so no surface, state or timer exists for this and none is needed. The bind-and-connect probe still gates the re-issue, which is what makes the release safe where a `setsid` Drone outlived its Fleet.

#### Surfacing a held span

**A held span stops being silent before exhaustion.** Past the held-span reminder threshold Fleet surfaces the hold. It releases nothing and changes no state — the alternative is learning about slow accumulation only when the range runs out.

Where it renders is open — see Open questions — and the Doctor precedent argues for a health strip rather than an Alerts row, since a held span is a standing condition and not a queued decision.

#### A timer would decide the Job is over

**No timer releases a span.** Why: a timer would put Fleet in the business of deciding an interrupted Job is over, which is the person's call and the reason `escalated` halts autonomous action at all.

## Where Fleet's behavior is actually documented

Each piece of Fleet's behavior lives where it is specified:

- CPU/memory resource gating before spawning a Drone — Scheduling and gating, above
- Owns every `Job.status` and `Job.workflow_status` transition — [Job](job.md), Ownership Split
- Evidence verification (Mechanical Check → Judge Check) — [Drone](drone.md), [Workflow](workflow.md)
- DAG scheduling and cross-workspace work — [Job Board](job-board.md), [Manifest](manifest.md), Cross-Workspace Jobs, [Convoy](convoy.md)
- Secrets brokering (Drone never holds secrets directly) — [Kit](kit.md), [Manifest](manifest.md), Secrets
- Live re-evaluation of the allowlist, the budget cap and dispatch freeze at every gated checkpoint, versus Skills, MCP, Agent files and Commands, which are frozen into a Drone at spawn — [Drone](drone.md), What's Frozen at Spawn vs. Live
- Auto-merge enforcement, VCS push/PR/merge, the sole actor touching Git credentials — [Manifest](manifest.md), auto_merge and review_gate
- Rebasing a Job's branch onto its base, and who reads a conflict — Catching a branch up, above
- Schema migrations applied on startup — `../contracts/system-architecture.md` section 5, [Kit](kit.md), Upgrade
- Structured JSON logging per Job — `../contracts/system-architecture.md` section 4
- Own health status — [Doctor](doctor.md), Fleet module
- What a repo may declare about ports, Check timeouts and teardown — [Manifest](manifest.md)

The Job shapes are single-workspace, linked-DAG sub-Jobs and [Convoy](convoy.md). Only the linked-DAG shape decomposes into sub-Jobs; a Convoy has **no children**. Shape is derived from `write_targets` and `atomic` rather than stored, so Fleet branches on those two fields and a new combination of them costs no migration.

## Open questions

- **[fleet-checks-runner-sweep-timing]** Does the sweep for orphaned process groups left behind by a dead `checks-runner` run at Fleet start, on a timer, or both? If Fleet stays up while `checks-runner` dies, nothing sweeps until the next restart under the current design.
- **[fleet-watch-merge-after-self-merge]** Should Fleet watch main after a merge it performed itself? Fleet's model of the world otherwise stops at the merge — nothing watches main afterwards, and a merge that breaks main is raised by a person today, with the response being a new Job pointing back through `subject`.
- **[fleet-port-range-degradation-visibility]** How does a person learn the port range is losing spans? An undeclared supervisor-held process holds its port past release, the probe refuses that span, and the range loses it — nothing currently surfaces that degradation. Related: past the held-span reminder threshold Fleet surfaces a held span, but where that renders is also undecided; the Doctor precedent argues for a health strip rather than an Alerts row, since a held span is a standing condition and not a queued decision.
- **[fleet-self-restart-limit-setting]** What happens to the *Fleet self-restart attempt limit/backoff* setting, given that launchd cannot implement a cap or a backoff curve? The setting describes behavior the supervisor does not offer.
