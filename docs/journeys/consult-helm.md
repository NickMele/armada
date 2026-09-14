# Consult Helm

**What it is:** "I want to ask about one repository's Jobs, without opening each one" — Helm's conversation, in the dock.

Design fidelity: drawn. Analysis: Complete. UI/UX design: Complete.

---

**Trigger:** You want to ask a question that spans multiple Jobs in one repository, or plan out a goal before dispatching individual Jobs.

**Concepts touched:** Helm.

## Flow

Open the dock (`⌘J`, or the edge strip below 1100px) → the conversation zone holds the repository Helm currently answers for → ask, or Start fresh → the picker never moves.

Which repository Helm answers for follows the most recent explicit act: picking a repository, "Discuss with Helm" on a question card, or the dock's own switch. Closing Bridge and restarting Fleet does not lose the conversation — Fleet resumes it from the stored session, and a follow-up lands in context.

Full design detail — action authority, audit trail, session model, budget treatment — lives on the Helm concept page. This journey is the entry point; Helm's own page is the reference for what it can actually do once you're in it.

## What is already decided and landed

- **What Fleet, Job and Doctor data Helm can read, and across which Manifests.** Helm is scoped to the selected Manifest, and Fleet-wide reasoning was dropped Aug 2026 because Bridge already carries a Manifest selection and a Fleet-wide Helm would be the only thing in the product ignoring it. Read set: every Fleet query call is Manifest-scoped, with Bridge-only events and Helm polling instead. **Doctor is the exception: it is readable machine-wide**, and the Manifest scope binds Job, Drone and evidence data only. Doctor's modules are machine-level by nature — the Fleet daemon, disk, Armada API reachability — so they are not Manifest-scoped and cannot be. Under strict scoping Helm could not answer whether the daemon is healthy, in a session whose stated purpose is why and what to do about it, and machine health is often the cause: three Jobs stalled because the disk filled is exactly the cross-Job pattern Helm exists to find, and a strictly-scoped Helm would see three stalls and no reason. Reading it leaks nothing, because the Manifest boundary exists to stop one project's work being visible from another, and machine health is shared context rather than another project's work. Rejected: withholding Doctor entirely, which makes Helm misdiagnose the most common systemic cause; and filtering Doctor to rows relevant to the selected Manifest, which Doctor cannot do because it is a rollup surface that owns no checks of its own.
- **How a session starts, is found and ends.** One running conversation per repository, not per topic — Start fresh replaces it rather than opening a second one, so there is nothing to name or browse. It clears after 30 quiet days.

## Related

Helm — the concept page carrying the full design: placement, action authority, audit trail, session model, budget treatment.

