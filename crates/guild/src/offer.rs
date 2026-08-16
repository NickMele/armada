//! The occasional offer to pull a guild that has fallen behind
//! (`docs/reserved/009-smaller-things-raised-in-use.md` item 4).
//!
//! *"I also wonder if this should just be, like, an automatic process ... it
//! will ask me if I want to pull the latest. I can either hit yes or no."* —
//! `oh-my-zsh`'s update prompt is the reference, and its shape is the whole of
//! this module: **look occasionally, ask, never act without a yes.**
//!
//! # Never automatic — the one rule this module exists to keep
//!
//! A `pull` that runs unasked can change how an agent behaves mid-session
//! (item 4's own wording). So nothing in this module ever calls
//! [`crate::repo::fast_forward`]; it decides *whether to ask* and reports what
//! it found. The write — `armada guild pull` itself — happens only after a
//! `yes` a caller above this module collects, which is why [`check`] returns
//! an [`Outcome`] and not a `Result<(), ArmadaError>` that a careless caller
//! could `?`-away into a silent pull.
//!
//! # When it fires: elapsed time, not every invocation
//!
//! A network round trip on every command would make every verb wait on a
//! remote — unacceptable on a hot path, and the opposite of "occasional."
//! `oh-my-zsh` solves this by checking how long it has been since the last
//! look, not by checking on every shell start, and [`due`] is the same
//! decision: a pure function of "how long since we last looked," bounded by
//! [`INTERVAL_MS`]. **A day**, chosen over `oh-my-zsh`'s 13 because a guild
//! is worked on across a session-to-session cadence rather than a
//! once-a-fortnight one — long enough that most invocations do nothing but
//! read a timestamp, short enough that a guild pulled to a second machine this
//! morning is offered on the first one before the day is out.
//!
//! # Where the state lives
//!
//! The last-checked reading is [`crate::machine::GuildSection::last_offer_ms`]
//! — machine-local by construction, because `~/.armada/machine.yml`'s
//! `guild:` section never syncs (`PLAN.md` §13.1). A synced timestamp would
//! make one machine's check suppress another's offer, which is wrong: they do
//! not fetch from the same place at the same moment, and "checked recently on
//! the laptop" is not evidence about the desktop.
//!
//! # Which invocations may even ask
//!
//! [`eligible`] is the same three-audience test
//! [`armada_core::scan::handover`] already applies to `config scan`'s
//! hand-over, restated here rather than imported because Guild may not name
//! Helm and the test is three booleans, not a dependency:
//!
//! - **not `--json`** — a parser waiting for one payload must never see a
//!   question interleaved with it.
//! - **both stdin and stdout are a terminal** — stdin decides whether an
//!   answer can arrive, stdout decides whether the question was seen, and a
//!   prompt written to a pipe while stdin happens to be a tty is one nobody
//!   read.
//! - **not inside a Job** (`ARMADA_JOB` unset) — a Drone's exchange reads
//!   stdout and has no stdin to type an answer into; a question there is one
//!   nobody will ever answer, which is exactly the failure mode `handover`
//!   was built to prevent for `config scan`.
//!
//! # Offline is normal, not a fault
//!
//! [`check`] answers [`Outcome::Offline`] rather than propagating the error a
//! failed fetch or `remote get-url` produced — the same choice
//! `crates/helm/src/verbs/doctor.rs`'s `drift` makes, for the same reason: a
//! `doctor` (or here, a caller of any verb) that failed because a laptop is on
//! a train is a `doctor` nobody runs on a train. The caller records the
//! attempt regardless of outcome, so a stretch with no signal is retried once
//! per [`INTERVAL_MS`] rather than on every command typed during it.

use crate::layout::Guild;
use armada_core::ctx::Run;

/// How long to leave the offer alone after it last looked.
///
/// **A day.** `oh-my-zsh` defaults to thirteen because a shell prompt update
/// is worth interrupting rarely; a guild is the setup a session is running
/// against right now, so the offer should not lag a same-day sync between two
/// machines by very long. A day is short enough for that and long enough that
/// a person running Armada twenty times before lunch is checked once.
pub const INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;

/// Whether enough time has passed since the last look to look again.
///
/// **Pure, and the whole of the "not a hot path" decision.** No file, no
/// network, no clock read inside it — a caller reads `machine.yml` once, reads
/// the wall clock once, and asks this. `None` (never checked, including a
/// fresh install) is due at once, the same "unconfigured is not configured"
/// direction every other machine-local default in this crate takes.
pub fn due(now_ms: u64, last_offer_ms: Option<u64>, interval_ms: u64) -> bool {
    match last_offer_ms {
        None => true,
        Some(last) => now_ms.saturating_sub(last) >= interval_ms,
    }
}

/// Whether this invocation is even allowed to put the question.
///
/// **All three must hold.** See the module doc for what each guards against;
/// the short form is that this is [`armada_core::scan::handover`]'s
/// both-streams-and-not-json rule plus the one guard `config scan` did not
/// need: a Job's Drone has no person on the other end of stdin at all.
pub fn eligible(json: bool, can_ask: bool, in_job: bool) -> bool {
    !json && can_ask && !in_job
}

/// What asking the remote found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// There is no guild on this machine yet. Nothing to check, nothing to
    /// record — the local stat this cost is cheap to repeat.
    NoGuild,
    /// Sync is off. The documented default, not a problem
    /// (`crates/helm/src/verbs/doctor.rs`'s `drift` treats it the same way).
    NoRemote,
    /// The remote could not be reached. Normal, not a fault — see the module
    /// doc.
    Offline,
    /// Nothing to pull.
    InStep,
    /// This machine has commits the remote does not; nothing to pull.
    AheadOnly(usize),
    /// Both sides have commits the other does not. **Never offered as a
    /// pull** — `crate::repo::fetch`'s docs are explicit that a divergence is
    /// stop-and-report, never auto-resolved, and offering a pull here would
    /// be asking a `yes` to paper over a decision only a person can make.
    Diverged,
    /// A clean fast-forward is available: this many commits behind.
    Behind(usize),
}

/// Ask the remote how far this machine's guild is behind, without changing
/// anything.
///
/// **Read-only**, same as `armada doctor`'s drift check and for the same
/// reason: this runs as a side effect of some other verb the caller typed,
/// and a check that could accidentally write would surprise every one of
/// them.
pub fn check(run: &impl Run, guild: &Guild) -> Outcome {
    if !guild.exists() {
        return Outcome::NoGuild;
    }
    match crate::repo::remote(run, guild.root()) {
        Ok(Some(_)) => {}
        Ok(None) => return Outcome::NoRemote,
        Err(_) => return Outcome::Offline,
    }
    match crate::repo::fetch(run, guild.root()) {
        Ok(apart) if apart.diverged() => Outcome::Diverged,
        Ok(apart) if apart.behind > 0 => Outcome::Behind(apart.behind),
        Ok(apart) if apart.ahead > 0 => Outcome::AheadOnly(apart.ahead),
        Ok(_) => Outcome::InStep,
        Err(_) => Outcome::Offline,
    }
}

/// Record that the offer looked, right now — whatever it found.
///
/// **Called after every real attempt, [`Outcome::NoGuild`] excepted**, so an
/// offline stretch is retried once per [`INTERVAL_MS`] instead of on every
/// command typed during it. `NoGuild` is deliberately not recorded: there is
/// nothing to space out when the check that answered it was a local stat
/// rather than a network call.
pub fn record(armada_home: &std::path::Path, now_ms: u64) -> std::io::Result<()> {
    let mut section = crate::machine::read(armada_home);
    section.last_offer_ms = Some(now_ms);
    crate::machine::write(armada_home, &section)
}

/// The last time the offer looked, or `None` if it never has.
pub fn last_offer_ms(armada_home: &std::path::Path) -> Option<u64> {
    crate::machine::read(armada_home).last_offer_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::GuildSection;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};
    use std::cell::RefCell;

    // ---------------------------------------------------------------- due

    #[test]
    fn never_checked_is_due_at_once() {
        assert!(due(1_000, None, INTERVAL_MS));
    }

    #[test]
    fn checked_a_moment_ago_is_not_due() {
        assert!(!due(1_000, Some(999), INTERVAL_MS));
    }

    #[test]
    fn checked_exactly_one_interval_ago_is_due() {
        assert!(due(INTERVAL_MS, Some(0), INTERVAL_MS));
    }

    #[test]
    fn checked_just_under_one_interval_ago_is_not_due() {
        assert!(!due(INTERVAL_MS - 1, Some(0), INTERVAL_MS));
    }

    /// A clock that stepped backwards (NTP, a restored snapshot) must not
    /// panic the subtraction — `saturating_sub` is the whole of the guard.
    #[test]
    fn a_clock_that_moved_backwards_is_not_due_and_does_not_panic() {
        assert!(!due(500, Some(1_000), INTERVAL_MS));
    }

    // ----------------------------------------------------------- eligible

    #[test]
    fn a_terminal_with_no_json_and_no_job_is_eligible() {
        assert!(eligible(false, true, false));
    }

    #[test]
    fn json_is_never_eligible_even_at_a_terminal() {
        assert!(!eligible(true, true, false));
    }

    #[test]
    fn a_non_terminal_invocation_is_never_eligible() {
        assert!(!eligible(false, false, false));
    }

    #[test]
    fn inside_a_job_is_never_eligible_even_at_a_terminal() {
        assert!(!eligible(false, true, true));
    }

    #[test]
    fn every_combination_agrees_only_all_three_conditions_pass() {
        for json in [false, true] {
            for can_ask in [false, true] {
                for in_job in [false, true] {
                    let want = !json && can_ask && !in_job;
                    assert_eq!(
                        eligible(json, can_ask, in_job),
                        want,
                        "{json} {can_ask} {in_job}"
                    );
                }
            }
        }
    }

    // --------------------------------------------------------------- check

    /// `Ok((exit-was-zero, stdout, stderr))`, or `Err(spawn failure message)`.
    type GitCall = Result<(bool, String, String), String>;

    #[derive(Default)]
    struct Scripted(RefCell<Vec<GitCall>>);

    impl Run for Scripted {
        fn call(&self, _request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let next = self.0.borrow_mut().remove(0);
            match next {
                Ok((ok, stdout, stderr)) => Ok(RunOutput {
                    code: Some(if ok { 0 } else { 1 }),
                    signal: None,
                    stdout,
                    stderr,
                    timed_out: false,
                }),
                Err(message) => Err(SpawnError {
                    program: "git".to_string(),
                    kind: armada_core::ctx::SpawnErrorKind::Other,
                    message,
                }),
            }
        }
    }

    fn scripted(calls: Vec<GitCall>) -> Scripted {
        Scripted(RefCell::new(calls))
    }

    fn guild(dir: &std::path::Path) -> Guild {
        Guild::at(dir)
    }

    #[test]
    fn no_guild_directory_is_reported_without_running_git() {
        let home = tempfile::tempdir().unwrap();
        let run = scripted(vec![]);
        assert_eq!(check(&run, &guild(home.path())), Outcome::NoGuild);
    }

    #[test]
    fn a_guild_with_no_remote_is_reported_and_not_fetched() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("guild").join(".git")).unwrap();
        let run = scripted(vec![Ok((false, String::new(), String::new()))]);
        assert_eq!(check(&run, &guild(home.path())), Outcome::NoRemote);
    }

    #[test]
    fn an_unreachable_remote_url_lookup_is_offline() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("guild").join(".git")).unwrap();
        let run = scripted(vec![Err("no network".to_string())]);
        assert_eq!(check(&run, &guild(home.path())), Outcome::Offline);
    }

    #[test]
    fn a_fetch_that_cannot_reach_the_remote_is_offline_not_an_error() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("guild").join(".git")).unwrap();
        let run = scripted(vec![
            Ok((
                true,
                "git@example.com:me/guild.git\n".to_string(),
                String::new(),
            )),
            Ok((false, String::new(), "could not resolve host".to_string())),
        ]);
        assert_eq!(check(&run, &guild(home.path())), Outcome::Offline);
    }

    // ------------------------------------------------------------- record

    #[test]
    fn recording_is_read_back() {
        let home = tempfile::tempdir().unwrap();
        record(home.path(), 42).unwrap();
        assert_eq!(last_offer_ms(home.path()), Some(42));
    }

    #[test]
    fn a_never_recorded_machine_reads_as_none() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(last_offer_ms(home.path()), None);
    }

    /// Another module's section, and this section's own other fields, survive
    /// a record — the same discipline `machine::record` already proves for
    /// `remote` and `withheld`.
    #[test]
    fn recording_leaves_the_remote_and_other_sections_alone() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "somebody_else:\n  cpu_slots: 6\nguild:\n  remote: git@example.com:me/guild.git\n",
        )
        .unwrap();

        record(home.path(), 7).unwrap();

        let section = crate::machine::read(home.path());
        assert_eq!(
            section,
            GuildSection {
                remote: Some("git@example.com:me/guild.git".to_string()),
                withheld: Vec::new(),
                last_offer_ms: Some(7),
            }
        );
        let text = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        assert!(text.contains("cpu_slots: 6"), "{text}");
    }
}
