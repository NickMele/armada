# Running Armada locally

**Kind:** practice. **Governs:** starting, checking and stopping a local Fleet,
running a Check by hand, and giving a Job's worktrees back.

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

**A healthy start prints, then goes quiet.** The repository and its workflow,
the pid, port and protocol version, what reconciliation found, the turn
interval, and how many operations are being served. Quiet is a Fleet with
nothing to do, not a wedge.

**It refuses before it binds a port.** Every fault is on its own line, and a
refusal exits non-zero.

| Fault |
|---|
| `armada.yml` missing or malformed |
| `.armada/workflows/` holding no definition, or more than one |
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

## A Fleet of your own

**`scripts/dev-fleet <scratch-dir>` starts a Fleet that cannot touch yours.** It
has its own home, its own store, a local clone of this repository with the Fleet
data copied in, and a Drone that exits at once — so a Job it dispatches
escalates rather than doing work or spending anything. Add `--copy-store` to
start it on a copy of your Jobs; leave it off for an empty store.

**Reach for it when you want a Job as Fleet serves it, without your Fleet** — to
record one for Storybook with `scripts/record-job.mjs`, or to point a surface at a
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

## Recording a Job for Storybook

**`node scripts/record-job.mjs <job> <slug> "<the state, as a sentence>"` records
one Job off a running Fleet** into `packages/screens/src/fixtures/recorded/<slug>/`,
and Storybook draws it as a story of `Screens/Job detail`. It writes what Fleet
answered — every read the job-detail screen makes, and every message on the
Job's two sockets — and the story replays that through the same fold Bridge
runs, so what you see there is what the app would draw.

**Run it when a Job is sitting in a state the stories do not have yet.** Pass
`--dev-fleet <scratch-dir>` to read a dev Fleet (above) rather than yours. It
needs a running Fleet and nothing else. A Job still running is recorded as far
as it had got: each socket is cut after two quiet seconds, and the story draws
it still watching.

**It scrubs before it writes, and writes nothing it could not scrub.** Home paths
become `~`, your account name `owner`, and the machine's name and any email
address are replaced. A line naming what was left means nothing was written —
this repository is public. On success it prints each read's status and how many
messages each socket carried. A read that answered 409 is recorded as the
refusal it was, and the story draws it that way.

**`node scripts/record-job.mjs --board <slug>` records the whole Board instead**:
every row the Job list serves, and the workflows and manifests they name, into
`packages/screens/src/fixtures/boards/<slug>.json`. `Screens/Board` replays it
beside the rows it builds. It makes only those three reads, so it changes
nothing on the Fleet it reads, and it scrubs and refuses exactly as above. It
prints how many Jobs and workflows it wrote.

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
pull request open against it. Fleet does all four — a Drone is denied `git`.

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
