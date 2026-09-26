# What does a running Job's run draw?

**Decided 2026-09-21.**

The boards disagreed: three phases on one, seven steps on another, four in the shipped workflow file.

**Chosen:** The steps the workflow's own file declares. The implement step opens into groups and tasks.

**Cost he took:** The three-phase header waits for workflow-as-a-profile, which is the last backend phase.

**Where it landed:** #1530, #1533
