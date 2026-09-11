# Spike 15 — Can a person answer a Drone's blocked command while it waits?

**Yes, through a permission tool, and over HTTP the wait is short.** The agent CLI puts a call
outside the allowlist to the MCP tool named by `--permission-prompt-tool` and acts on its
answer, but never under `--permission-mode dontAsk`. Over HTTP it stops waiting after 60 seconds
by default, and after 300 seconds without a reply once that is raised.

## Provenance

Measured on 2026-09-11 against Claude Code 2.1.268 with `--model haiku`. The `015-stdio-*` cases
used a stdio stub for the tool; the `015-fleet-*` and `015-http-*` cases used the flags
`crates/adapters/src/harness.rs` renders and a stub served over HTTP the way Armada serves its
tools. Every case asked the Drone to write a file, because a read-only command is never put to
the tool.

| Harness | Drives |
|---|---|
| `015-stdio-run-case.py`, `015-stdio-stub.py` | the `015-stdio-*` cases |
| `015-fleet-case.py`, `015-http-stub.py` | the `015-fleet-*` and `015-http-*` cases |

Each case has a `.jsonl` stream, one line per stream line with the seconds since the spawn. Where
the tool was asked, a `-tool.log` beside it says what it was asked and when. Machine paths are
scrubbed.

## Which mode asks the tool

| Case | Mode | Tool answers | Ran | What the stream carries |
|---|---|---|---|---|
| `stdio-allow-default` | asking | allow | yes | the tool got the tool name, the input and the call id |
| `stdio-allow-explicit-default` | `default`, passed explicitly | allow | yes | the same |
| `stdio-deny-default` | asking | deny | no | an error result and one `permission_denials` entry |
| `stdio-allow-dontask` | `dontAsk` | allow | no | the tool was never asked |
| `fleet-dontask` | `dontAsk`, no tool | none | no | a `system/permission_denied` line |
| `fleet-tool-deny` | asking | deny | no | no `system/permission_denied` line; one `permission_denials` entry |

- **A tool's deny leaves no `system/permission_denied` line.** That line is what
  `crates/adapters/src/transcript.rs` reads a refusal from, so Fleet records a refusal its own
  tool makes.
- **The tool is not in the model's tool list.** The `init` line of every asking case names none
  of the stub server's tools.
- **The answer is one text content item holding JSON:** `{"behavior":"allow","updatedInput":…}`
  with the input it was given, or `{"behavior":"deny","message":…}`.

## How long the CLI waits on the tool

| Case | Transport | Tool held | Set in the environment | Outcome |
|---|---|---|---|---|
| `stdio-allow-wait-5m` | stdio | 5 minutes | nothing | ran |
| `stdio-allow-wait-20m` | stdio | 20 minutes | nothing | ran |
| `stdio-allow-wait-60m` | stdio | 60 minutes | nothing | abandoned near 30 minutes; not run; no denial recorded |
| `http-allow-30s` | HTTP | 30 seconds | nothing | ran |
| `http-allow-60s` | HTTP | 60 seconds | nothing | "The operation timed out"; not run; no denial recorded |
| `http-allow-90s-timeout-env` | HTTP | 90 seconds | `MCP_TOOL_TIMEOUT=600000` | ran |
| `http-allow-25m-timeout-env` | HTTP | 25 minutes | `MCP_TOOL_TIMEOUT=1800000` | aborted at 300 seconds without a reply; not run |
| `http-allow-7m-idle-env` | HTTP | 7 minutes | both timeouts at 1800000 | "The operation timed out" near 6 minutes; not run |

The 300-second abort names its own way past it: a per-server `timeout` in milliseconds in the MCP
configuration, or `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` in milliseconds, where 0 disables it. Raising
it exposed a further limit near 360 seconds that no variable tried here moved. A progress
notification also resets the idle abort, and Armada's server sends none, because it answers each
call with a single JSON body.

**Over HTTP, a person's answer fits inside the call only when it comes within five minutes.**
`MCP_TOOL_TIMEOUT` is still needed for that, because its default ends the call at 60 seconds.

## When the tool fails

| Case | Failure | Outcome |
|---|---|---|
| `http-tool-error` | the tool answers with an error result | not run; the Drone is shown the tool's text |
| `http-fleet-unreachable` | nothing listening at the server's address | not run; the tool is reported not found |

Every failure leaves the call unrun, which is the guarantee `dontAsk` gave a detached Drone.

## The operator's own allow rules

| Case | Settings loaded | Mode | `git push` |
|---|---|---|---|
| `stdio-leak-dontask` | the operator's, allowing `git push` | `dontAsk` | ran |
| `stdio-leak-asking` | the operator's, allowing `git push` | asking | ran; the tool was never asked |
| `stdio-leak-local-dontask` | `--setting-sources local` | `dontAsk` | refused |

A command the loaded settings allow skips the tool in every mode.

## What this does not cover

- Armada's own server in place of the stub. The wire shape is the same: one POST, one JSON reply.
- Which flag kept the `system/permission_denied` line out of the stdio `dontAsk` cases. The stdio
  harness passed `--setting-sources local` and `--no-session-persistence` and left out
  `--replay-user-messages`.
- Whether the repository's own `.claude/settings.json` reaches a Drone in a worktree of it.
- What sets the stdio limit near 30 minutes, and the HTTP one near 360 seconds.
