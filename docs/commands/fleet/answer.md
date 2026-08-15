# `armada fleet answer`

Give a waiting Job your decision and let it continue.

> **Status: built — M3.**

## Synopsis

```sh
armada fleet answer <id|job> "<answer>" [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<id\|job>` | inbox entry id, or a Job name or uuid | — | Which row, or which Job. Required. An entry id is a prefix of the `ID` column [`inbox.md`](inbox.md) draws; a prefix matching two open entries is refused, as is a Job name meaning two Jobs. |
| `"<answer>"` | string | — | What to tell it. Required. |

## How it works

0. **The handle is tried as an open entry's id first, and as a Job second.** Naming the row is
   what makes a table of things to deal with something you can act on one item at a time
   ([`../../reserved/001-raised-items-need-identity.md`](../../reserved/001-raised-items-need-identity.md));
   naming the Job answers whichever of its open entries is oldest, which is what every caller
   did before ids were drawn and is still right when a Job has asked exactly one thing. The two
   spaces are not merged — a Job's uuid and an entry's are identities of different things — so
   the order is the whole rule, and nothing that worked stops working because a Job handle has
   never been an open entry's uuid.
1. Marks the entry answered. **The entry is found by the Job's uuid, never by its name** ([`../../reserved/005-inbox-label-not-identity.md`](../../reserved/005-inbox-label-not-identity.md)): a name is handed out again once the Job holding it is over, so answering by name let a fresh Job close its namesake's questions.
2. **Resumes the Job** with `--resume <uuid>` and your answer as the next turn, in its own
   worktree with its context intact ([`PHASES.md`](../../PHASES.md) §9.1 F1). The resumed
   Drone is detached exactly as a fresh one is: an answer *starts* a turn and does not wait for
   one, so a Job you answered before lunch is working while you are out.
3. The Job continues its workflow from where it stopped. **Its budget is not reset** — an
   answer is a continuation, not a new run, and resetting the ceiling here would make budgets
   unenforceable for any Job that asks a question. The resumed session appends its own `result`
   to the same transcript, so continuing costs what it costs and the sum keeps counting.

**A Job that has ended is refused, and told why.** Its entries were closed when it reached
`DONE` or `ABORTED`, so there is nothing to answer — and the refusal says *it has ended* rather
than *nothing is open*, because those are two different facts with two different remedies.

**A Job that has already reached a ceiling is refused rather than resumed.** `on_exhausted:
needs_human` means a person decides what happens next; silently continuing past a ceiling is how
a budget stops being one. The refusal names the ceiling and points at
[`board.md`](board.md).

## Output

```
  STATUS    JOB            DETAIL                TIME
  answered  nightly-flake  yes, raise it to 90s     -

RUNNING  nightly-flake, 7 iterations remaining
```

**The summary line is where a reader sees that the budget was not reset.** An answer is a
continuation, not a new run.

`--json` returns `job`, `uuid`, `entry`, `answer`, `state`, `budget_remaining` and the resumed
Drone's `pgid`. **Not what the turn spent** — it has not finished one.

## Dependencies

An existing Job with an open inbox entry, and `claude` to resume it.

## Exit codes

`0` resumed · `1` `tool_failed` — the resume failed · `2` `bad_invocation` — unknown Job, an id matching two open entries, no open entry, or a ceiling already reached · `6` `environment` — `claude` is not on PATH.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`inbox.md`](inbox.md) · [`board.md`](board.md)
