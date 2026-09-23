# A test covers files two groups both touch.

**Decided 2026-09-22.**

Running it at each group means a group fails a test a later group was always going to finish.

**Chosen:** At the end of the last group that touches its files, and again at handoff.

**Cost he took:** If the first group broke it, you find out two groups later.

**Where it landed:** #1530, #1542
