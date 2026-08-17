# PLAN — one `arm inbox`, and the screen it lands on

Implements [`docs/reserved/021-the-work-hierarchy.md`](docs/reserved/021-the-work-hierarchy.md)'s
*"The design, decided 2026-08-17"* and
[`docs/reserved/035-the-bridge-becomes-a-ratatui-application.md`](docs/reserved/035-the-bridge-becomes-a-ratatui-application.md).
Both were decided with the owner over a rendered mock-up; this is the order to build them in and
what proves each piece.

**The previous plan — 034 stage one, the Job daemon's mechanical half — landed**, and the daemon is
enabled on the owner's machine. Nothing here depends on it.

## Two halves, and the second is larger

| Half | What | Reserved design |
|---|---|---|
| **A · the listing** | Four origins under one verb, one sort, a fourth status word, `Entry` → `Signal` | [`021`](docs/reserved/021-the-work-hierarchy.md) |
| **B · the screen** | The Bridge becomes a ratatui application, with fixtures that can see it | [`035`](docs/reserved/035-the-bridge-becomes-a-ratatui-application.md) |

**A can land alone; B cannot.** Half A is a verb and a reader — it ships useful on its own, at the
command line, against the goldens the non-interactive path already has. Half B needs the merged
payload to exist before there is anything to draw. So A goes first, and it is not a "phase one" in
the sense of being incomplete: `arm inbox` at a terminal is the thing the owner asked for.

## Half A — one listing

### A1 · `Entry` → `Signal`, and the fourth state

`crates/core/src/failure.rs`. The rename is mechanical and internal; the state is not.

```rust
pub enum State { Open, NeedsHuman, Fixing, Cleared }
```

**`NeedsHuman` is not a new word.** `arm fleet inbox` already draws it for exactly these rows
(`crates/fleet/src/inbox.rs`'s `Kind::word()`), and it is already the Job verdict word for the same
condition — so this promotes a word that ships on one screen to the state on the merged one.

What it splits: `FIXING` currently means both *a Job is working on it* and *a Job is stopped, waiting
on you*. `crates/fleet/src/inbox.rs:181-185` argues the current mapping well — *"a raised item is not
a row nobody has started, it is a row with a Drone stopped in front of it"* — and that is why `Open`
was wrong, not why `Fixing` was right. In one listing the distinction is the difference between
nothing needed from you and everything waiting on you.

`as_entry` (`inbox.rs:200`) maps an open raised entry to `NeedsHuman`; answered and closed stay
`Cleared`. `docs/glossary.md`'s status table gains the word — it fixes the vocabulary, and every
other word in it describes *a thing Armada did*, which this one also does.

### A2 · The reader sorts

`crates/helm/src/verbs/failures.rs:587-599` extends the folded list (newest-first,
`failure.rs:1114`) with inbox entries **in the inbox's own file order, and never sorts after**.
`grep -n "sort" crates/helm/src/verbs/failures.rs` returns nothing.

Invisible today only because `Lens::shows` excludes `Listing::Raised` from both listings. It becomes
visible the instant one listing draws all origins, which is A3. One `sort_by` on `Reverse(last_ms)`,
matching the fold's own order, and a test that a raised entry sorts among faults by time rather than
landing at the end.

### A3 · `arm inbox`, replacing three verbs

| Goes | Becomes |
|---|---|
| `arm failures` | `arm inbox` |
| `arm tasks` | `arm inbox --origin task` |
| `arm fleet inbox` | `arm inbox --origin raised` |
| `arm failures fix` / `arm tasks start` | `arm inbox start` — one name for one operation |
| `arm failures show` / `clear` | `arm inbox show` / `clear` |

**Removed, not aliased.** An alias keeps the five nouns alive, which is the *"running in circles"*
complaint [`020`](docs/reserved/020-the-tui-decided.md) §3 recorded. `arm report` and `arm task`
survive unchanged, because capture is a different act from reading.

`crates/helm/src/args.rs`: `TOP_LEVEL_VERBS` goes 16 → 14; `INBOX_VERBS` is
`["show", "start", "clear"]`; `fleet inbox` leaves the fleet verbs. `Lens` grows a raised arm — and
`failures.rs:77-87` records why that has to be an enum rather than a bool: *"a raised item appeared
under `armada tasks`"* the moment a fourth origin existed.

`crates/helm/src/render/help.rs`: the three `THIS MACHINE` rows (`:987`, `:1070`, `:1108`) become
one. `untried` stays — see A5.

Columns, per [`033`](docs/reserved/033-the-command-centre-designed.md)'s rule and the owner's choice:

```
  ID        STATUS       ORIGIN  DETAIL                              TIME
  4f2a91c8  NEEDS_HUMAN  raised  armada-failed is blocked on a de…    2h
  d1ba9078  FIXING       fault   pulling a guild needs the remote…    3h
  f1b22f05  OPEN         report  Bridge TUI freezes for several s…   40m
```

`ID` first. Three shipped goldens change, and `render_golden.rs`'s cross-audience invariants
(`:3576`, `:3615`, `:3635`) hold them.

### A4 · `--long`, because a few words is not enough

The owner's own words on the truncated single line: *"A few words is not enough for me to determine
if that is the one I want to act on."* At a terminal the answer is the preview pane (B3). At the
command line and through a pipe it is `arm inbox --long`, which prints each signal's whole body under
its summary row.

That is what `Table`'s existing `row.note` is for — and it is also why `Table::spans` dropping the
note (B0) blocks the Bridge half rather than this one.

### A5 · What does not change

- **`untried` stays its own verb**, per [`017`](docs/reserved/017-what-you-have-not-tried-yet.md) and
  `021`'s correction. Four origins. Its record shares no field with `Signal` except a count, and it
  has no id, no state and no promotion path to fold in.
- **The stores stay three files.** `~/.armada/failures.jsonl` + `inbox.jsonl` + `untried.jsonl`,
  append-only. Helm's Stop hook and monitor read `inbox.jsonl` at a hardcoded path; merging the
  stores breaks the only mechanism that makes a raised item reach anybody.
- **`resolve::parse`'s `map(|d| d.manifest)` is untouched.** Manifest gains no new import.

### A6 · `Line::Promoted` carries a uuid

`crates/core/src/failure.rs:528-536` stores the Job's **name**, written as `spawned.data.name`
(`verbs/failures.rs:530`). That is [`005`](docs/reserved/005-inbox-label-not-identity.md)'s defect
reproduced in the failure log — and it is live: `failures.jsonl` holds two `promoted` lines for
`d1ba9078`, both `"job": "armada-failed"`, and signal `0a0c3b82` is the ambiguity error that caused.

The inbox already learned this (`inbox.rs:105-120`): `job_uuid` is the identity, `job` is a label
shown and never resolved against. `Line::Promoted` gains `job_uuid` and keeps `job` as the label.
Old lines carrying only a name still fold — the field is `Option`, and absent means the label is all
there is.

`verbs/failures.rs:623-657`'s `no_longer_being_fixed` — which opens the Job store on every listing
read to repair a stale `FIXING` — resolves against the uuid instead, and its
`named.all(|job| job.state.is_over())` avoidance goes.

## Half B — the screen

### B0 · `Table::spans` stops dropping notes — first, and alone

`Table::render` draws a hanging note under the second column (`render/table.rs:364-372`);
`Table::spans` never reads `row.note` (`:414-419`). Every ratatui surface loses it, and the parity
test at `:658` cannot catch it because it compares against a line `spans` never emits.

A test that a row **with** a note renders the same line count through both emitters, failing first.
Nothing else in Half B can draw a second line until this is true.

### B1 · `TestBackend` fixtures, before any widget changes

Today **no golden can see an interactive surface**: all three Bridge fixtures route through
`render.rs:882`, the `--once` path, which passes `None` for the cursor and `false` for keys
(`:901-902`). Measured — no golden contains the focus marker, and none contains a KEYS box.

So the fixtures come first, against the screen **as it is**: `TestBackend::new(w, h)`, a `Terminal`
over it, `paint`, and the buffer dumped as text beside the existing `.plain`/`.tty` files, under the
same no-update-flag discipline (`render_golden.rs:24-29`, `ARCHITECTURE.md:433`). Cases: the cursor on
a row, the KEYS box, the detail pane, the reap preview, the keys page, the compose box, the filter
line — the seven surfaces with no byte-level coverage.

**This is the step that makes the rest reviewable.** Every widget change after it has a snapshot that
either agrees or says exactly what moved.

### B2 · `Layout` and `Block`

`Layout::vertical`/`horizontal` with `Constraint` for the panel rects, and `Block` with `Borders` for
each panel's frame, replacing `frame::titled_box` and `frame::hjoin` **in the Bridge only**. `--once`
keeps `frame.rs` and its own goldens.

This is where `80d452a5` (resize) and `14cd98ab` (tint) become possible rather than fixed: content
composed at `f.area().width` per frame instead of a width captured in `main`, and `Block::style`
carrying a per-section background. **Fixing them is the separate pass** the owner chose; this makes
them one-liners rather than rewrites.

### B3 · The preview pane

Side by side above `render::WIDE` (138), stacked below it — the branch `render.rs:1009` already makes,
and `frame::shed_to_narrow` already implements for the five panels.

The pane shows the selected signal's whole body plus its origin, age and count, and the next action as
an offer rather than a record — `inbox.rs:193-199`'s reasoning about the `TYPED` column applies here
too.

### B4 · Scrolling, which needs a height

Three facts, each verified rather than assumed:

| | |
|---|---|
| `Terminal` has `usable_width()` and **no height accessor** | `render/term.rs:98` |
| No panel caps its rows; the screen is one `Paragraph` with no `.scroll()` | `bridge.rs:700` — ratatui clips the bottom silently |
| `Cursor` is a bare wrapping index — no offset, no window | `core/fleet/bridge.rs:238-279` |

So the cursor can already sit on a row nobody drew, and a preview pane makes the list shorter, which
makes it arrive sooner. `Terminal` gains a height, `Cursor` gains an offset, each panel draws a
window, and `Scrollbar` + `ScrollbarState` draw the indicator — all of it already in the pinned tree.

**`Cursor`'s offset stays in `crates/core` and stays pure**: it is state the screen owns, and core may
not open a file or name a backend (`ARCHITECTURE.md` §1.5). The window's *height* is passed in,
exactly as `needs`/`decide` take their facts as arguments.

## Order, and why it is this one

1. **A1, A2, A6** — the record and the reader. No user-visible change; every later step reads them.
2. **A3, A4** — the verb. Ships useful alone.
3. **B0** — `spans` stops lying. Blocks everything after it.
4. **B1** — fixtures over the screen as it is. Makes 5 reviewable.
5. **B2, B3, B4** — `Layout`/`Block`, the pane, the scroll.
6. The six recorded TUI complaints, **their own pass**, on top of B1's coverage. `docs/HANDOVER.md`
   puts the freeze (`f1b22f05`) first in the owner's priority order, and it stays first — after this.

Steps 1 and 2 are one Job; 3 and 4 are one Job; 5 is one Job. Splitting 5 further would mean two Jobs
editing `bridge.rs` at once, which is what produced the `--once`/screen divergence
`render.rs:885-893` records.

## Testing, each shown failing first

| # | Test | Fails today because |
|---|---|---|
| A1 | an open raised entry reads `NEEDS_HUMAN`, not `FIXING` | `as_entry` maps it to `Fixing` |
| A2 | a raised entry sorts among faults by time | nothing sorts the merged list at all |
| A3 | `arm inbox` shows all four origins; `--origin task` shows one; `arm failures` is an unknown verb | `inbox` is not a verb |
| A4 | `--long` prints a body longer than the DETAIL column, in full | there is no `--long` |
| A5 | `arm untried` is unchanged, and `inbox` never lists an untried row | inverted — it must stay green |
| A6 | a promoted signal resolves its Job by uuid when two Jobs share the name | it resolves by name and refuses |
| B0 | a row with a note renders the same line count through `render` and `spans` | `spans` drops the note |
| B1 | a `TestBackend` snapshot contains the focus marker and the KEYS box | no fixture can see either |
| B4 | with more rows than height, the cursor stays inside the drawn window | there is no window and no height |

`armada manifest check` at the end, not as a substitute.

**`armada:test` is not reliable while another Job is running, and that is now a blocker rather than
a nuisance.** Measured 2026-08-17 on the machine this plan was written on: load average **297** with
two Jobs running their own suites, and six `armada-helm::fleet` tests failing at
`fleet.rs:697` — *"the stub Drone never finished a turn"* — against an unmodified tree. Three
separate Jobs hit it the same day on three different sets of tests, each passing in isolation. Job
`e69b6235` (`test-flakiness-under-load`) is researching whether the answer is serialising test
execution across Jobs, isolating per-Job resources, or reducing in-run parallelism under load.

Until that lands, a red `armada:test` needs its log read before it is believed, and this plan's own
steps should not be gated by a suite two other Jobs are competing for. Do **not** treat re-running
as the fix — that advice was written here before the cost was visible, and it is what let three Jobs
burn a day on it.

## Open questions

1. **Does `arm inbox` default to open signals or to everything?** `arm failures` reads the whole
   machine and `arm tasks` filters by scope (`failures.rs:138-141`), so the merged verb has to pick
   one. Leaning: open-and-needing-you by default, `--all` for cleared — the listing's job is what is
   still yours.
2. **Does `Signal` reach the envelope?** `ShowData`/`FailureRow` are `schema_version: 2`. Renaming the
   Rust type is internal; renaming a JSON field is a schema change with `--json` consumers behind it.
   Leaning: the type renames, the wire keeps its names, and that divergence gets one line of doc
   comment saying so.
3. **The KEYS panel.** `033` specifies bordered and two lines; `fa770bfc` says the shipped single line
   is illegible. That sits in the separate complaints pass, but B2 is where a `Block` makes it cheap —
   confirm the owner wants it there rather than earlier.
