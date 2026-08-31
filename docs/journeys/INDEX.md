# User journeys

The journeys carried out of the Armada User Journeys database in Notion, and the ones designed since. Each names a trigger — the moment a person reaches for Armada — and the surfaces that answer it. **These describe intent, not build order.** Notion's own phased plan is explicit that design order is not build order: journeys are designed in the order their conventions get reused, not in the order the milestones build them.

Every journey names a **Milestone** — Notion decides what Armada v2 builds, and the milestone is where this journey's answer is currently expected to land, not a promise of when.

Read the journey that covers what you are about to design or build, before you build it.

| Journey | Read it before you |
| --- | --- |
| [1 — Dispatch a Job](1-dispatch-a-job.md) | Design or build the approval flow — Job Board, the approval card, or any surface (Triage, Alerts, Helm, the pre-approved queue) that reuses its pattern |
| [2 — Check System Health](2-check-system-health.md) | Design or build Doctor's health grid, or add a module to it |
| [3 — Triage Queue](3-triage-queue.md) | Design or build the proactive, self-initiated scan across Alerts and the Job Board's review filter |
| [3.5 — Respond to a Push Alert](3.5-respond-to-a-push-alert.md) | Design or build the reactive single-item Debug view, the Intervention Ladder, or the Alert Levels a push notification carries |
| [4 — Monitor Active Work](4-monitor-active-work.md) | Design or build the Active Jobs list or the job detail rail — the M1 monitoring subset, and what the full surface defers |
| [9 — Run and edit a Manifest](9-run-and-edit-a-manifest.md) | Design or build Bridge's Manifest surface — running a Check or Command on demand, editing a manifest, or Verify's drift-plus-dry-run |
| [10 — Guild Setup & Configuration](guild-setup-and-configuration.md) | Design or build the Kit/Machine settings surface — first-time init or an ongoing settings edit |
| [11 — Consult Helm](consult-helm.md) | Design or build Helm's entry point — starting a session, or what it can read |
| [12 — Set Up a Project (Manifest)](set-up-a-project-manifest.md) | Design or build onboarding a new repo — Locate, Scan, Proposal, Write, Verify |
| [13 — First-Run Onboarding](first-run-onboarding.md) | Design or build the hard-gated first-launch sequence that chains Guild Setup, Set Up a Project, Check System Health and Dispatch a Job together |
| [14 — Take Over a Job](take-over-a-job.md) | Design or build Pilot's confirmation modal, or the flow for taking a stuck Job away from its Drone |
| [15 — Read a failed Job](read-a-failed-job.md) | Design or build what M1 shows when a Job reaches a terminal failed state |
| [16 — Read the work and merge by hand](read-the-work-and-merge-by-hand.md) | Design or build what M1 shows when a Job reaches `completed_success` and there is still no auto-merge, no PR, and no push |
| [Dispatch a milestone](dispatch-a-milestone.md) | Design or build approving one Job that names a milestone, and what a person sees while it decomposes into Jobs |

## On the numbering

Six journeys carry a number the design project has itself assigned, stated inside each journey's own page as the name of its drawing file in the Armada Mockups project: `Journey 1 - Dispatch a job.dc.html`, `Journey 2 - Check system health.dc.html`, `Journey 3 - Triage queue.dc.html`, `Journey 3.5 - Respond to a push alert.dc.html`, `Journey 4 - Monitor active work.dc.html`, and `Journey 9 - Run and edit a manifest.dc.html`. Those six numbers are load-bearing here and line up exactly.

The journeys carried out of Notion that have not been drawn — Guild Setup & Configuration, Consult Helm, Set Up a Project (Manifest), First-Run Onboarding, Take Over a Job, Read a failed Job, and Read the work and merge by hand — have no drawing (`UI/UX Design` reads `Not started` on five of them, and the other two exist only as blocks inside the M1 milestone drawing rather than as their own `Journey N - ...` file). They are numbered 10 through 16 here only so the file set has a stable order; if the design project later assigns one of them a different number, this index and that file's name should both move to match. Each of those files carries the same note at its own top.

**An entry with no number at all is a journey designed after that sequence was assigned, and no drawing exists for it.** It takes a number when it is drawn, and not before.

## Both ways

Every file above is a real file in this directory, and every real file in this directory (besides this one) is listed above. A gate rule checks both directions, so an entry with no file, or a file with no entry, fails the build.
