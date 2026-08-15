//! `armada failures` — what Armada broke on, and how to put a Job on one.
//!
//! Four things, and only the first is new work. The design is
//! [`armada_core::failure`]:
//!
//! | Verb | What it is |
//! |---|---|
//! | `armada failures` | the log, folded — one row per distinct failure, with a count |
//! | `armada failures show <id>` | one entry whole, and the prompt a Job would get |
//! | `armada failures fix <id>` | `armada fleet spawn`, with the failure as the task |
//! | `armada failures clear <id>` | discarded, because a log you cannot clear stops being read |
//!
//! **`fix` invents no mechanism.** It is a `fleet spawn` on the `bug` workflow
//! (PLAN.md §14.6) whose task is the recorded failure, and the only thing this
//! module adds is the line that links the two — the same link
//! `docs/reserved/002-tasks.md` wants between a task and the Job that does it.
//!
//! **The workflow is named rather than classified**, and that is a decision, not
//! a shortcut. Classification costs a model call, and every entry here is by
//! construction a failure of Armada's — `bug` is the answer before the question
//! is asked. It also means this path spends no tokens, which is what makes it
//! testable.

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{Envelope, FailureData, FailuresData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::failure::{self, Entry, Line, State};

use crate::ask::{Ask, Choice};
use crate::render::progress::Progress;
use crate::verbs::Output;

pub use crate::verbs::fleet::Where;
pub use crate::verbs::guild::Look;

/// `armada failures` — the log, folded, **and at a terminal a way through it**.
///
/// **Cleared entries are hidden unless asked for**, which is the same lens
/// `armada fleet ls --all` applies to finished Jobs: the default answers "what
/// is still mine to deal with", and the flag answers "what has this machine
/// ever done".
///
/// `interactive` is decided at the entrypoint from whether a person is there
/// and whether they asked for the envelope (`ARCHITECTURE.md` §1.4), never
/// sniffed here. There is deliberately **no flag that opts into the
/// interaction** — a terminal is the flag, exactly as it is for `armada guild
/// ls`, and an interactive-only verb would be a bug rather than a feature
/// (PLAN.md §3.1.1).
///
/// **The listing is read again after the navigating rather than before it**, so
/// a session that put a Job on two entries and discarded a third reports what
/// the log says now instead of what it said when it opened.
#[allow(clippy::too_many_arguments)]
pub fn ls<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    all: bool,
    ask: &mut dyn Ask,
    interactive: bool,
    look: Look,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    if interactive {
        wander(run, now, place, all, ask, look, progress)?;
    }
    Ok(listing("failures", shown(read(now, place)?, all)))
}

/// What the last option says, and what it says about itself.
///
/// **Named because [`rows`] has to reserve room for it.** The selector pads
/// every label to the widest, so a detail that used the whole line would push
/// this row's aside off the end — which is a wrapped row, which is the one
/// thing every listing in Armada refuses to be.
const DONE: (&str, &str) = ("done", "stop looking");

/// Columns a navigable row spends on something other than the detail: the
/// selector's indent, cursor, option number and their spaces (6), the gap
/// before the aside (2), the two spaces after the status and the two after the
/// id (4), and the widest aside any row carries — [`DONE`]'s, which is longer
/// than any age.
const ROW_FURNITURE: usize = 6 + 2 + 4 + DONE.1.len();

/// The entries this lens shows.
fn shown(entries: Vec<Entry>, all: bool) -> Vec<Entry> {
    entries
        .into_iter()
        .filter(|entry| all || entry.state != State::Cleared)
        .collect()
}

/// Navigating the listing: pick a failure, then pick what to do about it, until
/// you are done.
///
/// **This is the whole point of the verb being interactive**, in the words it
/// was asked for in: *"so that I can navigate the list and quickly dispatch a
/// job rather than having to remember the ID and copy it and then run fix with
/// the ID."* The selection carries the id into [`fix`], so there is nothing to
/// retype and nothing to mistype.
///
/// **Two selections rather than a row of verbs on every line**, which is
/// `armada guild ls`'s shape and its reasoning: a reader opens this to find out
/// what has broken and only then decides what to do about one of them, and a
/// list carrying three actions per row is a list nobody can scan.
///
/// **It reads the log again on every turn.** A Job put on an entry changes what
/// that row says about itself, and a session holding the listing it opened with
/// would offer `fix` on something already being fixed.
fn wander<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    all: bool,
    ask: &mut dyn Ask,
    look: Look,
    progress: &mut dyn Progress,
) -> Result<(), ArmadaError> {
    loop {
        let entries = shown(read(now, place)?, all);
        if entries.is_empty() {
            return Ok(());
        }
        let mut options: Vec<Choice> = rows(&entries, look);
        // **`done` is the last option and it is the default**, so `esc` and a
        // stream that ended both leave rather than acting on the way out.
        options.push(Choice::new(DONE.0, DONE.1));
        let done = options.len();
        let picked = ask.choose("What has Armada broken on?", &options, done);
        if picked >= done || picked == 0 {
            return Ok(());
        }
        let entry = entries[picked - 1].clone();
        act(run, now, place, ask, look, progress, &entry)?;
    }
}

/// One row per entry, in the listing's own shape.
///
/// **`STATUS · ID · DETAIL · TIME`, the same four cells the table draws and in
/// the same order**, because the person navigating and the person reading a
/// pipe are looking at one listing. The status leads and is always a word — no
/// tick, no cross — for the reason `render/palette.rs` gives: a row told apart
/// by a glyph or a colour is a row a monochrome terminal cannot tell apart at
/// all. The detail comes from [`crate::render::failure_detail`], so the two
/// audiences cannot drift.
///
/// **The status is padded and the detail is truncated**, both so the ids line
/// up in a column: a column of ids that starts somewhere different on every row
/// is a column nobody can scan, and scanning is what this list is for.
fn rows(entries: &[Entry], look: Look) -> Vec<Choice> {
    let widest = entries
        .iter()
        .map(|entry| entry.state.word().len())
        .max()
        .unwrap_or(0);
    // What is left for the detail once the status, the id, the cursor, the
    // option number and the gap before the aside have had theirs. **Truncated
    // rather than wrapped**, the same rule `render/term.rs` states for every
    // table: a wrapped row loses the column that made the listing worth having.
    let id = entries
        .iter()
        .map(|entry| entry.id.len())
        .max()
        .unwrap_or(0);
    let room = look
        .terminal
        .usable_width()
        .saturating_sub(widest + id + ROW_FURNITURE);
    entries
        .iter()
        .map(|entry| {
            Choice::new(
                &format!(
                    "{:<widest$}  {}  {}",
                    entry.state.word(),
                    entry.id,
                    // **No floor under `room`.** A window too narrow for a
                    // sentence still has to draw a row a person can pick, and
                    // the status and the id are what they pick by — so the
                    // detail is what gives, down to nothing.
                    crate::render::term::truncate(&crate::render::failure_detail(entry), room)
                ),
                &crate::render::format::elapsed(entry.age_s * 1_000),
            )
        })
        .collect()
}

/// What to do about the one failure that was picked.
///
/// **Three things and a way out**, and they are the three the verb already has:
/// show it whole, put a Job on it, discard it. Nothing here is a fourth verb —
/// each option runs the same function `armada failures show|fix|clear` runs, so
/// there is one implementation of each and one place a field can be added to
/// it.
///
/// **Discarding asks nothing first**, which is where this parts company with
/// `armada guild ls`'s delete. That one removes a file from a guild that syncs
/// to every machine; this appends a line to an append-only log, keeps the id
/// and the entry, and reopens on the next recurrence. There is nothing to
/// confirm because there is nothing to lose.
fn act<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    ask: &mut dyn Ask,
    look: Look,
    progress: &mut dyn Progress,
    entry: &Entry,
) -> Result<(), ArmadaError> {
    let options = vec![
        Choice::new("show", "the failure, whole"),
        Choice::new("fix", "spawn a Job on it"),
        Choice::new("discard", "clear it; a recurrence brings it back"),
        Choice::new("back", "leave it alone"),
    ];
    let back = options.len();
    let chosen = ask.choose(&format!("{} — {}", entry.id, entry.message), &options, back);
    // **An answer outside the list leaves rather than acting**, the same rule
    // `guild ls` follows: the first option must never be the one an
    // out-of-range answer lands on, and here the first option would spawn.
    let Some(action) = chosen
        .checked_sub(1)
        .and_then(|at| options.get(at))
        .map(|choice| choice.label.clone())
    else {
        return Ok(());
    };

    let output = match action.as_str() {
        "show" => show(now, place, &entry.id)?,
        // **The id goes through from the selection**, which is the feature.
        // `dry_run` is false because a person who picked *fix* off a list asked
        // for the Job, and `--dry-run` is how a caller asks for the rehearsal.
        "fix" => fix(run, now, place, &entry.id, false, Some(&mut *ask), progress)?,
        "discard" => clear(now, place, Some(&entry.id), false)?,
        _ => return Ok(()),
    };
    crate::verbs::guild::report(ask, look, &output);
    Ok(())
}

/// `armada failures show <id>` — one entry, whole.
pub fn show<C: Clock>(now: &C, place: &Where, id: &str) -> Result<Output, ArmadaError> {
    let entry = resolve(&read(now, place)?, id)?;
    let task = failure::task(&entry);
    Ok(Output::Failure(Box::new(Envelope::ok(
        "failures show",
        None,
        Status::Ok,
        FailureData {
            results: vec![entry],
            task,
        },
    ))))
}

/// `armada failures clear` — discard one entry, or every one of them.
///
/// **The line is appended, and what it clears stays on disk.** A rewrite would
/// mean reading the whole file, dropping some rows and writing it back — three
/// chances to lose every other entry, in the file whose one property is that it
/// survives a crash.
pub fn clear<C: Clock>(
    now: &C,
    place: &Where,
    id: Option<&str>,
    all: bool,
) -> Result<Output, ArmadaError> {
    let entries = read(now, place)?;
    let cleared: Vec<Entry> = match id {
        Some(id) => vec![resolve(&entries, id)?],
        None => entries
            .into_iter()
            .filter(|entry| entry.state != State::Cleared)
            .collect(),
    };
    // Said rather than reported as an empty success: "there was nothing to
    // clear" and "the flag did nothing" read identically otherwise.
    if all && cleared.is_empty() {
        return Ok(listing("failures clear", Vec::new()));
    }

    let path = armada_manifest::failures::path(&place.armada_home);
    let at_ms = now.wall_ms();
    for entry in &cleared {
        if !armada_manifest::failures::append(
            &path,
            &Line::Cleared {
                id: entry.id.clone(),
                at_ms,
            },
        ) {
            return Err(unwritable(&path));
        }
    }
    Ok(listing(
        "failures clear",
        cleared
            .into_iter()
            .map(|mut entry| {
                entry.state = State::Cleared;
                entry
            })
            .collect(),
    ))
}

/// `armada failures fix <id>` — a Job on the `bug` workflow, with the recorded
/// failure as its task.
///
/// **The Job branches from where the failure happened**, not from where this was
/// typed. The entry carries that directory precisely so the fix starts in the
/// repository the bug is in, and `-C` is how `fleet spawn` already accepts one.
///
/// **The link is written after the spawn returns and never before.** A promotion
/// line for a Job that failed to start would put `FIXING` on a row nobody is
/// fixing, which is the one state worse than `OPEN`.
pub fn fix<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    id: &str,
    dry_run: bool,
    ask: Option<&mut dyn Ask>,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    let entry = resolve(&read(now, place)?, id)?;
    let spawn = crate::args::Spawn {
        json: false,
        task: failure::task(&entry),
        // **Named, not classified.** No model call, so this path spends nothing
        // and a test can take it.
        workflow: Some("bug".to_string()),
        name: None,
        budget: Vec::new(),
        at: Some(place.expand(&entry.cwd).display().to_string()),
        confidence: None,
        dry_run,
    };
    let output = crate::verbs::fleet::spawn(run, now, place, &spawn, ask, progress)?;

    if let (false, Output::Spawn(spawned)) = (dry_run, &output) {
        // **Best effort, exactly as recording is.** The Job exists; failing to
        // write down that it exists must not turn a spawn that worked into a
        // verb that reports failure.
        let _ = armada_manifest::failures::append(
            &armada_manifest::failures::path(&place.armada_home),
            &Line::Promoted {
                id: entry.id.clone(),
                at_ms: now.wall_ms(),
                job: spawned.data.name.clone(),
            },
        );
    }
    Ok(output)
}

/// The listing envelope both `failures` and `failures clear` answer in.
///
/// **`open` counts what is still yours**: not cleared, and with no Job on it. An
/// entry somebody has already put a Job on is not a thing waiting for you, and
/// counting it as one is how a number stops being read.
fn listing(verb: &str, results: Vec<Entry>) -> Output {
    let open = results
        .iter()
        .filter(|entry| entry.state == State::Open)
        .count();
    Output::Failures(Box::new(Envelope::ok(
        verb,
        None,
        Status::Ok,
        FailuresData { results, open },
    )))
}

/// The log, with every entry's age filled in against this clock.
fn read<C: Clock>(now: &C, place: &Where) -> Result<Vec<Entry>, ArmadaError> {
    let mut entries =
        armada_manifest::failures::read(&armada_manifest::failures::path(&place.armada_home))?;
    failure::age(&mut entries, now.wall_ms());
    Ok(entries)
}

/// The entry an id names — **by prefix, when the prefix names exactly one**.
///
/// An id is eight hex characters and the reader is retyping it off a table, so
/// four are usually enough and the row is right there to check against. Two
/// matches is refused rather than resolved: picking one for the caller would put
/// a Job on a bug they did not name.
fn resolve(entries: &[Entry], id: &str) -> Result<Entry, ArmadaError> {
    let matched: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.id == id || entry.id.starts_with(id))
        .collect();
    match matched.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: id.to_string(),
            message: format!("no recorded failure is called `{id}`"),
            next_action: Some("`armada failures` lists them".to_string()),
        }),
        many => Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: id.to_string(),
            message: format!(
                "`{id}` names {} recorded failures: {}",
                many.len(),
                many.iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            next_action: Some("give more of the id".to_string()),
        }),
    }
}

fn unwritable(path: &std::path::Path) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: "the failure log could not be written".to_string(),
        next_action: Some("check ~/.armada/ is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::error::ErrClass;

    fn entry(id: &str) -> Entry {
        Entry {
            id: id.to_string(),
            state: State::Open,
            class: ErrClass::ArmadaBug,
            r#where: "spawn".to_string(),
            message: "the worktree was not there".to_string(),
            next: None,
            argv: "armada bridge".to_string(),
            cwd: "~/code/api".to_string(),
            count: 1,
            first_at: "2026-08-14T09:00:00Z".to_string(),
            last_at: "2026-08-14T09:00:00Z".to_string(),
            last_ms: 1_000,
            age_s: 0,
            job: None,
        }
    }

    /// **A navigable row never wraps**, which is the one thing every listing in
    /// Armada refuses to do (`render/term.rs`): a wrapped row loses the column
    /// that made the listing worth having. The selector pads every label to the
    /// widest and then draws its own indent, cursor, number and aside, so this
    /// measures the whole line as it will be drawn — including [`DONE`]'s
    /// aside, which is longer than any age and is what once fell off the end.
    #[test]
    fn a_row_and_everything_the_selector_draws_around_it_fits_the_terminal() {
        let mut long = entry("a1b2c3d4");
        long.message = "the worktree was not there, and neither was the branch it \
                        was supposed to have been created on, nor the repository"
            .to_string();
        long.state = State::Cleared;
        let entries = [long, entry("ff001122")];

        for width in [
            crate::render::term::Terminal::MIN_WIDTH,
            80,
            120,
            crate::render::term::Terminal::FALLBACK_WIDTH,
        ] {
            let look = Look {
                style: crate::render::style::Style::plain(),
                terminal: crate::render::term::Terminal::at(width),
            };
            let mut options = rows(&entries, look);
            options.push(Choice::new(DONE.0, DONE.1));
            let widest = options
                .iter()
                .map(|option| option.label.chars().count())
                .max()
                .unwrap_or(0);
            for option in &options {
                // Indent, cursor, space, number, space, the padded label, two
                // spaces, the aside — the line `ask::select::row` builds.
                let drawn = 6 + widest + 2 + option.aside.chars().count();
                assert!(
                    drawn <= look.terminal.usable_width(),
                    "at {width} columns a row draws {drawn}: {}",
                    option.label
                );
            }
        }
    }

    /// The row is the table's row: status first and always a word, then the id,
    /// then the same detail sentence, then how long ago.
    #[test]
    fn a_navigable_row_is_the_listing_row_in_the_listings_order() {
        let mut fixing = entry("ff001122");
        fixing.state = State::Fixing;
        fixing.age_s = 540;
        let rows = rows(&[entry("a1b2c3d4"), fixing], Look::default());

        assert!(rows[0].label.starts_with("OPEN   "), "{:?}", rows[0].label);
        assert!(rows[1].label.starts_with("FIXING "), "{:?}", rows[1].label);
        assert!(rows[0].label.contains("a1b2c3d4"), "{:?}", rows[0].label);
        assert!(
            rows[0]
                .label
                .contains("armada_bug, the worktree was not there"),
            "{:?}",
            rows[0].label
        );
        assert_eq!(rows[1].aside, "9m");
        // No glyph carries meaning — the status is the whole of the signal.
        for row in &rows {
            assert!(!row.label.contains('✔') && !row.label.contains('✗'));
        }
    }

    #[test]
    fn a_prefix_that_names_one_entry_resolves_to_it() {
        let entries = [entry("a1b2c3d4"), entry("ff001122")];
        assert_eq!(resolve(&entries, "a1b2").unwrap().id, "a1b2c3d4");
        assert_eq!(resolve(&entries, "a1b2c3d4").unwrap().id, "a1b2c3d4");
    }

    /// **Two matches is refused rather than resolved.** Picking one would put a
    /// Job on a bug nobody named.
    #[test]
    fn a_prefix_that_names_two_entries_is_refused_with_both() {
        let entries = [entry("a1b2c3d4"), entry("a1b2ffff")];
        let error = resolve(&entries, "a1b2").unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(error.message.contains("a1b2c3d4"), "{}", error.message);
        assert!(error.message.contains("a1b2ffff"), "{}", error.message);
    }

    #[test]
    fn an_id_nobody_recorded_says_so_and_names_the_verb_that_lists_them() {
        let error = resolve(&[entry("a1b2c3d4")], "nope").unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(error.next_action.unwrap().contains("armada failures"));
    }

    /// **`open` is what is still yours.** An entry with a Job on it is being
    /// dealt with, and counting it as waiting is how a number stops being read.
    #[test]
    fn the_open_count_excludes_what_already_has_a_job() {
        let mut fixing = entry("ff001122");
        fixing.state = State::Fixing;
        fixing.job = Some("fix-ff001122".to_string());
        let Output::Failures(envelope) = listing("failures", vec![entry("a1b2c3d4"), fixing])
        else {
            panic!("a listing answers as one");
        };
        assert_eq!(envelope.data.results.len(), 2);
        assert_eq!(envelope.data.open, 1);
        assert_eq!(envelope.status, Status::Ok);
    }
}
