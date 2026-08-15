# Reserved designs

Items Armada's own use has raised — a gap, a bug, a thing worth building next — recorded here
one file per item so each can carry a status and be referenced by id instead of by a `§` that
moves every time another is added. Formerly `PLAN.md`'s numbered subsections 15.3.1 through
15.3.9; see [`PLAN.md`](../PLAN.md) §15.3 for the inbox mechanism these build on.

Numbered in the order each was thought of. Numbers are never reused or reassigned.

**Four statuses, and the closing one depends on what was opened.** `RESERVED` is a design that
wants its own pass; `BUG` is a defect found in use. A `RESERVED` item closes as `BUILT` and a
`BUG` closes as `FIXED` — the words are not interchangeable, because *"we built the thing"* and
*"the thing was wrong and is not now"* are different claims and a reader scanning this table is
usually asking which one applies.

| id | title | status | module | hook |
|---|---|---|---|---|
| [001](001-raised-items-need-identity.md) | Raised items need identity | RESERVED | helm | An item Helm surfaces in prose has no id to acknowledge against |
| [002](002-tasks.md) | Tasks | BUILT | helm | `armada task` / `armada tasks` — capture a thing to do without spending anything on it yet, then start it as a Job |
| [003](003-bridge-command-centre.md) | The Bridge as the command centre | RESERVED | helm | One screen for Fleet, Manifest and Guild, not three |
| [004](004-guild-inventory.md) | Seeing what is in your guild | BUILT | guild | `armada guild ls` / `show` — no verb showed you what you had |
| [005](005-inbox-label-not-identity.md) | The inbox records a label, not an identity | FIXED | fleet | Inbox entries keyed by Job name collided; they carry the Job's uuid, close when it ends, and `ls` shows the id |
| [006](006-guild-has-no-way-to-learn.md) | The guild has no way to learn | RESERVED | guild | No provenance means no path for Armada's own updates to reach an existing guild |
| [007](007-scanner-should-propose.md) | The scanner should propose, not only report | RESERVED | manifest | Deterministic findings (package manager, workspaces, exact-name checks) should become proposals, not just report text |
| [008](008-armada-injects-its-own-skills.md) | Armada injects its own skills | RESERVED | cross-cutting | Drones need Armada's own tools and the standing to propose manifest/guild changes |
| [009](009-smaller-things-raised-in-use.md) | Smaller things raised in use, each with its reason | RESERVED | cross-cutting | Five small gaps — `doctor` health, `config scan`'s missing middle option, guild remote status, sync prompts, `--help` drift |
| [010](010-armada-records-its-own-failures.md) | Armada records its own failures | BUILT | cross-cutting | `armada failures` — a failure Armada reported was gone as soon as the scrollback was; navigable at a terminal, and a refusal Armada meant is not one |
| [011](011-what-a-drone-may-do-unattended.md) | What a Drone may do unattended | BUILT | fleet | Nothing ever granted a Drone permission, so every Job stalled on its first `git commit` |
| [012](012-a-drones-progress-through-its-workflow.md) | A Drone's progress through its workflow | BUILT | fleet | A Job stored the current step name and nothing else — no history, no timing, and no way to tell a Drone's *"done"* from a gate agreeing |
| [015](015-batch-a-job-from-many-entries.md) | One Job from many entries | RESERVED | helm | Select several entries and dispatch them as one Job, not one each |
| [013](013-secrets-must-resolve-before-the-run-detaches.md) | Secrets must resolve before the run detaches | RESERVED | manifest | `check --detach` shipped; a resolver written into the run loop would hang a detached run at a prompt nobody can see |
| [014](014-report-what-you-know-went-wrong.md) | Reporting what you know went wrong | BUILT | cross-cutting | `armada report` — a dry-run printed `CREATED` for work it had correctly not done and exited 0, so [010](010-armada-records-its-own-failures.md) could never hold it |
| [016](016-what-the-gate-cannot-prove.md) | What the gate cannot prove | RESERVED | fleet | M4's evaluator decides five of the eight predicates, asks you one, and refuses two — `review_clean` and `subjob_passed` both need a Job that Fleet does not spawn |
| [017](017-what-you-have-not-tried-yet.md) | What you have not tried yet | BUILT | cross-cutting | `armada coverage` — fifteen features landed in a day and nothing could say which had ever been run |

**Armada's task system exists** ([002](002-tasks.md)), so these can become its first rows — one
file per row is what makes that a script rather than a rewrite. It is not run automatically:
`armada task` writes down what *you* decided, and a backlog that fills itself is one nobody
trusts.
