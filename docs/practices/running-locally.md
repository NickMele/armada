# Running Armada locally

**Kind:** practice. **Governs:** starting, checking and stopping a local Fleet,
running Bridge with no Fleet at all, running a Check by hand, and giving a Job's
worktrees back.

`README.md` carries the commands. This carries what they do and what they
refuse.

---

## Node and pnpm

**`.nvmrc` names the Node version, and `engines.node` is the floor.**
`engineStrict: true` in `pnpm-workspace.yaml` makes a mismatch refuse the
install rather than warn and carry on.

| Where | Says |
|---|---|
| `.nvmrc` | The exact version — `nvm use` reads it |
| `engines.node` in `package.json` | The floor a wrong Node is measured against |
| `pnpm-workspace.yaml` | That the floor is enforced, not advisory |
| `packageManager` in `package.json` | The pnpm version corepack fetches |

**pnpm comes from corepack, which is bundled with Node.** A Node old enough
carries a corepack whose npm signing keys have expired, and every `pnpm` call
then dies in `verifySignature` with `Cannot find matching keyid` — which reads
as a registry outage and is a stale Node. The version in `.nvmrc` ships a
corepack with current keys; on an older Node, `npm i -g corepack@latest` fixes
it in place.

## The two halves

**Fleet is started first, always.** Fleet binds a loopback port and publishes a
runtime file; Bridge reads that file to find where to connect, so a Bridge
started first has nothing to read.

**They ship as a pair and version together.** A major and a minor in
`protocol-version.toml` govern both.

| Skew | What happens |
|---|---|
| Major mismatch | The connection is refused in either direction |
| Fleet ahead of Bridge | Connects, and Bridge shows a banner |
| Fleet behind Bridge | Refused — Bridge would read fields Fleet cannot send |

Minor versions are additive-only, which is why the two directions differ.
`docs/practices/protocol.md` says which number moves when.

## The development loop

**`pnpm dev` reinstalls `armada` on every run.** `cargo install` copies rather
than links, so an edit does not reach the installed command until it is run
again.

**Rebuilding one side and not the other is the mistake the script prevents.** A
stale `armada` publishing an older protocol to a current Bridge reads as
version skew rather than as the stale binary it is. It installs `--debug` for
the same reason: a release build is a minute every time and the same program.

**Ctrl-C stops both, which the script does and Armada does not.** Closing
Bridge in earnest leaves Fleet running.

**Arguments to `pnpm dev` reach `cargo install` and nothing else.** `--force` is
the one worth knowing: `cargo install` refuses a binary name another package
owns, which is what an install left behind by a renamed or deleted crate looks
like.

**`scripts/dev` is not an agent's to run.** It kills the Fleet in use and
reinstalls the binary. An agent starts Bridge alone against a Fleet already up,
and works from `target/debug/armada` rather than an installed copy.

**The app is called Electron until it is packaged.** `pnpm dev` and
`pnpm --filter @armada/desktop start` launch Electron's own bundle, and macOS
reads the dock and the app switcher from whichever bundle it launched — so both
say `Electron` there whatever `app.setName` is given, and the dock tile is only
right because Bridge sets it at runtime after the window is ready.
`pnpm --filter @armada/desktop package` builds an `Armada.app` that carries its
own name and its own icon, and connects to a running Fleet exactly as the
unpackaged app does. It is unsigned, so it runs on the machine that built it and
nowhere else. `apps/desktop/electron-builder.yml` says where it lands and what
goes in it.

## Starting Fleet

**A healthy start prints, then goes quiet.** The repository, each workflow and
where it came from, the pid, port and protocol version, what reconciliation
found, the turn interval, and how many operations are being served. Quiet is a Fleet with
nothing to do, not a wedge.

**It refuses before it binds a port.** Every fault is on its own line, and a
refusal exits non-zero.

| Fault |
|---|
| `armada.yml` missing or malformed |
| Two definitions in one place naming the same `workflow_id` |
| A step naming a Check the Manifest does not declare |
| An agent CLI a Drone would not find on its own `PATH` |

**Started against a Fleet already running it exits 0** and names the pid — the
state you asked for already holds. That is the reliable way to ask whether one
is up; the runtime file says something published it and not whether that pid is
still held.

## Stopping Fleet

**SIGTERM is what it waits for.** It finishes the turn in flight before
exiting, and a turn running a Check can hold it for the whole Check budget.

A terminal gone quiet after `stopping: letting the turn in flight finish` is
working. The runtime file is removed on the way out; a SIGKILL leaves it behind
and the next start replaces it, saying so.

## Restarting onto a new build

**`scripts/restart` moves the running Fleet — and Bridge, once it needs to —
onto whatever is in the working tree**, without the owner leaving Bridge for a
terminal. Two callers: an agent, through Bash, after a merge that touches
Fleet or Bridge; and Bridge itself, through `#810`'s act.

**It refuses while a Drone is working**, naming the Job. Read off the live
roster and each Drone's Job rather than a status count — an escalated Job
keeps its Drone alive and idle, and a count derived from statuses would either
miss that Drone or refuse a restart nothing is using. A queued Job, a Job at a
human gate, or one a person is piloting holds no process the restart would
interrupt, so none of those refuse it.

**Only one restart runs at a time.** An exclusive lock is taken before the
first Drone check, under the same support directory as the plist and
runtime file. macOS has no `flock(1)`, so the lock is a symlink naming its
holder's pid — `ln -s` is one syscall, so the link is never observed to
exist without already naming who holds it, which a directory created and
then written into separately cannot promise. A second run refuses at once,
naming the pid of the one already in progress, rather than racing it to
stop and rebuild Fleet. The lock is released on every exit, including a
refusal or a signal.

**A lock left behind by a run that died holding it is never broken
automatically.** A dead pid refuses too, naming the lock and the pid that
died holding it, and says to remove it by hand and run again. An earlier
attempt broke a dead-pid lock on the caller's behalf, and a race loop kept
finding a way through it no matter how the removal itself was made atomic:
two runs that both read the same dead pid could each act on what the other
had just recreated, because reading "it is stale" and taking it are still
two separate steps for a second run to land between. A lock only outlives
its holder when a restart itself was killed, which happens rarely, so a
person clearing it by hand beats a machine racing to guess it is safe — the
one case this lock exists for.

**The Bridge build stamp records when the build it reflects started, not
when the run around it ended.** A marker is written the moment Bridge's
build begins, before `pnpm build` runs, and only that marker's time — never
"now" — becomes the stamp once this run decides the running Bridge matches
it: after a successful reopen, or after deciding none was needed. Stamping
"now" at the end would mark a run as covering edits made *during* its own
multi-minute build, which a later run would then never see as changed. A
run that fails or refuses anywhere after the build (the late Drone check,
Fleet's own restart, a Bridge that would not quit) leaves the stamp exactly
as it was, so the next run still finds Bridge changed and rebuilds and
reopens it rather than trusting a build nobody is running yet.

**It runs Fleet under a launchd job it bootstraps on first use**, the design
`docs/concepts/fleet.md` (Daemon lifecycle, Restarting Fleet) specifies: the
plist lives beside the runtime file, outside `~/Library/LaunchAgents` so it is
never loaded at login, `KeepAlive={SuccessfulExit:false}` and
`ThrottleInterval` 2 so a crash is a restart and not a page. `launchctl
kickstart -k` is what moves an already-running Fleet onto the fresh build —
never a second `armada serve`, which is the "two Fleets" failure `armada-local`
already warns about, this time from a script rather than a name.

**It rebuilds Bridge only where `apps/` or `packages/` moved since its last
build**, and reopens it only then — otherwise Bridge reconnects to the new
Fleet on its own (`apps/desktop/src/main/socket.ts`). Reopening is a second,
separate launchd job for the same reason Fleet's is one: the window has to
outlive the session that asked for it.

**Never on an allow list.** `.claude/skills/restart-fleet/SKILL.md` is when an
agent reaches for it and what to tell the owner first — the confirmation is
Claude Code's own permission prompt, not a flag this script reads.

**`ARMADA_FLEET_LABEL` is for testing only.** `scripts/restart` derives its
plist, its runtime file and its launchd label from `$HOME`, so a Fleet started
under `scripts/dev-fleet`'s scratch home cannot reach the owner's. The label
needs its own override because launchd's `gui/$UID` domain is one namespace
per machine account, not per home directory — two scratch Fleets left on the
default label would still collide with each other and, unset, with the
owner's own `com.armada.fleet`.

## A Fleet of your own

**`scripts/dev-fleet <scratch-dir>` starts a Fleet that cannot touch yours.** It
has its own home, its own store, a local clone of this repository with the Fleet
data copied in, and a Drone that exits at once — so a Job it dispatches
escalates rather than doing work or spending anything. Add `--copy-store` to
start it on a copy of your Jobs; leave it off for an empty store.

**Reach for it when you want a Job as Fleet serves it, without your Fleet** — to
record one for the mock with `scripts/record-job.mjs`, or to point a surface at a
real daemon. A second plain `armada serve` is not the same thing on another
port: Fleet's store is found through `HOME`, so it would share yours, and its
boot reconciliation would escalate your running Jobs as `interrupted`.

It needs a built workspace (`cargo build --workspace`); `--copy-store` also
needs `sqlite3`, which macOS ships. **It prints what `armada serve` prints**,
because it ends by running it, and the port is also in
`<scratch-dir>/home/user/Library/Application Support/Armada/fleet.json`. It refuses a
scratch directory inside the repository and answers a second start with the pid
already running. Stop it the way you stop Fleet, then delete the directory — it
holds a copy of your transcripts.

## Recording a Job for the mock

**`node scripts/record-job.mjs <job> <slug> "<the state, as a sentence>"` records
one Job off a running Fleet** into `packages/screens/src/fixtures/recorded/<slug>/`,
and the mock below opens it as `recorded/<slug>` and a row of `every-state`. It
writes what Fleet answered — every read the job-detail screen makes, and every
message on the Job's two sockets — and the mock replays that through the same
fold Bridge runs, so what you see there is what the app would draw.

**Run it when a Job is sitting in a state the mock does not have yet.** Pass
`--dev-fleet <scratch-dir>` to read a dev Fleet (above) rather than yours. It
needs a running Fleet and nothing else. A Job still running is recorded as far
as it had got: each socket is cut after two quiet seconds, and the mock draws
it still watching.

**It scrubs before it writes, and writes nothing it could not scrub.** Home paths
become `~`, your account name `owner`, and the machine's name and any email
address are replaced. A line naming what was left means nothing was written —
this repository is public. On success it prints each read's status and how many
messages each socket carried. A read that answered 409 is recorded as the
refusal it was, and the mock draws it that way.

**`node scripts/record-job.mjs --board <slug>` records the whole Board instead**:
every row the Job list serves, and the workflows and manifests they name, into
`packages/screens/src/fixtures/boards/<slug>.json`. The mock's `recorded-board`
scenario replays `real-board.json`; another slug needs its own scenario in
`apps/desktop/src/renderer/src/mock/scenario.ts`. It makes only those three reads, so it changes
nothing on the Fleet it reads, and it scrubs and refuses exactly as above. It
prints how many Jobs and workflows it wrote.

## Bridge on a mock Fleet

**`pnpm mock`, from `apps/desktop`, opens Bridge's renderer in your browser on
a fake Fleet.** It runs `App.tsx` unchanged, with no Electron, no Fleet and no
network, so you can see the real app in a state no running Fleet will hold still
for. It needs `pnpm install` and nothing else, and it opens the page itself.

**Run it when you change something Bridge draws and want to see it in place.**
Vite prints the address and reloads the page as you edit. Ctrl-C stops it, and
nothing is left running or written.

**`?scenario=<name>` picks what the window shows**, and the picker in the
bottom-right corner switches by reloading onto another. An unknown name falls
back to the first scenario and says so in the browser console.

| Scenario | What the window holds |
|---|---|
| `every-state` | One Job in every state, each opening onto its own detail. The page opens here |
| `fleet-not-running` | No runtime file, so what Bridge draws with no Fleet |
| `first-launch` | A Fleet serving no repository, so Add a repository opens by itself |
| `empty-store` | One repository and no Job yet |
| `recorded-board` | The Board as Fleet served it, from `--board` above |
| `setting-up` | A set-up repository and a folder nobody set up, over a Fleet that scans, proposes, applies each Setup edit and Write, and adds or clones a repository |
| `manifest` | This repository's own Manifest, on a Fleet that saves the file, applies the forms' edits, and lists runs, drift and an always-allowed command |
| `studios` | This repository's Studios, on a Fleet that keeps them and takes the writes Helm makes on one |
| `job/<builder>` | One Job, already open, for each builder `packages/screens/src/fixtures/build/index.ts` exports |
| `recorded/<slug>` | One recorded Job, already open, for each recording under `packages/screens/src/fixtures/recorded/` |

**Every Job is a Storybook fixture, never data made up for the mock.** A
recording added to `packages/screens` is a scenario and an `every-state` row with
no other edit. A builder needs its name added to `BUILDERS` in
`apps/desktop/src/renderer/src/mock/scenario.ts`, and the test beside it fails
until it is.

### What the fake answers

**The fake is typed against `BridgeApi` member by member**, in
`apps/desktop/src/renderer/src/mock/fake.ts`. A capability added to the preload
fails typecheck there until the fake answers it.

| Call | Answer |
|---|---|
| A per-Job read — `watchJob`, `readHistory`, `readDiff` and the rest | The fixture's read, published as main would publish it |
| A read that returns a value — `readCall`, `readFrame`, `readCheckOutput` | The fixture's answer, where it has one |
| Any read the scenario holds nothing for | A failure whose sentence says it is not in this mock scenario |
| An act | Succeeds. Where it changes one field on a Job — approve, kill, reject, a model, a clear — that field moves |
| A Studio read or write | The scenario's own Studios, kept by the fake and written to as Fleet would |

**Every scenario keeps Studios**, so the surface opens wherever it is reached. A
scenario naming none keeps an empty list and draws its empty state, never a read
failure — the defect #1341 fixed. `every-state` keeps one Studio holding a node
of every kind and an edge of every kind, and a second nobody has named.

**Anything a Fleet would have to decide moves nothing.** A redispatch makes no
new Job, a review approval does not advance the Job, and a proposal comes back
unanswered. Guessing Fleet's next state would draw a Fleet that does not exist.

**A scenario can answer a flow as Fleet would.** Its `behaves` replaces the
fake's answer to the calls it names, over state it can publish. `setting-up`
does this for Setup and Locate in `setup-fleet.ts`, and `manifesting()` in
`manifest-fleet.ts` does it for the Manifest surface's saves, edits, runs and
drift, because an edit is only worth drawing if the next read shows it
applied.

**A little is made up, and none of it is what a real Fleet says.** The Fleet
panel's pid and port, the model list, and the root folder of a repository a
recording names are invented. Bridge's clock is real while the fixtures' times
are fixed, so how long ago something happened reads in days.

### In a browser test

**`mountApp` in `apps/desktop/src/renderer/src/mock/mount.tsx` mounts `App` on a
scenario** for a vitest browser test. Name the file `.test.tsx` under
`apps/desktop/src/renderer/` and it runs in Chromium with `pnpm -C apps/desktop
test`. `every-state.test.tsx` beside it opens every `every-state` row.

**`testing.ts` is what a test starts from.** `mount(scenario)` mounts it and
`unmountAfterEach()` takes it down. `openBoard()` reaches the Board by the rail,
and `mountTwo` sets two windows on one main. `onBoard(jobs, …)` in
`scenario.ts` builds a scenario from rows a test picks. A test that used to be a
Storybook `play` asserts what `App` does with a press — the dialog, the
composer, the file written — rather than that a callback was called.

## Annotating Bridge

**⌥⌘A turns the annotation layer on** in Bridge under `pnpm dev` and in the mock
above, and it is absent from a packaged app. Click anything, write a note, and
⌘Enter saves it as one file under `.armada/annotations/` at the repository root,
gitignored. Each file names the component under the click and the ones around
it, a selector, the screen and the mock scenario, which is what an agent needs to
find the code without asking where you clicked.

**A note reaches work one of two ways.**

| Way | What happens |
|---|---|
| *Send to Fleet*, on the note or for every open note on the screen | Proposed as a Job at the approval gate, with a screenshot of the area attached. Nothing runs until you approve it on the Board, and no session has to be open. The pin says where it went |
| `/annotations`, in a session | The agent reads the open notes, asks what is the owner's to decide, dispatches a subagent for each change, verifies it, and marks it done — `.claude/skills/annotations/` |

**Send needs Bridge and its Fleet.** In the mock there is no Fleet, so the layer
says so and the note stays a file for `/annotations`.

**A note marked done draws no pin and takes no number**, so the numbers run over
the open notes alone and a new note takes the next one rather than counting
everything ever written. Its file stays where it was; the bar says how many are
done and *Show done notes* draws them again, unnumbered, for as long as the layer
is on.

## Running a Check or a Command by hand

**`armada check` and `armada run` need no Fleet.** They read `armada.yml` and
execute through the same runner a Job's gate uses.

**There is no shell**, so a `run` string that pipes or redirects does not work
here either. The command's own exit code comes back out.

**Output is captured and printed when the command ends, not streamed.** A long
Check prints nothing while it runs, which reads as a hang and is not one.

**A name in the wrong registry is refused with the verb that would have
worked**, and a name in neither is refused by listing what is declared.

**A Check's `requires` runs here too, before the Check does, for any Check
that declares one.** A prerequisite that fails is reported as itself: the line
names the Command and the line it ran, and says the Check never started.
`format` declares none — it once did, and that meant `armada check format`
rewrote your working tree and then read what it had just written, so it could
never fail. `armada.yml` says why it does not any more. A failing `format`
says `armada run fmt`, which is a step you take, not one the Check takes for
you.

**Prefer these over retyping the command they wrap.** The Check a person runs is
the Check a Drone is measured by.

## What a finished Job leaves behind

A Job that passes every Check ends with its work committed on its own branch,
that branch brought up to date with the branch it merges into, pushed, and a
pull request open against it. Fleet does all four. A Drone is denied the `git`
commands that change a repository, whatever the operator's own settings allow,
and a branch that already holds commits its base has not got is still delivered
when Fleet finds nothing left to commit.

**The branch it merges into is `base:` in `armada.yml`.** Left out, Armada
infers one: what `origin/HEAD` names, then `main`, then `master`. A declared
branch the repository has not got is refused by name rather than replaced with
a guess.

**A repository with no remote is ordinary.** The work is committed, nothing is
pushed, no pull request is invented, and the Job completes. The branch is the
whole of the work.

**Opening the pull request needs `gh` on the `PATH` and signed in.** Without it
the branch is pushed and the pull request is yours to open; the Job does not
fail over it.

### Rebasing at every step boundary

**Fleet rebases at every step boundary, not only at the end.** A Job that runs
for an hour is a Job the base branch moves under, and finding that out at the
end is finding it out too late.

At a boundary the Drone has just submitted and nothing is in flight, so git
answers on its own and no question reaches the Drone.

| What git says | What happens |
|---|---|
| Not behind | Nothing at all, and nothing is announced |
| Behind, and it replays | The Drone is told what moved, in its next turn |
| Behind, and it conflicts | The conflict is handed to the Drone as work, every file named |

**Uncommitted work is never destroyed by this.** Fleet commits only at the last
step, so mid-Job the worktree is full of uncommitted changes; the rebase carries
them across and puts them back.

Where they will not go back cleanly the files are left with conflict markers and
git keeps its own copy in a stash. Where the branch's own commits will not
replay, the branch is put back where it was and nothing is pushed.

### The pull request body

**A pull request's body is assembled from the record, never written by an
agent.** It carries the brief, what the Job had to satisfy, every step with its
verdict, every Check with its outcome and a link to what it printed, and a
closing section naming what nothing checked.

What the agent claimed is not in it. A claim is a signal the gate ruled on, and
the record is what Fleet verified.

## Asking what happened to a Job

```sh
./scripts/job 1-board-s-clear-button-should-reclaim-worktr
./scripts/job 1
./scripts/job 01M22TYSAE0023MADDP5ZQEYGW
```

**One command that prints the whole story of one Job**: what it is and which
step it is on, every transition with times, each step's verdict with the
Judge's findings and what the Checks did, **what its Drone was refused and the
argument it was refused on**, what the run cost, and the tail of the Job's own
log. It needs nothing built and no arguments but the Job.

**All three name the same Job, and the first is the one Bridge shows.** A Job
answers to its handle, to the number that handle starts with, or to its id —
on this command and on every route under `/jobs/:job_id`. The number is the
shortest thing there is to type and counts within one repository, so it means
nothing without one and is refused rather than guessed at.

**The refusal is the reason it exists.** A refused call is written down as a
tool name and a tool-use id and never the argument, which sits on the `called`
row a fraction of a second earlier under the same id. Reading a
`blocked_by_policy` escalation therefore meant joining two rows of one JSONL
file by hand, and until this command nothing did it. A Job that goes quiet is
usually an argv the allowlist declined, so this is the first section to read.

**It reads two sources and says which answered each section.** Fleet holds the
Job record — the steps, the verdicts, the Judge's findings, the spend — and is
asked for it over HTTP on the loopback port in the runtime file. The
repository's own share of Fleet's data directory holds the Drone transcripts
and the Job's log, which no route serves.

| Fleet | What you still get |
|---|---|
| Running, and holds the Job | Everything |
| Running, and does not hold it — `armada clean` took it, or it was forgotten | The transitions from the Job's log, the refusals, what the run cost. It says which of these two happened |
| Not running | The same, and the header says Fleet could not be reached |

**With Fleet down, all three forms still resolve**, off `.armada/logs/` under
the repository's records — named by the handle, like every other directory
there. `records_root` in `scripts/job` computes the same directory
`fleet::records::root` does, from the same repository root and the same
digest, so a post-mortem with the daemon down still finds what a running Fleet
would have answered from its store. A number matching more than one is refused
there too.

So a post-mortem works with Fleet down, which is usually when one is done.

| Flag | For |
|---|---|
| `--full` | Do not cut a claim, a finding or a refused argument to an excerpt |
| `--transcript` | Also print every transcript row, one line each. Hundreds of lines |
| `--log N` | Lines of the Job's log, 20 by default, `0` for all of them |
| `--repo PATH` | The checkout Fleet is serving, if it is not the one you are standing in |

**Run from a worktree it still finds the records.** A Drone's worktree has an
`.armada/` of its own holding only the tracked workflows, so the default is the
main checkout rather than the directory you are in.

**`armada clean` does not touch what this reads.** A Job's log, its Drones'
transcripts, its Checks' output and its kept deliverables outlive the worktree
`armada clean` reclaims — `crates/armada/src/clean.rs` is where that is
argued — so nothing here needs reading before clearing up. `--all` removes the
machine's store as well, which forgets the row that named these files; the
files themselves are still on disk, still readable by `scripts/job`, and
findable by nothing that asks Fleet.

**It is not a Rust verb, and that was decided rather than deferred.**
`crates/armada` may not call `serde_json::from_*` — `xtask` rule five and the
write-hook both refuse it — so a client living there could fetch these answers
and not read them. And the refusal join is on no HTTP route at all: refusals
arrive on the `observe_job` WebSocket, so a Rust client would carry a WebSocket
dependency into the binary every crate links into, to reach a fact that is
already a file read away.

## Clearing up

**Destructive. Read this before running `armada clean`.**

| Form | Removes |
|---|---|
| `armada clean` | This repository's worktrees under `.armada/`, the branch each is on, and that Manifest's Jobs |
| `--all` | And the machine's store, its write-ahead files, the runtime file, the MCP configuration |
| `--force` | And the unmerged branches, and their commits |

**`--force` and `--all` are separate questions.** One is *delete work nobody has
taken*; the other is *clear this machine's store too*.

**Both forms refuse while Fleet is running**, naming the pid. The Jobs being
forgotten are the ones it is holding.

### It derives what it deletes

**Every branch it deletes comes from a Job it is deleting, never a name
pattern.** A worktree with no Job behind it is reported and left where it is.

**A row the store cannot rebuild is cleared too**, by the id it still carries. A
migration can leave a Job the current build no longer folds — Fleet reports
those on start as *unreadable*. Clearing one needs no rebuild, so `clean` takes
its worktree, branch and row like any other and says why the row would not
rebuild while the row is still there to say it.

A row belonging to another Manifest is counted and left for the repository that
owns it.

### It keeps a branch nobody has taken

**A branch the base branch cannot reach is named, counted and left standing**,
while its worktree still goes. A checkout can be made again and a commit cannot.

The count is stated as *2 commit(s) of its own are not on `main`*. Where nothing
answers what the base branch is, nothing can say what merged means, so every
branch is kept and the line says so.

**What to do about one it left:** merge it, then `git branch -d
armada/<job-id>`. Git refuses that itself while the branch is unmerged, so the
two checks agree.

### What it prints

**It prints what it removed item by item, including the commit each deleted
branch pointed at.** That SHA is the only thing that makes a branch
recoverable, so the output is not discarded.

**`git branch -D` over the `armada/` namespace is what this verb exists so that
nobody types.** A glob once destroyed nine unmerged branches belonging to no
Job.

**An Armada worktree is never removed with `rm -rf`.** Git keeps an
administrative record that outlives the directory and refuses the branch delete
afterwards; `clean` does it in the order git needs.
