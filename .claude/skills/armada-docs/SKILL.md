---
name: armada-docs
description: How to clean an Armada page and how an open question is filed, referenced and answered. Load before editing documentation, before filing a question, and before answering one.
---

# Working on Armada's documentation

Armada's written record lives in two places, and which one you are in decides
almost everything about how you edit it.

| | Holds | Edited |
|---|---|---|
| **The repo** | Contracts, practices, registries, spikes, and every open question | Directly, in a commit, checked by the gate |
| **The design workspace** | Concepts, journeys, job scenarios, and the decision record | Through the procedure below, one page at a time |

The two procedures here were written for the second and still govern it. Most
of what they say is true of a file as well, and where it is not, this document
says so.

**The rule that outranks everything else in both:** never assert what a decision
is from memory, from a summary, or from what the conversation says. Read the
record. A conversation summary collapses a suggestion and a reaction into one
phrase, and a confidently wrong claim about a settled decision costs more than
the whole check saves. Say "I need to look" instead.

---

# Part one — cleaning a page

It exists because the same five failures recur: prose restating a decision that
has since moved, changelog entries read as current state, counts that go stale
and then get copied, tables that should be a queryable set, and the same fact
stated on three pages that will drift apart.

## Order, and why it is this order

### 1. Stale sweep, first

A stale reference may be attached to a section a later step would otherwise
preserve, so find them before deciding what to keep. Sweep for:

- **Archived structure.** The nine-phase plan was replaced by milestones.
  Anything naming Ground Zero, Phase 0 through 6, or numbered implementation
  steps is reference only. **M0 — Foundations replaced Ground Zero.**
- **Retired terminology.** The lexicon is in `docs/contracts/design-system.md`.
  Guild split into Kit and Machine — a split, not a rename, so each site needs
  judgment about which one it became.
- **Counts stated in prose.** "Eight modules", "twelve crates". Every one goes
  stale and is then copied onto other pages.
- **Self-links and broken links.**
- **Internal contradictions**, where a page disagrees with itself two sections
  apart.

### 2. Verify before fixing

Split findings into two piles.

**Settled elsewhere.** Applying these is enforcement, not a new decision. Fix
them.

**Genuinely open.** Never silently reconcile one. File it, per part two, and
link the inline site to it — so the page says "this is open" rather than
stating something false.

### 3. Reconcile changelog into decisions

Sections reading "Resolved", "Amended", "Corrected" should read as the current
decision, stated once.

- Keep reasoning a reader would otherwise re-litigate.
- Drop the narrative of how it was arrived at, and which alternative was
  considered, unless the alternative is one someone will propose again.
- Delete completed "pages this contradicts" tables outright. A record of work
  already done is what makes a page long without making it true.
- If a date-stamped note is the only thing marking a real distinction, keep the
  distinction and drop the date.

### 4. Cut fluff, keep information

Remove restatement, throat-clearing, and justification of decisions nobody is
disputing. **Never lose a measurement, a number, or reasoning that would have
to be re-derived.** The test is not length — it is whether removing the
sentence loses a fact.

### 5. Extract

**In the workspace, prefer a database to a static table.** A table is dead
text: every cell is the same size, nothing outside can reach a row, and the
moment one cell needs a paragraph the whole table becomes a wall. A row is a
page — it opens, other pages link to it by address rather than by reference,
its properties filter and relate, and its views travel instead of being copied.

**The test:** does something outside this page want to point at an individual
row? If four other pages name items from a table, it is a database.

**Leave it as prose when nothing links to it and it has a code counterpart** —
schema, migrations, config. A database mirroring code becomes a second source
that drifts the moment a migration lands, and nothing reconciles them.

**A static table is still right** for a small closed set read as a unit and
never cited a row at a time — an enum's legal values, a two-by-two of options.

**In the repo the same question has a different answer.** A set that code reads
is a data file with a check over it — `packages/tokens`, `packages/icons`,
`crates/config/settings.toml` are all this shape. A set nobody's code reads
stays a table in the document.

### 6. Deduplicate across pages

Fetch every page this one links to and read it. **This is the step most likely
to be skipped and the one that pays most.**

- Where this page restates something the linked page owns, delete the copy and
  leave a pointer. The linked page almost always holds strictly more, so the
  copy can only drift downward.
- Check both directions — the linked page may be the one duplicating.
- Where a fact belongs to neither, the page owning the *rule* keeps it.

### 7. Contents, last

Written before the sections settle, it has to be rewritten.

## Every fact has one owner

| Page type | Owns | Never restates |
|---|---|---|
| Concept | What the thing is, and the decisions governing it | Another concept's decisions |
| Capability | Whether it is built, and the acceptance criteria | What it means |
| Step | What to do inside this milestone | Any decision. Steps are disposable |
| Journey, contract, surface | How the thing is used or shown | The rule it is applying |

**Restatement between concept pages is where drift starts.** The rule that
steps do not restate concepts has no equivalent between concepts themselves, so
they copy each other freely — and a copy can only go stale, with nothing
checking it.

---

# Part two — open questions

## The rule that matters most

**The question link lives at the exact place the answer will go.**

A relation names a page. It does not name the sentence. That gap is where drift
lives — a decision lands somewhere on the right page while the stale sentence
three sections down survives.

An inline link is a placeholder, not a citation. The text around it states that
the matter is open, and when the question is answered that text is replaced by
the answer. Whoever edits the page cannot fail to see it, because it is in the
paragraph they are already editing.

**In the repo this is the `## Open questions` section** at the foot of the
document that blocks on it, with a `[slug]` so the question can be cited from
code. `cargo xtask verify-docs` collects them into `docs/OPEN.md`, and a
citation of a slug nothing asks fails the gate — so answering a question means
deleting its bullet, which names whatever was waiting on it.

## The question body

Five parts, in this order.

| Part | Holds |
|---|---|
| **Raised from** | The page and section that produced it |
| **Lands on** | Every page and section that changes when this is answered, what changes there, and whether it has landed |
| **The question** | One sentence. What must be decided |
| **What decides it** | Facts, constraints, measurements, positions already taken — enough to decide without re-deriving |
| **On answer** | The closing instruction |

**Lands on is a checklist, not a description.** Its last column is ticked per
site with `✅` — one character, so a half-written table cannot be mistaken for a
styled one. That is what makes "applied" checkable rather than a judgment.

**It is written at filing time**, before anyone knows the answer, which is the
point. A site list written at closing time is written by someone with an
interest in the list being short.

## The bar for filing

**A row is filed only when all three hold.**

- It came up in the course of doing something else, and cannot be settled now.
- It will change what gets built or written. Something is waiting on it.
- **A person deferred it.** An agent may propose a question; it may not file one
  on its own judgement.

**Do not file when:**

- It was settled in the session and the page reflects it. Write the page.
- You are unsure but could find out by reading. That is research, not a
  decision.
- It is work rather than a call. That is a step.
- Nobody is blocked and nobody asked.

**Filing is not free.** Every question is something a person must later read,
judge and close. Enough of them stop the queue being readable at all — which is
the failure it exists to prevent. **Two were once filed in one session that
were already answered in documents that had been read, one of them written that
same day.**

**If a question was filed and then answered in conversation, close it.** A
question left open after its answer exists is worse than never filing it: it
advertises an open decision that is not open.

## Answering one

1. The decision is made.
2. **Walk Lands on, one row at a time. Update the page in place, then tick the
   row.** Write the fact, not the announcement — a page states what is true,
   never that a decision was made, when, or by whom.
3. Where a page needs updating that Lands on did not name, **add the row first,
   then write it.** The omission is the bug, not a reason to skip the site.
4. When every row is ticked, the question is applied.

**A partly-walked question stays open with some rows ticked.** That is a
complete and honest record: the unticked rows are the remaining work.

**Do the write-back in the same sitting as the decision.** Deferred, it rots.
That is how pages arrive at contradicting settled decisions.

## Supersession

A later decision sometimes overturns an earlier one, and the earlier record
then asserts something false with confidence.

| | Wholly superseded | Superseded in part |
|---|---|---|
| When | Answered completely, and better, elsewhere | Some of it stands, some was overturned |
| Body | A `## Superseded` section at the top | A `## Superseded in part` section at the top |
| Lands on | Outstanding rows move to the superseding record | Narrows to what still stands |

**The resolution is never rewritten.** It records what was decided at the time,
and rewriting it destroys the only account of why. The correction goes at the
**top**, because it has to be read before the thing it corrects.

**The superseding record names what it overturned.** A decision that silently
invalidates another is how this starts.

**What is not supersession:** a resolution that admits a gap is a gap, and gets
its own question. A decision that builds on another depends on it. A resolution
that was never true is a correction — fix it in place, nothing was overturned.

---

# Standing rules, both parts

- **Propose, wait for an explicit yes, then write.** This applies hardest to
  filing a question, because filing feels like diligence and is often the
  opposite.
- **One decision at a time.** Numbered options, a recommendation, a one-line
  reason.
- **Never state a count in prose.** Carry the rule that generates the list.
- **A page never says a decision was made.** It states what is true. No date
  stamps in body text, no "resolved", no "amended".
- **Prefer a table, a list or a worked example to prose.** Prose is where a fact
  hides.
- **Record disagreements explicitly** rather than reconciling them.
- **Never write an address into this repository.** The workspace is one
  person's and the repository is public; a link into it publishes an address to
  something nobody outside can open. Name what a thing is, not where it is. The
  gate enforces this.

## Workspace mechanics worth knowing

- A bare UUID fetches reliably; copy-link URLs with slugs sometimes need
  stripping.
- Creating a page needs an explicit data-source parent. Omitting it makes an
  orphan.
- Relation properties accept bare page IDs as an array.
- Additive schema changes only: add new options, retag, then drop the old ones.
- **Re-fetch immediately before writing where another session may be active.**
  Identity is not freshness: a page fetched an hour ago passes every identity
  check and still misses an edit landed since. A stale-read overwrite reverts
  decided work silently, and afterwards reads as a disagreement rather than an
  accident — which is what makes it expensive to find.
- **Re-fetch after writing.** A silent no-op on a whitespace mismatch is the
  known failure mode of a content update, which needs exact string matches.

## Queue order

Clean the high-degree nodes first — the pages many others link to, where drift
costs most. A page nothing links to can wait.
