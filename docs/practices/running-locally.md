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

## Running a Check or a Command by hand

**`armada check` and `armada run` need no Fleet.** They read `armada.yml` and
execute through the same runner a Job's gate uses.

**There is no shell**, so a `run` string that pipes or redirects does not work
here either. The command's own exit code comes back out.

**Output is captured and printed when the command ends, not streamed.** A long
Check prints nothing while it runs, which reads as a hang and is not one.

**A name in the wrong registry is refused with the verb that would have
worked**, and a name in neither is refused by listing what is declared.

**A Check's `requires` runs here too, before the Check does.** `armada check
format` runs `cargo fmt --all` and then reads, which is what the gate does — so
it rewrites files in your working tree, and that is the point rather than a
surprise. A prerequisite that fails is reported as itself: the line names the
Command and the line it ran, and says the Check never started.

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
