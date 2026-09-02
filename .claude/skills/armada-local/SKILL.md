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
- **Never leave a dialog up.** Dismiss it, or quit the app.

**`scripts/dev` and `pnpm dev` are not yours.** Not because they start Bridge,
but because they reinstall `armada` and kill the Fleet the owner is using. Start
Bridge alone against a Fleet already up:

```sh
pnpm --filter @armada/desktop build && pnpm --filter @armada/desktop start
```

**The build is not optional.** `start` previews what is in `out/`, and main
always loads the built renderer rather than a dev server. Bridge finds Fleet
through the runtime file, so nothing here needs a port.

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
| Reinstall the binary from a hook | v1's cold build was four minutes because a hook ran `cargo install` on every merge. `docs/practices/rust.md` section 8 |
