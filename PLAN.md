# Show the time a Job was created, in the jobs list

The request: "I would love to be able to see what time a job was created in
the job list." Today the list shows *elapsed* — how long a Job has been
running — never the clock time it was created.

## What already exists

| Piece | State | Detail |
|---|---|---|
| `created_at` on the model | done | `crates/core-model/src/job/record.rs:134` — set once at `create()`, immutable |
| `created_at` on the wire | done | `crates/ipc/src/job.rs:68` `JobSummary.created_at: Instant`, populated in `JobSummary::of()` |
| `created_at` on the frontend type | done | `apps/desktop/src/shared/protocol.ts` `JobSummary` — already read at `Jobs.tsx:102, 358` |
| Elapsed display | done | `Jobs.tsx` `elapsedOf()` (`:356-360`) — `span(job.created_at, now)`, shown only while the Job is **non-terminal**; the field is omitted entirely once terminal |
| Absolute time formatting | **missing** | `duration.ts` has `instant()` and `span()` (relative only). No `Intl.DateTimeFormat`-based formatter exists yet |

No backend, IPC, or protocol change is needed — `created_at` is already fully
exposed. This is a frontend-only change.

## The design decision this plan makes

**Reuse the row's existing "time" track; do not add a sixth field.**
`JobRowStacked`'s doc comment (`packages/components/.../JobRowStacked.tsx:294-315`)
names the row's drawn shape as exactly five tracks — origin, bar, step, time,
spend — of which the list currently draws four, spend deliberately omitted.
`Jobs.tsx`'s own header comment is explicit that this shape is a contract, not
a suggestion (`Jobs.tsx:14-49`). A "created at" field is not one of the five,
so it does not get a new track; it goes into the **time** track that already
exists, which today holds only the relative elapsed reading.

**Terminal Jobs (previously blank) get the absolute created time.**
`elapsedOf` returns `undefined` once a Job is terminal, so today's terminal
rows draw no time-track field at all. That gap gets filled with the Job's
absolute `created_at`, formatted — a value where there was none, not a second
value competing with elapsed.

**Running Jobs keep elapsed as the primary reading, with the absolute time on
hover.** Elapsed exists specifically to answer "is this stuck" at a glance
(`Jobs.tsx:41-45`); replacing it outright would lose that. Instead the elapsed
string is wrapped in a plain `<span title="…">` carrying the absolute
`created_at` — a native tooltip, using `JobRowField.value`'s existing
`ReactNode` type, so it requires **no change to `JobRowField` or
`JobRowStacked`** in `packages/components`.

## Order, and what proves each step

1. **`duration.ts`: add an absolute formatter**
   `absoluteOf(at: string): string | null`, using
   `Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" })`,
   following `instant()`'s existing convention: `null` where the string will
   not parse, never a formatted "Invalid Date".
   *Proof:* a couple of direct calls (valid RFC 3339 string → formatted
   string; garbage string → `null`) — this repo has no existing unit-test
   harness under `apps/desktop`, so verify inline via `pnpm typecheck` plus a
   manual check in the running app rather than inventing a test framework for
   one function.

2. **`Jobs.tsx`: populate the time track for every row**
   In `Row()` (`:315-319`), replace the current "push elapsed if present"
   logic with: if non-terminal, push `{ value: <span title={created ? `Created ${created}` : undefined}>{elapsed}</span>, mono: true, quiet: true }`;
   if terminal, push `{ value: created, mono: true, quiet: true }` when
   `absoluteOf(job.created_at)` resolves. A Job whose `created_at` won't parse
   keeps today's behavior (no field), consistent with `newestFirst`'s existing
   "sorts last, not first" handling of the same failure (`Jobs.tsx:97-98`).
   *Proof:* run the app (`docs/practices/bridge.md` / `armada-local` skill for
   how to start a local Fleet) — confirm a running Job's elapsed figure still
   reads as before and its tooltip shows an absolute time, and a terminal
   Job's time track shows the absolute created time where it previously
   showed nothing.

## Cost, honestly

Small: one new pure function, one call site. No protocol, IPC, or shared-
component change. The only judgment call is the UX split (visible for
terminal, hover for running) — the alternative of always showing the absolute
time outright would be simpler code but throws away the "is this stuck"
elapsed reading the row was deliberately built to carry.

## Not claimed

- No change to `JobRowField` or `JobRowStacked` in `packages/components` —
  the tooltip is a plain HTML attribute inside `Jobs.tsx`'s own `ReactNode`,
  not a new capability on the shared row.
- Does not address the unrelated gap noted in `Jobs.tsx:56-69` (statuses with
  no icon-registry glyph) — untouched, out of scope for this request.
- Does not add a unit-test harness to `apps/desktop`; verification here is
  manual/in-app, matching the rest of this file's existing (untested) state.
