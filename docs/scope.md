# Scope

**What it is:** What Armada is for, in the owner's words, and what it is not. Read before proposing work.

---

**Kind:** Reference.

Armada exists because one person was running five coding agent sessions at once
and could not stop watching them. Everything here is downstream of that. A
proposal that does not serve it is a rabbit hole, however good it is on its own
terms — **and that has already cost a night's work**, so the rule is written
down rather than assumed.

## What it is: a workflow system

The owner's own description, and the shape everything is measured against:

1. He provides a job or task that needs doing.
2. The system determines the type of workflow needed.
3. It creates a workspace to do the work in.
4. Agents do the steps, and the hard parts — coding, building artifacts.
5. **The system does the deterministic parts**: running tests and checks, and
   establishing that the result is what was asked for.
6. **A Judge verifies the work before he needs to review it**, and kicks back
   anything that seems off.
7. When the work is complete he has a set of work he can review, in a pull
   request or a document.

> Anything outside of that is bonus.

**A Studio is where step 1 comes from, and it is part of the product.** You
run the app and point at what is wrong, ask what the code does, read in what
you already have, and promote what holds up into a Job. See
[Studio](concepts/studio.md).

## The four pains it exists to remove

He was juggling five or more agent sessions and hit these, in these words:

| Pain | Answered by |
|---|---|
| *Which agents need me right now* | The Board. One list, live, saying which Jobs are waiting on a person |
| *Oh, that agent didn't make a worktree* | Fleet makes one, always. A Drone never decides where it works |
| *This agent is stuck because it doesn't know how to run checks properly* | A Drone never runs the Checks. Fleet does, from the repository's own Manifest |
| *This one got ahead of itself and did all of the work in one PR instead of three* | Workflow steps, each gated, and a Judge criterion comparing the diff to the step's own scope note. **Partly solved** — it caught one on Aug 28 2026, and it exists only on the coding workflows' `implement` steps, only where a scope note precedes them |

**Armada is the orchestrator above agents.** It babysits them, and surfaces a
review only when it is confident the work is ready to be looked at.

## What a milestone may claim

**A milestone is one thing a person can complete, including its Bridge half.**
There is a moment where it is finished and they can do something they could not
do before.

A milestone naming a part of the system rather than an outcome absorbs work
forever without closing. Surface named the surfaces, and "everything I need to
know is in the app" is a standing bar rather than a finish line — every future
screen either meets it or does not. Its items each turned out to belong to an
event somewhere else, and what was left was never a milestone. It was retired.

**A milestone that ships an act with no surface to invoke it from has not
shipped the act.** It has shipped a route, and left a person with the API. The
half that is easy to file is the half that does nothing alone.

The cost of cutting this way is real: a milestone drawn around one person's
event touches more of the codebase at once than one confined to a crate, and is
harder to fan out across agents. That is the reason the earlier cut existed. It
is worth paying, because the alternative optimises for how work is dispatched
rather than for whether anything is finished.

## Four attempts got here

Each one was abandoned for a reason, and each reason is now a constraint. A
proposal that reintroduces one of them is answering a question already settled.

| Attempt | Why it was abandoned | What it became |
|---|---|---|
| Skills in a repository | Worked, but was not portable to other projects | The Manifest. A repository carries its own setup, and one Fleet serves every repository a person adds rather than being configured per project |
| A CLI | Did not surface the information he needed | Bridge, and why it is a board that is scanned rather than output that is read |
| Orchestrator agents with sub agents | **Having a conversation was not the tool he was looking for** | Dispatch and report is how work gets done, and reading a transcript is not. What a conversation produces is kept as structure — a Studio's nodes, a Job's Evidence, a `helm.changed_checkout` — never a transcript read as a claim. Helm is a conversation with the tools of a terminal session, and the structure is what survives it |
| Armada v1 | Close | This |

**The third one explains more of the design than it looks like.** Evidence is
structured because prose is not a claim. Fleet decides because a self-report is a
signal. The Board is scanned because reading a transcript is the thing being
escaped. Every time a design reaches for "ask the agent", it is reaching for the
attempt that was already rejected.

## What Armada is not

**It is not a sandbox, and confinement is not the point.** A Drone runs in a
worktree with a near-empty environment and no credential, which bounds what it
can reach. That is a floor and is deliberately not a fence:
`crates/adapters/src/harness.rs` records the measurement — `--allowedTools` is a
permission allowlist rather than a toolset, and it removed none of the built-in
tools in three spike runs. A Drone can run a shell.

Real confinement means containers with nothing mounted, and that is a different
system that can be added later. **Time spent hardening the current arrangement
is time not spent on the seven steps above.**

The one confinement that does earn its place is `--strict-mcp-config`, because
without it a Drone comes up holding every MCP server the operator has connected
— measured at seven servers, ninety-five tools, personal accounts. That is not
hypothetical tightening; it is the v1 defect that made a Drone unusable.

The flag stays, and Kit does not weaken it: `#1275` lets a person connect an MCP server from Bridge, and a server reaches a Drone only where that person turned it on for Kit or for the Manifest. A Drone still comes up holding what it was given. See [Kit](concepts/kit.md).

**It is a Drone's and not Helm's.** A Drone is unattended, and the flag is what
stops one reaching an operator's accounts with nobody watching. Helm is that
operator, in their own checkout, reading the reply as it is written — so a Helm
session comes up holding what they hold, measured at nineteen servers on this
machine in
[spike 18](spikes/018-what-can-a-helm-session-do-in-each-permission-mode.md).
The two launches are rendered apart and a test holds them apart.

**It is not built for every kind of repository.** What Armada brings — its
carried workflows and the gates on their steps — assumes code with tests: a Check
a shell can run in a worktree, and a diff a Judge can read. A repository whose
correctness shows up another way needs evidence of its own kind, and Armada does
not bring it.

| Shape | Where its correctness shows up instead |
|---|---|
| A game | A scene or an asset behaving in an engine, not a readable diff |
| Firmware | A toolchain outside the tree, and hardware |
| Notebooks and data | Outputs and cells a text diff renders as JSON |
| Infrastructure as code | Only after `apply`, against real state |
| Prose | There is no behaviour to test |

**In such a repository a coding step can have nothing to check.** Its gate then
passes on a non-empty diff and the Judge alone — an empty Check registry expands
to nothing in `crates/config/src/resolve.rs` — and #847 is what tells a person so.

**A conversation is not a record.** Helm holds what a terminal session holds and edits the checkout on your ask (`#1373`), so it does real work in conversation — and what persists is the Studio's typed graph, the event that names each write, and the Jobs it drafted. Nothing downstream reads the thread. See the third attempt: what was rejected was reading a transcript as a claim, not talking.

**It is not about throughput.** Running more agents was never the problem.

## Before proposing work

Ask which of the seven steps it serves, or which of the four pains it removes.
If the answer is neither, say so and ask before building it — the owner defers
work; an agent does not defer it on his behalf, and does not start it on its own
judgement either.
