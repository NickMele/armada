# Journey 2 — Check System Health

**What it is:** Entry point for "is everything okay right now?" — a passive scan, not an action.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** You want to confirm nothing's broken before starting work, or you're spot-checking after being away.

**Concepts touched:** Bridge (Doctor).

**Milestone:** Board. Doctor is #79, and the layout is what answers this journey's three open questions.

**Design file:** `Journey 2 - Check system health.dc.html` in the Armada Mockups project. One file per journey; this document is the record, the file is the drawing.

## Settled in design, 2026-08-21

- **A result is a word, not a glyph.** `pass` / `warn` / `fail` in the status colour, no icon. Ten rows of `pass` read as a column you scan in one movement; ten circle glyphs read as decoration, and Iconography's first rule is to default to no icon. The word also survives greyscale, which colour alone does not. This also dissolves a glyph collision: `circle-check` and `circle-x` were reserved to Judge criterion verdicts on 21 Aug, so Doctor cannot own them too. Dropping the glyph resolves it without inventing a fourth family or taking anything back.
- **Rows order by result, worst first.** Not alphabetical, not by architectural layer — the question is whether anything is broken, so the answer sits at the top and passes fall away beneath. On a healthy machine every row reads pass and the order stops mattering.
- **Every row states what its probe read**, not just its verdict — "2.44.0, above the 2.34 minimum" rather than a bare pass. A verdict you cannot check is a claim, and you opened Doctor because you stopped trusting one.
- **No blended score, and the headline names the cost.** "2 failing, 3 warning — Fleet is down, so nothing will dispatch." A percentage would hide which 20% was broken.
- **Restart Fleet lives in the Fleet row**, secondary, and only on fail — it is the only thing it acts on, and a header button would have to say which module it meant. **It is not called Restart.** Under launchd restart is automatic and uncapped, so the button only skips the throttle wait; where Fleet exited 0 deliberately, launchd leaves it down on purpose and the row offers **Start Fleet** with the reason stated. A button labelled Restart would be a lie in the one state a person most needs the truth.
- **The denial rollup sits below the grid, not in it.** It is a suggestion about configuration, not a module that can be up or down — and the rule for earning a row is that Armada depends on it and it can fail.
- **No accent fill anywhere on the surface.** Doctor reports; it does not ask.

## Corrections this journey found on Doctor

- **The count is not a contract — the rule generates the list.** Doctor's pages have asserted eight, nine and ten module counts at different times, and every one of those was treated as a fact to reconcile. It is not: a module earns a row where Armada depends on it and it can be up or down, and the number is an output of that rule. It is whatever needs surfacing for a person to be confident the systems are in place and working. That is why SQLite and Keychain could simply be added, and why network reachability can join without anything being renegotiated. What the Doctor concept page should carry is the rule and the current list; a count stated in prose only ever goes stale and then gets copied — which is exactly how "nine" reached three different pages.
- **The result-vocabulary block still names `circle-check` / `triangle-alert` / `circle-x`.** Needs rewriting to words-only, per Settled in design above.
- **`triangle-alert` now has no user.** It was reserved to Doctor, `octagon-alert` was kept out of generic warnings specifically to protect it, and Doctor draws no glyphs. The reservation should be released.
- **The Doctor concept page still says a module fail queues in Alerts at Waiting.** Triage Queue reversed that: Doctor is a standing condition and reaches you as a strip above the Triage Queue tabs, never as an Alerts row. Both journey pages record it; the Doctor concept page needs the same amendment.

## Flow

Open Bridge → Doctor tab → glance at the module grid → drill into a module only if it warns or fails.

## Doctor Module Grid

Per-module status, each independently **pass, warn or fail**. No single blended score — a problem in one module (e.g. Docker) shouldn't be obscured by an unrelated healthy one.

**Doctor owns no probe logic and no state.** It invokes the probes and renders what comes back. Probes live beside their subjects. **A module earns a row where Armada depends on it and it can be up or down** — Doctor reports service health, not Job-readiness, so Docker running with nothing using it is a true pass. The Doctor concept page is the citable source for the module list, the result vocabulary and the glyphs.

| Module |
| --- |
| Fleet |
| Armada API |
| Kit |
| Manifest |
| SQLite |
| Git |
| Docker |
| Claude |
| Keychain |
| System stats |

Fully passive — no push notifications on module failure, checked on demand only.

## Open questions

- **[doctor-row-icon-and-word]** Does a Doctor health row carry both an icon and a status word, or the icon alone?
  Doctor's health grid uses `circle-check`, `triangle-alert`, and `circle-x` at 16px, inheriting the cell's status colour — deliberately circle-wrapped rather than the bare `check` and `x` used in badges, so a Doctor result never reads as a Job state. Whether a health row carries both an icon and a status word, or the icon alone, is undecided; it depends on the Doctor layout, which is not yet designed. Icon-alone is denser and the three glyphs are unambiguous, but it encodes state in a single visual channel, which is the argument Iconography used to give all sixteen Job badges an icon rather than relying on hue alone. Nine or ten modules is a short list — the density saving may not be worth the redundancy loss.

  This tension is already partly settled by "Settled in design, 2026-08-21" above, which chose a word with no icon at all for the result column. What remains open is scoped to the module grid's row treatment more broadly, not the result value specifically.

## Related

Kit (Kit module status, formerly Guild) — see Guild Setup & Configuration · Job Board (Manifest module status).
