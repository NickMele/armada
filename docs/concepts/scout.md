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
| Spend | Its own budget cap per scout, a Machine setting |
| Stop | A stop on its node, at any time |
| Record | Its Finding lists every file and source it read |
| Start | A person's ask, and nothing else |

> **Rule.** A scout takes no place under Fleet's concurrency cap.
> Why: research and Jobs then never wait on each other.

What a scout is told and never told is `../contracts/agent-prompt.md`, section 2.
