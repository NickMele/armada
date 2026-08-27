# Attach files and images to a Job brief

The request: composing a brief in Bridge, a person cannot reference a file or
paste/upload a screenshot the way they can in Claude Code directly. Value is
in the Drone seeing it, not just a human reviewer — "especially when filing a
bug brief" names the case (a screenshot of what's wrong) most needing this.

## What already exists

| Piece | State | Detail |
|---|---|---|
| Brief field | done | `apps/desktop/src/renderer/src/Composer.tsx:115-121` — plain `Textarea`, no attach affordance |
| `Draft` type | done | `apps/desktop/src/shared/bridge.ts:235-251` — `brief: string`, nothing else free-form |
| `ProposeJob` DTO | done | `crates/ipc/src/job.rs:190` — mirrors `Draft`, no attachment field |
| Proposal → Job | done | `crates/fleet/src/drafting.rs` `drafted()` — converts `ProposeJob` to `NewJob`, four named refusals, none about files |
| `Facts` field | done | `crates/core-model/domain/job-fields.toml:151` — free text, "handed to a model whole," redacted from every list |
| First-turn assembly | done | `crates/fleet/src/briefing.rs:88-102` `job_brief()` — renders title + Facts + acceptance criteria only |
| Worktree creation | done | `crates/fleet/src/dispatch.rs:136` `dispatch()`, via `adapter_traits::Vcs::create_worktree` (`crates/adapter-traits/src/lib.rs:152`) → `Worktree::path()` (`worktree.rs:207`) |
| `Prompt` type | done, and staying that way | `crates/adapter-traits/src/harness.rs:83` — `Prompt(String)`. No multimodal content-block support anywhere in the harness |
| Attachment storage of any kind | **missing** | no table, no field, no IPC surface, no UI. Confirmed by grep — the only "attachment"-adjacent hit anywhere is `context_paths` (Evidence Scope for a Judge's read, `docs/concepts/workflow.md:41`), an unrelated concept I initially conflated with this and am flagging as a corrected assumption, not carrying forward |

## The design decision this plan makes

**Attachments are files on disk with a metadata row, not bytes inlined into
`Facts`.** Mirrors Evidence exactly (`job-fields.toml:135-149`: "captured Check
output goes to disk with a pointer in the row... a blob on the Job row would
rewrite whole on every append"). Basing it into `Facts` would also break the
one property that field is defined by — free text a person reads whole and
that gets redacted from every list — by putting binary-derived text in the
same field a secret-scan already has to cover.

**The Drone reads the file itself; nothing teaches `Prompt` about images.**
Files are copied into the worktree before the first turn, and `job_brief()`
appends one line per file naming its worktree-relative path. Claude Code (the
Drone here) already renders an image when its own Read tool opens one — that
capability is not something Armada needs to build, only something it needs to
hand the Drone a path to. Changing `Prompt(String)` into a content-block type
would be a harness-wide, cross-adapter change for a v1 feature that doesn't
need it.

**Staging happens before a Job exists, promotion happens at creation, the
worktree copy happens at dispatch.** A brief is composed before `propose` is
called — there is no Job id yet to key storage on. Three points, three owners:

1. Renderer picks/pastes bytes → preload writes them to a **temp staging
   dir** and hands back a token (bytes never round-trip through the existing
   `bridge:propose-job` JSON channel as base64).
2. `ProposeJob` carries the staged **paths** (same-machine assumption already
   established in `docs/practices/protocol.md`: "a person, through Bridge, on
   the machine Fleet is running on"). `drafted()` promotes each into Fleet's
   own data dir under `<data_dir>/attachments/<job_id>/` and a new `attachments`
   table row, alongside the other three drafting-time refusals.
3. `dispatch()` copies the promoted files into the fresh worktree, and
   `job_brief()` names their worktree-relative paths.

## Order, and what proves each step

1. **Renderer: attach affordance on `Composer`**
   Hidden `<input type="file" multiple>` behind an "Attach" button, plus an
   `onPaste` handler on the Brief `Textarea` reading `clipboardData.items` for
   image types. Local state: `attachments: {name, mimeType, bytes}[]`, one
   removable chip per entry.
   *Proof:* an RTL test added to `apps/desktop`, written to fail against
   today's `Composer.tsx` first (no attach control exists to select), then
   passing once built — asserting (a) picking a file adds a chip, (b) pasting
   a clipboard image item adds a chip, (c) removing a chip drops it from
   state, (d) `propose()` calls `onPropose` with the attachments included.

2. **Preload/main: staging**
   New preload method, e.g. `stageAttachment(bytes, filename, mimeType) →
   Promise<{path: string}>`, writing under `app.getPath("temp")/armada-attachments/<uuid>/<filename>`.
   `Draft.attachments` becomes `{path, filename, mimeType}[]`
   (`apps/desktop/src/shared/bridge.ts:235`).
   *Proof:* main-process test staging bytes and reading the written file back
   from the returned path.

3. **Protocol: `ProposeJob` gains `attachments`**
   `crates/ipc/src/job.rs:190`, new optional `attachments: Vec<AttachmentRef>`
   field (`AttachmentRef { staged_path: String, filename: String, mime_type:
   String }`). Per `docs/practices/protocol.md`'s table, an added optional
   field on an existing DTO is a **minor** bump — move `protocol-version.toml`
   accordingly. Regenerate with `pnpm --filter @armada/desktop codegen`, then
   `cargo xtask verify-protocol`.
   *Proof:* a `crates/ipc` deserialization test fixture with an `attachments`
   array; `verify-protocol` green.

4. **Core-model: an `Attachment` record, own table**
   New `[fields.Attachments]` entry in `crates/core-model/domain/job-fields.toml`,
   shaped like `Evidence`'s (`type = "array<object>"`, `table = "attachments"`,
   `storage = "Own table"`), documenting `filename`, `mime_type`, `byte_size`,
   `storage_ref`, `created_at`. This is a schema decision, not yet code — no
   `in_code = "Yes"` claim until step 5 lands it.
   *Proof:* none by itself; proved by step 5's test reading a row back.

5. **Fleet: promote staged files at Job creation**
   `crates/fleet/src/drafting.rs` `drafted()` — for each `AttachmentRef`, copy
   `staged_path` into `<data_dir>/attachments/<job_id>/<filename>` and insert a
   row. A `staged_path` that does not exist is a fifth named refusal, in the
   same table `drafting.rs`'s own doc comment keeps (today's four: empty
   title, empty model, unknown workflow id, unknown manifest id) — not a
   silent drop, which is the failure mode that doc comment exists to call out
   in the other four cases.
   *Proof:* a `crates/fleet` test proposing a Job with one staged file,
   asserting the row and the on-disk copy exist keyed by the returned Job id,
   and a second test asserting a missing `staged_path` is refused rather than
   silently ignored.

6. **Fleet: worktree copy + brief reference**
   `crates/fleet/src/dispatch.rs`, right after `create_worktree` succeeds
   (line 136 is the `dispatch` function's start; exact insertion point is
   after the `Worktree` is in hand and before `briefing::first_turn` is
   called — I have not pinned the precise line and flag that as needing a
   fresh read at implement time, not assumed here) — copy each stored
   attachment into `<worktree.path()>/.armada/attachments/<filename>`.
   `crates/fleet/src/briefing.rs` `job_brief()` (lines 88–102) appends one
   line per attachment naming that relative path, after `Facts` and before
   acceptance criteria.
   *Proof:* extend an existing dispatch/briefing test asserting the assembled
   first-turn string contains the attachment's relative path, and that the
   file is present in the worktree fixture after dispatch.

## Cost, honestly

This is bigger than "add a file picker." It's a new cross-cutting record type
(own table, mirroring Evidence's precedent — the biggest structural piece),
one protocol bump, a staging→promotion→worktree-copy pipeline across three
processes (renderer, main, Fleet), and a new small UI primitive (an attachment
chip; `packages/components` has nothing like it today, and per
`.claude/skills/armada-components/SKILL.md` that has to go through the design
system contract, not get improvised in `Composer.tsx`). If the reader wants a
cheaper first slice: attachments visible only in the read-only `JobBrief`
composition (`packages/components/src/compositions/JobBrief/JobBrief.tsx`) for
a human reviewing the Job, without steps 5–6, is real but does not satisfy the
brief — the ask is specifically that the Drone sees it, not just a person.

## Open questions

- **Size/count limits.** Nothing in the brief says how large a screenshot
  attachment may be before it's refused, and an unbounded copy into every
  worktree is a real disk-growth question `dispatch.rs` doesn't currently
  have to think about. Needs a number before step 5 ships.
- **Retention of staged-but-never-promoted files.** A drafted-then-abandoned
  brief leaves files in the temp staging dir with nothing to sweep them.
  Deferred out of this plan; flagging rather than silently dropping it (per
  this skill's own instruction not to leave a gap unnamed).
- **Attachment table's exact type key.** I've mirrored `Evidence`'s TOML
  shape by inspection (`job-fields.toml:135-149`) but have not written the new
  entry — confirm the field name (`Attachments` vs. lowercase) against
  `job-fields.toml`'s existing mixed casing convention before step 4 commits
  it (both `Facts` and `evidence` appear as table names in that file, cased
  differently from each other, and I don't want to guess which this should
  follow).
