# Spike 17 — What confines a scout to one checkout, and what does a stop leave?

**`--tools` narrows the session to the read tools, and `--restricted` is what keeps
those reads inside the working directory.** A bare `--allowedTools Read` let a session
read a file in a sibling directory; with `--restricted` the same read was refused as
outside the working directory. **A stop has to be an interrupt**: `SIGINT` to the
process group ends the turn with a `result` line carrying the cost, and `SIGTERM` ends
the process with no `result` at all, so nothing says what the run cost.

## Provenance

Measured on 2026-09-17 against Claude Code 2.1.274 with `--model haiku`, over
`--input-format stream-json --output-format stream-json --verbose`, `--permission-mode
dontAsk`, `--strict-mcp-config` with an empty server file, and
`--no-session-persistence`, for `#1292`. The working directory was a one-commit git
repository holding `routing.rs`; a sibling directory held `notes.txt`. Every run's
stdout is beside this file, with the scratch path written `<spike>`, home paths as
`~`, and each `system/init` inventory other than `tools` collapsed to a count. Hook
events are reduced to their subtype.

| File | Case |
|---|---|
| `017-a-bare-allow-reads-another-directory.jsonl` | `--tools Read,Grep,Glob --allowedTools Read,Grep,Glob`: read inside, read outside, grep |
| `017-restricted-refuses-another-directory.jsonl` | The same, with `--restricted` |
| `017-stop-terminate.jsonl` | `--restricted`, `SIGTERM` to the group at the first `tool_use` |
| `017-stop-interrupt.jsonl` | `--restricted`, `SIGINT` to the group at the first `tool_use` |

## 1. The toolset

With `--tools Read,Grep,Glob` the `init` line lists exactly `Glob`, `Grep` and `Read`,
in both runs. Without `--tools`, a session given the same empty server file listed
thirty-one built-ins, among them `Bash`, `Edit`, `Write`, `NotebookEdit`, `Task`,
`WebFetch`, `WebSearch`, `EnterWorktree`, `RemoteTrigger`, `SendMessage` and
`PushNotification` — and neither `Grep` nor `Glob`, which `--tools` added. That capture
is not kept; its tool list is the deny list in `crates/adapters/src/scouting.rs`.

**`--tools` is a toolset, where `--allowedTools` is not.** `docs/scope.md` records the
second removing none of the built-ins for a Drone; the first removes them.

## 2. Reads outside the checkout

| Run | Read of `<spike>/other/notes.txt` | Grep in `<spike>/other` |
|---|---|---|
| Bare allow | **Answered**, contents returned (line 12) | not asked |
| `--restricted` | Refused: `permission_denied`, then a `tool_result` with `is_error: true` saying the path is outside the working directory | Refused the same way |

Both refusals appear in the `result` line's `permission_denials`. A read the agent
refused arrives as a `tool_result` with `is_error: true` on the call's id, which is what
lets Fleet leave it off the files a Finding lists.

**`--restricted` did not stop the operator's hooks from running**: `hook_started` and
`hook_response` appear before `init` in both runs. What a hook does is outside what a
scout's launch controls.

## 3. What a stop leaves

| Signal | Exit | Last line | Cost |
|---|---|---|---|
| `SIGTERM` | 143 | the `tool_use` it was stopped at | **none reported** |
| `SIGINT` | 0 | `result`, after a `tool_result` rejected by the interrupt and a user-channel `[Request interrupted by user]` | `total_cost_usd` reported |

So a scout's stop interrupts its group first and ends it only if the interrupt is not
heeded. What happens when the CLI is interrupted during a long model response, rather
than at a tool call, was not measured.
