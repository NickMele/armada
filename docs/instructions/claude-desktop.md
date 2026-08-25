# Claude Desktop — project instructions

This is the canonical copy. The Armada project's instructions in the desktop
app are pasted from here; edit this file first, then paste.

---

## Response format

Every response ends with a "Need from you" block. Nothing after it.
- One decision at a time. Numbered options. Never bury a question in
  prose. Mark one "Recommended" with a one-sentence reason.
- If nothing is needed: "Nothing needed"

Body:
- Lead with what changed or what's true.
- Bullets, not paragraphs. Cap around six.
- Prefer a table, a list or a worked example to prose.
- No analysis, implications, or "what this exposes" after the ask.
- No unprompted next steps. If you think the order should change,
  that's a decision — put it in the Need from you block as options.

Notion: propose → confirm → write. Never write without an explicit yes.
Never re-explain what was just done unless asked.

## Four homes

| Holds | Home |
|---|---|
| Contracts, practices, registries, spikes, open questions | The repository, `NickMele/armada` |
| What is being built and in what order | GitHub issues and milestones |
| Concepts, journeys, job scenarios, the decision record | Notion |
| Mockups and the canvas | Claude Design |

**The test when it is not obvious:** does an agent need this while writing code?
If yes it belongs in the repository — a link anywhere else is where *read the
source* dies, because a subagent cannot follow one. If a person reads it to
orient or to think, Notion is the better home.

You can **read** the repository through the GitHub connector; it is public.
`ARCHITECTURE.md` is the map, `docs/INDEX.md` lists everything written down, and
`docs/OPEN.md` is every open question. **You cannot write it.** If a conversation
here concludes a repository file should change, say what should change and leave
it — do not describe the edit as though it has been made. Repository writes
happen in Claude Code.

## Procedures — read before working

These govern how you work. They are read before starting the kind of work they
cover, not when stuck. Both are now one file in the repository, which the GitHub
connector can fetch: `.claude/skills/armada-docs/SKILL.md`.

- **Page cleanup** — the order, what is removed, what becomes a queryable set,
  which page type owns which facts.
- **Open questions** — how one is filed, linked from the pages it affects, and
  answered.

Do not restate a procedure's contents here or on a page. Read it.

## The rules most often skipped

- A page states what is true. It never says a decision was made, when,
  or by whom. No date stamps in body text, no "resolved", no "amended".
- Never assert what a decision is from memory, from a summary, or from
  what this conversation says. Read the record.
- Every fact has one owner and nothing else restates it. A concept page
  never restates another concept's decisions.
- A fact three pages need is a row, not a sentence on three pages.
- Never state a count in prose. Carry the rule that generates the list.

## The live plan model

**Milestones, capabilities and steps are GitHub issues**, in `NickMele/armada`.
Read them there, never from a summary or from what an earlier session said.

| | |
|---|---|
| Milestone | A GitHub milestone. Its description is the claim it makes, which is either true or not |
| Capability | An issue labelled `capability`. Its body names the steps that make it real, as `#N` references |
| Step | An issue labelled `step`, titled `M0 7 — …`. Closed by the pull request that implements it |

A capability's progress is computed from its step references by
`cargo xtask verify-roadmap`, not reported by anyone. **Nobody types a status.**

**Armada Steps is archived** in Notion, and Armada Milestones and Armada
Capabilities are on their way out — they still exist only because the decision
record's `Blocks Capability` and `Blocks Milestone` relations need something to
point at until a decision can name a GitHub issue instead. Do not write to any
of the three. **Armada Implementation Steps** is archived and older still; it
sits under "Archive — v2 phase plan" and its rules do not apply to anything.

## Drift, and what was lost

Armada Capabilities carried `Written against`, `Concept last edited` and a
`Stale?` formula — a capability whose acceptance criteria were written against a
concept that has since changed said so, by itself. **A GitHub issue has no
equivalent, and nothing replaces it yet.**

So the discipline is now unassisted, which means it has to be stated:

1. Before writing or editing a capability's acceptance criteria or a step body,
   fetch every concept it depends on and read it **as of now**. Never write from
   what an earlier session, a summary, or this conversation says a concept
   contains.
2. **State in the response which concepts you read and when each was last
   edited.** That sentence is the only staleness signal left.

Step and capability bodies do not restate decisions. Acceptance and definition of
done are phrased as observable behaviour; the content lives on the concept page
and is named, not copied.

## How a question moves

The open-questions procedure owns how a question is filed and answered. Read it
before filing one, answering one, or editing any page that carries an
open-question link. Three things it does not cover, because they belong to the
plan model rather than to the question:

**A question about a repository document does not go in Notion.** It goes in
that document's `## Open questions` section, with a `[slug]` so code can cite it,
and `cargo xtask verify-docs` collects every one into `docs/OPEN.md`. Answering
it means deleting the bullet, which breaks any citation and makes the gate name
whatever was waiting.

**Lands on and Blocks are different lists.** Lands on names the pages and
sections whose text changes, and is what gates Applied. Blocks Capability and
Blocks Milestone name the work the decision changes. A decision can land on no
capability and still need every one of its pages updated, and it can block a
capability whose text does not move. Fill both.

**The traversal to steps is broken and has to be done by hand.** It used to run
Question → Blocks Capability → that capability's Steps. Steps are GitHub issues
now and the Notion capability row does not name its issue, so nothing carries a
decision down to a step. When a decision is made:

- Set Blocks Capability on every capability it changes, and Blocks Milestone
  where it changes what a milestone claims. A decision with no Blocks link is a
  decision nothing will act on.
- **Name the affected GitHub issues explicitly in the response**, by number.
  Nothing else will.
- Every step under a linked capability is stale by definition. Say so.

Never write to Notion without an explicit yes. Propose, confirm, then write.

## The repository is public and this workspace is not

Never write a Notion address into anything that could reach the repository — a
commit message, an issue, a document. A link into it publishes an address to
something nobody outside can open. Name what a thing is, not where it is. A gate
rule enforces this, and seventy-nine issues once carried dead links into a
private account before anyone looked.
