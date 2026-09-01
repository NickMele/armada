# Read the work and merge by hand

**What it is:** What happens at the end of a successful M1 Job, when there is no auto-merge, no PR, and no push, and the last act of the Job is a person reading the branch and merging it by hand.

Design fidelity: M1 only.

---


**Trigger:** A Job reaches `completed_success` and its branch is waiting.

**Concepts touched:** Job, Evidence.

**Milestone:** M1 — Dogfood.

## Why this journey needs to exist

**M1 has no auto-merge, no PR, and no push.** The Drone-facing VCS type has no push method. So the last act of every successful Job is me reading the branch and merging it by hand — and that is the step where the whole milestone gets judged, since I am standing in for the Judge that does not exist yet.

Nothing in the other journeys covers it. Review a Job's work before it lands is a Surface-milestone version, with a diff viewer and an approve/reject decision inside the app.

## The flow, reduced to M1

1. The Job shows as complete, with its branch name.
2. Its Evidence is readable — what the Drone submitted at each part, in order.
3. I read the diff in my own tools.
4. I merge, or I do not. Armada is not involved either way.

## What it must not do yet

No in-app diff viewer, no approve/reject buttons, no merge action. The app's job here is to hand me a branch name and the evidence trail, and get out of the way.

## Design note

The interesting question this journey answers for the milestone is **whether the Evidence trail is worth reading**. If I merge without looking at it, the submission schema is wrong, and that is a finding.

**From the milestone design pass.** The branch name is the largest value on the screen and copies on click; the Evidence trail is the largest element, one entry per step in submission order with the Check that let it pass, rather than a panel to expand — if the trail is the reason to open the screen it should not be the thing you have to open. Two controls, both secondary and neither filled: copy the branch, open the worktree. No approve, no reject, no merge, no in-app diff.

The sentence carrying the handover is "the branch is unpushed and unmerged. Armada does not push and has no merge action — read the diff in your own tools and land it yourself." The accent appears nowhere on this screen; it is spent on Approve and dispatch, because there is no decision here that Armada participates in.

Produced one new component row: **Evidence trail** in Armada Components.

## Related

Read a failed Job — the unsuccessful counterpart to this journey, for a Job that reaches a terminal failed state instead.
