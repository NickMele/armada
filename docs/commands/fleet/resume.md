# `armada fleet resume`

Start a paused Job's session again.

> **Status: built — M3.**

## Synopsis

```sh
armada fleet resume <job> [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name or uuid prefix | — | Which Job. Required. |

## How it works

A new Drone, on the **same session**: `claude --resume <uuid>`, detached, exactly as
[`answer.md`](answer.md) starts one. A resume that minted a session would start the Job's second
turn as its first, and the transcript is the ledger.

**The words it continues with are Armada's, and they are deliberately not an instruction.**
Resuming means "carry on with what you were doing", so the prompt is one short sentence to that
effect — the headless form needs something to print about, and an empty argument starts a turn
that has nothing to answer. If you have words of your own, they belong in
[`answer.md`](answer.md).

**The budget is not reset**, for the same reason an answer does not reset it: a ceiling that
started over every time a Job was held and let go again would not be a ceiling.

### Three refusals, and each names the verb that would have worked

| When | Because |
|---|---|
| the Job is not `PAUSED` | there is nothing held to let go — [`ls.md`](ls.md) says what each Job is doing |
| it has an open question | it is waiting on **your words**, not on a resume — [`answer.md`](answer.md) |
| it reached a ceiling | `on_exhausted: needs_human` means a person decides what happens next; silently continuing past a ceiling is how a budget stops being one |
| its worktree is gone | there is nowhere to run — [`kill.md`](kill.md) ends it and releases what it holds |

The worktree is checked **before** a Drone is started in it, because `chdir` fails inside the
detached child where the only evidence is a Drone that recorded a group and died immediately.

## Output

```
  STATUS   JOB         DETAIL                       TIME
  resumed  rate-limit  started a Drone, group 902      -

RUNNING  rate-limit, 18 iterations remaining
```

**It starts a turn; it does not wait for one.** What the turn costs lands in the transcript and
is read by [`ls.md`](ls.md).

## Exit codes

`0` resumed · `2` `bad_invocation` — no such Job, or one of the first three refusals above ·
`6` `environment` — the worktree is gone, or `claude` is not on `PATH`.

## See also

[`pause.md`](pause.md) · [`answer.md`](answer.md) · [`board.md`](board.md) ·
[`../helm/bridge.md`](../helm/bridge.md) — `p` on the live screen
