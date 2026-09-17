# Spike 18 — What does a Helm session resolve, and what may it do in each permission mode?

**Dropping `--strict-mcp-config` is the whole of what a session needs to *hold* what a
person holds in a terminal, and the permission mode decides how much of it it may
*call*.** The same launch, with Armada's door passed on `--mcp-config` and nothing else
withheld, came up with nineteen MCP servers, two hundred and seventy-two tools,
eighty-six commands, six agent definitions and the machine's own hooks running.

| Mode | A write to the checkout | A shell line the settings do not cover | Another server's tool |
|---|---|---|---|
| `default` | refused | refused | refused |
| `acceptEdits` | **ran**, the file changed on disk | refused | refused |
| `bypassPermissions` | — | **ran** | — |

**Only the third meets `#1373`'s own definition of done**, which asks how many issues
are in a milestone — a `gh` call. **And the second is the most that can be built without
a way for a person to answer**, which the dock does not have.

**A refusal is clean, not a hang.** Every refused call arrives as a `system`
`permission_denied` line, a `tool_result` with `is_error: true`, and an entry in the
`result` line's `permission_denials` — the shape `crates/adapters/src/transcript.rs`
already reads into a refusal row. Nothing waits, and nothing runs.

## Provenance

Measured on 2026-09-17 against Claude Code 2.1.274 with `--model haiku`, over
`--input-format stream-json --output-format stream-json --verbose`, for `#1373`. The
working directory was a one-commit git repository holding `main.rs`. Every run passed
`--mcp-config` naming a file with an empty `mcpServers` object, standing in for Armada's
door, and **no `--strict-mcp-config`** — so what resolved beside it is the person's own
configuration on this machine and nothing Armada wrote.

Each run's stdout is beside this file, with the scratch path written `<spike>`, home
paths as `~`, each `system/init` inventory collapsed to a count, and hook events reduced
to their name.

| File | Case |
|---|---|
| `018-bypass-runs-a-shell-command.jsonl` | `bypassPermissions`: run `echo parity-ok` |
| `018-default-runs-what-the-settings-allow.jsonl` | `default`: the same command |
| `018-default-refuses-what-they-do-not.jsonl` | `default`: run `chmod 777 <a path>` |
| `018-acceptedits-edits-but-is-refused-a-command.jsonl` | `acceptEdits`: edit `main.rs`, then run `gh issue list` |
| `018-acceptedits-is-refused-another-servers-tool.jsonl` | `acceptEdits`: call an MCP tool from a server the person has connected |

## 1. What a session resolves with the strict flag off

The `init` line is identical across every run above:

| Inventory | Count | Where it came from |
|---|---|---|
| `tools` | 272 | 34 built-ins, and the rest the MCP servers' |
| `mcp_servers` | 19 | user scope and enabled plugins; the door's own file added none, being empty |
| `slash_commands` | 86 | user and plugin |
| `agents` | 6 | user scope and a plugin |
| `skills`, `plugins`, `memory_paths` | present | user and project scope |

The built-ins are the full set — `Bash`, `Edit`, `Write`, `NotebookEdit`, `Read`,
`Task`, `WebFetch`, `WebSearch`, `Skill` and the rest — without `--tools` or
`--allowedTools` being passed at all. **A `SessionStart` hook from the machine's own
settings ran before `init` in every case**, and the repository's `PreToolUse` hooks sit
in the same position: what a hook does is outside what this launch controls, which spike
17 recorded first.

None of it needed a flag. `--strict-mcp-config` was withholding all of it, and it is the
one flag `docs/scope.md` names as earning its place **on a Drone** — an unattended
process that came up holding the operator's accounts. A person talking to Helm is the
operator.

## 2. Resolving a tool and being allowed to call it are different questions

This is the finding that cost the most to learn, because the inventory above reads like
parity and is not. Under `acceptEdits` the session found `mcp__gitnexus__list_repos`
through `ToolSearch`, called it, and was told *Claude requested permissions to use
mcp__gitnexus__list_repos, but you haven't granted it yet*. The server was connected,
the tool was listed, and the call did not run.

The same session, in the same turn, read `main.rs` and edited it without being asked
about either.

So the person's own `settings.json` is what stands between a resolved tool and a called
one — and on this machine that file carries four `allow` rules. **Whoever owns that file
owns Helm's reach**, which is [Kit](../concepts/kit.md)'s job (`#1275`) rather than
something Armada re-derives.

## 3. What a refusal leaves behind

The `default`/`chmod` case, line by line:

- a `system` line, `subtype` `permission_denied`, naming the tool, the call id and
  `This command requires approval`;
- a `tool_result` on that call id with `is_error: true` carrying the same text;
- one entry in the `result` line's `permission_denials`, with the tool's whole input.

The session then said what it had been about to do and stopped, rather than looking for
another way round. A mode that refuses is legible on a Helm thread without anything new
being written.

## What this does not cover

- Armada's own door in place of the empty server file. What the door serves is unchanged
  by any flag here.
- Whether the repository's own `.mcp.json` — which Fleet writes the door into — resolves
  as a second registration of the same server beside `--mcp-config`. Both name the same
  relay, so a duplicate costs a process and changes no answer.
- Whether `bypassPermissions` also bypasses a person's own `deny` rules. Nothing on this
  machine has one to test against.
- How a person answers a call mid-turn. That is spike 15's measurement for a Drone, and
  it needs a surface the dock does not have.
