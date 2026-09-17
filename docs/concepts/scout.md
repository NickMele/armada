# Scout

**What it is:** A read-only agent a person starts from a Studio to read the repository or an outside source, which comes back as a Finding. It never writes.

---

**Kind:** Agent.

You ask a [Studio](studio.md) how routing is decided across three packages. A scout reads the files, lists what it read, and returns a Finding on the Studio. Nothing it does changes the repository or anything outside Armada.

## What it is

> **Rule.** A scout reads and never writes: no edit, no commit, no filed issue, no change to an outside document.
> Why: anything a Studio sends outward comes back as a draft a person sends, so a mistake is never published on their behalf.

> **Rule.** A scout starts only on a person's ask, made directly or through [Helm](helm.md).
> Why: a scout spends money, and nothing on a Studio spends until a person asks.

> **Rule.** A scout has no worktree. It reads the repository's checkout as it is on disk, and its Finding records the commit and whether uncommitted changes were present.
> Why: it answers about the code a person is looking at, and the Finding says which state that was.

| | Drone | Helm | Scout |
|---|---|---|---|
| Reads | Its Job's worktree | Fleet, through the Fleet MCP | The checkout on disk, and allowed sources |
| Writes | Code, in its worktree | Through commands, on ask | Nothing |
| Lifetime | One Job | A conversation | One ask |
| Started by | The dispatch gate | A person's message | A person's ask |

## What it may read

| Source | Reads |
|---|---|
| The repository | Files in the checkout on disk |
| GitHub issues and pull requests | Title, body, labels, linked commits |
| Web pages | Headings and text |
| Claude Code sessions | Sessions started in this repository or its worktrees |
| Helm threads | This repository's past Helm conversations |

> **Rule.** A scout never reads a session, a thread or a file belonging to another repository.
> Why: a Studio belongs to one repository, and the boundary Helm keeps holds here too.

> **Rule.** A Link to any other source stays a Link. Its address is kept and its contents are not read.

> **Rule.** The connections a scout reads through are managed in [Kit](kit.md), in the same place as a Drone's MCP servers and plugins.

Reading Helm threads needs an operation: `observe_helm` in `crates/ipc/operations.toml` refuses every agent today.

## What bounds it

| Bound | How |
|---|---|
| Spend | No cap. Its cost is shown on its Finding when it ends |
| Stop | A stop on its node, at any time |
| Record | Its Finding lists every file and source it read |
| Start | A person's ask, and nothing else |

> **Rule.** A scout has no budget cap. A person ends one with the stop on its node.
> Why: its cost is shown on its Finding, and a person's ask is what started the spending.

> **Rule.** A scout takes no place under Fleet's concurrency cap.
> Why: research and Jobs then never wait on each other.

> **Rule.** A stop interrupts the scout first, and ends its process group only if it has not ended ten seconds later.
> Why: interrupted, the agent ends its turn and reports what it cost; ended outright, it reports nothing. Spike 017.

## What it is started as

| | How |
|---|---|
| Tools | Read, search and list, and nothing else: `--tools`, with every other built-in denied by name |
| Directory | The repository's checkout, held there by `--restricted`, which refuses a read outside it |
| Servers | None, under `--strict-mcp-config` |
| Asked | Once, on stdin, never resumed. Its wording is `../contracts/agent-prompt.md`, section 5b |

A Finding lists a file when the agent answered the read, so a read refused as outside the checkout is not listed. A search is listed as its pattern and where it looked. **A search that returns lines of files is a read of them**, so the files it returned lines from are listed with the files read; a listing that returns only names reads nothing, and adds none.

| Operation | Who | Does |
|---|---|---|
| `ask_scout` | A person, on Bridge | Adds a Finding Gathering and starts its scout |
| `start_scout` | Helm on a person's ask, or a person | Starts a Proposed Finding |
| `stop_scout` | A person, on Bridge | The stop on its node |

A Finding whose Fleet stopped while its scout was reading is frozen as failed when Fleet next starts, keeping what it read.

What a scout is told and never told is `../contracts/agent-prompt.md`, section 2.
