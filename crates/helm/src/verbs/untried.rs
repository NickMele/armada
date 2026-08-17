//! `armada untried` — **which of Armada's verbs you have never run.**
//!
//! ## The complaint this exists to fix
//!
//! *"I need a task list of what CLI needs me to test. (I wish we could observe
//! this as I use it.)"* Fifteen features landed in one day, most of them
//! exercised once or not at all, and the only way to find out which was to read
//! `~/.armada/recent.jsonl` by hand — a buffer that keeps ten runs, so it can
//! say what you just did and can never say what you have never done.
//!
//! **So it is observed as you use it**, which is the parenthesis in the ask. The
//! counter is written on the same path the ring buffer is, at the end of every
//! run, and it costs one small file rewrite.
//!
//! ## Why it is not a fourth list of things needing attention
//!
//! Failures are what Armada noticed, reports are what you noticed, tasks are
//! what you intend — and each is a record with an id, a state and a Job it can
//! become. **A verb you have not run is none of those.** Its identity is its own
//! name, there is nothing to acknowledge, and the row disappears the moment you
//! type the verb. `docs/reserved/001-raised-items-need-identity.md` argues
//! against a parallel list of *raised items*; this is a counter, and giving it
//! ids and states would be inventing the parallel list rather than avoiding one.
//!
//! **What it does offer is the crossing.** At a terminal, picking a row writes
//! `armada task "try armada <verb>"` — the store that already exists, with an id
//! that already means something. It is offered and never automatic: *a backlog
//! that fills itself is one nobody trusts*.
//!
//! ## The roster is derived, never retyped
//!
//! [`crate::args::every_verb`] builds it from the same constants the parser and
//! the help pages read, and there is already a test that fails when a verb ships
//! without a page. Retyping the list here is the mistake `docs/reserved/009`
//! item 5 records — two lists, one of them the one nobody edits.
//!
//! **Sub-verbs count against their parent.** `armada tasks start` is not a page
//! `--help` reaches, so it counts as `tasks`; see
//! [`armada_core::untried::matched`]. The alternative is a row that can never
//! be satisfied, which is a row that lies.
//!
//! ## Not telemetry
//!
//! `~/.armada/untried.jsonl` never leaves this machine and never syncs — only
//! `guild/` does (`PLAN.md` §13.1). It holds a verb name off Armada's own roster
//! and three numbers; nothing a person typed can reach it.

use armada_core::ctx::Clock;
use armada_core::envelope::{Envelope, UntriedData};
use armada_core::error::{ArmadaError, Status};
use armada_core::untried::Row;

use crate::ask::{Ask, Choice};
use crate::verbs::Output;

pub use crate::verbs::fleet::Where;
pub use crate::verbs::guild::Look;

/// What the last option says, and what it says about itself.
/// **`write nothing` rather than `stop looking`.** Paired with the prompt above:
/// a reader who has just been told the list writes tasks needs the way out to
/// say that it writes none, because *stop looking* reads as *I have finished
/// reading* and not as *do not file anything*.
///
/// **Kept short because it sets the width budget for every other row.**
/// `choices` subtracts this string's length from the room each label gets, and
/// the selector's own width test refused a longer wording outright: at 40
/// columns `done · write nothing and stop looking` drew 43.
const DONE: (&str, &str) = ("done", "write nothing");

/// `armada untried` — the roster, and what this machine has done with each.
///
/// **`untried` leads the listing**, because the question is *what have I not got
/// to yet*. `--all` is what asks for the whole roster including the verbs you
/// use every day; without it the listing is the answer to the question rather
/// than the data behind it.
///
/// `interactive` is decided at the entrypoint from whether a person is there and
/// whether they asked for the envelope (`ARCHITECTURE.md` §1.4), exactly as
/// `armada failures` decides it.
pub fn ls<C: Clock>(
    now: &C,
    place: &Where,
    all: bool,
    ask: &mut dyn Ask,
    interactive: bool,
    look: Look,
) -> Result<Output, ArmadaError> {
    if interactive {
        wander(now, place, all, ask, look)?;
    }
    Ok(listing(rows(now, place, all)))
}

/// Every row this lens shows.
fn rows<C: Clock>(now: &C, place: &Where, all: bool) -> Vec<Row> {
    let seen = armada_manifest::untried::read(&armada_manifest::untried::path(&place.armada_home));
    armada_core::untried::against(&crate::args::every_verb(), &seen, now.wall_ms())
        .into_iter()
        .filter(|row| all || row.count == 0)
        .collect()
}

/// Picking a verb writes a task for it — the one action there is, and it is the
/// crossing into the store that already has ids.
///
/// **Nothing is written until a row is picked.** The ask is explicit about it:
/// *do not auto-create tasks he did not ask for.*
fn wander<C: Clock>(
    now: &C,
    place: &Where,
    all: bool,
    ask: &mut dyn Ask,
    look: Look,
) -> Result<(), ArmadaError> {
    loop {
        let rows = rows(now, place, all);
        if rows.is_empty() {
            return Ok(());
        }
        let mut options: Vec<Choice> = choices(&rows, look);
        // `done` is the last option and the default, so `esc` and a stream that
        // ended both leave rather than acting on the way out.
        options.push(Choice::new(DONE.0, DONE.1));
        let done = options.len();
        // # The prompt names what picking does
        //
        // Reported by the owner: *"When I was in the untried view, I was hitting
        // enter on the item thinking that it would just run it, but instead it
        // seems like it created reports for someone to try it. It didn't tell me
        // that hitting enter was doing that. I thought it was just broken."*
        //
        // The prompt was *"What have you not tried?"* — a question about status,
        // asked by a screen whose whole answer is already on it. So enter looked
        // like the only thing enter usually means, which is *do this*, and what
        // it did instead was file a task and print a confirmation for something
        // the reader had not asked for. A person who reads that as broken is
        // reading it correctly: the screen said nothing about writing anything.
        //
        // Naming the verb in the question is the whole fix. It is the same rule
        // `docs/reserved/027` states for a raised item — the thing a keystroke
        // does belongs where the keystroke is offered, not in the confirmation
        // afterwards.
        let picked = ask.choose("Write a task to try which one?", &options, done);
        if picked >= done || picked == 0 {
            return Ok(());
        }
        let verb = rows[picked - 1].verb.clone();
        let output = crate::verbs::tasks::capture_text(
            now,
            place,
            &format!("try `armada {verb}` and report anything wrong with it"),
            &["task".to_string()],
            &place.cwd.clone(),
            None,
        )?;
        crate::verbs::guild::report(ask, look, &output);
    }
}

/// One row per verb, in the listing's own shape.
///
/// **`VERB · WHEN`, and `never` is a word rather than a blank.** An empty cell
/// reads as a rendering bug; the one fact this listing exists to carry has to be
/// spelled out.
fn choices(rows: &[Row], look: Look) -> Vec<Choice> {
    let widest = rows.iter().map(|row| row.verb.len()).max().unwrap_or(0);
    // The selector draws its own indent, cursor, number and the gap before the
    // aside; the label is truncated rather than wrapped, the rule every listing
    // in Armada follows.
    let room = look
        .terminal
        .usable_width()
        .saturating_sub(4 + rows.len().to_string().len() + 1 + 2 + DONE.1.len());
    rows.iter()
        .map(|row| {
            Choice::new(
                &crate::render::term::truncate(&format!("armada {:<widest$}", row.verb), room),
                &when(row),
            )
        })
        .collect()
}

/// How long ago this verb last ran, or that it never has.
pub(crate) fn when(row: &Row) -> String {
    match row.age_s {
        None => "never".to_string(),
        Some(age) => match row.ok {
            true => crate::render::format::elapsed(age * 1_000),
            // **A verb whose last run failed is not a tried verb**, and the cell
            // says so where a reader is already looking.
            false => format!("{}, failed", crate::render::format::elapsed(age * 1_000)),
        },
    }
}

/// The envelope the verb answers in.
fn listing(results: Vec<Row>) -> Output {
    let untried = results.iter().filter(|row| row.count == 0).count();
    Output::Untried(Box::new(Envelope::ok(
        "untried",
        None,
        Status::Ok,
        UntriedData {
            tried: results.len() - untried,
            untried,
            results,
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(verb: &str, age_s: Option<u64>, ok: bool) -> Row {
        Row {
            verb: verb.to_string(),
            count: u64::from(age_s.is_some()),
            ok,
            age_s,
        }
    }

    /// **`never` is a word.** An empty cell in the one column this listing
    /// exists for reads as a rendering bug rather than as the answer.
    #[test]
    fn a_verb_never_run_says_so_and_a_failed_one_says_that_too() {
        assert_eq!(when(&row("doctor", None, false)), "never");
        assert_eq!(when(&row("doctor", Some(540), true)), "9m");
        assert_eq!(when(&row("doctor", Some(540), false)), "9m, failed");
    }

    /// **`untried` counts the rows with nothing behind them**, and it is in the
    /// envelope rather than left to a caller to derive.
    #[test]
    fn the_summary_counts_what_was_never_run() {
        let Output::Untried(envelope) = listing(vec![
            row("doctor", None, false),
            row("failures", Some(1), true),
        ]) else {
            panic!("a listing answers as one");
        };
        assert_eq!(envelope.data.untried, 1);
        assert_eq!(envelope.data.tried, 1);
        assert_eq!(envelope.status, Status::Ok);
    }

    /// **A navigable row never wraps**, which is the one thing every listing in
    /// Armada refuses to do: the selector pads every label to the widest and
    /// then draws its own furniture around it.
    #[test]
    fn a_row_and_everything_the_selector_draws_around_it_fits_the_terminal() {
        let rows: Vec<Row> = (0..12)
            .map(|i| row(&format!("manifest a-very-long-verb-name-{i}"), None, false))
            .collect();
        for width in [
            crate::render::term::Terminal::MIN_WIDTH,
            80,
            crate::render::term::Terminal::FALLBACK_WIDTH,
        ] {
            let look = Look {
                style: crate::render::style::Style::plain(),
                terminal: crate::render::term::Terminal::at(width),
            };
            let mut options = choices(&rows, look);
            options.push(Choice::new(DONE.0, DONE.1));
            let widest = options
                .iter()
                .map(|option| option.label.chars().count())
                .max()
                .unwrap_or(0);
            let digits = options.len().to_string().len();
            for option in &options {
                let drawn = 4 + digits + 1 + widest + 2 + option.aside.chars().count();
                assert!(
                    drawn <= look.terminal.usable_width(),
                    "at {width} columns a row draws {drawn}: {}",
                    option.label
                );
            }
        }
    }
}
