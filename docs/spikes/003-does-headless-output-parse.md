# Spike 3 — Does headless Claude Code emit parseable structured output?

**Answer: yes, fully. Not a blob.** Every field the design needs is a named JSON key on a typed
event. No heuristic, no regex over prose, no ordering assumption is required to tell status, edits
and tool calls apart. **Denials are visible too** — in three independent places, so the fallback of
inferring them from a tool call with no result is not needed.

Measured 2026-08-23 against Claude Code **2.1.241**, three runs against one throwaway cargo crate.

| Run | Flags | Transcript |
|---|---|---|
| 1 — the real task | `--allowedTools "Read,Edit,Write,Bash(cargo test:*),Bash(cargo build:*)"` | [`003-transcript.ndjson`](003-transcript.ndjson) |
| 2 — the denial | `--permission-mode default --allowedTools "Read"`, task needs `Bash` | [`003-transcript-denial.ndjson`](003-transcript-denial.ndjson) |
| 3 — the control | no flags, every `CLAUDE_*` env var scrubbed | [`003-transcript-clean-env.ndjson`](003-transcript-clean-env.ndjson) |

## Run 1 — the real task

A cargo crate whose `slugify` lowercases but does not hyphenate, and a test that fails on exactly
that. The task was to fix it and prove it with `cargo test` — a real edit plus a real command.
It succeeded: 17 NDJSON lines, exit 0, 19.7s, $0.117761.

| # | `type` / `subtype` | Carries |
|---|---|---|
| 1–2 | `system` / `hook_started`, `hook_response` | the operator's own SessionStart hook firing |
| 3 | `system` / `init` | session_id, model, cwd, permissionMode, full tool/MCP/skill/agent inventory |
| 4–5 | `system` / `thinking_tokens` | `estimated_tokens`, `estimated_tokens_delta` |
| 6 | `assistant` | a `thinking` content block |
| 7, 9, 12, 14 | `assistant` | one `tool_use` block each: `Read`, `Read`, `Edit`, `Bash` |
| 8, 10, 13, 15 | `user` | the matching `tool_result`, plus a **typed** `tool_use_result` |
| 11 | `rate_limit_event` | `rate_limit_info` — see below |
| 16 | `assistant` | the final `text` block |
| 17 | `result` / `success` | cost, usage, turns, denials, stop reason |

### What each mechanism reads

| Mechanism | Field | Available |
|---|---|---|
| Thrashing detection — turn count | `result.num_turns` (=5); live, count `assistant` events carrying `tool_use` | during **and** after |
| Liveness | `timestamp` on every `assistant` and `user` event | during |
| Cost accounting | `result.total_cost_usd`, `result.usage`, `result.modelUsage[model].costUSD`; per-message `message.usage` | during and after |
| What a Drone did — edits | `tool_use.input` (`file_path`, `old_string`, `new_string`) and `tool_use_result.structuredPatch`, a real hunk list | during |
| What a Drone did — commands | `tool_use.input.command` and `tool_use_result.{stdout,stderr,interrupted}` as separate keys | during |
| Activity Feed | the same typed events in order, each with `uuid` and `parent_tool_use_id` | during |

## Run 2 — a denied tool call is not silent

Run with `Bash` outside the allowlist and a task that cannot be done without it. The denial surfaces
**three times**, each machine-readable:

**1. A dedicated event, the moment it happens.**

```json
{"type":"system","subtype":"permission_denied","tool_name":"Bash",
 "tool_use_id":"toolu_01PLXzb…","decision_reason_type":"other",
 "decision_reason":"This command requires approval","uuid":"…","session_id":"…"}
```

**2. The tool result, tagged.** `is_error: true`, and a sibling key
`tool_result_meta: [{"id":"toolu_01PLXzb…","non_execution_kind":"user-rejected"}]` — so a denial is
distinguishable from a tool that ran and failed, which `is_error` alone would not give.

**3. The terminating event.** `result.permission_denials[]` carries every denial with its **full
`tool_input`** — both attempts at `cargo test 2>&1` appeared.

### The finding that matters more than the denial

**The run reported success having accomplished nothing.** Exit code 0, `subtype: "success"`,
`is_error: false`, `stop_reason: "end_turn"`, `terminal_reason: "completed"`, and a final message
politely asking the operator to approve the command. The tests were never run.

**The `result` event is not a verdict.** Any Armada gate that reads `subtype`, `is_error` or the
process exit code as "the Drone did the work" will pass a Drone that did nothing. `permission_denials`
being non-empty is the cheap, reliable contradiction — and it is exactly the case that a Drone's
own prose report would describe as blocked while the envelope says success.

## Run 3 — the control: a headless Drone is not isolated

Run 1's `init.tools` listed **every** tool, including 95 MCP tools from the operator's seven
connected servers. Run 3 scrubbed every `CLAUDE_*` environment variable to test whether that was an
artefact of spawning from inside a Claude Code session. **It is not.**

| Inherited | Run 1 | Run 3 (scrubbed env) |
|---|---|---|
| Built-in tools | 30 | 30 |
| MCP servers / tools | 7 / 95 | 7 / 128 |
| Plugins | 4 | 4 |
| Subagents | 9 | 9 |
| Skills | 42 | 42 |
| Operator's `SessionStart` hook | fired | fired |
| `permissionMode` | `auto` | `auto` |

**`--allowedTools` did not remove one built-in tool.** All three runs exposed the same 30, including
`Task`, `Workflow`, `WebFetch`, `Bash` and `Write` — `--allowedTools` is a permission allowlist, not
a toolset. `--permission-mode default` *was* honoured in run 2; `auto` in runs 1 and 3 is the
operator's own configured default, not a flag being ignored.

Confinement is M1 step 8's job. The measurement belongs here: **isolation is opt-out, the opt-out is
not `--allowedTools`, and the thing being opted out of includes the operator's MCP credentials.**

## Two things observed that other spikes are chasing

**Quota is already in the stream.** A `rate_limit_event` arrived mid-run, unprompted:
`{status:"allowed", rateLimitType:"five_hour", resetsAt:<unix>, overageStatus, isUsingOverage}` —
direct evidence for *which quota window gates dispatch*, delivered without asking for it.

**`result` and `system` events carry no `timestamp`.** Only `assistant` and `user` events do. A
liveness monitor keyed on that field sees nothing during a long tool call and nothing from the
terminating event; it must use event arrival time.

## Consequence for the design

The Drone adapter is a typed deserialiser over a discriminated union, not a parser. Nothing that
assumed structured output needs to change. Two things do:

1. **The gate cannot read `result` as a verdict.** Success is reported for a run that did nothing.
2. **A spawned Drone does not start from a clean toolset.** It starts from the operator's.
