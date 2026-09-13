# Spike 16 — Can Fleet resume a headless session after it exits?

**Yes.** `claude -p --resume <session-id>` in stream-json mode recovers everything said
before, keeps the same `session_id` across the resume, works from a different working
directory than the run that created the session, and fails cleanly — exit code 1, one
`result` line, no hang — when the id is not found. What it needs is the transcript
under `~/.claude/projects/`, and that transcript's own lifetime is not something a
resuming process can set for itself: retention is one age-based sweep reading whichever
`cleanupPeriodDays` the sweeping process resolves at the time, not a value stamped onto
a transcript by the session that wrote it.

## Provenance

Measured on 2026-09-13 against Claude Code 2.1.270 with `--model haiku`, over
`--input-format stream-json --output-format stream-json --verbose`. Every run's raw
stdout is captured verbatim to a `.jsonl` file beside this one. Home paths, the account
name and the messaging socket path are scrubbed; each `system/init` event's own
inventory (tools, skills, agents, plugins, MCP servers, slash commands) is collapsed to
a count, the way spike 4's transcripts already do — nothing the findings below rest on
is in that inventory.

| File | Case |
|---|---|
| `016-resume-turn1-fact.jsonl` | turn 1: give a fact, exit |
| `016-resume-turn2-recall.jsonl` | `--resume`, turn 2: ask for it back |
| `016-resume-turn3-second-fact.jsonl` | `--resume` again, turn 3: give a second fact |
| `016-resume-turn4-both-facts.jsonl` | `--resume` a third time, turn 4: ask for both, in order |
| `016-resume-different-cwd.jsonl` | `--resume` of the same session from a different cwd |
| `016-resume-unknown-id.jsonl` | `--resume` of an id that was never issued |
| `016-live-process-two-turns.jsonl` | one process, stdin held open, two turns sent without exiting |

`crates/adapters/src/harness.rs` renders none of `--resume`, `--fork-session` or
`--settings` today; nothing here changes it.

## 1. Resume by id

One process, one turn, exit. A second process with `--resume <id>` recovers the fact
and answers correctly:

| Turn | Process | Asked | Answered | `session_id` reported |
|---|---|---|---|---|
| 1 | new | "The codeword is grackle." | OK | `4e0fb413-…` (`016-resume-turn1-fact.jsonl:4`) |
| 2 | `--resume 4e0fb413-…` | "What is the codeword?" | `grackle` | `4e0fb413-…` — **same id** (`016-resume-turn2-recall.jsonl:3,9`) |
| 3 | `--resume 4e0fb413-…` | "The second codeword is thistle." | OK | `4e0fb413-…` (`016-resume-turn3-second-fact.jsonl:7`) |
| 4 | `--resume 4e0fb413-…` | "List both codewords, in order." | `grackle, thistle` | `4e0fb413-…` (`016-resume-turn4-both-facts.jsonl:9`) |

**A resumed run reports the same `session_id` it was given, not a new one** — `--resume`
was passed without `--fork-session`, which the CLI's own `--help` names as the flag that
forks a new id on resume. Three resumes deep, all four turns are still in view: the model
lists both codewords in the order they were given. This is the shape `#939`'s definition
of done asks for, one level past it — two messages remembering each other, then a third.

## 2. Turns within one live process

Spike 4 already measured injection latency and mid-tool-call delivery for a live,
stdin-held-open process; this only confirms the piece `#939` asks for — that the second
turn remembers the first without an exit between them. It does, `016-live-process-two-turns.jsonl`:
turn 1 gives a fact, turn 2 (sent after the first `result` line, same process, same stdin)
recalls it, both `result` events name `session_id: b9eb4434-…` (lines 9 and 13).

**A second `system/init` line arrives with the second turn**, at line 10, same
`session_id` as the first at line 4 — restating spike 4's finding that `init` is a
per-turn event, not a once-per-process one. Nothing here contradicts spike 4; this is
the same mechanism, re-confirmed for memory rather than for latency.

## 3. What resume needs

**A different working directory than the one the session started in.** `--resume` from
`cwd-b` recovered a session started and previously resumed from `cwd-a`, and answered
correctly (`016-resume-different-cwd.jsonl:9`, `result: "grackle, thistle"`). The
transcript itself never moved — it stayed at
`~/.claude/projects/<hash of cwd-a>/4e0fb413-….jsonl` — so `--resume` is doing a lookup
by session id across the project store, not a lookup scoped to the invoking cwd. Fleet
does not need to resume Helm's session from the same directory it was started in.

**An unknown id fails cleanly.** `--resume 00000000-0000-0000-0000-000000000000` exits
`1`, with stderr `No conversation found with session ID: …` and exactly one stdout line:
a `result` event, `subtype: "error_during_execution"`, `is_error: true`,
`errors: ["No conversation found with session ID: …"]`, zero cost, zero turns
(`016-resume-unknown-id.jsonl:1`). No hang, no partial stream — a Fleet that gets this
back knows at once to start fresh rather than wait.

## 4. Transcript cleanup

**Answered from the docs, not measured.** `code.claude.com/docs/en/claude-directory#cleaned-up-automatically`:

> Claude Code deletes the files in the paths below once they're older than
> `cleanupPeriodDays`, as long as it can safely determine the retention period. The
> default is 30 days and the minimum is 1; setting `0` fails with a validation error.

and `code.claude.com/docs/en/data-usage#data-retention`:

> Local caching: Claude Code clients store session transcripts locally in plaintext
> under `~/.claude/projects/` for 30 days by default to enable session resumption.
> Adjust the period with `cleanupPeriodDays`.

**It is one sweep, reading whichever `cleanupPeriodDays` the process running the sweep
resolves at that moment — not a value a session attaches to its own transcript.** Nothing
in the docs describes a per-transcript stored retention; `cleanupPeriodDays` is an
ordinary settings key, resolved by the usual precedence (managed, then `--settings` for
that invocation, then project-local, shared-project, user). A Helm resume started with
`--settings '{"cleanupPeriodDays": 90}'` would apply 90 days to whatever sweep *that
invocation* runs — it does not mark Helm's transcript file as a 90-day file for the next
process (a person's own terminal session, say) that happens to run the sweep under the
ordinary 30-day default. The docs also name two ways the sweep does not touch anything:
`claude -p --bare` skips it outright, and it pauses whenever settings can't be read.

`#73`'s own decision — "30 quiet days, then the conversation clears" — already matches
the CLI's unconfigured default, so Helm's design does not need to override
`cleanupPeriodDays` at all.

**Where transcripts land, confirmed by running one:** `~/.claude/projects/<cwd, slashes
turned to dashes>/<session-id>.jsonl` — one file per session, at the project directory
keyed by the cwd the session was *started* from (find, above).

**What this does not establish.** Whether the sweep runs at process start, at exit, or on
some other cadence, and whether our own tiny single-turn runs actually triggered one, were
not confirmed empirically. The natural test — age an old transcript's mtime and a run a
session with a short `cleanupPeriodDays` in the same project to see if it disappears — was
attempted and refused by Claude Code's own permission classifier as "Session Transcript
Tampering." That refusal is itself informative (the harness treats transcript files as
sensitive enough to guard against scripted edits) but it means this question is answered
from the docs alone, as the step allows, and no further.

## What this does not cover

- Whether Fleet's own use of `--resume` needs `--fork-session`, or ever will — not
  exercised here, since `#939`'s shape reuses one id per repository.
- `--settings` combined with anything else `crates/adapters/src/harness.rs` renders
  (`--strict-mcp-config`, `--permission-mode`, `--disallowedTools`); this spike only adds
  `--resume` and, for the cleanup question, a bare `--settings` flag on its own.
- What happens on `--resume` of a session whose transcript was already swept — not
  reproduced, since ordering a real 30-day-old file to test against is the same
  tampering the classifier above refused.
