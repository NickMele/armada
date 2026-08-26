# Read a failed Job

**What it is:** What a person sees when a Job's Check fails, or its Drone dies, and the Job reaches a terminal failed state.

Design fidelity: M1 only.

---


**Trigger:** A Job's Check fails, or its Drone dies, and the Job reaches a terminal failed state.

**Concepts touched:** Job, Check.

**Milestone:** M1 — Dogfood.

## Why this journey needs to exist

At M1 a failed Check ends the Job — no retry, no triage, no escalation. That is deliberate: the point of the milestone is to see what real failures look like before building a taxonomy to sort them into. But it means **failure is a first-class outcome from day one** and nothing currently describes what I see when it happens.

## The flow, reduced to M1

1. The Job shows as failed, with the step it failed on.
2. The failing Check's captured output is readable without opening a log file.
3. The worktree and branch are still there — stated, not implied.
4. I go read the branch myself and decide what to do.

## What it must not do yet

No retry button, no escalation category, no suggested action. Every one of those is a Recovery-milestone answer, and offering a weak version now would hide the failure modes this milestone exists to observe.

## Design note

What this journey mostly owes is **legibility of a dead end**. The screen's job is to make it obvious what failed, where the work is, and that nothing further is going to happen automatically.

**Milestone design file:** `M1 - Dogfood.dc.html`, block `1f`. Drawn 2026-08-23. The screen states four things in order — what failed, that the job is over, where the branch is, and where the log is — and the sentence saying nothing happens automatically is written out rather than left to be inferred from an absence of buttons. All three controls take you to the work; none offers to act on it.

Two decisions came out of drawing it. **A failed step gained an activity value**, `--step-failed` with a `--step-failed-bg` surface and a bare `x` glyph: it was drawn neutral first, on the grounds that a Check result is measured and measured facts render flatly, and that was reversed — at M1 a failed Check ends the Job, so that row is the entire reason a person opened the screen, and making them find it by weight in a rail of muted rows is the frustration this journey exists to prevent. The gate row beneath stays neutral: the step's state is hued, the Check's exit code is measured. **The per-job log gained a surface**, the first place in Armada that points at a log sink.

## Related

Read the work and merge by hand — the successful counterpart to this journey, for a Job that reaches `completed_success` instead.

This journey has no number because the design project has not drawn it. A number in a filename here means a `Journey N` drawing exists to match it; inventing one would assert a correspondence that does not.
