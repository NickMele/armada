# v1's commit messages and PR bodies, as prompt exemplars

Source: `git log v1-final` and `git show v1-final:<path>` for every commit; `gh pr list --state all --repo NickMele/armada` and `gh pr view <n>` for the PR bodies. `gh` reaches the repo directly, so the PR bodies below are real, not substituted from commits.

## The cutoff

**Where:** the run `b62d214d` through `6ce61248`, landed 2026-08-16 between 22:03 and 23:47, inside the branch that became the "table binds cells to columns" and "the Job drives, the Drone reports" work.

**The tell is mechanical, not a matter of taste: the trailer names the model.** Every commit in this repository carries `Co-Authored-By: Claude <model>`. Through nearly the whole history that name is "Claude Opus 5", with a late exception naming Sonnet 5 on 2026-08-17 in the daemon-lands-work step. Inside that one run on 2026-08-16 it reads "Claude Haiku 4.5" instead, and only there.

That is where Armada's own fleet, dogfooding itself, first landed a Drone's raw implementation commits into history unreviewed and unsquashed, rather than folding a Job's work into one human-and-Opus-composed merge commit the way every surrounding "Merge Job '<name>'" commit does. The voice changes with the model, immediately and legibly:

- `0f9cedd2` (Haiku): "Changes: - Column struct now has key field to identify columns - Added row_keyed() and row_keyed_with_note() methods..." — a changelog, not an argument.
- `0f9cedd2` also carries `Fixes: https://github.com/anthropics/claude-code/issues/xyz` — a template placeholder, left unfilled, never resolving to a real issue.
- `b62d214d`: "**Tests:** 178 tests passing locally (154 lib + 24 MCP integration)" — a metric recited, not a case made for why it matters.

The best contrast is that the surrounding Opus commits describe the *same underlying work*, once reviewed and merged. `13760f02`, "Merge Job 'table-binds-cells-to-columns'", covers the identical `row_keyed` change `0f9cedd2` made, and reads: "The proof is that nothing moved: 1478 lines changed across render.rs, table.rs, help.rs and bridge.rs, and not one byte of the 80 golden fixtures... One instruction it declined, with a reason worth keeping." Same diff, two voices.

**Confidence: high on the mechanism, narrow on the footprint.** The trailer is not inference, it is the record naming the model that wrote the message. But this run is short against the whole history, and it sits inside a wider window (roughly 2026-08-16 14:00 onward) where Armada starts landing its own work on itself: `wip` commits stamped "Committed by Helm so it survives the worktree. Not verified", Jobs paused and taken over by a person, and "Merge Job" as a new commit shape. Everything selected below as an exemplar predates or sits outside this window.

## What the post-cutoff messages do badly

Three failure modes, each with the commit that shows it:

- **Restates the diff instead of arguing a case.** `938468f3`: "When Drone reports stuck, also record StepEvent::Attempted to mark that the Drone attempted the step... This makes the handling symmetric with Done status." It says what changed twice, in two sentences, and never says why the asymmetry was a problem worth having in the first place.
- **A checklist wearing a commit message's clothes.** `b62d214d`'s "Changes:" section is eight bullet points of API surface with no throughline connecting them to the defect the change fixes.
- **A placeholder that was never meant to ship.** `0f9cedd2`'s `Fixes: https://github.com/anthropics/claude-code/issues/xyz` is a template variable left in place. A prompt that produces this is one that filled a structural slot without checking whether it had real content for it, exactly the failure mode `armada-voice`'s "a summary that would read plausibly under a different job has failed" names for generated text.

## Commit exemplars, by the quality they demonstrate

Every exemplar below carries `Co-Authored-By: Claude Opus 5`, is from before the cutoff, and is a real defect, decision or measurement, not a routine change.

### Says why, and the why is a defect it found

| sha | subject | what it does well |
|---|---|---|
| `2841679` | the replay property, a recorded event sequence must reproduce the persisted state | names two real defects and a false assumption of its own author's, in that order |
| `246d67e` | a row may only point at a log that exists, and the run's message must count the run | found by building a worked example, not by reading the code, and says so |
| `bee2ed8` | narrow implied_class, a row that reached no verdict implies no class | the fix is a net deletion, and the message argues that as the evidence it was the right diagnosis |
| `6dd7334` | an empty fleet spent nothing, and it now says so | root cause is one sentence (`-0.0` vs `0.0`) and states plainly that no fixture could have caught it |
| `97b809a` | port_base is the machine's, so the suite stops asserting on 5460 | turns a report of "flaky" into a named cause: the suite was reading global machine state |
| `a29abaf` | --verbose, without which no Drone ever started | states the lesson as a rule ("asserting on argv proves you built the argv you intended; it does not prove the argv is accepted") before describing the two fixes that follow from it |
| `d0fa0067` | the relay waits on the Drone, not on the hook | names and refutes three earlier wrong diagnoses of its own, each by measurement |
| `5f38bc5` | the zombie-group entry was wrong a third time, and how | states the pattern across all three corrections rather than only fixing the third |

### Specific enough to act on: a number, a path, a measured line

| sha | subject | what it does well |
|---|---|---|
| `3695240` | the muted colour was below the readability floor, and now is not | cites the WCAG ratio, "not a preference, it was the number" |
| `a08e7c1d` | artifact_exists matches a pattern, not only a literal path | dates and names the exact Job the bug was measured against |
| `53bb86e2` | give an inline viewport's rows back without asking where the cursor is | reproduces three separate symptoms against a real pty and states each one's exact garbled output |
| `154785c9` | the grace expiring is a second deadline, and SIGKILL follows | measured timing before and after (60s wedged vs 7s), and the exact `trap '' TERM` case that exposed it |
| `d41fcd90` | a queueing check no longer parks the run loop | "The proof is not that acquisition still works. It is [the named test]." |
| `44154358` | reap before reading a group as empty, so Linux and darwin agree | states the measured divergence per platform and ties it to a traps.md entry nobody had acted on |

### Names the alternative it rejected, and why

| sha | subject | what it does well |
|---|---|---|
| `6bbd4f0` | selectors and the glob matcher behind match: | four design decisions, each stated with the failure it prevents |
| `d55cb68` | the skills: config contract | "deliberately no cmd:... if that were the design the honest move would be to delete skills: entirely" |
| `10c3197` | the starter orchestrator persona, and the four things it fixes | four behaviours, each justified by the failure mode it only shows after weeks of use |
| `61b1b93` | Job and Drone, the workflow document, and the argv nobody may run | grounds the file's placement in a stated principle rather than convention |
| `a98f893` | a guess says so, and the classifier is given nothing to work with | ties a UI change to a design principle already on record, with a measured latency number alongside it |
| `95e365f` | a guess stops and asks, or refuses when nobody is there | the confidence threshold is argued from asymmetric costs, and states plainly that it is not tuned to pass a test |

### Honest about what it did not do, or what it got wrong first

| sha | subject | what it does well |
|---|---|---|
| `a7a45c5` | the hand-over passes the skill's prose, not its name | "Two problems stacked, and only one of them was mine" |
| `e0be3e5` | an aborted check is not a failed one, and a running row says its budget | discloses two more scheduler holes found while tracing this, explicitly not fixed here |
| `89a5036a` | answering a gate settles it instead of resuming the Drone | ends with "It is not yet enough to reach DONE," naming exactly what remains |
| `3cc8913` | the four decisions taken before M3 was dispatched | records what each decision ruled out, not only what it chose |
| `1af7153` | the done-when, run against real git, and two bugs it caught | names the four things this milestone deliberately did not build, each with its reason |
| `d55cb68` | (also) the skills: config contract | names exactly what is outstanding and why it cannot land yet |

### A trap recorded so the next person does not rediscover it

| sha | subject | what it does well |
|---|---|---|
| `da1ebc5` | the dispatch record, written at dispatch | a normalisation bug that "survived every test in the suite" until a mutation surfaced it, with the exact collision it caused |
| `a29abaf` | (also) --verbose | records two probes that look free and are not, so they are not tried again |
| `b2321f9d` | three ways a suite reads the machine instead of the build | states which failures turned CI red and which one hid the other for a day |
| `2e7fb9ff` | record that the wildcard-holder test is container-only | corrects its own earlier uncited claim, and says what the correction was for |
| `901ab810` | two golden snapshots are flaky under parallel load | records a known-unknown rather than inventing a diagnosis, and says why (an agent had just burned an hour on a different one) |

### Small and exact, no ceremony

| sha | subject | what it does well |
|---|---|---|
| `aad41b2` | the flag is --concurrency, as the spec has said since M1 | one paragraph, states the drift and the blast radius, done |
| `807c6f8e` | a long aside must never starve the label | "One of the two tests this replaces asserted the bug in its own name" |
| `fb1cdab` | / filter is the first key to overflow, not r reap | explains in one line why the fixtures did and did not move |
| `3e2c961e` | the handed-over session is given something to do | distinguishes instructions from a task in one sentence, then fixes exactly that |
| `0998a87` | an entry names its Job by uuid, not by its name | the bug in one sentence, the two migration lines each explained separately |

## PR bodies

`gh pr list --state all --repo NickMele/armada` reaches the repo directly. It lists the merged PRs below, plus a closed-unmerged one (`gnhf/this-is-a-public-git-31bdcf`, the privacy rewrite, excluded because it never landed) and a later closed design doc. This repository ran review-sized PRs early and moved to direct milestone commits after, so the merged set below is the whole pool, not a sample from a larger one, and asking for roughly ten PR bodies asks for more than exist. Every merged body is genuinely good: each opens with "## Intent", states which document it is conforming to, and lists the decisions a reviewer would not see from the diff alone.

| # | title | what it does well |
|---|---|---|
| 1 | the char.yml config contract, schema, structs, and resolution | five decisions the author was explicitly asked to make, each with its reasoning, not just its answer |
| 2 | add the merge gate, the layers check, and the licence | opens with "three recorded decisions that never produced a file", naming a gap between the spec and what shipped |
| 3 | the ownership core, workspace identities, leases, and the init/clean/status verbs | states which departures from ordinary Rust practice are the corpus being obeyed, not oversights, and lists them |
| 4 | correct the zombie process-group entry in traps.md | corrects its own earlier entry, three ways wrong, each re-measured rather than merely restated |
| 5 | aggregate errors by precedence and implement clean --force-rebuild | for each of three gaps: what broke, why, and why the fix is not a bigger change than it looks |
| 6 | record phase 2's learnings where each fact is owned | four learnings, each tied to an incident that produced it, not stated as advice |
| 8 | behaviour spec of the check engine, with every trap found | names exactly what is and is not exempt from the contamination rule, and why, before anyone else has to ask |

## For Armada's Drone prompt

The exemplars above should be sampled from the "quality" tables, never from the post-cutoff set. A few-shot set built from `git log` without the cutoff applied would silently include the Haiku run and teach the checklist voice alongside the argued one.
