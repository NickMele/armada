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

A configurable concurrency cap exists but is **informational and display-only**, never auto-enforced. Sub-dispatches inside an already-approved Job need no separate approval — see [Workflow](workflow.md), Dispatch Approval, Two Levels. Away-from-desk pre-authorized batches are supported.

### Concurrency gating (resources)

**Fleet polls CPU and memory headroom before spawning each Drone.** If capacity is unavailable the Job queues and shows "waiting" on the [Job Board](job-board.md). This determines resource eligibility only; the approval gate above determines whether a Drone starts at all.

**That poll only covers Jobs that have not started.** A Job that exhausts CPU or memory while already running has nowhere to queue back to and escalates as `resource_exhausted`.

A **port span that cannot be re-claimed during a scope revision** is the case with a graceful path and does not escalate: the revision fails rather than the Job, which continues on its pre-revision claim and never leaves `running`.

### DAG scheduling

**Fleet schedules by dependency graph in topological order**, on top of the approval and resource gates. **The tiebreak between ready peers reads a Job's `urgency` field**, which is what the concurrency/priority tiebreak setting names.

The dependency model itself is a property of a Job — see [Job](job.md). Only the linked-DAG Job shape decomposes into sub-Jobs; a Convoy has no children.

**An edge releases on the upstream's terminal status, and not every terminal releases it.** `completed_success` makes the dependent dispatchable; `superseded` unblocks it and surfaces it with the dependency marked unsatisfied, since the work landed outside the Job rather than not landing; any other terminal escalates it as `dependency_failed`, so a person decides rather than one failure terminating a chain unattended.

An edge carries no strength of its own — every edge gates identically and the variation is in that terminal status.

### Scope revision mid-Job

**Rescope-and-respawn.** Fleet terminates the Drone, re-resolves configuration against the new declared set, and spawns a fresh Drone **on the same worktree and branch** — work survives, session context does not, since Facts and Evidence live on the Job rather than on the Drone.

**Narrowing proceeds unchallenged; widening returns to the dispatch approval gate.** Why: one-by-one approval is the primary control above, and a Job must not reach further than it was authorised to one revision at a time.

Known cost: permissions intersect across the declared set, so a respawned Drone can come back less capable than the one that asked to widen.

### Job proposal

**Fleet makes the [Job proposer](job-proposer.md) call on every dispatch path**, single-Workspace repos included, and surfaces what comes back at the dispatch approval gate above rather than acting on it. What it proposes, and why it runs uniformly, are on that document.

What Fleet takes from it: every Job arrives carrying `write_targets`, which is what the overlap warning below compares.

### Write-scope overlap

**Surfaced, never serialised.** Fleet compares a Job's `write_targets` against those of every Job already running under the same root and, where they intersect, says so on the approval card — naming the Job and offering to make this one `depends_on` it instead. Approving anyway is allowed and is the common case.

**It is deliberately not a lease.** Why: `write_targets` is a declaration and a Drone's worktree is a whole-repo checkout, so a hold over declared paths would serialise the Jobs that declared honestly and miss the one that wrote somewhere it never named — which is the collision nobody saw coming.

The remedy needs no new state: `depends_on` already sequences Jobs and already parks the waiting one at `blocked_by_dependency`.

### Network loss mid-Job

**The Drone and Job auto-retry on reconnect** and resume where they left off. Process-crash recovery flags `interrupted` instead of resuming — see the daemon lifecycle above.

### Fleet's model of the world stops at the merge

**Nothing watches main after a merge.** Under `auto_merge: never` a person merges and Armada is not party to it, so a post-merge signal would cover only the subset Fleet merged itself. A detector that fires on some breakages and not others is worse than none, because its silence reads as an all-clear.

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
- Live re-evaluation of allowlist, budget caps and dispatch freeze at every gated checkpoint, versus Skills, MCP, Agent files and Commands, which are frozen into a Drone at spawn — [Drone](drone.md), What's Frozen at Spawn vs. Live
- Auto-merge enforcement, VCS push/PR/merge, the sole actor touching Git credentials — [Manifest](manifest.md), auto_merge and review_gate
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
