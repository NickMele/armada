//! `armada failures` — what Armada broke on, and how to put a Job on one.
//!
//! Four things, and only the first is new work (PLAN.md §15.3.5):
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
//! module adds is the line that links the two — the same link PLAN.md §15.3.2
//! wants between a task and the Job that does it.
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

use crate::ask::Ask;
use crate::render::progress::Progress;
use crate::verbs::Output;

pub use crate::verbs::fleet::Where;

/// `armada failures` — the log, folded.
///
/// **Cleared entries are hidden unless asked for**, which is the same lens
/// `armada fleet ls --all` applies to finished Jobs: the default answers "what
/// is still mine to deal with", and the flag answers "what has this machine
/// ever done".
pub fn ls<C: Clock>(now: &C, place: &Where, all: bool) -> Result<Output, ArmadaError> {
    let entries = read(now, place)?;
    let shown: Vec<Entry> = entries
        .into_iter()
        .filter(|entry| all || entry.state != State::Cleared)
        .collect();
    Ok(listing("failures", shown))
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
        let Output::Failures(envelope) = listing("failures", vec![entry("a1b2c3d4"), fixing]) else {
            panic!("a listing answers as one");
        };
        assert_eq!(envelope.data.results.len(), 2);
        assert_eq!(envelope.data.open, 1);
        assert_eq!(envelope.status, Status::Ok);
    }
}
