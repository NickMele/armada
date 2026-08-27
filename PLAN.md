# Sort the job list, newest first

## What already exists

| Piece | State | Detail |
|---|---|---|
| `created_at` on `JobSummary` | done | `apps/desktop/src/shared/protocol.ts:43` — already on the wire type. `Jobs.tsx:37,45`'s comments claim `JobSummary` carries no `created_at`; that is stale, not current. |
| RFC 3339 → epoch parser | done | `instant(at: string): number \| null` in `apps/desktop/src/renderer/src/duration.ts:12`. Returns `null` on an unparseable date rather than throwing or defaulting to zero. |
| Job list ordering | **missing** | `Jobs.tsx:114` does `jobs.slice(0, DRAWN)` with no sort. Row order is whatever `state.jobs` happens to be in when it reaches the component. |
| Why the order looks broken today | done (root cause, not the fix) | `apps/desktop/src/main/connection.ts:481` `fold()` prepends a freshly created Job (`[job, ...this.current.jobs]`), which is why a new Job shows at the top. A resync or `reread()` (`connection.ts:208,277`) later replaces the whole array with Fleet's `/jobs` response, whose order is Fleet's, not Bridge's — that's the swap that reads as "goes to the bottom" once a Job is dispatched and a resync lands. |
| JS test runner for `apps/desktop` | **missing** | No `vitest`/test config anywhere in the repo (checked `apps/desktop`, `packages/*`, root). `apps/desktop/package.json` has no `test` script. The only gate today is `cargo xtask verify-foundations` (Rust). This is newly-discovered scope: proving the sort with a failing-then-passing test means standing up test infra for this package first, not just writing a test file. |

## The fix

Sort in `Jobs.tsx`, at the view layer, rather than touching `connection.ts`'s state management:

- No other reader of `state.jobs` cares about order (`atTheGate` filters and counts; `App.tsx:155` does a `.find()` by id) — so the minimal, isolated change is where the list is drawn, not where the state is folded.
- Sort by `created_at` descending (newest first), using the existing `instant()` helper, **before** `jobs.slice(0, DRAWN)` at `Jobs.tsx:114` — sorting after the slice would bound the wrong 200 rows.
- A Job whose `created_at` fails to parse (`instant` returns `null`) sorts last, not first — a corrupt date must not shove real Jobs off the visible 200.
- Don't mutate the `readonly JobSummary[]` prop — copy before sorting.

File touched: `apps/desktop/src/renderer/src/Jobs.tsx` only.

## Order

1. Add a small exported pure function (e.g. `newestFirst(jobs)`) near the top of `Jobs.tsx`, so it can be unit-tested without rendering a component.
2. Wire it in at `const bounded = jobs.slice(0, DRAWN)` → sort first, then slice.
3. Test: construct `JobSummary` fixtures with out-of-order `created_at` (including one unparseable value), assert `newestFirst` returns them newest-first with the bad date last. Must be shown failing against the current (unsorted) code before the fix lands — which first requires resolving the missing-test-runner gap above.

## Not claimed

- `Jobs.tsx:37,45`'s stale comment (claims `JobSummary` has no `created_at`) is not corrected here — out of scope for a sort fix, flagged so implement isn't misled by it.
- `connection.ts`'s fold/resync order mismatch is understood but left alone — the view-level sort makes display order independent of it, so reworking main-process state ordering for a display-only ask is not done.
- Whether to add `vitest` (or another runner) for `apps/desktop` is an open call for the implement step, not decided here — it's a real, repo-wide first-time addition, not a one-line test file.

## Open questions

- None blocking. The test-runner gap above is the one thing implement must resolve before it can satisfy "show the test failing first," and it's called out rather than silently worked around.
