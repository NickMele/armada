# Consult Helm

**What it is:** "I need to reason across the Fleet" — cross-Job pattern analysis, natural-language control, or planning help.

Design fidelity: not set. Analysis: Complete. UI/UX design: Not started.

---


**Trigger:** You want to ask a question that spans multiple Jobs/Drones, or plan out a goal before dispatching individual Jobs.

**Concepts touched:** Helm.

**Milestone:** Helm. Design note: conversational surface, sibling to Bridge rather than a view inside it. Shares least with the other journeys, so it can be designed last without inheriting or owing conventions. Helm's authority is defined in terms of Debug's tiers, so the step that builds Debug's tiers must exist first.

## Flow

Open Helm (separate top-level surface, outside Bridge) → start a new session scoped to your current question → session closes when you're done investigating.

Full design detail — action authority, audit trail, session model, budget treatment — lives on the Helm concept page. This journey is the entry point; Helm's own page is the reference for what it can actually do once you're in it.

## What is already decided and landed

- **What Fleet, Job and Doctor data Helm can read, and across which Manifests.** Helm is scoped to the selected Manifest, and Fleet-wide reasoning was dropped Aug 2026 because Bridge already carries a Manifest selection and a Fleet-wide Helm would be the only thing in the product ignoring it. Read set: every Fleet query call is Manifest-scoped, with Bridge-only events and Helm polling instead. **Doctor is the exception: it is readable machine-wide**, and the Manifest scope binds Job, Drone and evidence data only. Doctor's modules are machine-level by nature — the Fleet daemon, disk, Armada API reachability — so they are not Manifest-scoped and cannot be. Under strict scoping Helm could not answer whether the daemon is healthy, in a session whose stated purpose is why and what to do about it, and machine health is often the cause: three Jobs stalled because the disk filled is exactly the cross-Job pattern Helm exists to find, and a strictly-scoped Helm would see three stalls and no reason. Reading it leaks nothing, because the Manifest boundary exists to stop one project's work being visible from another, and machine health is shared context rather than another project's work. Rejected: withholding Doctor entirely, which makes Helm misdiagnose the most common systemic cause; and filtering Doctor to rows relevant to the selected Manifest, which Doctor cannot do because it is a rollup surface that owns no checks of its own.

## Open questions

- **[helm-session-lifecycle]** How do you start, name and find Helm sessions?
  Session-per-topic is decided. Its surface is not: how a session starts, whether it is named or auto-titled, how past sessions are found again, and what "done" looks like. Session retention/expiry already exists as a Guild config row, which implies past sessions are findable for some window — that window and the way you browse it are unspecified.

## Related

Helm — the concept page carrying the full design: action authority, audit trail, session model, budget treatment.
