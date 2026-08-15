//! **Which of Armada's own verbs this machine has ever run** — the format and
//! the fold.
//!
//! ## The complaint this exists to fix
//!
//! *"I need a task list of what CLI needs me to test. (I wish we could observe
//! this as I use it.)"* Fifteen features landed in a day, most of them exercised
//! once or not at all, and nothing on the machine could say which. The answer
//! required reading `~/.armada/recent.jsonl` by hand — and that buffer keeps ten
//! runs ([`crate::recent::KEEP`]), so it can say what you *just* did and can
//! never say what you have **never** done.
//!
//! ## It is a counter, not a raised item, and that is the whole of the design
//!
//! `docs/reserved/001-raised-items-need-identity.md` argues against a parallel
//! list of things needing attention, and this is deliberately not one. A failure,
//! a report and a task are all *records of a decision or an event*: each has an
//! id, a state and a Job it can become. **A verb you have not run yet is none of
//! those.** Its identity is its own name, there is nothing to acknowledge, and
//! the row disappears the moment you type the verb.
//!
//! So this is machine state, exactly as [`crate::recent`] is: a fact about this
//! laptop, kept beside the other facts about it. What turns a row here into
//! something with an id is `armada task "<try it>"` — which the person writes,
//! because a backlog that fills itself is one nobody trusts.
//!
//! ## The roster is passed in, never held here
//!
//! Coverage is a **diff against the list of verbs Armada owns**, and that list is
//! `armada_helm::args::every_verb()` — derived from the same constants the help
//! pages and the parser read. Retyping it here would be the mistake
//! `docs/reserved/009` item 5 records: two lists, one of them the one nobody
//! edits. Helm is above this crate, so the roster arrives as an argument
//! (`ARCHITECTURE.md` §1.5) and this module never learns a verb name.
//!
//! ## Nothing here is telemetry
//!
//! `~/.armada/coverage.jsonl` is machine state and machine state does not sync —
//! `PLAN.md` §13.1's line, that what describes *you* syncs and what describes
//! *this machine* does not, puts it on the same side as `recent.jsonl` and
//! `failures.jsonl` and the opposite side from `guild/`. It is never sent
//! anywhere, and it holds a verb name and two numbers: **nothing a person typed
//! reaches it**, because [`matched`] only ever writes back a name that was
//! already on Armada's own roster.

use serde::{Deserialize, Serialize};

/// One verb, and what this machine has done with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    /// The verb, as the roster spells it — `fleet spawn`, `doctor`.
    pub verb: String,
    /// How many times it has been run here.
    pub count: u64,
    /// The first time, wall clock milliseconds.
    pub first_ms: u64,
    /// The most recent, wall clock milliseconds.
    pub last_ms: u64,
    /// Whether the **most recent** run of it answered without an error.
    ///
    /// **Not "has it ever worked".** The question this file answers is *what
    /// still needs looking at*, and a verb whose last run failed needs looking
    /// at whether or not an earlier one passed. Keeping the optimistic version
    /// would let one lucky run hide every failure after it.
    pub ok: bool,
}

/// One row of `armada coverage` — a roster verb, tried or not.
///
/// **Every verb on the roster gets a row, including the ones with no `Seen`
/// behind them.** That is the entire feature: *what have I never run* is
/// unanswerable from a file that only records what happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// The verb, as `armada <verb>` would be typed.
    pub verb: String,
    /// How many times it has been run here. `0` for one never run.
    pub count: u64,
    /// Whether the most recent run answered without an error. `false` when it
    /// has never run, which the `count` disambiguates.
    pub ok: bool,
    /// How long ago the most recent run was, in seconds. `null` for a verb this
    /// machine has never run — **null rather than zero**, because "never" and
    /// "a moment ago" are the two answers this listing exists to tell apart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_s: Option<u64>,
}

/// The roster name this run counts against, if any.
///
/// [`crate::recent::verb_of`] labels a run with at most two words, which is
/// exactly the shape of every entry on the roster — so the two-word label is
/// tried first and the leading word second. **`armada tasks start a1b2` counts
/// as `tasks`**, because `tasks start` is not a page `--help` reaches and a
/// coverage row that could never be satisfied is a row that lies.
///
/// **`None` for anything off the roster**, which is what keeps a person's own
/// words out of this file: a `commands:` child declared by a repository is not
/// one of Armada's verbs, and nothing about it is written down here.
pub fn matched(label: &str, roster: &[String]) -> Option<String> {
    let on_roster = |name: &str| roster.iter().any(|verb| verb == name);
    if on_roster(label) {
        return Some(label.to_string());
    }
    let head = label.split(' ').next().unwrap_or(label);
    on_roster(head).then(|| head.to_string())
}

/// Fold this run into what the file already held.
///
/// **A pure function over the whole set**, so the file is rewritten rather than
/// appended to — the choice [`crate::recent`] explains and for the same reason:
/// this file is *bounded by the roster*, one line per verb, and an append-only
/// file that has to be compacted is two mechanisms where one will do.
pub fn note(mut seen: Vec<Seen>, verb: &str, at_ms: u64, ok: bool) -> Vec<Seen> {
    match seen.iter_mut().find(|entry| entry.verb == verb) {
        Some(entry) => {
            entry.count += 1;
            entry.last_ms = at_ms;
            entry.ok = ok;
        }
        None => seen.push(Seen {
            verb: verb.to_string(),
            count: 1,
            first_ms: at_ms,
            last_ms: at_ms,
            ok,
        }),
    }
    seen
}

/// Every line the file holds. A torn line is skipped, the same rule the inbox
/// and the failure log follow.
pub fn parse(text: &str) -> Vec<Seen> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Seen>(line.trim()).ok())
        .collect()
}

/// The file's contents, one JSON object per line.
///
/// **Sorted by verb**, unlike the ring buffer's most-recent-first: this file is
/// keyed rather than ordered, and a stable order means a diff of it shows what
/// changed instead of what moved.
pub fn render(seen: &[Seen]) -> String {
    let mut sorted: Vec<&Seen> = seen.iter().collect();
    sorted.sort_by(|a, b| a.verb.cmp(&b.verb));
    sorted
        .iter()
        .filter_map(|entry| serde_json::to_string(entry).ok())
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The listing: **every verb on the roster**, with what this machine has done
/// with it.
///
/// **Never run first, then least recently run.** The question is *what have I
/// not got to yet*, so the rows that answer it are the ones a reader should not
/// have to scroll for — and among the tried ones, the stalest is the closest
/// thing to untried. Ties break on the name, so two runs in the same second
/// keep an order a reader can predict.
///
/// **A `Seen` for a verb no longer on the roster is dropped.** Armada renames
/// its own verbs, and a row for a name `--help` cannot reach is a row nobody can
/// act on.
pub fn against(roster: &[String], seen: &[Seen], now_ms: u64) -> Vec<Row> {
    let mut rows: Vec<Row> = roster
        .iter()
        .map(|verb| match seen.iter().find(|entry| &entry.verb == verb) {
            Some(entry) => Row {
                verb: verb.clone(),
                count: entry.count,
                ok: entry.ok,
                // Saturating, because a clock can go backwards — the rule
                // `crate::failure::age` states, arriving at a second listing.
                age_s: Some(now_ms.saturating_sub(entry.last_ms) / 1_000),
            },
            None => Row {
                verb: verb.clone(),
                count: 0,
                ok: false,
                age_s: None,
            },
        })
        .collect();
    rows.sort_by(|a, b| match (a.age_s, b.age_s) {
        (None, None) | (Some(_), Some(_)) => b.age_s.cmp(&a.age_s).then(a.verb.cmp(&b.verb)),
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster() -> Vec<String> {
        ["doctor", "failures", "fleet spawn", "tasks"]
            .iter()
            .map(|verb| (*verb).to_string())
            .collect()
    }

    /// **A sub-verb counts against its parent.** `tasks start` is not a page
    /// `--help` reaches, so a row for it could never be satisfied.
    #[test]
    fn a_run_counts_against_the_roster_entry_it_is_under() {
        let roster = roster();
        assert_eq!(matched("doctor", &roster).as_deref(), Some("doctor"));
        assert_eq!(
            matched("fleet spawn", &roster).as_deref(),
            Some("fleet spawn")
        );
        assert_eq!(matched("tasks start", &roster).as_deref(), Some("tasks"));
    }

    /// **Nothing off the roster is written down**, which is what keeps a
    /// person's own words — a repository's declared command, a typo — out of
    /// this file entirely.
    #[test]
    fn a_verb_armada_does_not_own_is_never_counted() {
        let roster = roster();
        assert_eq!(matched("manifest deploy", &roster), None);
        assert_eq!(matched("ghp_deadbeef", &roster), None);
        assert_eq!(matched("armada", &roster), None);
    }

    /// The count rises, the last run wins, and the first is kept.
    #[test]
    fn a_second_run_of_one_verb_is_one_line_with_a_count() {
        let seen = note(Vec::new(), "doctor", 1_000, true);
        let seen = note(seen, "doctor", 5_000, false);
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].count, 2);
        assert_eq!(seen[0].first_ms, 1_000);
        assert_eq!(seen[0].last_ms, 5_000);
        assert!(
            !seen[0].ok,
            "the most recent run decides, so one lucky run cannot hide the failures after it"
        );
    }

    /// **Every roster verb gets a row, tried or not** — the whole feature, since
    /// *what have I never run* is unanswerable from a record of what happened.
    #[test]
    fn the_listing_holds_the_whole_roster_and_leads_with_what_was_never_run() {
        let seen = note(Vec::new(), "doctor", 9_000, true);
        let seen = note(seen, "failures", 1_000, true);
        let rows = against(&roster(), &seen, 10_000);

        assert_eq!(rows.len(), 4, "{rows:?}");
        // Never run first, in name order.
        assert_eq!(rows[0].verb, "fleet spawn");
        assert_eq!(rows[1].verb, "tasks");
        assert_eq!(rows[0].age_s, None, "never is not zero seconds ago");
        assert_eq!(rows[0].count, 0);
        // Then the stalest of the ones that have been.
        assert_eq!(rows[2].verb, "failures");
        assert_eq!(rows[2].age_s, Some(9));
        assert_eq!(rows[3].verb, "doctor");
        assert_eq!(rows[3].age_s, Some(1));
    }

    /// A verb the roster no longer has is dropped rather than drawn: a row
    /// naming something `--help` cannot reach is a row nobody can act on.
    #[test]
    fn a_counted_verb_that_left_the_roster_leaves_the_listing() {
        let seen = note(Vec::new(), "guild verify", 1_000, true);
        let rows = against(&roster(), &seen, 2_000);
        assert!(
            rows.iter().all(|row| row.verb != "guild verify"),
            "{rows:?}"
        );
    }

    /// The file round-trips, and a torn last line does not hide the ones before
    /// it — the rule every append-only file here follows.
    #[test]
    fn a_torn_line_costs_its_own_row_and_no_others() {
        let seen = note(note(Vec::new(), "doctor", 1, true), "failures", 2, true);
        let mut text = render(&seen);
        assert_eq!(text.lines().count(), 2);
        text.push_str("{\"verb\":\"fle");
        let read = parse(&text);
        assert_eq!(read.len(), 2, "{read:?}");
        assert_eq!(read[0].verb, "doctor", "sorted by name, stably");
    }
}
