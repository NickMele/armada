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

## The four pains it exists to remove

He was juggling five or more agent sessions and hit these, in these words:

| Pain | Answered by |
|---|---|
| *Which agents need me right now* | The Board. One list, live, saying which Jobs are waiting on a person |
| *Oh, that agent didn't make a worktree* | Fleet makes one, always. A Drone never decides where it works |
| *This agent is stuck because it doesn't know how to run checks properly* | A Drone never runs the Checks. Fleet does, from the repository's own Manifest |
| *This one got ahead of itself and did all of the work in one PR instead of three* | Workflow steps, each gated. **Not yet solved** — nothing stops a Drone doing step two's work during step one |

**Armada is the orchestrator above agents.** It babysits them, and surfaces a
review only when it is confident the work is ready to be looked at.

## Four attempts got here

Each one was abandoned for a reason, and each reason is now a constraint. A
proposal that reintroduces one of them is answering a question already settled.

| Attempt | Why it was abandoned | What it became |
|---|---|---|
| Skills in a repository | Worked, but was not portable to other projects | The Manifest. A repository carries its own setup, and Fleet is pointed at a repository rather than configured per project |
| A CLI | Did not surface the information he needed | Bridge, and why it is a board that is scanned rather than output that is read |
| Orchestrator agents with sub agents | **Having a conversation was not the tool he was looking for** | Armada has no chat. You dispatch, and it reports. The only conversational surface is Helm, and it is not built |
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

**It is not a chat.** See the third attempt.

**It is not about throughput.** Running more agents was never the problem.

## Before proposing work

Ask which of the seven steps it serves, or which of the four pains it removes.
If the answer is neither, say so and ask before building it — the owner defers
work; an agent does not defer it on his behalf, and does not start it on its own
judgement either.
