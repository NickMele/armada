---
name: armada-local
description: Running the armada binary in this repository as an agent — what is yours to run, and how to look at Bridge without stranding it on the owner's screen. Load before starting a Fleet, launching Bridge, or cleaning up after Jobs.
---

# Running Armada in this repository

**`docs/practices/running-locally.md` owns the mechanics** — what a healthy
start prints, what Fleet refuses, version skew, what a finished Job leaves
behind, and what `armada clean` will and will not delete. Read it for any of
that. This file is only what is different about being an agent here.

**Work from the built binary, not an installed one.** `cargo build --workspace`
puts it at `target/debug/armada`, and nothing below works from a stale build.
`armada` is the whole CLI — there is no `fleet-bin`, and `cargo run` is the
wrong way in because it rebuilds and interleaves with the daemon's output.

**Ask whether a Fleet is already running before starting one.** Two Fleets over
one store is the failure the runtime file exists to prevent, and `serve` answers
it for free.

**A Fleet of your own is `scripts/dev-fleet`, not a second `serve`.** The store
is found through `HOME`, so a second `serve` on another port still shares the
owner's, and its boot escalates his running Jobs. `.claude/skills/dev-fleet/`.

**Run `serve` in the background and keep the handle.** It runs until it is
signalled, and SIGTERM is what it waits for.

## Looking at Bridge

**An agent may launch Bridge to look at its own work.** A change to a screen
that nobody looked at is a change nobody verified, and no gate here can see a
layout — a screen once shipped about thirty differences from its drawing with
every gate green.

**What is not yours is the owner's attention.** Bridge is a windowed Electron
process and it comes up in front of whatever he is doing, so launching it takes
the screen for as long as it is up. That is worth paying to answer a question
and never worth paying to have a look around.

**There is no hidden-window path.** `createWindow` shows the window on
`ready-to-show` and takes no flag to suppress it.

Three rules, all three from one incident: an agent left a modal dialog on the
owner's screen with no way to dismiss it. **The ban this file used to carry was
wider than what happened**, and it cost more than the dialog did — agents
stopped verifying screens at all.

- **Write down the question before you launch.** Which screen, which state, what
  would tell you it is wrong. A launch with no question is a look around.
- **Quit what you launched**, and say you did. A window left running is one the
  owner has to find and close.
- **Quit it by what you started, never by what it is called.** Confirmed 4 Sep
  2026: `pkill -9 -f "storybook"` was run to stop two Storybooks on ports 6007
  and 6009 and killed a third on 6006 that the session had never started. Keep
  the handle, or match the port you chose — a pattern broad enough to catch
  yours is broad enough to catch the owner's.
- **Never leave a dialog up.** Dismiss it, or quit the app.

## Anything you background, you kill in the same breath

**A Bash call gets a fresh shell.** Whatever you put in the background and do
not kill before that call returns is reparented to init, and it keeps running
long after the session that started it has gone. Nothing reaps it. The owner
finds it on his own machine.

Confirmed 9 Sep 2026, from one flaky-test hunt: a session and its subagent left
250 `yes`, twenty shell spin loops and six forking scripts running for three
hours at load average 433. Every one of them was CPU load raised on purpose, to
reproduce a flake that only appeared under contention — the load was the right
idea. Not one of them was killed. An earlier call in the same session had done
it correctly, capturing the job PIDs and killing them at the end; the next call
dropped that line and detached twenty more.

- **Prefer `run_in_background`** over `nohup`, `disown`, `setsid` or `(cmd &)`.
  The harness tracks what it starts and reaps it when the session ends. A shell
  you detached by hand has left the harness's sight for good.
- **If it must be a plain background job, kill it in the same call.** Keep the
  PIDs — `HOGS=$(jobs -p)` — and `kill $HOGS` before the command returns, or
  `trap 'kill 0' EXIT`. A `wait` inside a `while true` loop is not cleanup; it
  never reaches the end.
- **Bound the load.** `timeout 60` around a load generator makes the worst case
  a minute, not a weekend.
- **Say what you started and that you stopped it**, the same as a window.

The rule is the one two sections up, applied to something with no window: quit
what you launched, and quit it by the handle you kept rather than by what it is
called.

**`scripts/dev` and `pnpm dev` are not yours.** Not because they start Bridge,
but because they reinstall `armada` and kill the Fleet the owner is using. Start
Bridge alone against a Fleet already up:

```sh
pnpm --filter @armada/desktop build && pnpm --filter @armada/desktop start
```

**The build is not optional.** `start` previews what is in `out/`, and main
always loads the built renderer rather than a dev server. Bridge finds Fleet
through the runtime file, so nothing here needs a port.

## Asking what one Job did

**`./scripts/job <job-id>` prints the whole record of one Job** — its
transitions, each step's verdict, what its Drone was refused, and how the run
ended. Reach for it before opening anything under `.armada/` by hand.
`.claude/skills/what-happened-to-a-job/SKILL.md` is how to read what it prints.

## Prefer the Manifest over the command it wraps

**Run a Check through `armada check`, not by retyping what it declares.** That
is the point of the Manifest: the Check a person runs is the Check a Drone is
measured by.

```sh
./target/debug/armada check test    # a Check — gates advancement
./target/debug/armada run fmt       # a Command — gates nothing
```

## What not to do

| | |
|---|---|
| Leave Bridge, Storybook or a browser running after you looked | The owner has to find the window and close it. Quit it, and say you did |
| Leave a dialog up in anything you launched | Dismiss it or quit the app. An agent that cannot dismiss its own modal has taken the screen and not given it back |
| Run `pnpm dev` or `scripts/dev` | It is the owner's loop: it reinstalls `armada` and kills the Fleet he is using. Start Bridge on its own instead |
| Discard what `armada clean` printed | The commit each deleted branch pointed at is the only thing that makes it recoverable |
| `git branch -D` over the `armada/` namespace | `armada clean` derives what it deletes. A glob does not, and one destroyed nine unmerged branches belonging to no Job |
| `rm -rf` an Armada worktree | Git keeps a record that outlives the directory and refuses the branch delete afterwards. `armada clean` does it in the order git needs |
| Detach a process with `nohup`, `setsid` or `(cmd &)` | It outlives the session and nothing reaps it. Use `run_in_background`, or kill it in the call that started it |
| Raise CPU load and not kill it | One flaky-test hunt left 276 processes at load average 433 for three hours. Keep the PIDs, `trap 'kill 0' EXIT`, and bound it with `timeout` |
| Reinstall the binary from a hook | v1's cold build was four minutes because a hook ran `cargo install` on every merge. `docs/practices/rust.md` section 8 |
