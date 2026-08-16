---
id: 027
title: The CLI under a user's hands
status: RESERVED
module: cross-cutting
raised: a review of the whole CLI, driven against scratch repos, 2026-08-16
---

# 027 — The CLI under a user's hands

**What this is.** A review of Armada's ergonomics from the position of somebody using it rather
than reading it: the binary built and driven against two scratch repositories and a scratch
`$HOME`, `--help` on every verb, every interactive surface opened at a real pty. Three defects
found in ordinary use that morning set the shape — an id the listing prints and the verb
refuses, a `--budget` key spelled one way going in and another coming out, and `armada helm`
printing a command for the reader to carry somewhere else — and the review's job was to find the
rest of that family before they were found the same way.

**Organised by how badly each one interrupts somebody, not by which module it lives in.** Six
stop people; the rest are a table. That ordering is the argument: a list of forty equal-weight
nits gets skimmed and nothing gets fixed, and the six below are all reachable inside the first
twenty minutes of using Armada for real.

**Every output quoted here was produced by the binary at `4b0d17b`**, not reconstructed. Where a
reading depends on guessing at intent, it says so.

---

## The six that stop people

### 1 · The tick list prints an instruction that is false at a terminal

**The worst defect found, because it is silent and it writes a file.** `armada manifest config
scan` at a terminal offers to write the proposals; choosing that opens the tick list, whose last
line reads:

```
  Which of these did it get right?
  1  [X]  components.scratch-api               the repository itself
  2  [X]  components.scratch-api.setup         npm install
  3  [X]  components.scratch-api.checks.test   npm run test
  4  [X]  components.scratch-api.checks.lint   npm run lint
  5  [X]  components.scratch-api.checks.build  npm run build
  enter writes all of them, or numbers to leave out
```

Pressing `2` and then `enter` writes **five of five**, and says so — `proposals confirmed · 5 of
5` — and the `setup:` line the reader had just declined is in `armada.yml`. The instruction is
not merely incomplete; it names the one key that does not do what it says, and omits the one that
does. `armada_helm::ask::select::apply` binds a digit to [`Press::Moved`] and `space` to
[`Press::Toggled`], with a comment explaining exactly why a digit must not choose: *"a digit that
chose on its own would hand that `enter` to the next prompt"*. The reasoning is right. The legend
was written for the other reader.

**Because that legend is true through a pipe.** `armada_helm::ask::mod`'s line fallback does read
the typed numbers as the set to leave out, which is a sound design for a stream with no cursor.
So one sentence is true in one mode and false in the other, and the mode is invisible — the same
words are printed either way. **This is a defect of the same family the tick list exists to
prevent**: `config scan` will not infer a `test:` from a script called `test:changed`, on the
grounds that a proposal nobody can audit is worse than no proposal, and then hands the audit a
control that does not respond.

**What it should do.** The terminal legend names the keys the terminal has: `space` ticks,
`enter` writes what is ticked, `esc` writes nothing — and, given §6 below, digits move. The pipe
legend keeps its own wording, because through a pipe it is correct. Both come from one function
that takes the mode, so they cannot drift again. **Cost: small** — one function, two strings, and
a test that opens each mode and asserts the legend names only keys that surface binds.

### 2 · The inbox prints an id `fleet answer` refuses, and blames the wrong noun

The reported defect, reproduced verbatim. Two entries seeded, one open and one already answered:

```
$ armada fleet inbox --all
  STATUS       ID        JOB             DETAIL                             TIME
  NEEDS_HUMAN  4f2a91c8  nightly-flake   The flaky test passes on retry. …  245h
  BLOCKED      7c05e3d1  port-collision  Port 5432 is held by something o…  245h

OK  1 open, armada fleet answer <id> "…"

$ armada fleet answer 7c05e3d1 "no"
error: no Job called `7c05e3d1`
  where: 7c05e3d1
  class: bad_invocation
  next:  `armada fleet ls` lists them; there are none yet
```

**Three things are wrong and only one of them is the id.** `armada_fleet::inbox::find_open`
matches on `entry.is_open()`, so a closed entry matches nothing; `armada_helm::verbs::fleet`'s
`answer` then falls through to `armada_fleet::jobs::Store::find`, which is the **Job** space, and
reports the miss in that space's vocabulary. So a string this listing printed one second earlier
comes back as *"no Job called"* — a noun the reader never mentioned — followed by a pointer to
`fleet ls`, which is empty, offered as the way to find something `fleet inbox` had just shown
them. The fall-through is deliberate and documented and it is also the whole mechanism: **an
ordered fall-through between two id spaces reports every miss in the vocabulary of the last space
tried**, which is the space the reader was least likely to have meant.

**And the listing gives the reader no way to know.** `--all` draws the closed row in the same
shape as the open one; the `STATUS` column holds the entry's *kind* (`NEEDS_HUMAN`, `BLOCKED`),
never its openness, and only the summary line's `1 open` discloses that one of the two rows is
history. A reader scanning a table for the thing to act on has no column that answers *can I act
on this*.

**Related, and the same root.** An entry whose Job record is gone — reaped, or a `~/.armada`
restored without `jobs/` — resolves its `job_uuid`, fails to open the file, and answers:

```
error: could not read the Job index: No such file or directory (os error 2)
  where: ~/.armada/jobs/a3f91c02-….json
  class: environment
  next:  check ~/.armada/jobs/ is readable, then retry unchanged
```

`environment` means *fix the machine and retry unchanged*, and retrying will never work: the file
is not unreadable, it is absent, and the honest answer is that the entry outlived its Job.

**What it should do.** Answering resolves against both spaces and reports the miss against
**both**, naming what it looked for: `` `7c05e3d1` is not an open inbox entry and not a Job ``.
When the id *does* match a closed entry — which the store can see — say so, with the answer that
closed it, because that is nearly always what the reader wants to know. Add an openness column to
`--all`, or drop the closed rows from the default and let `--all` earn its name. Distinguish a
missing Job record from an unreadable one. **Cost: small** — the resolution already visits both
stores; this is what it says on the way out.

### 3 · Six verbs refuse where a selector already exists three files away

**The largest single item in the review, and the one with the most in it.** Every one of these
answers the same way:

```
$ armada fleet answer
error: `armada fleet answer` needs a Job and what to tell it
  next:  `armada fleet answer nightly-flake "yes, raise it to 90s"`

$ armada fleet board
error: `armada fleet board` needs a Job
  next:  `armada fleet ls` lists them
```

So does `show`, `pause`, `resume`, and `kill`. In each case the verb knows the whole set it would
accept, the terminal is a terminal, and **Armada already owns the widget**: `armada fleet reap`,
`armada guild ls`, `armada failures`, `armada tasks`, `armada untried`, `armada guild delete`,
`armada guild upgrade` and `armada fleet spawn`'s workflow question all put a list up and let the
person pick. The refusal is not a missing feature, it is an inconsistency — half the CLI offers
the list and half tells you to go and read it.

**`fleet answer` is the sharpest case, because the second argument is worse than the first.** An
answer to a Drone is prose — *"yes, raise it to 90s, and quarantine the other one while you're at
it"* — and the CLI takes it as one shell-quoted argv string, where a stray apostrophe eats the
line. `armada_helm::ask::editor` is a full inline text area with wrapping, arrow keys and
bracketed paste, it is already what the guild interview uses, and it is already what the Bridge's
`a` key opens. **The Bridge can answer a Job properly and the CLI cannot**, using the same code,
in the same binary.

**Where a selector would be wrong, and this is the constraint that shapes the fix.** `armada
fleet kill` and `armada fleet reap` run in gates and in the Stop hook; `armada fleet answer` is
called by Helm through MCP. A prompt that appears when stdin is not a terminal is a hang, and a
hang inside a hook is the failure mode [`020`](020-the-tui-decided.md) §1 was written about. So
the rule is the one `config scan` already follows: **the offer is what a terminal gets, the
refusal is what a pipe gets, and `--json` never prompts at all.** That is a decided pattern in
this codebase, not a new one.

**What it should do.** With no argument and a terminal: `answer` lists the open entries, takes
the pick, opens the text area, and sends what is in it. `board`, `show`, `pause`, `resume` and
`kill` list the Jobs the verb can act on — `pause` offering only running ones, `resume` only
paused. **Cost: medium**, and most of it is once: one helper that takes a set, a prompt and a
non-TTY refusal, then six call sites of two lines each.

### 4 · Armada still prints commands for a person to carry somewhere else

The archetype, and it has two live instances. `armada helm`, on a machine where `--exec` has not
been enabled:

```
  STATUS     WIRED         DETAIL
  WRITTEN    toolbelt      ~/.armada/helm/mcp.json
  WRITTEN    monitor       ~/.armada/helm/plugin
  WRITTEN    backstop      ~/.armada/helm/stop-inbox.sh
  UNCHANGED  voice         ~/.armada/guild
  WRITTEN    conversation  ~/.armada/helm/session.json

  enter with claude --agent helm --mcp-config /Users/…/.armada/helm/mcp.json --plugin-dir /Users/…/.armada/helm/plugin --settings /Users/…/.armada/helm/settings.json --session-id 6ef5ceb5-…

OK  helm, conversation new, nothing started; --exec is off on this machine
```

**The summary names the switch's state and not the switch.** A reader who has just been told
`--exec is off on this machine` has been told the one fact that is no use without the next one:
`armada helm enable` exists, is four words, and appears nowhere on this screen. The `next:` field
is what the envelope has for exactly this and it is empty, because this is a success rather than
an error — which is the gap: **a verb can succeed and still leave somebody stuck.**

**And the same paths are written two ways, four lines apart.** The table tildes them; the `enter
with` line does not, so the reader gets `~/.armada/helm/mcp.json` and
`/Users/…/target/scratch/home/.armada/helm/mcp.json` for the same file in the same frame. The
convention is stated in [`commands/reference.md`](../commands/reference.md) — *"Paths: written
relative to the repo or as `~/`. Never absolute."*

**`armada fleet board` is the same shape and it is worse, because printing is the documented
default.** `--print` prints a worktree path and `claude --resume <36-char uuid>`; `--exec` does
the `cd` and the `exec` and is one flag away. So the ordinary case — *take this Job over* — costs
a copy, a `cd`, a second paste, and a tilde the shell has to expand, and
[`commands/fleet/board.md`](../commands/fleet/board.md) already records that the tilde was the
bug that broke `--exec` on every Job on the machine. The reasoning for `--print` is sound in its
own terms — *Armada does not own a terminal* — but it is an argument for not building a
multiplexer, not an argument for making `exec` opt-in.

**What it should do.** `armada fleet board <job>` execs when stdout is a terminal and prints when
it is not, with `--print` kept for the person who wants the strings; that inverts a default and
changes nothing else. `armada helm` without `--exec` ends on `next: armada helm enable` — and the
gate itself is worth a second look, since a switch whose only effect is to make the tool do the
thing its name says is a switch most people will turn on and never think about again. Tilde every
path through one function. **Cost: small** for the paths and the `next:`; **small** for board's
default, plus the release note that says a default changed.

### 5 · `--budget` is validated after the call that costs money, and renamed on the way out

Three faults compound into one bad minute. First, the flag's own `--help` never names a legal
key:

```
FLAGS
  --budget <k>=<v>    override one ceiling, repeatable
```

Second, `armada_core::fleet::workflow::override_budget` runs at
`crates/helm/src/verbs/fleet.rs:346` — **after** `classify` at `:311`, which is a real Claude
Code call, measured in that file's own comment at *"7.5s in the run that reported this, a measured
20.6s for a one-line task"*. So this:

```
$ armada fleet spawn "…" --budget iterations=40
error: `iterations` is not a ceiling
  where: iterations=40
  class: bad_invocation
  next:  --budget max_iterations=12, max_tokens=400000 or max_wall_clock=45m
```

costs a classification call before it refuses. **The refusal is excellent** — it names all three
keys, which is exactly what the reported defect said was the good half — and it is arriving after
the reader has paid for it. `bad_invocation` is by definition decidable from argv alone, and
argv is available before anything runs.

Third, the key the reader typed is not the key the reader is shown. `--budget max_iterations=40`
produces:

```json
{ "budget": { "iterations": 40, "tokens": 600000, "wall_clock_ms": 5400000 } }
```

**One ceiling, two spellings**, which is precisely what [`glossary.md`](../glossary.md) calls a
defect rather than a style preference — *"If a word here starts meaning two things, that is a
defect"*. The `max_` prefix earns its place on the way in, where `--budget tokens=…` would read
like a quantity to spend rather than a ceiling not to cross; it earns nothing on the way out,
where the field sits beside `spend`. Whichever survives, the reader should be able to type back
what they were shown.

**And `--dry-run` does not report the thing `--dry-run` says it reports.** `--help` promises
*"report the plan: workflow, worktree, ports, budget"*; the table has four rows and none of them
is the budget, though `--json` carries it:

```
  STATUS      STEP      DETAIL
  CLASSIFIED  workflow  bug, you named it
  WOULD       worktree  ~/.armada/workspaces/repo2/rate-limiting
  WOULD       ports     -
  WOULD       drone     reproduce step
```

The one flag whose effect you would want to confirm before spending anything is the one the
preview drops.

**What it should do.** Parse and validate `--budget` in the argument layer, beside every other
`bad_invocation`, so a typo costs nothing. Name the three keys in `--help`. Add the budget row to
the dry-run table. Settle on one spelling and migrate the other. **Cost: small** for the first
three; **small–medium** for the rename, which touches the envelope and therefore `schema_version`.

### 6 · The keyboard means different things on surfaces one keypress apart

Eleven interactive surfaces, and no shared contract. The three that will actually catch somebody:

| Key | On the Bridge | On the selector and tick list | In the text area |
|---|---|---|---|
| `esc` / `ctrl-c` | quits — or clears the filter first, if one is set | **takes the documented default**, silently | keeps the file as it was |
| `enter` | boards a Job; in the reap preview, **reaps** | chooses the row | inserts a newline |
| `ctrl-d` | submits the compose box; **inert** in the filter box | not bound | submits |

**`esc` is the one that will bite.** Everywhere in a terminal, `esc` means *get me out without
doing anything*. In `armada_helm::ask::select` it means *answer with the default*, which on
`armada fleet reap` is safe (`keep them`) and on `armada fleet spawn`'s workflow question commits
a workflow. A reader who learns `esc` on the Bridge and presses it on the selector has answered a
question they meant to abandon. **The Bridge's own `esc` is overloaded a second time** — it
clears the filter when one is set and quits when none is — and nothing on screen says which of
the two it is about to be.

**Confirmation has two idioms for one verb.** Aborting a Job on the Bridge is `y` and nothing
else; reaping from the CLI is `enter` on a selector whose default is *keep them*. Same
destruction, same session, two muscle memories.

**`?` exists on exactly one surface.** The Bridge's keys page is the best-built thing in the
review — `bridge_keys_hidden` asks the same function that trims the key line which keys it cut,
so the overlay and the line cannot disagree. Nothing else has one. The selector, the tick list,
the text area, the filter box and the confirm prompt each carry one hint line and no way to ask
for more, and on the filter box even that is missing: it draws `filter <typed>▏` and never says
that `enter` applies, `esc` cancels or `ctrl-d` is dead.

**And one legend instructs the reader to press a key that does nothing.** `armada guild ls` at a
terminal, against the guild Armada itself ships:

```
  10  schema       workflow.schema.json  what every workflow is checked against
  11  permissions  permissions.yml       dontAsk, 8 alloweds, 16 denieds
  12  done                               stop looking
  a number, or enter for 12
```

`apply` matches a **single** ASCII digit, so `10`, `11` and `12` are unreachable — typing `1`
then `2` moves to row one and then to row two. Twelve rows is not an edge case; it is the shipped
guild's own inventory on the day it is created. The same selector draws its `STATUS` column in
lower case (`schema`, `permissions`) while `armada guild ls`'s own table prints the identical
column SCREAMING, which [`commands/render.md`](../commands/render.md) settles in as many words
and records having already decided once.

**Neither tick list can be cleared or inverted.** `config scan` opens with everything ticked;
declining nineteen of twenty proposals is nineteen keypresses, and there is no `a`, `A`, `*` or
`n`.

**What it should do.** One key table, stated once and shared: `esc` cancels without answering on
every surface, and a widget that needs a default takes it on `enter` rather than on the key that
means *stop*; `ctrl-d` submits every multi-line surface, `enter` submits every single-line one;
`?` opens a keys page everywhere, generated the way the Bridge's already is; `space` toggles and
`a` inverts on every tick list; digits stop being advertised past nine, or start accepting a
second one. **Cost: medium**, and it is mostly deletion — the bindings live in two functions
(`ask::select::apply` and `core::fleet::bridge`) and the cost is in the tests that pin the
current meanings and in one release note about `esc`.

---

## The rest, ranked

Each of these is real and none of them stops anybody. Owner is whoever picks the item up; nothing
here needs the reader of this document to act today.

| # | What happens | Should | Cost |
|---|---|---|---|
| 7 | Nothing lists run ids. `check --detach` prints `run 01M05A492SCZTNGM`, `--status` demands exactly 16 Crockford characters with no prefix, and the id appears nowhere else. `RunId::parse`'s `next_action` says *"`armada manifest status` lists the runs this workspace has kept"* — it lists none. | `manifest status` grows a runs section, or `--status` accepts a prefix. Fix the `next_action` either way; it names a listing that does not exist. | small |
| 8 | Nothing waits for a detached run. `--wait` is about the run lease, so a gate must poll `--status` in a loop. `AGENTS.md`'s *"a gate uses `--wait`, never `--status`"* reads as though it does. | `--status --follow`, blocking until the run decides. Fix the derived line in `AGENTS.md`. | small |
| 9 | `manifest config verify` reports `FAILED  scratch-api:lint  exited 127` and no log path; `check --status`, running the same check, prints `.armada/run/<id>/logs/scratch-api.lint.log` under each failure. | `verify` keeps a run directory and points at it, like `check`. | small |
| 10 | No verb suggests a near miss. `fleet lst`, `manifest chek`, `--componant`, `guild show voice` (where `voice.md` exists) are each one edit from a real name, and the roster is already in memory — `armada untried` walks `every_verb()`. | *"did you mean `ls`?"* on verbs, flags and guild items. | small |
| 11 | `--componant`'s `next_action` is *"`armada --help` lists what each verb takes"*. `armada --help` lists modules; the flag belongs to `manifest check`. | Point at the verb's own `--help`. | trivial |
| 12 | `armada settings guild.permissions.yml` silently ignores the argument and prints all ten rows. | Refuse it, or filter by it. Silently discarding an argument is the failure `override_budget`'s own comment argues against. | trivial |
| 13 | `armada guild init --no-import --defaults` reports `IMPORTED  ~/.claude/  —` and closes on *"7 kept as imported"*. The `USAGE` block does not offer `--defaults` on the `--no-import` line, and takes it anyway. | Say `SKIPPED` when import was refused, and refuse the combination the usage does not offer — or offer it. | small |
| 14 | `config scan` in an empty repository draws a `DETAIL` column in which every row is the placeholder. [`render.md`](../commands/render.md) says a column no row filled is dropped, header and all — and separately that `DETAIL` keeps its placeholder. The two rules meet here and the table is the loser. | Decide which rule wins when *every* row is empty, and say so in `render.md`. | trivial |
| 15 | The `proposals` table is `WHAT · WRITES · BECAUSE` — no `STATUS` column at all, in a house style whose first rule is status first. It reads well; it is a departure, and an undocumented one. | Either record the exception in `render.md` or give it a status. Deciding it is the work. | trivial |
| 16 | `armada doctor` is not a table at all — grouped sections, an indented `-> remedy` under each finding. It is the most readable screen in the CLI and it obeys none of `render.md`. | Write the exception down. A rule with a popular violation is a rule about to be violated again. | trivial |
| 17 | `doctor`'s remedy for a fresh, never-`init`ed machine is `armada init --force`. `--force` on a machine with nothing to overwrite reads like a threat. | `armada init` should complete a partial `~/.armada` without `--force`; keep `--force` for the destructive case. | small |
| 18 | `manifest clean` reports `CLEAN  0f45b189  ports released` for a workspace whose only owned resource was a pgid. | Say what was actually released. | trivial |
| 19 | `armada untried` and `armada failures` report `0` and `nothing recorded` inside `~/.claude/worktrees/` and `~/.armada/workspaces/`, where `armada_core::failure::scratch` correctly suppresses recording. The reader is shown an empty count rather than *not recorded here*. This is `render.md`'s own `DETAIL`-placeholder argument — *nobody looked* is not *nothing found* — applied to a whole verb. **Inferred from the code, not driven: this review ran entirely inside a worktree and so could not observe the other branch.** | Say *not recorded in a worktree* instead of `0`. | small |
| 20 | The same id space is printed at four widths: `4` in `fleet spawn`'s drone row, `8` in `fleet ls` / `inbox` / `reap`, `36` in `fleet show`'s `ASKED` column, `36` again inside `board`'s resume command. | One width, or a stated reason per exception. | small |
| 21 | `fleet inbox`'s footer offers `armada fleet answer <id> "…"`; `render.rs:241`, `core::fleet::bridge` and `args.rs` still offer `<job>` for the same argument. | One placeholder, from one constant. | trivial |
| 22 | [`glossary.md`](../glossary.md) says Helm is *"Reached by `armada`, or `armada helm`"* and [`render.md`](../commands/render.md) puts the banner on *"`armada` with no arguments — entering Helm"*. Bare `armada` prints the banner and then the help page. | [`020`](020-the-tui-decided.md)'s eighth decision has already settled what bare `armada` becomes; until it is built, the two documents describe something that does not ship. Mark them, or build it. | trivial to mark |
| 23 | `armada fleet spawn --dry-run` answers with `"workspace": null` while standing in a claimed workspace. | Fill it, or say why a spawn is not workspace-scoped. | trivial |
| 24 | `fleet answer`'s `next_action` demonstrates on `nightly-flake`, a Job that does not exist, on a machine with no Jobs at all. | Name a real Job when there is one. | trivial |

---

## What this review could not check, and one thing it found missing

**No Job was spawned.** `armada fleet spawn` costs a classification call and a Drone, so
everything about a live Job — `tick`, `board`, `reap` against real rows, the Bridge with anything
in it — was read rather than driven. §2's reproduction was built by seeding
`~/.armada/inbox.jsonl` by hand, which exercises the resolution path and not the raising one.

**The Bridge is built.** [`020`](020-the-tui-decided.md) opens *"Nothing here is built"*, which
is true of its nine decisions and not of `armada bridge`, which ships with a modal state machine,
a keys page, a filter, a compose box, a detail pane and a reap preview. Of 020's own list the
detail pane exists without its `SAID` row, ids are still not shown in the table, and the tagline
is unchanged. Recorded here so the next reader of 020 does not conclude the screen is absent.

**`cargo xtask reserved` does not exist.** `cargo xtask doclint` runs `xref`, `blocks`, `keys`
and `privacy`, and nothing checks that a reserved design's filename, frontmatter `id:`, H1 number
and [`README.md`](README.md) row agree. This document's own consistency was therefore checked by
hand. The check is worth having for the same reason every other one here is: four places holding
one number is three chances to disagree.

## One line on a decision already taken

[`020`](020-the-tui-decided.md)'s ninth decision puts Helm beside the Bridge rather than inside it, and that is
right; but §4 of this document is the argument that **the Bridge is currently the only surface
that can answer a Job properly**, and that gap belongs to the CLI rather than to the TUI.

## See also

[`001`](001-raised-items-need-identity.md) · [`005`](005-inbox-label-not-identity.md) ·
[`007`](007-scanner-should-propose.md) · [`009`](009-smaller-things-raised-in-use.md) ·
[`020`](020-the-tui-decided.md) · [`../commands/render.md`](../commands/render.md) ·
[`../glossary.md`](../glossary.md)
