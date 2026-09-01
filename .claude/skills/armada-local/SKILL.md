---
name: armada-local
description: Starting, checking and stopping a local Fleet, cleaning up after Jobs, and looking at Bridge without stranding it on the owner's screen. Load before running the armada binary in this repository.
---

# Running Armada in this repository

**An agent may launch Bridge to look at its own work.** A change to a screen
that nobody looked at is a change nobody verified, and no gate in this
repository can see a layout — `pnpm shoot` exists because a screen shipped
about thirty differences from its drawing with every gate green.

**What is not yours is the owner's attention.** Bridge is a windowed Electron
process and it comes up in front of whatever he is doing, so launching it is
taking the screen for as long as it is up. That is a cost worth paying to
answer a question and never worth paying to have a look around.

Three rules, and all three come from one incident: an agent left a modal dialog
on the owner's screen with no way to dismiss it. **The ban this file used to
carry was wider than what happened**, and it cost more than the dialog did —
agents stopped verifying screens at all.

- **Write down the question before you launch.** Which screen, which state,
  what would tell you it is wrong. A launch with no question is a look around.
- **Quit what you launched**, and say you did. A window left running is one the
  owner has to find and close.
- **Never leave a dialog up.** Dismiss it, or quit the app. A modal an agent
  cannot dismiss is the whole of the incident above.

**Prefer a hidden window wherever one will answer.** `pnpm shoot` drives
Bridge's own Electron with nothing on screen and writes PNGs an agent can read
back — `docs/practices/comparing-to-the-drawing.md`. **It captures the
components gallery and not Bridge**, so a screen assembled in `apps/desktop`
is outside it: the gallery's `Screens/Inside a job` hand-builds its own header
rather than rendering the app's. Bridge itself has no hidden path —
`createWindow` shows on `ready-to-show` and takes no flag — so until it has
one, seeing a real Bridge screen means taking the screen.

**`scripts/dev` and `pnpm dev` are still not yours.** Not because they start
Bridge, but because they reinstall `armada` and kill the Fleet the owner is
using. Start Bridge alone against a Fleet that is already up:

```sh
pnpm --filter @armada/desktop build && pnpm --filter @armada/desktop start
```

The build is not optional — `start` previews what is in `out/`, and main always
loads the built renderer rather than a dev server. Bridge finds Fleet through
the runtime file, so nothing here needs a port.

## Build it first

```sh
cargo build --workspace          # the binary lands at target/debug/armada
```

Nothing below works from a stale build. `armada` is the whole CLI — there is no
`fleet-bin`, and `cargo run` is the wrong way in because it rebuilds and the
output interleaves with the daemon's.

## Is one already running?

Ask before starting. Two Fleets over one store is the failure the runtime file
exists to prevent, and the answer costs nothing.

```sh
cat ~/Library/Application\ Support/Armada/fleet.json
```

| What you see | What it means |
|---|---|
| No such file | No Fleet has run, or the last one exited cleanly |
| A file with a `pid` and `port` | Something published it. Whether that pid is **still held by the process that wrote it** is what `armada serve` checks, and you cannot tell from the file alone |

The reliable check is to try to start one — `armada serve .` exits **0** and
prints `Fleet is already running as pid N on port P` when one is. That is not a
failure; it is the state you asked for already holding.

## Start it

```sh
./target/debug/armada serve .
```

**It runs until it is signalled**, so run it in the background and keep the
handle. A healthy start prints the repository and its workflow, the pid, port
and protocol version, what reconciliation found, the turn interval and how many
operations are being served — then goes quiet. Quiet is a Fleet with nothing to
do, not a wedge.

**What a refusal looks like.** Every fault is on its own line and reaches the
terminal before a port is bound: a missing or malformed `armada.yml`, a
`.armada/workflows/` holding no definition or more than one, a step naming a
Check the Manifest does not declare, or an agent CLI a Drone would not find on
its own `PATH`. A refusal exits non-zero.

## Stop it

```sh
kill <pid>          # SIGTERM, which is what it waits for
```

It finishes the turn in flight before exiting — a turn running a Check can hold
it for the whole Check budget, so a terminal that has gone quiet after
`stopping: letting the turn in flight finish` is working, not stuck. The
runtime file is removed on the way out; a `SIGKILL` leaves it behind and the
next start replaces it, saying so.

## Run a Check or a Command without a daemon

`armada check` and `armada run` need no Fleet at all. They read this
repository's `armada.yml` and execute through the same runner a Job's gate
uses.

```sh
./target/debug/armada check test    # a Check — gates advancement
./target/debug/armada run fmt       # a Command — gates nothing
```

The command's own exit code comes back out. A name in the wrong registry is
refused with the verb that would have worked; a name in neither is refused by
listing what is declared. **Output is captured and printed when the command
ends, not streamed** — a long Check prints nothing while it runs, which reads
as a hang and is not one.

Prefer these over retyping the command they wrap. That is the point of the
Manifest: the Check a person runs is the Check a Drone is measured by.

## Clean up

**Destructive. Read this before running it.**

```sh
./target/debug/armada clean          # this repository's worktrees, branches, Jobs
./target/debug/armada clean --all    # and the machine's store beside them
```

Bare, it removes the Armada worktrees under `.armada/worktrees/`, deletes the
branch each one is on, and forgets that Manifest's Jobs. `--all` additionally
removes `armada.db` and its write-ahead files, the runtime file and the MCP
configuration under Application Support.

**Both refuse while Fleet is running**, naming the pid. Stop it first.

**It never deletes a branch by pattern.** Every branch it removes is derived
from a Job it is removing, and a worktree with no Job behind it is reported and
left where it is. `git branch -D $(git branch --list 'armada/*')` is the thing
this verb exists so that nobody types — it destroyed nine unmerged branches
that belonged to no Job.

It prints what it removed item by item, including the commit each deleted
branch pointed at. **That SHA is the only thing that makes the branch
recoverable**, so do not discard the output.

## What not to do

| | |
|---|---|
| Leave Bridge, Storybook or a browser running after you looked | The owner has to find the window and close it. Quit it, and say you did |
| Leave a dialog up in anything you launched | Dismiss it or quit the app. An agent that cannot dismiss its own modal has taken the screen and not given it back |
| Run `pnpm dev` or `scripts/dev` | It is the owner's loop: it reinstalls `armada` and kills the Fleet he is using. Start Bridge on its own instead |
| `git branch -D` over the `armada/` namespace | `armada clean` derives what it deletes. A glob does not |
| `rm -rf` a worktree | git keeps an administrative record under `.git/worktrees/<name>` that outlives the directory and refuses the branch delete afterwards. `armada clean` does it in the order git needs |
| Reinstall the binary from a hook | v1's cold build was four minutes because a hook ran `cargo install` on every merge. `docs/practices/rust.md` section 8 |
