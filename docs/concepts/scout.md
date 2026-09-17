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

| Source | Reads | Fetched by |
|---|---|---|
| The repository | Files in the checkout on disk | The scout's own read tools |
| GitHub issues and pull requests | Title, body, labels, linked commits | Fleet, through the forge's CLI |
| GitHub milestones | Every issue on it, as a Link each | Fleet, and no scout at all |
| Web pages | Headings and text | Fleet, over HTTP |
| Claude Code sessions | Sessions started in this repository or its worktrees | Fleet, off the transcript file |
| Helm threads | This repository's past Helm conversations | Fleet, off the thread file |

> **Rule.** Fleet fetches a source and hands the scout its text. A scout reaches nothing itself.
> Why: its launch denies every tool that could fetch one and leaves it no MCP server, and that confinement is what makes it safe to run on a person's checkout — spike 017. Reading a source in widens none of it.

> **Rule.** What a scout is handed is bounded, and the Finding says how much was cut.
> Why: a page and a 114-turn session are both unbounded, and a Finding that said nothing would claim a scout read a source whole when it read the front of one.

> **Rule.** A source is material to read and never instructions to follow, and the brief says so before the text arrives.
> Why: an issue or a page is text somebody else wrote. `../contracts/agent-prompt.md` section 5c, and the order is what the brief's own test holds.

> **Rule.** A credential never reaches a scout. The fetch is Fleet's own process, and what crosses is the text that came back.

An address a scout is handed:

| Address | Read as |
|---|---|
| `https://github.com/<owner>/<repo>/issues/<n>` | An issue |
| `.../pull/<n>` | A pull request |
| `.../milestone/<n>` | A milestone |
| Any other `http`/`https` address | A page |
| `armada:session/<id>` | A session of this repository or one of its worktrees |
| `armada:thread` | This repository's Helm thread |

> **Rule.** A session is found under the project directory this checkout's own path keys, and nowhere else.
> Why: another repository's session is then not addressable, rather than addressable and refused. The boundary is where the lookup happens.

> **Rule.** A scout never reads a session, a thread or a file belonging to another repository.
> Why: a Studio belongs to one repository, and the boundary Helm keeps holds here too.

> **Rule.** A Link to any other source stays a Link. Its address is kept and its contents are not read.

> **Rule.** The connections a scout reads through are managed in [Kit](kit.md), in the same place as a Drone's MCP servers and plugins.

Reading Helm threads needs no operation, and gets none: `observe_helm` in `crates/ipc/operations.toml` refuses every agent, and Fleet reads the thread file itself rather than opening that door.

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
