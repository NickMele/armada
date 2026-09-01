# Journey 2 — Check System Health

**What it is:** Entry point for "is everything okay right now?" — a passive scan, not an action.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** You want to confirm nothing's broken before starting work, or you're spot-checking after being away.

**Concepts touched:** Bridge (Doctor).

**Milestone:** Board. Doctor is #79, and the layout is what answers this journey's three open questions.

**Drawn against** `docs/contracts/design-system.md` and `docs/contracts/system-architecture.md`.

## Settled in design, 2026-08-21

- **A result is a word, not a glyph.** `pass` / `warn` / `fail` in the status colour, no icon. Ten rows of `pass` read as a column you scan in one movement; ten circle glyphs read as decoration, and Iconography's first rule is to default to no icon. The word also survives greyscale, which colour alone does not. This also dissolves a glyph collision: `circle-check` and `circle-x` were reserved to Judge criterion verdicts on 21 Aug, so Doctor cannot own them too. Dropping the glyph resolves it without inventing a fourth family or taking anything back.
- **Rows order by result, worst first.** Not alphabetical, not by architectural layer — the question is whether anything is broken, so the answer sits at the top and passes fall away beneath. On a healthy machine every row reads pass and the order stops mattering.
- **Every row states what its probe read**, not just its verdict — "2.44.0, above the 2.34 minimum" rather than a bare pass. A verdict you cannot check is a claim, and you opened Doctor because you stopped trusting one.
- **No blended score, and the headline names the cost.** "2 failing, 3 warning — Fleet is down, so nothing will dispatch." A percentage would hide which 20% was broken.
- **Restart Fleet lives in the Fleet row**, secondary, and only on fail — it is the only thing it acts on, and a header button would have to say which module it meant. **It is not called Restart.** Under launchd restart is automatic and uncapped, so the button only skips the throttle wait; where Fleet exited 0 deliberately, launchd leaves it down on purpose and the row offers **Start Fleet** with the reason stated. A button labelled Restart would be a lie in the one state a person most needs the truth.
- **The denial rollup sits below the grid, not in it.** It is a suggestion about configuration, not a module that can be up or down — and the rule for earning a row is that Armada depends on it and it can fail.
- **No accent fill anywhere on the surface.** Doctor reports; it does not ask.

## Settled in design, 2026-08-31

- **Rows hold position while the read is in flight and sort worst-first once the last one lands.** Sorting as answers arrive moves a row a person has already started reading, and the ordering only earns its keep when every result is in.
- **A pending row states `reading` in `--fg-subtle`, in the result column.** No spinner, no shimmer, no skeleton — motion on a data surface is spent on the running mark. Row heights are fixed, so nothing reflows as results arrive.
- **Arriving from an error marks the module with the row focus bar and scrolls to it.** Nothing is reordered and nothing takes a highlight hue: the grid a person arrives at is the grid they would have opened from the rail. A dismissible line above carries which error sent them and its code.
- **The first-run gate lives in the onboarding footer, not in the grid.** The grid is identical to the one Doctor renders from the rail. The released Dispatch a job button is the one accent fill near the surface, and it belongs to the sequence rather than to Doctor, which still reports and asks nothing.
- **The gate names the module holding the step open.** A blocked Continue with no reason beside it is the failure the gate exists to prevent.
- **A threshold is stated in the probe text, and Machine is a link.** "Above the 2.34 minimum set on Machine" — thresholds are settings rather than constants, and the row that shows one says where it is changed. The action column stays for things that act.
- **A row offers no control for an act Armada cannot perform.** Git below the minimum hands over `brew upgrade git` as a mono value that copies on click. SQLite keeps its Migrate button, because the migration is Armada's own.

## Corrections this journey found on Doctor

- **The count is not a contract — the rule generates the list.** Doctor's pages have asserted eight, nine and ten module counts at different times, and every one of those was treated as a fact to reconcile. It is not: a module earns a row where Armada depends on it and it can be up or down, and the number is an output of that rule. It is whatever needs surfacing for a person to be confident the systems are in place and working. That is why SQLite and Keychain could simply be added, and why network reachability can join without anything being renegotiated. What the Doctor concept page should carry is the rule and the current list; a count stated in prose only ever goes stale and then gets copied — which is exactly how "nine" reached three different pages.
- **~~The result-vocabulary block still names `circle-check` / `triangle-alert` / `circle-x`.~~ Applied 2026-08-31.** The prose had already been words-only; what remained was a vestigial glyph table beneath it, which is the shape a correction takes when the sentence is fixed and the block under it is not.
- **~~`triangle-alert` now has no user, so the reservation should be released.~~ Reversed 2026-08-31.** The premise is right and the conclusion was wrong. It was reserved to Doctor, `octagon-alert` was kept out of generic warnings specifically to protect it, and Doctor draws no glyphs — but a reservation with no current user is not a spare glyph. `triangle-alert` means a check warns, Doctor is the only surface that can say that, and releasing it would let another surface take the mark that has to be free the day Doctor needs it. `[doctor-warn-glyph]` is closed on Iconography on those grounds.
- **The Doctor concept page still says a module fail queues in Alerts at Waiting.** Triage Queue reversed that: Doctor is a standing condition and reaches you as a strip above the Triage Queue tabs, never as an Alerts row. Both journey pages record it; the Doctor concept page needs the same amendment.

## Flow

Open Bridge → Doctor tab → glance at the module grid → drill into a module only if it warns or fails.

## Doctor Module Grid

Per-module status, each independently **pass, warn or fail**. No single blended score — a problem in one module (e.g. Docker) shouldn't be obscured by an unrelated healthy one.

**Doctor owns no probe logic and no state.** It invokes the probes and renders what comes back. Probes live beside their subjects. **A module earns a row where Armada depends on it and it can be up or down** — Doctor reports service health, not Job-readiness, so Docker running with nothing using it is a true pass. The Doctor concept page is the citable source for the module list, the result vocabulary and the glyphs.

**The list lives on [Doctor](../concepts/doctor.md) and is not copied here.** The copy that stood here omitted Machine, which is the failure a second roster always has: the rule generates the list, and a page that restates the output rather than pointing at the rule goes stale the next time the rule admits something.

Fully passive — no push notifications on module failure, checked on demand only.

## Open questions

Nothing. `[doctor-row-icon-and-word]` was a second copy of Doctor's own
`[doctor-icon-and-word]`, and the Doctor drawing on 2026-08-31 answered it
there: **the word alone, no icon**, holding at the current row count and while
rows are still resolving — a pending row states `reading` in the same column, in
`--fg-subtle`. See [Doctor](../concepts/doctor.md), Result vocabulary. It is not
answered twice here.

## Related

Kit (Kit module status, formerly Guild) — see Guild Setup & Configuration · Job Board (Manifest module status).
