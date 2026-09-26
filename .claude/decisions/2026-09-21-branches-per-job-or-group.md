# Branch per task, group, or Job?

**Decided 2026-09-21.**

The design had every agent sharing one working copy and also offered a branch per task. One copy can only be on one branch.

**Chosen:** Per Job or per group. Per-task branches dropped.

**Cost he took:** A single bad task's commits cannot be dropped in isolation, only a whole group's.

**Where it landed:** #1530
