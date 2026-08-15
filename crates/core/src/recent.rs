//! The last few things Armada was asked to do — **so that a report can attach
//! the run instead of asking a person to paste it.**
//!
//! ## The problem, in the words it was raised in
//!
//! *"The thing that just happened, I had to copy and paste the output and then
//! bring it back to you … it would be sweet if we just had a command like
//! `armada report` that included the logs and the output and any other
//! diagnostics."*
//!
//! [`crate::failure`] cannot answer that. It records what **Armada noticed**,
//! and the run that prompted this exited `0`: `armada fleet spawn --dry-run`
//! printed `CREATED worktree`, `STARTED drone` and `QUEUED` for work it had
//! correctly not done. The render lied, the exit code said success, and no
//! failure entry could ever exist for it. A description alone would have lost
//! the one artefact that made it diagnosable — the output.
//!
//! ## Three ways to know what just happened, and why this is the one
//!
//! | Option | What it captures | Why not |
//! |---|---|---|
//! | `armada report --last`, re-running the previous command | a *second* run | re-running a mutating verb to describe the first one is a bug-report that spawns a Job |
//! | piping or pasting the output in | whatever the person kept | it is the manual step the ask exists to remove, and scrollback is already gone |
//! | **a ring buffer, written as each run ends** | the run he already did | the only one that can attach a run nobody knew would matter |
//!
//! The third is the only one that works *after the fact*, and after the fact is
//! the only time anybody files a report. The other two require having known in
//! advance.
//!
//! ## What that costs, stated rather than waved at
//!
//! It is a new file on disk holding every command typed, which is a real
//! privacy surface and the reason to argue it rather than assume it. Four
//! things bound it, and the first is the one that settles it:
//!
//! - **Armada already writes this down.** `~/.armada/failures.jsonl` records the
//!   argv and the directory of every failure it reports. This extends that to
//!   the runs that *succeeded and lied* — exactly the set the failure log
//!   cannot see, and the set the ask is about. It is not a new kind of data.
//! - **It is bounded.** [`KEEP`] entries, rewritten in place; the file cannot
//!   grow. A log of everything since installation would be a different thing
//!   with a different argument.
//! - **It is redacted before it is written, not before it is read.** The
//!   `redact` argument to [`note`] runs over the argv, the directory and the
//!   captured output on the way in, so a credential is never at rest here even
//!   for the runs nobody ever reports. Scrubbing at report time would leave a
//!   window, and the window is the whole file's lifetime.
//! - **`$HOME` never appears**, by the same chokepoint and for the same reason
//!   [`crate::failure`] gives: this machine's records feed a report that is
//!   meant to become a GitHub issue one day, and a home path cannot be
//!   un-published once it is.
//!
//! ## What is captured, and what deliberately is not
//!
//! The **envelope**, not the terminal. Armada renders every answer from one
//! (`PLAN.md` §3.1.1), so the envelope is what the render was drawn *from* —
//! which makes "the render said CREATED and nothing was created" a question a
//! reader can settle, and makes the attachment useful to the agent reading
//! `--json` rather than only to a person. Capturing the drawn terminal instead
//! would mean teeing every byte of every stream, storing ANSI escapes, and
//! keeping a copy of output that was never structured in the first place.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// How many runs are kept.
///
/// **Ten, and the number is a judgement about how people file reports.** The
/// run being described is almost always the last one, occasionally two or three
/// back — "I ran the thing, then ran it again with a flag, then came here". Ten
/// covers that with room and still bounds the file to something a person could
/// read in one screen if they went looking, which is the property that makes
/// the privacy argument checkable rather than asserted.
pub const KEEP: usize = 10;

/// The most bytes of one run's envelope that are kept.
///
/// **A cap rather than a whole payload**, because `armada manifest check` over a
/// large repository answers in tens of kilobytes and ten of those is a file
/// nobody would want written on every invocation. The head is kept rather than
/// the tail: `status`, `verb` and the first rows are what say whether the answer
/// was the right shape, and the shape is what a report is usually about.
pub const OUTPUT_CAP: usize = 8_192;

/// One run of Armada, as it ended.
///
/// **Deserialized as well as serialized**, unlike the failure log's [`Entry`],
/// because this file is rewritten rather than appended: every run reads back the
/// nine before it.
///
/// [`Entry`]: crate::failure::Entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ran {
    /// When, wall clock, RFC 3339.
    pub at: String,
    /// Wall clock milliseconds, so "4m ago" is a subtraction.
    pub at_ms: u64,
    /// The verb, as the roster spells it — `fleet spawn`, `manifest check`.
    ///
    /// **Derived from the argv rather than from the parse**, so a line Armada
    /// refused still names what it was refusing. It is what the NAME column of
    /// the attached table prints.
    pub verb: String,
    /// The whole command that was typed, with `$HOME` abbreviated and any
    /// credential-shaped value replaced.
    pub argv: String,
    /// Where it was typed, with `$HOME` abbreviated.
    pub cwd: String,
    /// The exit code it left with. `0` is the interesting case here — a run
    /// that failed is already in the failure log.
    pub exit: u8,
    /// The envelope Armada answered with, capped at [`OUTPUT_CAP`] and
    /// redacted. `None` when the run produced no envelope at all.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub envelope: Option<String>,
}

impl Ran {
    /// The word the STATUS column prints.
    ///
    /// **`OK` and `FAILED`, never a glyph**, the rule every table in Armada
    /// follows: a row told apart by a tick is a row a monochrome terminal cannot
    /// tell apart at all.
    pub const fn word(&self) -> &'static str {
        match self.exit {
            0 => "OK",
            _ => "FAILED",
        }
    }
}

/// Record one run, with every string redacted and then tilde'd on the way in.
///
/// **`redact` is supplied by the caller and applied here**, which is the seam
/// that keeps the secret detector in one place without inverting the module
/// stack. The detector lives in Guild (`armada_guild::secrets`) because that is
/// where the guild importer needs it, and this crate is below Guild and may not
/// name it (`ARCHITECTURE.md` §1.5). Passing the function in means the *walk*
/// over every field lives here — so a field added to [`Ran`] cannot be added
/// without passing through it — while the *rule* stays the one Guild already
/// enforces.
///
/// **Redaction runs before [`tilde`](crate::failure::tilde)**, deliberately: a
/// token that happens to contain the home path would otherwise be split in half
/// and stop looking credential-shaped.
#[allow(clippy::too_many_arguments)]
pub fn note(
    argv: &[String],
    home: &Path,
    cwd: &Path,
    exit: u8,
    envelope: Option<&str>,
    at: &str,
    at_ms: u64,
    redact: &dyn Fn(&str) -> String,
) -> Ran {
    let clean = |text: &str| crate::failure::tilde(&redact(text), home);
    Ran {
        at: at.to_string(),
        at_ms,
        verb: verb_of(argv),
        argv: clean(&crate::failure::argv_line(argv)),
        cwd: clean(&cwd.display().to_string()),
        exit,
        envelope: envelope.map(|text| clean(&cap(text))),
    }
}

/// The verb an argv names, as the help roster spells it.
///
/// **Two words at most, and flags are skipped.** `armada --json fleet spawn -C
/// /x` is `fleet spawn`; `armada doctor` is `doctor`. It is a display label, so
/// a line that names no verb at all answers `armada` rather than an empty cell —
/// a blank NAME column is the one thing a table must not print.
pub fn verb_of(argv: &[String]) -> String {
    let words: Vec<&str> = argv
        .iter()
        .map(String::as_str)
        .filter(|word| !word.starts_with('-'))
        .take(2)
        .collect();
    match words.is_empty() {
        true => "armada".to_string(),
        false => words.join(" "),
    }
}

/// The head of a payload, and a note saying the rest was dropped.
///
/// **Cut on a character boundary**, because an envelope is UTF-8 and a byte
/// truncation in the middle of a multi-byte character produces a string that
/// cannot be serialized back out.
fn cap(text: &str) -> String {
    if text.len() <= OUTPUT_CAP {
        return text.to_string();
    }
    let mut end = OUTPUT_CAP;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… truncated at {OUTPUT_CAP} bytes", &text[..end])
}

/// The buffer after this run — **most recent first, and never longer than
/// [`KEEP`]**.
pub fn roll(existing: Vec<Ran>, latest: Ran) -> Vec<Ran> {
    let mut rolled = Vec::with_capacity(existing.len() + 1);
    rolled.push(latest);
    rolled.extend(existing);
    rolled.truncate(KEEP);
    rolled
}

/// Every run the file holds, most recent first.
///
/// **A line that does not parse is skipped**, the same rule the failure log and
/// the inbox follow: this file is rewritten rather than appended, but a machine
/// that lost power mid-write still leaves one, and one torn line must not hide
/// the runs around it.
pub fn parse(text: &str) -> Vec<Ran> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Ran>(line.trim()).ok())
        .collect()
}

/// The file's contents, from the buffer.
pub fn render(runs: &[Ran]) -> String {
    let mut out = String::new();
    for run in runs {
        if let Ok(line) = serde_json::to_string(run) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The detector Guild owns is not reachable from here, so the tests supply
    /// the one rule they are about: a value that looks like a token goes.
    fn redact(text: &str) -> String {
        text.split(' ')
            .map(|word| match word.starts_with("ghp_") {
                true => "[redacted]".to_string(),
                false => word.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn ran(argv: &[&str], at_ms: u64) -> Ran {
        let argv: Vec<String> = argv.iter().map(|word| (*word).to_string()).collect();
        note(
            &argv,
            Path::new("/scratch/home"),
            Path::new("/scratch/home/code/api"),
            0,
            Some("{\"verb\":\"fleet spawn\"}"),
            "2026-08-15T09:00:00Z",
            at_ms,
            &redact,
        )
    }

    /// **The motivating run**: it exited 0, so the failure log will never hold
    /// it, and the envelope it answered with is what makes the complaint
    /// checkable.
    #[test]
    fn a_run_that_succeeded_is_kept_with_the_envelope_it_answered_with() {
        let run = ran(&["fleet", "spawn", "--dry-run"], 1_000);
        assert_eq!(run.verb, "fleet spawn");
        assert_eq!(run.exit, 0);
        assert_eq!(run.word(), "OK");
        assert_eq!(run.cwd, "~/code/api");
        assert!(run.envelope.unwrap().contains("fleet spawn"));
    }

    /// No absolute `$HOME` reaches the file, by the same chokepoint the failure
    /// log uses.
    #[test]
    fn the_home_path_is_abbreviated_in_every_field() {
        let run = ran(&["manifest", "check", "-C", "/scratch/home/code/api"], 1);
        let text = render(&[run]);
        assert!(!text.contains("/scratch/home"), "{text}");
        assert!(text.contains("~/code/api"), "{text}");
    }

    /// **Redaction happens on the way in.** A token typed on the command line
    /// is not in the file even for the runs nobody ever reports.
    #[test]
    fn a_credential_on_the_command_line_never_reaches_the_buffer() {
        let run = ran(&["manifest", "check", "--token", "ghp_deadbeefdeadbeef"], 1);
        assert!(!run.argv.contains("ghp_"), "{}", run.argv);
        assert!(run.argv.contains("[redacted]"), "{}", run.argv);
    }

    /// **Bounded, which is half the privacy argument.** Eleven runs leave ten.
    #[test]
    fn the_buffer_never_grows_past_what_it_keeps() {
        let mut buffer: Vec<Ran> = Vec::new();
        for step in 0..(KEEP as u64 + 5) {
            buffer = roll(buffer, ran(&["doctor"], step));
        }
        assert_eq!(buffer.len(), KEEP);
        assert_eq!(buffer[0].at_ms, KEEP as u64 + 4, "most recent first");
    }

    /// A large answer is capped rather than stored whole, and the cut lands on
    /// a character boundary so the line still serializes.
    #[test]
    fn a_large_envelope_is_capped_and_still_round_trips() {
        let big = format!("{{\"note\":\"{}\"}}", "é".repeat(OUTPUT_CAP));
        let run = note(
            &["manifest".to_string(), "check".to_string()],
            Path::new("/scratch/home"),
            Path::new("/scratch/home/code/api"),
            0,
            Some(&big),
            "2026-08-15T09:00:00Z",
            1,
            &redact,
        );
        let held = run.envelope.clone().unwrap();
        assert!(held.len() < big.len());
        assert!(held.ends_with("bytes"), "{held}");
        assert_eq!(parse(&render(std::slice::from_ref(&run))), vec![run]);
    }

    /// A torn line is skipped and the runs around it still read.
    #[test]
    fn a_torn_line_does_not_hide_the_runs_around_it() {
        let text = format!(
            "{}{{\"at\":\"2026\n{}",
            render(&[ran(&["doctor"], 1)]),
            render(&[ran(&["bridge"], 2)])
        );
        let runs = parse(&text);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].verb, "doctor");
        assert_eq!(runs[1].verb, "bridge");
    }

    /// A line with nothing but flags on it still names something in the NAME
    /// column, because a blank cell is the one thing a table must not print.
    #[test]
    fn a_line_that_names_no_verb_still_has_a_name() {
        assert_eq!(verb_of(&[]), "armada");
        assert_eq!(verb_of(&["--json".to_string()]), "armada");
        assert_eq!(
            verb_of(&["--json".to_string(), "fleet".to_string(), "ls".to_string()]),
            "fleet ls"
        );
    }
}
