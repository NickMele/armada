# User journeys

The journeys carried out of the Armada User Journeys database in Notion, and the ones designed since. Each names a trigger — the moment a person reaches for Armada — and the surfaces that answer it. **These describe intent, not build order.** Notion's own phased plan is explicit that design order is not build order: journeys are designed in the order their conventions get reused, not in the order the milestones build them.

Every journey names a **Milestone** — Notion decides what Armada v2 builds, and the milestone is where this journey's answer is currently expected to land, not a promise of when.

Read the journey that covers what you are about to design or build, before you build it.

| Journey | Read it before you |
| --- | --- |
| [Dispatch a Job](dispatch-a-job.md) | Design or build the approval flow — Job Board, the approval card, or any surface (Triage, Alerts, Helm, the pre-approved queue) that reuses its pattern |
| [Check System Health](check-system-health.md) | Design or build Doctor's health grid, or add a module to it |
| [Triage Queue](triage-queue.md) | Design or build the proactive, self-initiated scan across Alerts and the Job Board's review filter |
| [Respond to a Push Alert](respond-to-a-push-alert.md) | Design or build the reactive single-item Debug view, the Intervention Ladder, or the Alert Levels a push notification carries |
| [Monitor Active Work](monitor-active-work.md) | Design or build the Active Jobs list or the job detail rail — the M1 monitoring subset, and what the full surface defers |
| [Set Up a Project (Manifest)](set-up-a-project-manifest.md) | Design or build onboarding a new repo — Locate, Scan, the picker, the proposal sheet, ports, Write and Verify |
| [Change a Job's Scope](change-a-jobs-scope.md) | Design or build widening or narrowing a dispatched Job — the scope picker, the second approval gate, the narrowing confirmation, and what a respawn costs |
| [Run and edit a Manifest](run-and-edit-a-manifest.md) | Design or build Bridge's Manifest surface — running a Check or Command on demand, editing a manifest, or Verify's drift-plus-dry-run |
| [Guild Setup & Configuration](guild-setup-and-configuration.md) | Design or build the Kit/Machine settings surface — first-time init or an ongoing settings edit |
| [Consult Helm](consult-helm.md) | Design or build Helm's entry point — starting a session, or what it can read |
| [First-Run Onboarding](first-run-onboarding.md) | Design or build the hard-gated first-launch sequence that chains Guild Setup, Set Up a Project, Check System Health and Dispatch a Job together |
| [Take Over a Job](take-over-a-job.md) | Design or build Pilot's confirmation modal, or the flow for taking a stuck Job away from its Drone |
| [Read a failed Job](read-a-failed-job.md) | Design or build what M1 shows when a Job reaches a terminal failed state |
| [Read the work and merge by hand](read-the-work-and-merge-by-hand.md) | Design or build what M1 shows when a Job reaches `completed_success` and there is still no auto-merge, no PR, and no push |
| [Dispatch a milestone](dispatch-a-milestone.md) | Design or build approving one Job that names a milestone, and what a person sees while it decomposes into Jobs |

## Both ways

Every file above is a real file in this directory, and every real file in this directory (besides this one) is listed above. A gate rule checks both directions, so an entry with no file, or a file with no entry, fails the build.
