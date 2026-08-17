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
use armada_core::failure::{self, Entry, Line, Listing, State};

use crate::ask::{Ask, Choice};
use crate::render::progress::Progress;
use crate::verbs::Output;

pub use crate::verbs::fleet::Where;
pub use crate::verbs::guild::Look;

/// **Which half of one store a verb is looking at.**
///
/// `armada failures` and `armada tasks` read the same file, resolve ids out of
/// the same space, promote through the same `fleet spawn` and discard through
/// the same appended line. What they do not share is the question they ask —
/// *what is broken* against *what did I say I would do* — and
/// `docs/reserved/002-tasks.md` left that open deliberately, because *"a single
/// flat list of everything may be unreadable"*.
///
/// **So the answer is one store, one id space, two lenses.** A flat list would
/// mix a `bad_config` from Tuesday with *"rename the port allocator"* and make
/// both harder to find; two stores would mean two `show`s, two `fix`es and two
/// ids, which is the thing `docs/reserved/001-raised-items-need-identity.md`
/// exists to forbid. The split is [`Origin::is_fault`], one function, so a
/// fourth origin cannot arrive without deciding which listing it belongs in.
///
/// **`show` takes any of them.** An id is an id: `armada failures show <a
/// task>` answers rather than refusing, because a reader who has the id in hand
/// has already told you which row they mean. Since
/// `docs/reserved/001-raised-items-need-identity.md` that includes an inbox
/// entry's id, which is the fourth kind of item and the only one that was ever
/// outside this id space.
///
/// **There is no `Lens::Raised`, and its absence is the decision.** A raised
/// item is in [`read`]'s output — so it resolves, and `show` answers about it —
/// and it is in neither listing, because it already has one:
/// `armada fleet inbox`. Drawing the same row in two tables is how two tables
/// start disagreeing about it, and `armada fleet inbox` is where the answering
/// happens. One id space is not the same claim as one listing, and `001` asks
/// for the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lens {
    /// `armada failures` — what went wrong, observed or reported.
    Failures,
    /// `armada tasks` — what you wrote down.
    Tasks,
}

impl Lens {
    /// Whether this entry is one of the rows this listing is about.
    ///
    /// **Matched on [`Listing`] rather than on a `bool`.** This read
    /// `entry.origin.is_fault() == matches!(self, Lens::Failures)`, which
    /// silently meant *anything that is not a fault is a task* — so the moment
    /// a fourth origin existed, a raised item appeared under `armada tasks`
    /// offering to `start` a Job for a row whose Job is already running.
    const fn shows(self, entry: &Entry) -> bool {
        match (entry.origin.listing(), self) {
            (Listing::Faults, Lens::Failures) | (Listing::Written, Lens::Tasks) => true,
            (Listing::Faults | Listing::Written | Listing::Raised, _) => false,
        }
    }

    /// The verb the envelope names, and the prefix every sub-verb's name takes.
    const fn verb(self) -> &'static str {
        match self {
            Lens::Failures => "failures",
            Lens::Tasks => "tasks",
        }
    }

    /// What the listing asks when a person is navigating it.
    const fn question(self) -> &'static str {
        match self {
            Lens::Failures => "What has Armada broken on?",
            Lens::Tasks => "What did you write down?",
        }
    }

    /// What promoting a row is called here.
    ///
    /// **`fix` and `start` are not synonyms** and the vocabulary rule
    /// (`docs/glossary.md`) is why they are both here rather than one word
    /// doing both: you fix something that is broken and you start something
    /// that was never begun, and a task offered `fix` would read as though
    /// Armada thought the thought was wrong.
    const fn promote(self) -> (&'static str, &'static str) {
        match self {
            Lens::Failures => ("fix", "spawn a Job on it"),
            Lens::Tasks => ("start", "spawn a Job to do it"),
        }
    }
}

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
///
/// **`scope`, when given, is a second filter on top of `lens`.** Only
/// [`crate::verbs::tasks::ls`] ever passes one — `armada failures` reads the
/// whole machine, because what Armada broke on does not stop mattering when
/// you change directory. `None` always, for every other caller.
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
    lens: Lens,
    scope: Option<&str>,
) -> Result<Output, ArmadaError> {
    if interactive {
        wander(run, now, place, all, ask, look, progress, lens, scope)?;
    }
    Ok(listing(
        lens.verb(),
        shown(read(now, place)?, all, lens, scope),
    ))
}

/// What the last option says, and what it says about itself.
///
/// **Named because [`rows`] has to reserve room for it.** The selector pads
/// every label to the widest, so a detail that used the whole line would push
/// this row's aside off the end — which is a wrapped row, which is the one
/// thing every listing in Armada refuses to be.
const DONE: (&str, &str) = ("done", "stop looking");

/// Columns a navigable row spends on something other than the detail: the
/// selector's indent, cursor, the option number and their spaces, the gap
/// before the aside (2), the two spaces after the status and the two after the
/// id (4), and the widest aside any row carries — [`DONE`]'s, which is longer
/// than any age.
///
/// **The option number's width is not fixed at one digit.** `total_options`
/// is `entries.len() + 1` — [`DONE`] is always appended after [`rows`]
/// returns — because the tenth option and every one after it number two
/// digits, and a row's budget that assumed one silently overran by the width
/// of the second: found by capturing the render at a real terminal once the
/// log first held ten entries, where `stop looking` came back `stop lookin`.
fn row_furniture(total_options: usize) -> usize {
    let digits = total_options.to_string().len();
    4 + digits + 1 + 2 + 4 + DONE.1.len()
}

/// The entries this lens shows.
///
/// **Three filters, and they are different questions.** [`Lens`] decides which
/// half of the store this listing is about and never changes; `all` decides
/// whether a row you already dealt with is still worth drawing; `scope`, when
/// given, decides whether a row about a different repository is.
fn shown(entries: Vec<Entry>, all: bool, lens: Lens, scope: Option<&str>) -> Vec<Entry> {
    entries
        .into_iter()
        .filter(|entry| {
            lens.shows(entry)
                && (all || entry.state != State::Cleared)
                && scope.is_none_or(|project| entry.cwd == project)
        })
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
#[allow(clippy::too_many_arguments)]
fn wander<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    all: bool,
    ask: &mut dyn Ask,
    look: Look,
    progress: &mut dyn Progress,
    lens: Lens,
    scope: Option<&str>,
) -> Result<(), ArmadaError> {
    loop {
        let entries = shown(read(now, place)?, all, lens, scope);
        if entries.is_empty() {
            return Ok(());
        }
        let mut options: Vec<Choice> = rows(&entries, look);
        // **`done` is the last option and it is the default**, so `esc` and a
        // stream that ended both leave rather than acting on the way out.
        options.push(Choice::new(DONE.0, DONE.1));
        let done = options.len();
        let picked = ask.choose(lens.question(), &options, done);
        if picked >= done || picked == 0 {
            return Ok(());
        }
        let entry = entries[picked - 1].clone();
        act(run, now, place, ask, look, progress, &entry, lens)?;
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
        .saturating_sub(widest + id + row_furniture(entries.len() + 1));
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
#[allow(clippy::too_many_arguments)]
fn act<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    ask: &mut dyn Ask,
    look: Look,
    progress: &mut dyn Progress,
    entry: &Entry,
    lens: Lens,
) -> Result<(), ArmadaError> {
    let (promote, promote_aside) = lens.promote();
    let options = vec![
        Choice::new(
            "show",
            match lens {
                Lens::Failures => "the failure, whole",
                Lens::Tasks => "the task, whole",
            },
        ),
        Choice::new(promote, promote_aside),
        Choice::new(
            "discard",
            match lens {
                Lens::Failures => "clear it; a recurrence brings it back",
                Lens::Tasks => "clear it; it is done, or it is not happening",
            },
        ),
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
        // No workflow is named: a failure's is decided by its origin and a
        // task's is the one thing about it nobody has decided yet, so the
        // person who is already sitting here is the one to ask.
        picked if picked == promote => fix(
            run,
            now,
            place,
            &entry.id,
            false,
            None,
            Some(&mut *ask),
            progress,
        )?,
        "discard" => clear(now, place, Some(&entry.id), false, lens)?,
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
/// **`--all` clears this listing and never the other one.** The store is
/// shared, so a `tasks clear --all` that swept the failure log too would
/// discard rows the person never had on screen — the one mistake an
/// append-only log cannot make quiet.
pub fn clear<C: Clock>(
    now: &C,
    place: &Where,
    id: Option<&str>,
    all: bool,
    lens: Lens,
) -> Result<Output, ArmadaError> {
    let entries = read(now, place)?;
    let cleared: Vec<Entry> = match id {
        // **By id, across both lenses.** A person holding an id has already
        // said which row they mean, and refusing it because they typed the
        // other verb would be Armada knowing better than the id it printed.
        Some(id) => {
            let entry = resolve(&entries, id)?;
            already_has_a_job(&entry, "cleared")?;
            vec![entry]
        }
        None => entries
            .into_iter()
            .filter(|entry| lens.shows(entry) && entry.state != State::Cleared)
            .collect(),
    };
    // Said rather than reported as an empty success: "there was nothing to
    // clear" and "the flag did nothing" read identically otherwise.
    if all && cleared.is_empty() {
        return Ok(listing(&format!("{} clear", lens.verb()), Vec::new()));
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
        &format!("{} clear", lens.verb()),
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
///
/// # `armada tasks start` is this function, and the workflow is the difference
///
/// A failure and a report are both defects, so `bug` is the answer before the
/// question is asked and naming it costs nothing. **A task's workflow is the
/// one thing about it nobody has decided** — *"look into the flaky golden"* is
/// a `design`, *"rename the port allocator"* is a `feature` — so an unnamed
/// workflow falls through to the classification `armada fleet spawn` already
/// does, including its confirm-a-guess prompt (PLAN.md §14.2). `--workflow`
/// skips that, which is what a pipe and a test both pass.
///
/// **Capture is still free.** The model call, if there is one, happens when a
/// person starts the Job and not when they wrote the sentence down — which is
/// `docs/reserved/002-tasks.md`'s whole distinction between the two.
#[allow(clippy::too_many_arguments)]
pub fn fix<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    id: &str,
    dry_run: bool,
    workflow: Option<&str>,
    ask: Option<&mut dyn Ask>,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    let entry = resolve(&read(now, place)?, id)?;
    already_has_a_job(&entry, "promoted")?;
    let spawn = crate::args::Spawn {
        json: false,
        task: failure::task(&entry),
        // **Named, not classified, whenever the origin already answers it.** A
        // defect is a `bug`, so that path spends nothing and a test can take
        // it.
        workflow: workflow
            .map(str::to_string)
            .or_else(|| entry.origin.is_fault().then(|| "bug".to_string())),
        name: None,
        budget: Vec::new(),
        at: Some(place.expand(&entry.cwd).display().to_string()),
        confidence: None,
        dry_run,
        // A failure's task names no `${task.…}` placeholder.
        set: std::collections::BTreeMap::new(),
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
/// The log, with every entry's age filled in against this clock — **and the
/// inbox folded in beside it, so there is one id space.**
///
/// # Why the inbox is read here
///
/// `docs/reserved/001-raised-items-need-identity.md`: *every item Helm surfaces
/// is an inbox entry with an id.* Three origins were already one id space; the
/// inbox was the fourth kind of item and the only one outside it, so
/// `armada failures show <an inbox id>` answered *no recorded failure is called
/// that* about an id printed on the screen the reader had just read.
///
/// **The unification is in the reader, not the store.** The inbox stays in its
/// own file because Helm's Stop hook and its monitor are pointed at that exact
/// path (`armada_core::helm`); breaking the delivery mechanism to tidy the
/// storage would be the wrong half. What one reader gives is what the complaint
/// actually asked for: one `resolve`, one refusal when a prefix is ambiguous
/// across every item on the machine, and one `show`.
///
/// # No migration, and that is the point
///
/// `armada_fleet::inbox::read` is called rather than `fleet`'s `entries`, which
/// migrates legacy rows and needs the Job index to do it. This is a read on the
/// path of `armada failures`, `armada tasks` and every `show` — it must not pull
/// in the Job store, and a legacy row projects as a closed item that resolves by
/// id and appears in no listing, which is the correct treatment of a row nobody
/// can act on (`docs/reserved/005-inbox-label-not-identity.md`).
///
/// **An unreadable inbox does not fail this read.** `armada failures` answers
/// about the failure log; a machine whose inbox is corrupt should still be able
/// to ask what broke, and the ids it cannot offer are the ids of items it could
/// not have listed anyway.
fn read<C: Clock>(now: &C, place: &Where) -> Result<Vec<Entry>, ArmadaError> {
    let mut entries =
        armada_manifest::failures::read(&armada_manifest::failures::path(&place.armada_home))?;
    entries.extend(
        armada_fleet::inbox::read(&place.inbox())
            .unwrap_or_default()
            .iter()
            .map(armada_fleet::inbox::Entry::as_entry),
    );
    failure::age(&mut entries, now.wall_ms());
    no_longer_being_fixed(&mut entries, place);
    Ok(entries)
}

/// **`FIXING` means a Job is on it, so a Job that ended un-means it.**
///
/// The state is written when a failure is promoted (`failure.rs`'s
/// `Line::Promoted`) and nothing ever wrote it back. One entry sat at `FIXING`
/// for thirty hours on this machine while the Job named on it had aborted
/// twenty-nine hours earlier — the listing claimed somebody was working on a
/// bug that nobody was, which is worse than showing it open, because a reader
/// who trusts the word leaves it alone.
///
/// **Derived here rather than written by whoever ends the Job.** Armada is not a
/// daemon and nothing runs between commands (`ARCHITECTURE.md` §1.9), so a state
/// that has to be *corrected* by a later writer is a state that goes stale the
/// first time that writer is a `SIGKILL`. Reading it from the Job store at the
/// moment somebody looks cannot go stale at all.
///
/// **It could not live in the fold.** `failure::fold` is in `armada-core`,
/// below `armada-fleet`, and nothing points upward — the fold cannot look up a
/// Job. This is the lowest place that can see both stores.
///
/// The entry keeps its `job`, so `show` still names the Job that tried and the
/// reader can go read what it did. What changes is only the claim that it is
/// still trying.
fn no_longer_being_fixed(entries: &mut [Entry], place: &Where) {
    let fixing: Vec<String> = entries
        .iter()
        .filter(|entry| entry.state == State::Fixing)
        .filter_map(|entry| entry.job.clone())
        .collect();
    if fixing.is_empty() {
        return;
    }
    // One read of the Job store for the whole listing, not one per row.
    let store = armada_fleet::jobs::Store::at(&place.armada_home);
    let Ok(jobs) = store.all() else {
        // **A Job store that will not read leaves every word alone.** `failures`
        // reports rather than judges, and demoting a row because the fleet was
        // unreadable would be inventing a fact from a missing one.
        return;
    };
    let over: Vec<&str> = fixing
        .iter()
        .filter(|handle| {
            // **A handle that names no Job leaves the word alone: absence is not
            // evidence.** The first version of this treated "no such Job" as
            // over too, on the reasoning that a reaped Job is gone for good —
            // and a test caught it demoting a *raised* inbox entry, which names
            // the Job that asked the question and whose Job may legitimately not
            // be in this store. Reaping, a record written by another home and an
            // entry that outlived its Job all arrive as the same absence, so the
            // only fact worth acting on is a Job that is here and is finished.
            let mut named = jobs
                .iter()
                .filter(|job| &job.name == *handle || &job.uuid == *handle)
                .peekable();
            named.peek().is_some() && named.all(|job| job.state.is_over())
        })
        .map(String::as_str)
        .collect();
    for entry in entries.iter_mut() {
        if entry.state == State::Fixing
            && entry.job.as_deref().is_some_and(|job| over.contains(&job))
        {
            entry.state = State::Open;
        }
    }
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

/// **Refuse a raised item where a Job would be spawned or a line appended.**
///
/// The id space is one (`docs/reserved/001-raised-items-need-identity.md`), so
/// every verb here can now be handed an inbox entry's id — and two of them have
/// nothing honest to do with it:
///
/// - **`fix` / `start` would spawn a second Job** for a row whose Job already
///   exists and is stopped in front of the question. Two Drones on one question
///   is two answers to give.
/// - **`clear` would append to `failures.jsonl`** a line about an id that file
///   has never held, which is a row that reads as cleared and is not. An entry
///   stops being open by being answered or by its Job ending
///   (`docs/reserved/005-inbox-label-not-identity.md`), and both of those are
///   written to the inbox by the verb that owns it.
///
/// **Refused rather than silently forwarded**, because `armada failures clear`
/// and `armada fleet answer` are not the same act: one discards a row and the
/// other unblocks an agent. Doing the second when the first was asked for is
/// worse than saying no.
///
/// The message names what would have happened, and `next_action` is the verb
/// that does work — which is the whole of what a person with the id in hand
/// needs.
fn already_has_a_job(entry: &Entry, verb: &str) -> Result<(), ArmadaError> {
    match entry.origin.listing() {
        Listing::Faults | Listing::Written => Ok(()),
        Listing::Raised => Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: entry.id.clone(),
            message: format!(
                "`{}` is a question {} asked, not something to be {verb}",
                entry.id,
                entry.job.as_deref().unwrap_or("a Job")
            ),
            next_action: Some(format!("`armada fleet answer {} \"…\"`", entry.id)),
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
    use armada_core::fleet::JobState;

    /// A Job in one state, enough of one for the Job store to save and read.
    fn a_job(name: &str, state: armada_core::fleet::JobState) -> armada_core::fleet::job::Job {
        armada_core::fleet::job::Job {
            budget_set: Vec::new(),
            uuid: format!("uuid-of-{name}"),
            name: name.to_string(),
            workflow: "bug".to_string(),
            confidence: None,
            repo: "api".to_string(),
            repo_root: "~/code/api".to_string(),
            worktree: format!("~/.armada/workspaces/api/{name}"),
            branch: format!("armada/{name}"),
            port_block: None,
            budget: armada_core::fleet::workflow::DEFAULT_BUDGET,
            state,
            step: "reproduce".to_string(),
            verdict: None,
            drone: None,
            created_at: "2026-08-16T01:00:00Z".to_string(),
            created_ms: 1_000,
            spend: Default::default(),
            task: "fix the thing".to_string(),
            progress: Vec::new(),
            attempts: Default::default(),
            waited_ms: 0,
            waiting_from_ms: None,
            transitions: Vec::new(),
            pending: None,
            facts: Default::default(),
            kin: Default::default(),
            ticked_turns: 0,
            doing: None,
            daemon_acts: Vec::new(),
            main_moved_at: None,
        }
    }

    fn a_place(home: &std::path::Path) -> Where {
        Where {
            home: home.to_path_buf(),
            armada_home: home.join(".armada"),
            cwd: home.join("code/api"),
            exe: std::path::PathBuf::from("/usr/local/bin/armada"),
            boot_id: "boot".to_string(),
        }
    }

    /// **`FIXING` outlived the Job it was named after by twenty-nine hours.**
    ///
    /// The state is written on promotion and nothing wrote it back, so one entry
    /// on this machine claimed a Job was working on it while that Job had long
    /// since aborted. A reader who trusts the word leaves the row alone, which
    /// makes a stale `FIXING` worse than an honest `OPEN`.
    #[test]
    fn a_failure_whose_job_ended_is_open_again() {
        let home = tempfile::tempdir().unwrap();
        let place = a_place(home.path());
        let store = armada_fleet::jobs::Store::at(&place.armada_home);
        store
            .save(&a_job("still-going", JobState::Running))
            .unwrap();
        store.save(&a_job("gave-up", JobState::Aborted)).unwrap();
        store.save(&a_job("finished", JobState::Done)).unwrap();

        let mut entries = ["live", "aborted", "done", "not-in-the-store"]
            .iter()
            .zip(["still-going", "gave-up", "finished", "no-such-job"])
            .map(|(id, job)| {
                let mut entry = entry(id);
                entry.state = State::Fixing;
                entry.job = Some(job.to_string());
                entry
            })
            .collect::<Vec<_>>();
        no_longer_being_fixed(&mut entries, &place);

        assert_eq!(entries[0].state, State::Fixing, "a live Job is still on it");
        assert_eq!(entries[1].state, State::Open, "the Job aborted");
        assert_eq!(entries[2].state, State::Open, "the Job finished");
        // **A handle naming no Job keeps its word**: absence is not evidence.
        // Reaping, a record from another home and a raised entry that outlived
        // its Job all look identical from here.
        assert_eq!(entries[3].state, State::Fixing, "no such Job is not a fact");
        // The Job that tried is still named, so `show` can point at what it did.
        assert_eq!(entries[1].job.as_deref(), Some("gave-up"));
    }

    /// **An unreadable Job store leaves every word alone.** `failures` reports
    /// rather than judges, and demoting a row because the fleet could not be
    /// read would be inventing a fact out of a missing one.
    #[test]
    fn a_fleet_that_cannot_be_read_demotes_nothing() {
        let home = tempfile::tempdir().unwrap();
        // A file where the jobs directory should be: `all()` cannot read it.
        let place = a_place(home.path());
        std::fs::create_dir_all(&place.armada_home).unwrap();
        std::fs::write(place.armada_home.join("jobs"), "not a directory").unwrap();

        let mut entries = vec![entry("a1b2c3d4")];
        entries[0].state = State::Fixing;
        entries[0].job = Some("who-knows".to_string());
        no_longer_being_fixed(&mut entries, &place);

        assert_eq!(entries[0].state, State::Fixing);
    }

    fn entry(id: &str) -> Entry {
        Entry {
            id: id.to_string(),
            state: State::Open,
            origin: armada_core::failure::Origin::Observed,
            class: Some(ErrClass::ArmadaBug),
            r#where: "spawn".to_string(),
            message: "the worktree was not there".to_string(),
            next: None,
            argv: "armada bridge".to_string(),
            cwd: "~/code/api".to_string(),
            workspace: None,
            count: 1,
            first_at: "2026-08-14T09:00:00Z".to_string(),
            last_at: "2026-08-14T09:00:00Z".to_string(),
            last_ms: 1_000,
            age_s: 0,
            job: None,
            diagnostics: None,
        }
    }

    /// **A navigable row never wraps**, which is the one thing every listing in
    /// Armada refuses to do (`render/term.rs`): a wrapped row loses the column
    /// that made the listing worth having. The selector pads every label to the
    /// widest and then draws its own indent, cursor, number and aside, so this
    /// measures the whole line as it will be drawn — including [`DONE`]'s
    /// aside, which is longer than any age and is what once fell off the end.
    ///
    /// **Twelve entries, not two.** [`DONE`] is always the last option, so with
    /// ten or fewer entries it is also always a one-digit option — the case
    /// that hid this exact bug: `row_furniture` assumed one digit, the
    /// eleventh entry's list made [`DONE`] option thirteen, and `stop looking`
    /// came back `stop lookin` at a real terminal before this test ever saw a
    /// list that long.
    #[test]
    fn a_row_and_everything_the_selector_draws_around_it_fits_the_terminal() {
        let mut long = entry("a1b2c3d4");
        long.message = "the worktree was not there, and neither was the branch it \
                        was supposed to have been created on, nor the repository"
            .to_string();
        long.state = State::Cleared;
        let mut entries = vec![long];
        entries.extend((0..11).map(|i| entry(&format!("ff00{i:04}"))));

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
            // The option number is not always one digit — twelve entries plus
            // `done` is thirteen options, and the tenth one already needs two.
            let digits = options.len().to_string().len();
            for option in &options {
                // Indent, cursor, space, number, space, the padded label, two
                // spaces, the aside — the line `ask::select::row` builds.
                let drawn = 4 + digits + 1 + widest + 2 + option.aside.chars().count();
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

    /// **Each lens shows its own half and nothing of the other**, asserted in
    /// both directions: a filter tested only from the side it keeps is a filter
    /// that passes when it does nothing.
    #[test]
    fn each_lens_shows_one_half_of_the_store() {
        let mut task = entry("11112222");
        task.origin = armada_core::failure::Origin::Written;
        task.class = None;
        let mut reported = entry("33334444");
        reported.origin = armada_core::failure::Origin::Reported;
        reported.class = None;
        let all = vec![entry("a1b2c3d4"), reported, task];

        let failures = shown(all.clone(), false, Lens::Failures, None);
        let tasks = shown(all, false, Lens::Tasks, None);
        assert_eq!(failures.len(), 2, "{failures:?}");
        assert_eq!(tasks.len(), 1, "{tasks:?}");
        assert_eq!(tasks[0].id, "11112222");
        assert!(failures.iter().all(|entry| entry.origin.is_fault()));
    }

    /// **`start` and `fix` are not synonyms**, and the listing says the right
    /// one — a task offered `fix` reads as Armada calling the thought a defect.
    #[test]
    fn the_two_lenses_name_promotion_differently_and_ask_different_questions() {
        assert_eq!(Lens::Failures.promote().0, "fix");
        assert_eq!(Lens::Tasks.promote().0, "start");
        assert_eq!(Lens::Failures.verb(), "failures");
        assert_eq!(Lens::Tasks.verb(), "tasks");
        assert_ne!(Lens::Failures.question(), Lens::Tasks.question());
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
