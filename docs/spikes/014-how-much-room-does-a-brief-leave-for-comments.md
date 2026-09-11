# Spike 14 — How much room does an opening brief leave for chosen comments?

**The bound this spike measured was dropped by the owner's decision on
2026-09-11, `#648`: what a person picks off a pull request is what a Drone
gets, at any length.** A chosen set no longer shares a prompt's room at all —
it is written whole into a file in the Drone's worktree, and
`fleet::remarks::pointer` puts a short pointer to that file on the turn in
place of the words themselves. This spike's reading stays below as a record
of the number that used to gate a press and the arithmetic it came from; it no
longer describes anything the code does.

**About the size of the whole brief itself, and no measured brief comes close to spending it.**
A real opening brief runs 9,972 characters, of which 1,238 — the same on every one of 20 sampled
— are fixed regardless of the Job. Nothing chosen off a pull request has ever had to share space
with the rest of a turn before; `fleet::remarks::brief` is the first block built from words this
machine did not write. The bound this spike justifies is **8,000 characters**, applied once to
the whole set of comments a press chooses, not to any one of them.

## Provenance

**Not measured in this session.** The task that opened this worktree needed the briefs under
`.armada/briefs/` and `crates/fleet/src/briefing.rs`'s `assemble`, and the first is in the main
checkout — a worktree cannot read it, and no shell command outside `armada.yml`'s declared set
runs here to go and get it. The numbers below were taken on 2026-09-10 from 20 real opening
briefs by the person who restarted this task, and handed over rather than reproduced. There is no
`014-samples.csv` beside this file for that reason: the raw reading is somebody else's, and citing
it as this session's own would be the wrong kind of provenance for a number that governs a
refusal.

What follows is `assemble` read cold, to say which block each figure names, and the arithmetic the
bound is built from.

## What `assemble` puts in a brief, and which figure is which

`assemble` in `crates/fleet/src/briefing.rs` builds a first turn in a fixed order: `BASELINE`, then
`notekeeping`, `job_brief`, `where_you_are`, then whichever of `cleared` / `redirect` / `dispatched`
/ `overtaken` the boundary carried, then the step block and what it declares. Three of those blocks
line up with the three figures supplied:

| Figure | Block(s) | Varies with |
|---|---|---|
| 1,238 characters, identical on all 20 | `BASELINE` (a compile-time constant, 700-odd characters on its own) plus the fixed sentences in `notekeeping` and `step_block` | Nothing. `notekeeping` interpolates a Job id and `step_block` interpolates a step label, and neither is large enough to move a whole-brief total by more than a few characters. |
| ~45% of the whole, "about the Job" | `job_brief` — the requester's own title, facts, attachments and acceptance criteria | The request. A one-line title and no facts is short; a Job carrying a long brief and ten criteria is long. |
| ~32%, "lists written as sentences" | `where_you_are`'s numbered rail, `step_block`, and whichever of `Declaring` / `Checking` / `Delivering` the step offers — every one of these renders a `Vec` as prose, one sentence or one indented line per item | The workflow: how many steps it declares and how many Checks the current one runs. |

Summed, the three account for 45 + 32 = 77% by percentage, or 89.4% by the concrete count
(1,238 of 9,972 characters is 12.4%, not the "about 20%" the corpus average rounds to — this one
specimen carries more Job content or more steps than the 20-brief average, which is exactly what
pulls its fixed share below the mean). The remainder, 10 to 11 points either way, is the blank
line `Blocks::push` inserts between every block plus whatever the three rounded figures do not
cover between them. Nothing here depends on closing that gap to the character: every one of the
three named blocks is present on **every** brief this measurement or corpus could have drawn from,
and none of the three is where a comment lands.

## Where a comment lands, and why none of the three above is it

`take_up_remarks` in `crates/fleet/src/remarks.rs` builds its note from `brief(&picked)` and
hands it to `Redirection::saying`, which becomes the `crossed.redirect()` block the *next* spawn's
`assemble` call renders — grouped with `cleared`, `dispatched` and `overtaken`, "before the step
block, with the other things the boundary carried." It is additive: nothing above shrinks to make
room for it, so every character `brief(&picked)` produces is a character on top of the 9,972
already measured.

## The bound

**A comment must not be able to make itself most of a turn.** That is the request's own words —
"nothing says so" when one long comment or several medium ones "can become most of what the Drone
reads" — so the bound has one job: keep the comments block a minority of the whole, on a brief of
ordinary size, not only on the one specimen measured.

The measured brief is 9,972 characters with nothing chosen yet. For the comments block to stay
under half of the resulting turn, it must stay under the size of everything else — under 9,972.
**8,000 characters** is chosen rather than that ceiling itself, for two reasons pulling the same
way:

- Only 1,238 of the 9,972 is guaranteed on every brief; the rest is the Job's own content, and a
  terser Job than the one measured leaves less non-comment text for a bound at 9,972 to stay a
  minority against. 8,000 leaves headroom below the one full specimen in hand for a shorter one
  this spike did not see.
- It is a bound on `brief(&picked)`'s own output — the wrapper sentences plus every `quoted`
  comment, "> " marker and all — which is what actually reaches the next turn, not the raw text a
  person picked. Counting the wrapper is what makes the number comparable to the 9,972 it is being
  weighed against, both being turn text rather than review text.

**The bound is on the set, never on one comment.** Five comments of 2,000 characters each sum to
the same 10,000 that one comment of that size would, and the request's own wording rules the first
case in beside the second: "Five medium comments are the same problem as one long one." A check
that only measured the largest chosen comment would pass a press the request explicitly names as
the failure.

**Which comments are named back is a question for the check, not for this spike.** Bounding the
set says how big is too big; it does not by itself say which of several comments a person should
drop to get under it. That choice belongs to whichever refusal reads this bound.

## What this does not cover

- **A single comment near or over 8,000 characters on its own.** The bound is on the sum, so one
  comment can still fail it alone — the ten-thousand-word example in the request is exactly this
  case, and correctly refused by a set-level bound with one member in the set.
- **The 20 briefs' own spread.** Only one specimen's absolute size (9,972) and one absolute
  constant (1,238) were handed over; the corpus's shortest and longest briefs, and how far
  "about 45%" and "about 32%" swing around their means, were not. The 8,000-character margin below
  9,972 is what stands in for that unknown spread, not a computed confidence bound.
- **Tokens.** Every figure here is characters, because that is what was measured and what
  `String::len` on `brief`'s output would compare against. A Drone's own budget is tokens, and the
  ratio between the two is not established here.
