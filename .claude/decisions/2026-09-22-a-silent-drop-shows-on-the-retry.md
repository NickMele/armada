# A retry declares fewer files and a test falls out.

**Decided 2026-09-22.**

Only a person narrowing scope writes a revision, so a retry that declares less drops a test with no record.

**Chosen:** Show it as dropped on that retry, as well as on a person's scope revision.

**Cost he took:** Drops are recorded in two places, not one.

**Where it landed:** #1530
