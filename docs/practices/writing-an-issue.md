# Writing an issue

**Kind:** practice. **Governs:** the body of every GitHub issue in this
repository. Read before filing one, and before reformatting one.

An issue has two readers and they need opposite things. The owner reads it to
decide what to pick up, and stops after two sentences. An agent reads it to
build the thing, and needs every path, every precedent and every wrong turn
somebody already found.

**This page is not about writing less.** The median issue here is 382 words,
which is ninety seconds, and the complaint was never that they are long. It is
that the answer is in third position. Detail is the asset. **Position is the
defect.**

The military's writing standard has the rule and the word for it: main point
first, and everything else **demoted to an enclosure** rather than cut
([AR 25-50](https://armypubs.army.mil/epubs/DR_pubs/DR_a/ARN42124-AR_25-50-007-WEB-13.pdf),
1-38b and 1-39b). That is the whole of this page.

---

## The rule

**The first paragraph has no heading and names what a person cannot do.**

Two or three sentences, before the first `##`. If a real person hit it, say when
and what it cost them. Then the headings, and the detail under them, at whatever
length the work needs.

Everything else on this page follows from that one rule.

## What goes first, and what does not

| First | Not first |
|---|---|
| What a person cannot do | Where the issue came from |
| What it costs them | Which file the mechanism is in |
| When somebody hit it, if they did | Which frame of which drawing it was drawn in |

**Provenance is not the lede.** "Found by the #286 agent, which was told to
spell a responsive variant", "Drawn in `Journey 6`, frame `6a`", "Filing this
small because" — 31 of the last 60 issues open on a sentence like one of these.
Every one of them is worth keeping and none of them is the answer. They go last,
under `## How this was found`.

**A mechanism is not a consequence.** `JobDetail::of` taking twelve positional
arguments is a fact about a signature. The consequence is that two agents adding
one argument each produce a build that fails only once both land, which happened
twice in one night. The second sentence is the one to open with.

Simon Tatham's rule on bug reports is the same rule and it is worth having in
these words: *"always state the symptoms. The diagnosis is an optional extra, and
not an alternative"*
([How to report bugs effectively](https://www.chiark.greenend.org.uk/~sgtatham/bugs.html)).
An issue that opens on its own diagnosis has chosen the fix before anyone read
the problem.

## The headings

Four, and one optional footer. They are the same for a bug and for unbuilt work
apart from the first, because what an implementer needs does not change with why
the work was scheduled.

| Heading | Answers | For |
|---|---|---|
| *(no heading)* | What can a person not do | The owner |
| `## What happened` / `## What is missing` | What does the code do instead | Both |
| `## In` | What changes, and what it builds on | The agent |
| `## Watch for` | Which fix is wrong, and why | The agent |
| `## Definition of done` | How do we know it is closed | Both |
| `## How this was found` | Where did this come from | The agent |

**`## Why it matters` is gone as a heading.** It was the best section in the
corpus and it was in third position, where it got scrolled past. It is now the
opening paragraph, and it lost its heading because a heading is what let it be
skipped.

**One document, not two.** The dual-audience split is genuinely unwritten, and
the nearest measurement is about code rather than issues: optimal representations
*"share high semantic density for both humans and machines, with divergence only
in structural organization"* ([arXiv 2604.07502](https://arxiv.org/abs/2604.07502)).
That argues for one page with a hard break, not a summary page and a spec page.

**The headings stay because the summary alone is a trap.** Poynter's own critique
of the inverted pyramid is that it gives the reader *"a built-in excuse to
stop"* ([The nut graf](https://www.poynter.org/archive/2003/the-nut-graf-part-i/)).
That is exactly what the owner wants and exactly what would break an agent. The
`##` break is what lets one reader stop there and the other carry on.

## The title

**A title names a consequence, not a mechanism.** These are already right:

> Three kinds of file are named on a surface and none of them opens
> A finished Job's worktree cannot be given back without stopping Fleet
> Armada's own words render as the Drone's prose

These name the machine and make the reader open the issue to find out why they
should care:

> Carry the loop: structure, verdict_routing and iteration_cap
> HealthReport carries launchd intent for Fleet
> Doctor: the condition strip

A colon-prefixed grouping (`Pilot:`, `Doctor:`) is a milestone's job, and the
milestone field already does it.

## What not to write

- **The title again, as the first line.** It is the first line either way.
- **`**Area:** Bridge seam`.** That is a label.
- **`Updated 2026-08-31`, `Rewritten from the drawing`.** The edit history holds
  this and nobody reads it in the body.
- **Commentary on the filing.** "Filing this small because", "Recorded here
  rather than filed separately."
- **A `## Related` list of issue numbers already cross-linked.** GitHub renders
  the backlink. A related issue earns a line only when the sentence says what the
  relationship costs.

## Definition of done is a sentence a person can check

Present tense, as a person experiences it. Not "add a `typecheck` script and a
gate line" — that is `## In` restated. **"A story that references an undefined
symbol fails the gate"** is the same work, stated as the thing that becomes true.

**Name the check where one exists.** Anthropic's guidance for agents is that
without a command an agent can run, *"'looks done' is the only signal"*
([Claude Code best practices](https://code.claude.com/docs/en/best-practices)).
#293 does this correctly: *"a test reads the emitted CSS to assert no `@media`
contains `var(`"* is both the sentence and the check.

## `## Watch for` is the section with the least prior art, and keep it

Of everything on this page, the wrong-fix section is the one no bug-report
convention has. The nearest published vocabulary is Shape Up's **rabbit holes**
(*"too unknown, complex, or open-ended to bet on"*) and **no-gos**
(*"functionality or use cases we intentionally aren't covering"*)
([Shape Up, ch. 6](https://basecamp.com/shapeup/1.5-chapter-06)). This
repository's `## Watch for` already does both — #285 warns against widening into
`JobSummary::of` (a no-go) and against adding a default (a rabbit hole) — so it
needs no new headings, only the awareness that it is carrying two jobs.

## What a lint could hold, and what it cannot

Mechanically checkable: the first non-empty line is not a heading; the body
carries `## Definition of done`; the title is not prefixed with a word and a
colon. **Not checkable:** whether the opening paragraph names a person. That is
the whole rule, and it is a reader's judgement, which is why this page exists
rather than a rule in `verify-foundations`.

## What was considered and not adopted

| Rejected | Why |
|---|---|
| **`As a user I want X so that Y`** | The role slot is constant in a one-owner repository, so it occupies line one and carries nothing. Worse, *"I want"* states a wanted solution — *"as a user I want the cache invalidated on write"* has already picked the fix. Ron Jeffries, who co-invented it, says the card was never the artifact and that a standard card format was tried in 1998 and *"realized it was a bad idea then"* ([Three C's revisited](https://ronjeffries.com/articles/019-01ff/3cs-revisited/)). The load-bearing part is the conversation, and an issue an agent implements has no conversation phase. **The `so that` survives, as the opening paragraph** |
| **YAML issue forms** | Forms enforce required fields on the web form only. `gh issue create --body` takes a body directly and `--template` is opt-in starting text, so a form gates nothing on the path an agent files by. Markdown templates are also readable by an agent as a file it can follow; a form's YAML is not |
| **A `type:` field in the template** | GitHub issue types are an organisation-level feature. This is a personal-account repository, so `type:` is inert. Labels do the work |
| **Observed-vs-expected as two required fields** | Six templates across five surveyed projects enforce it as separate required fields — kubernetes, go, node, vue, Homebrew (twice). It suits a stranger reporting against software they did not write. Here the filer has read the code, and `## What happened` already carries both halves in one paragraph |
| **The job story's `When [situation]` clause** | The best-surviving fragment of the story formats — it is a reproduction step wearing a template — but `## What happened` already opens on the trigger in every issue that has one. Adding the clause would buy a keyword, not a fact |
| **Shape Up's *appetite*** | It presumes a betting table and a fixed cycle. Milestones already do that job |
| **A separate press release, PR/FAQ style** | The genre is fiction about a shipped future, which is the wrong shape for what is broken. The external-FAQ / internal-FAQ split is the useful half, and it is what the opening paragraph and the headings already are |

**One thing here is close to unprecedented.** A survey of twenty-two templates
across fifteen well-run projects found version and environment on all fifteen,
observed and expected on twelve, and **a field naming who is hurt on none of
them.** Leading with impact is an invention, not a convention. The reason to do
it anyway is that those templates are written for a stranger filing against a
maintainer, and this one is written for the person who decides what gets built.

**The one project that gets close is worth naming.** Homebrew requires *"What
were you trying to do (and why)?"* and places it **before** "what happened" —
the only intent-before-mechanism field in the survey
([bug.yml](https://github.com/Homebrew/brew/blob/master/.github/ISSUE_TEMPLATE/bug.yml)).
So the ordering has a precedent even though the impact field does not.

**The one measurement that appears to contradict this** is a study of 3,180
Copilot-authored PRs finding that *shorter* issue bodies merged more often, while
self-contained problem statements scored +16.65% and naming specific files
+6–7% ([arXiv 2512.21426](https://arxiv.org/html/2512.21426v1) — an unreviewed
preprint whose labels were model-assigned, so directional only). Both hold if the
fix is demotion rather than deletion, which is what this page prescribes.

## Where the shape lives

`.github/ISSUE_TEMPLATE/bug.md` and `work.md` carry the shape. They apply on the
web form and on an interactive `gh issue create`; **they do not apply when a body
is passed**, which is how an agent files. So the template is the copy of the
shape and this page is the rule. `armada-bug` and `reformat-issue` point here
rather than restating it, for the reason `armada-voice` gives: a copy is a second
source that drifts silently from the first.

## Open questions

- **Nothing gates the opening paragraph.** A lint can catch the shape and not the
  substance, and the substance is the rule. Whether the first-line-is-not-a-
  heading check is worth a `verify-foundations` line is unsettled.
