# v1's `.claude/settings.json` permission rules

**Finding: there is nothing to harvest.** v1's `.claude/settings.json` never
carried a `permissions` block, allow or deny, at any point in its git-visible
history. The premise this audit was assigned to check — that v1 tuned
permission rules against a Rust-workspace-with-a-daemon over months of real
use — does not hold for this file. That is itself the finding, and the
useful part of this note.

## The whole history

Two commits ever touch the file (`git log v1-final -- .claude/settings.json`):

| Commit | State after | What it did |
|---|---|---|
| `46d8d70` "charkit: initial public commit" | one `PreToolUse` hook, matcher `*`, running `.claude/hooks/clean-room.sh` | no `permissions` key, ever |
| `024cbf6` "refactor(xtask)!: retire the contamination grep and the clean-room rule" | `{}` | deleted the hook registration along with `clean-room.sh` itself |

That is the entire history. v1-final's `.claude/settings.json` is `{}`.

**Why there's no earlier trail to find:** `46d8d70`'s message says so
directly — *"this repository was developed privately and its history was
squashed before publication."* Whatever permission tuning happened during
that private development, if any, is not reachable through git. What
survived into the public tag is two commits and an empty object.

**What the one hook that did exist was for:** `clean-room.sh` blocked reads
of a source repository during the check-engine harvest, per
`docs/ARCHITECTURE.md` §2.7 ("The clean-room rule — retired"), so the
implementer of that one feature never opened the repo it was harvesting
test cases from. It is a one-feature scaffolding hook, not a permission
policy, and `024cbf6`'s message is explicit that it retired on its own
merits once the harvest landed — not because the pattern was wrong. It has
no v2 analog to port: it guarded a one-feature clean-room split that was
retired on its own merits, and nothing about it generalises.

## What v1 leaves behind here

Port: nothing. Reject: the premise, not any specific rule — there were no
`allow`/`deny` rules to accept or reject.

v2's current `.claude/settings.json` already contains everything that
transfers:

```json
{
  "deny": [
    "Bash(git push)",
    "Bash(git push *)"
  ]
}
```

## Proposed `permissions` block

No additions. The concrete proposal is: change nothing beyond what
`.claude/settings.json` already has. Any further rules (e.g. for `cargo
publish`, `rm -rf`, force-push, pnpm/electron-builder release commands)
would be a fresh design, not a harvest — v1 supplies no evidence for them,
and inventing rules under v1's authority when v1 never had them would
misrepresent the source. **The finding is the dead end itself:** the tuning
this subject was expected to yield does not exist, because the history that
would have held it was squashed before publication.

## Test cases implied

None. There is no mechanism here with a failure mode to reproduce.
