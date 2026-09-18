# Spike 19 — Is auto mode reachable for a Helm session, and what asks a person?

**No: `--permission-mode auto` is accepted and not in force.** A spawned `-p` session
passed `auto` reports `permissionMode: "default"` on its `init` line, makes no classifier
call, and behaves identically to `default` on the same command. **What does work is
`default` with `--permission-prompt-tool`** — a shell line, an edit, and another server's
MCP tool each arrive at the named tool, run on its `allow` and are refused on its `deny`,
and the tool may hold the call for minutes. That is the mode Helm runs.

| Mode | A command the settings allow | A command they do not | An edit | Another server's tool |
|---|---|---|---|---|
| `default`, no tool (spike 18) | ran | refused | refused | refused |
| `acceptEdits`, no tool (spike 18) | ran | refused | **ran** | refused |
| `auto` | **identical to `default` in every case below** | | | |
| `default` + `--permission-prompt-tool` | ran, tool never asked | **put to the tool** | **put to the tool** | **put to the tool** |

## Provenance

Measured on 2026-09-17 against Claude Code 2.1.275 with `--model haiku`, over
`--input-format stream-json --output-format stream-json --verbose`, for `#1389`. The
working directory was a one-commit git repository holding `main.rs`, with the machine's
own user settings loaded — the configuration a Helm session resolves since `#1373`.
`--permission-prompt-tool` named spike 15's stdio stub, unchanged
(`015-stdio-stub.py`), which logs what it was asked, waits, and answers.

`019-auto-case.py` drives one case and `019-scrub.py` writes its stream out in spike 18's
shape: machine paths as `<spike>` and `~`, each `init` inventory collapsed to a count,
hook events reduced to their name, ids and cost dropped, and the seconds since the spawn
added as `t`. Where the tool was asked, a `-tool.log` beside the transcript says what it
was asked and when.

| File | Case |
|---|---|
| `019-auto-degrades-to-default.jsonl` | `auto`: a local write, the tool allows |
| `019-default-asks-for-the-same-write.jsonl` | `default`: the same write, the same answer |
| `019-default-asks-and-runs-gh.jsonl` | `default`: `gh issue list`, the tool allows |
| `019-default-honours-a-refusal.jsonl` | `default`: the same, the tool denies |
| `019-default-asks-before-an-edit.jsonl` | `default`: an `Edit` to the checkout |
| `019-default-asks-for-another-servers-tool.jsonl` | `default`: `mcp__gitnexus__list_repos`, no `--strict-mcp-config` |
| `019-default-holds-the-ask-two-minutes.jsonl` | `default`: the tool holds `gh` for 120 seconds |

## 1. Auto mode is not in force for a spawned session

`--permission-mode` accepts six values — `acceptEdits`, `auto`, `bypassPermissions`,
`manual`, `dontAsk`, `plan`. Four of them reach the `init` line; **`auto` and `manual` do
not.**

| Passed | `init` reports |
|---|---|
| `default`, `acceptEdits`, `plan`, `dontAsk` | itself |
| `auto` | `default` |
| `manual` | `default` |

Three further readings say the same thing, and it is the third that settles it:

- **The same command, the same answer.** `touch spike-marker.txt && echo spike-ok` was put
  to the permission tool under `auto` and under `default`, with the same input, and ran in
  both.
- **No classifier call.** Auto mode is a rule classifier — `claude auto-mode defaults`
  prints 17 `allow`, 70 `soft_deny` and 1 `hard_deny` rules in English, and this machine's
  settings add two `soft_deny` rules of their own under `autoMode`. Classifying costs a
  model call, and the `result` line's `modelUsage` in the `auto` case names one model: the
  session's own.
- **`claude auto-mode` is a terminal command.** Its subcommands inspect, critique and
  reset a configuration; none of them is a launch flag.

**So there is nothing to pass down.** Auto mode is the harness's, not the session's, and a
Fleet-spawned process cannot ask for it.

## 2. What the person's own settings decide, either way

This machine's **user** settings carry no `permissions` block at all — the four `allow`
rules a Helm session picks up are the *repository's* `.claude/settings.json`
(`Bash(git push)`, `Bash(git push *)`, `Bash(scripts/land --status:*)`,
`Bash(scripts/land preflight)`). That is the short allow list, and auto mode is why it is
short.

A rule the settings carry is honoured in every mode and never reaches the tool — spike 15
measured that for a Drone and it holds here. The CLI also runs a small set of read-only
shell lines without asking anyone: `echo` and `cat main.rs` ran under `default` and under
`auto` with the tool untouched, where `touch` did not.

## 3. What reaches the tool, and what comes back

Under `default` with `--permission-prompt-tool`, every call the settings do not cover
arrives at the tool with **the tool's name, its whole input and the call id** — including
the two spike 18 recorded as flatly refused:

| Asked | Tool's answer | Outcome |
|---|---|---|
| `Bash(gh issue list --repo … --limit 1)` | `allow` | **ran**; the session answered with the issue |
| the same | `deny` | not run |
| `Edit(main.rs)` | `allow` | ran; the file changed on disk |
| `mcp__gitnexus__list_repos` | `allow` | ran; the session answered with the server's reply |

`#1373`'s own definition of done — *how many issues are in a milestone* — is the first
row.

**An edit now asks.** Under `acceptEdits` a write to the checkout was silent; under
`default` it is put to the tool like anything else. That is more asking than `#1373`
shipped, and it is what the person's settings deciding actually means.

**A deny leaves no `system/permission_denied` line**, which is spike 15's finding
unchanged. What the stream carries is a `tool_result` with `is_error: true` holding the
tool's own message, and one entry in the `result` line's `permission_denials` with the
whole input. `crates/adapters/src/transcript.rs` reads a refusal from the `system` line,
so **a refusal the tool makes is Fleet's own to record** — it already knows, because it
answered.

## 4. How long the tool may hold the call

The door a Helm session is handed is a **stdio** server: `.mcp.json` names the program
`armada mcp`, which relays to the daemon over loopback. So spike 15's stdio column is the
one that applies, not its HTTP column — **5 and 20 minutes held and ran; 60 minutes was
abandoned near 30.** The 60-second HTTP default that `#1389` quotes is the Drone's
transport and not this one.

Measured here at Helm's own shape: the tool held `gh issue list` for **120 seconds** and
the command then ran, with the session finishing at 127.4 seconds.

The binding limit is Armada's own. `fleet::helm::hosting::REPLY_BUDGET` ends a reply's
process at 15 minutes, and a person's answer has to fit inside one reply because
`ProcessHost` closes the session's stdin behind the message.

## What this does not cover

- Whether a future CLI exposes auto mode to a spawned session. This is a measurement of
  2.1.275 and the flag is already accepted, so it may begin working without a flag change
  — which is worth re-measuring rather than assuming.
- `--permission-prompts none`, which denies anything that would prompt. Not measured; it
  is the shape Helm had before this.
- Armada's own door in place of the stdio stub. The wire shape is the same: one
  `tools/call`, one JSON reply.
- What a `deny` message the model is shown does to what it says next, beyond the one line
  measured here.
