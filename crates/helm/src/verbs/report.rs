//! `armada report` — **what you know went wrong, when Armada does not.**
//!
//! ## The complaint this exists to fix
//!
//! *"Similar to the failures idea, but in this case, it's something that's not a
//! failure, but I wanna report it. The thing that just happened, I had to copy
//! and paste the output and then bring it back to you … it would be sweet if we
//! just had a command like `armada report` that actually reported issues like
//! this where it included the logs and the output and any other diagnostics that
//! would be helpful, but don't include secrets."*
//!
//! ## Why this is not [`crate::verbs::failures`] with a different name
//!
//! `armada failures` records what **Armada noticed**. The run that prompted this
//! exited `0`: `armada fleet spawn … --dry-run` printed `CREATED worktree`,
//! `STARTED drone` and `QUEUED` for work it had correctly not done. The render
//! lied, the exit code said success, and **no failure entry could ever exist for
//! it** — so the only way to get it fixed was to copy the terminal into a chat.
//!
//! Wrong output, missing output, misleading wording and anything that looks fine
//! to the program live only on this side of the line. `failures` is what Armada
//! thinks went wrong; `report` is what the person knows went wrong.
//!
//! ## One store, and the origin is a column
//!
//! Asked for in those terms — *"right now it can use the same failure system
//! that's local to a machine"* — and it is the right shape for the reason
//! `docs/reserved/001-raised-items-need-identity.md` gives: a thing needing
//! attention is useless until it has an id you can act on one at a time. A
//! second store would mean a second id space, a second `show`, a second listing
//! to remember, and a second promotion path, and a person triaging on a Monday
//! does not care which half of the machine noticed. So a report is a
//! [`Line::Reported`] in `~/.armada/failures.jsonl`, it lists under `armada
//! failures`, it shows under `armada failures show`, and `armada failures fix`
//! promotes it into a Job exactly as it promotes a failure.
//!
//! ## What is attached, and why each of them
//!
//! | Attached | Because |
//! |---|---|
//! | the last runs and their envelopes | **the thing that just happened** — see [`armada_core::recent`] |
//! | `armada doctor`'s findings | the verb whose whole subject is what this machine is missing |
//! | Armada's version, `claude`'s, the OS | the first three questions anybody asks about a bug |
//! | the workspace, and whether it has an `armada.yml` | half of Armada's behaviour turns on this and it is invisible from the report's text |
//! | the open failure ids | a report and a failure are often the same event from two sides |
//! | the Jobs in flight | the ask names Helm, and a report about a Job has to name it |
//!
//! **Every one of them is best effort.** A report is filed *because something is
//! wrong*, so every gatherer runs on a machine that may be broken in the way
//! being reported — a diagnostic that could fail the report is a bad diagnostic.
//! Each degrades to an absence, and the absence is itself evidence.
//!
//! ## Secrets
//!
//! Everything above is command lines, environment-derived text and captured
//! output, all of which routinely carry tokens. Nothing here decides what a
//! secret looks like: [`crate::redact`] is a tokeniser over
//! `armada_guild::secrets`, and the walk that applies it to every field of the
//! record is [`armada_core::failure::reported`] — so a field added to the
//! diagnostics cannot be written without passing through it.
//!
//! ## GitHub issues are deliberately not here
//!
//! *"In the future we can explore if it creates GitHub issues."* The local
//! record is the whole of this. What it does owe that future is a shape it can
//! be rendered into, which
//! [`armada_core::failure::task`] already is — and the redaction, which is the
//! part that cannot be added afterwards: a record that has already leaked a
//! secret locally cannot be un-published once it is.

use armada_core::ctx::{Clock, Run, RunRequest, StdioMode};
use armada_core::envelope::{Envelope, FailureData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::failure::{self, Diagnostics};
use armada_core::recent::Ran;

use crate::verbs::Output;

pub use crate::verbs::fleet::Where;

/// How long `claude --version` is given before its answer stops being worth
/// waiting for.
///
/// **Two seconds, and a timeout at all is the point.** A `claude` that hangs is
/// one of the things somebody would file a report *about*, and a report that
/// hung gathering evidence about a hang would be the bug reporting itself.
const VERSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// `armada report "<what happened>"` — file it, with the evidence attached.
///
/// **The description is required and there is no interactive fallback.** Every
/// other Armada prompt has a default that leaves something working; the default
/// here would be an empty report, which is a row that costs a reader a click and
/// tells them nothing. A person who typed the verb has the sentence in their
/// head already.
pub fn run<R: Run, C: Clock>(
    runner: &R,
    now: &C,
    place: &Where,
    what: &str,
    argv: &[String],
) -> Result<Output, ArmadaError> {
    let what = what.trim();
    if what.is_empty() {
        return Err(empty());
    }

    let diagnostics = gather(runner, now, place);
    let (id, line) = failure::reported(
        what,
        &place.home,
        &place.cwd,
        argv,
        &diagnostics,
        &now.wall_rfc3339(),
        now.wall_ms(),
        &|text| crate::redact::scrub(text),
    );

    let path = armada_manifest::failures::path(&place.armada_home);
    // **Unlike recording a failure, this one is allowed to complain.** The
    // recorder is a side effect of a run that is already failing and must never
    // change an exit code; this is the whole of what the caller asked for, and a
    // report that silently did not land is worse than one that refused.
    if !armada_manifest::failures::append(&path, &line) {
        return Err(unwritable(&path));
    }

    // **Read back rather than constructed.** The entry this answers with is the
    // one the fold produces from the line just written, so `armada report` and
    // `armada failures show <id>` cannot disagree about what was filed — and a
    // field that fails to round-trip through the file fails here, at the moment
    // somebody is looking.
    let mut entries =
        armada_manifest::failures::read(&armada_manifest::failures::path(&place.armada_home))?;
    failure::age(&mut entries, now.wall_ms());
    let entry = entries
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| unwritable(&path))?;
    let task = failure::task(&entry);

    Ok(Output::Failure(Box::new(Envelope::ok(
        "report",
        None,
        Status::Ok,
        FailureData {
            results: vec![entry],
            task,
        },
    ))))
}

/// Everything worth attaching, gathered without letting any of it fail the
/// report.
fn gather<R: Run, C: Clock>(runner: &R, now: &C, place: &Where) -> Diagnostics {
    let workspace = armada_manifest::discovery::resolve(runner, &place.cwd).ok();
    Diagnostics {
        armada: env!("CARGO_PKG_VERSION").to_string(),
        claude: claude_version(runner, place),
        system: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        cwd: tilde(place, &place.cwd.display().to_string()),
        workspace: workspace
            .as_ref()
            .map(|found| tilde(place, &found.root.display().to_string())),
        manifest: workspace
            .as_ref()
            .is_some_and(|found| found.root.join("armada.yml").exists()),
        doctor: doctor(runner, place),
        recent: recent(place),
        failures: open_failures(place),
        jobs: jobs(runner, now, place),
    }
}

/// The runs the ring buffer held.
///
/// **The report's own run is not among them.** The buffer is written as a run
/// *ends* and this one has not, which is exactly right: `armada report` is never
/// the run being reported.
fn recent(place: &Where) -> Vec<Ran> {
    armada_manifest::recent::read(&armada_manifest::recent::path(&place.armada_home))
}

/// What `claude --version` said, or nothing.
///
/// **Asked rather than looked up on `PATH`**, because "it is installed" and "it
/// answers" are different facts and only the second one matters to a Job that
/// will not start.
fn claude_version<R: Run>(runner: &R, place: &Where) -> Option<String> {
    let mut request = RunRequest::new(
        vec!["claude".to_string(), "--version".to_string()],
        place.cwd.clone(),
    );
    request.stdio = StdioMode::Capture;
    request.timeout = Some(VERSION_TIMEOUT);
    let output = runner.call(&request).ok()?;
    if !output.ok() {
        return None;
    }
    let answer = output.stdout.trim();
    match answer.is_empty() {
        true => None,
        false => Some(answer.to_string()),
    }
}

/// `armada doctor`'s findings, phrased the way `doctor` phrases them.
///
/// **Only what is not `ok`.** A report carrying eleven green rows buries the two
/// that matter, and the reader who wants the full page has a verb for it.
fn doctor<R: Run>(runner: &R, place: &Where) -> Vec<String> {
    let looking = crate::verbs::guild::Where {
        armada_home: place.armada_home.clone(),
        cwd: place.cwd.clone(),
        claude_home: place.home.join(".claude"),
    };
    let Ok(Output::Doctor(envelope)) = crate::verbs::doctor::run(runner, &looking) else {
        return Vec::new();
    };
    envelope
        .data
        .results
        .iter()
        .filter(|finding| finding.status != armada_core::envelope::Health::Ok)
        .map(|finding| {
            format!(
                "{} {}: {}",
                finding.status.word(),
                finding.check,
                finding.detail
            )
        })
        .collect()
}

/// The ids of failures still open, so a reader can go straight to each.
fn open_failures(place: &Where) -> Vec<String> {
    let Ok(entries) =
        armada_manifest::failures::read(&armada_manifest::failures::path(&place.armada_home))
    else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|entry| entry.state == failure::State::Open)
        .map(|entry| format!("{} {}", entry.id, entry.message))
        .collect()
}

/// The Jobs in flight, by handle and state.
///
/// **Skipped outright when the machine reports no boot id.** A Drone handle is
/// only meaningful against the boot it was taken on, so `fleet ls` refuses
/// without one — and *"Armada will not run on this machine"* is the single most
/// valuable report there is. Answering it with a refusal about Jobs would make
/// the report impossible at the one moment it matters most, so the diagnostic
/// gives way and the rest of the report stands.
fn jobs<R: Run, C: Clock>(runner: &R, now: &C, place: &Where) -> Vec<String> {
    if place.boot_id.is_empty() {
        return Vec::new();
    }
    let Ok(Output::FleetLs(envelope)) = crate::verbs::fleet::ls(runner, now, place, false, false)
    else {
        return Vec::new();
    };
    envelope
        .data
        .results
        .iter()
        .map(|job| format!("{} {} {}", job.name, job.state.word(), job.workflow))
        .collect()
}

fn tilde(place: &Where, text: &str) -> String {
    failure::tilde(text, &place.home)
}

fn empty() -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: "report".to_string(),
        message: "`armada report` needs a sentence saying what happened".to_string(),
        next_action: Some(
            "armada report \"the dry-run said CREATED and made nothing\"".to_string(),
        ),
    }
}

fn unwritable(path: &std::path::Path) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: "the report could not be written".to_string(),
        next_action: Some("check ~/.armada/ is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A description that is nothing but whitespace is the same as none.
    #[test]
    fn a_blank_description_is_refused_with_the_line_that_would_have_worked() {
        let error = empty();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(
            error.next_action.unwrap().starts_with("armada report \""),
            "the refusal shows the shape rather than describing it"
        );
    }
}
