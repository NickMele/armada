# Half-built things

**What it is:** The defect this codebase produces most — something that exists and is not connected — and how to tell it from the pattern that looks identical and is correct.

---

Read this before finishing a change, and before reporting one finished.

## The defect

**A thing that exists and nothing reaches.** In one night it appeared as all of
these, none of which looked like each other at the time:

| What existed | What was missing |
|---|---|
| The Judge's citation, written to `job_step_judgments` | No caller outside its own tests. `get_job` served nothing, so the reason a step was refused sat in the database with no path to a person |
| Gaming flags naming which pattern tripped | Only the trigger was persisted. A person saw that evidence was suspect and not why — the whole content of the finding |
| `TriggerLevel`, declared and mapped | Read by a test and nothing else |
| `.armada/logs/<job-id>.jsonl`, specified in the log envelope | Written by nothing |
| `stopped`, `retrying`, `awaiting_human` — three of six declared step states | No `StepTarget` reached them, so a refused step stayed `running` for ever |
| `steps[].judged[]`, served on the wire | Zero references in Bridge. The most useful thing Armada produces, drawn nowhere |
| `--allowedTools`, passed to every Drone | Measured: it removed none of the built-in tools. A fence that was a floor |

**The counterpart is a rule stated and not enforced.** `job-statuses.toml` said
an escalated Job keeps its Drone alive; Fleet killed it. It said a failed Job's
step machine is *"frozen at the failed step"*; Bridge drew it as running. Both
were true sentences that nothing made true.

## The pattern that looks identical and is correct

**`crates/ipc/operations.toml` declares forty operations and ten are served.**
That is deliberate, and `xtask/src/rules_protocol.rs` checks it one way on
purpose: *an operation with no route is not yet built rather than wrong.*

Declaring a thing before building it is how this repository works. The registries
are written first and the code catches up.

**So the defect is not the gap. The defect is a gap nothing tracks.**

| | |
|---|---|
| Declared in a registry, unbuilt, checked one way | Correct. The gap is visible and named |
| Written to a table nothing reads | A defect. Nothing says it is unfinished |
| A state declared with no transition reaching it | A defect, and the gate cannot see it |
| A path in a contract that nothing writes | A defect, until an issue says so |

## Before you say a change is finished

**Follow the value all the way out.** A finding that reaches a database and not
a person is not finished. A field on the wire that nothing draws is not
finished. Ask where it is *read*, not where it is written — and if the answer is
"a test", say so in your report rather than counting it as done.

**Read the registry before minting vocabulary.** Three separate agents drafted a
new trigger, a new state, or a new glyph rule and then found the decision already
written down — in `escalation-triggers.toml`, in `judge.md` line 71, in
`icons.toml`'s own reservation. The registries are older than any session and
they usually already answer the question. Minting a second answer is how a
vocabulary splits.

**A rule you cannot point at code for is a rule to check, not to trust.** Both
"Fleet never auto-kills" and "frozen at the failed step" read as descriptions of
behaviour and were descriptions of intent.

**Say what you did not connect.** A report that lists what was built and omits
what nothing reads is how the next agent inherits a half-built thing believing
it is finished. Every instance in the table above was found by somebody reading
code, not by a gate.
