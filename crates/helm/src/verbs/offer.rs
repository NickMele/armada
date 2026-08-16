//! The occasional offer to pull, wired into the entrypoint
//! (`docs/reserved/009-smaller-things-raised-in-use.md` item 4).
//!
//! [`armada_guild::offer`] is the whole of the decision: whether enough time
//! has passed to look, whether this invocation is even allowed to ask, and
//! what asking the remote found. Nothing in that module ever pulls. This file
//! is the one place the `yes` gets acted on, and it is a thin one on purpose —
//! everything worth arguing about already has its argument in
//! [`armada_guild::offer`]'s module doc.
//!
//! # Not its own verb
//!
//! This runs as a side effect *of* whatever verb the caller actually typed —
//! `main.rs` calls [`maybe_offer`] once, from the same place that already
//! hands `config scan`'s output over to a person
//! (`crates/helm/src/main.rs`'s `hand_over`), after the verb's own envelope
//! has been written and flushed. A prompt that arrived first would be a
//! prompt answered blind, same as that one.
//!
//! # Which verbs may offer
//!
//! Every one that reaches this point does, less what
//! [`armada_guild::offer::eligible`] excludes (`--json`, no terminal, inside a
//! Job) and less the guild's own verbs — asking *"pull now?"* right after
//! `armada guild pull` just ran, or mid-`guild init` before a remote even
//! exists, would be noise about the very thing the caller was already doing.
//! `main.rs` computes that exclusion from the `Invocation` before dispatch
//! moves it, because by the time an [`Output`] comes back the two are not
//! reliably distinguishable by shape alone.

use armada_core::ctx::{Clock, Run};
use armada_core::error::ArmadaError;
use armada_guild::offer::{self, Outcome};

use crate::ask::{Ask, Choice};
use crate::render::style::Style;
use crate::render::term::Terminal;
use crate::verbs::guild::Where;
use crate::verbs::Output;

/// Look, and — only on an explicit yes — pull.
///
/// **Never pulls without [`Ask::choose`] answering "pull."** Every path that
/// is not that one exit returns having done nothing but, at most, record that
/// it looked (`armada_guild::offer::record`) — see that module's doc for why
/// even a failed or empty look is worth recording.
#[allow(clippy::too_many_arguments)]
pub fn maybe_offer(
    run: &impl Run,
    clock: &impl Clock,
    place: &Where,
    ask: &mut dyn Ask,
    style: Style,
    terminal: Terminal,
    json: bool,
    in_job: bool,
) {
    if !offer::eligible(json, terminal.can_ask(), in_job) {
        return;
    }
    let now = clock.wall_ms();
    if !offer::due(
        now,
        offer::last_offer_ms(&place.armada_home),
        offer::INTERVAL_MS,
    ) {
        return;
    }

    match offer::check(run, &place.guild()) {
        // Nothing configured to check yet. Not recorded: the stat that
        // answered this was local and free to repeat, and there is nothing to
        // space out (`armada_guild::offer::record`'s own doc).
        Outcome::NoGuild => {}
        Outcome::NoRemote
        | Outcome::Offline
        | Outcome::InStep
        | Outcome::AheadOnly(_)
        | Outcome::Diverged => {
            let _ = offer::record(&place.armada_home, now);
        }
        Outcome::Behind(count) => {
            let _ = offer::record(&place.armada_home, now);
            ask_and_maybe_pull(ask, count, style, terminal, || {
                crate::verbs::guild::pull(run, place)
            });
        }
    }
}

/// Put the question, and act on "pull" and nothing else.
///
/// **The pull itself arrives as a closure** rather than a direct call to
/// [`crate::verbs::guild::pull`], which is what lets a test prove "not now"
/// never runs `git` at all without reproducing `pull`'s own call sequence —
/// that sequence already has its own tests in `crates/helm/src/verbs/guild.rs`
/// and does not need a second copy of them here.
fn ask_and_maybe_pull(
    ask: &mut dyn Ask,
    behind: usize,
    style: Style,
    terminal: Terminal,
    mut pull: impl FnMut() -> Result<Output, ArmadaError>,
) {
    // The selector takes this on `esc`, on an unreadable line, and on
    // anything else that is not a typed "pull" — every one of those must mean
    // nothing happened (`docs/reserved/009` item 4's hard rule), which is why
    // it is the option `ask.choose` is told is the default rather than one
    // this function infers from the answer.
    const NOT_NOW: usize = 1;
    const PULL: usize = 2;

    let question = format!(
        "your guild is {} behind — pull now?",
        crate::render::format::count(behind, "commit")
    );
    let chosen = ask.choose(
        &question,
        &[
            Choice::bare("not now"),
            Choice::new("pull", "armada guild pull"),
        ],
        NOT_NOW,
    );
    if chosen != PULL {
        return;
    }
    // **The same `pull` `armada guild pull` runs**, so the fast-forward-or-stop
    // contract (`crates/guild/src/repo.rs`) is exactly the one the reader would
    // have gotten typing the command themselves — this is a shortcut to it, not
    // a second implementation of it.
    if let Ok(output @ Output::GuildSync(_)) = pull() {
        ask.show(&crate::render::human(&output, style, terminal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ask::Scripted;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};
    use armada_core::envelope::{Envelope, GuildSyncData};
    use armada_core::error::Status;
    use std::cell::RefCell;

    /// A fake wall clock — no minute has to actually pass to test [`due`].
    ///
    /// [`due`]: armada_guild::offer::due
    struct FixedClock(u64);

    impl Clock for FixedClock {
        fn wall_rfc3339(&self) -> String {
            String::new()
        }
        fn wall_ms(&self) -> u64 {
            self.0
        }
        fn mono(&self) -> u64 {
            0
        }
        fn sleep_until(&self, _: u64) {}
    }

    /// `Ok((exit-was-zero, stdout, stderr))`, or `Err(spawn failure message)`.
    type GitCall = Result<(bool, String, String), String>;

    /// A `git` that panics on an unscripted call — that panic *is* the proof
    /// a caller reached further than a test intended it to.
    #[derive(Default)]
    struct FakeGit(RefCell<Vec<GitCall>>);

    impl Run for FakeGit {
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

    fn place(armada_home: &std::path::Path) -> Where {
        Where {
            armada_home: armada_home.to_path_buf(),
            cwd: armada_home.to_path_buf(),
            claude_home: armada_home.join("claude"),
        }
    }

    fn a_guild_behind_by_two(armada_home: &std::path::Path) -> FakeGit {
        std::fs::create_dir_all(armada_home.join("guild").join(".git")).unwrap();
        FakeGit(RefCell::new(vec![
            Ok((
                true,
                "git@example.com:me/guild.git\n".to_string(),
                String::new(),
            )),
            Ok((true, String::new(), String::new())), // fetch
            Ok((true, "2\t0\n".to_string(), String::new())), // rev-list --left-right --count: behind, ahead
        ]))
    }

    /// **The done-when.** Not eligible — piped, `--json`, or inside a Job —
    /// never even reads `machine.yml`, let alone runs `git`: the fake would
    /// panic on an unscripted call if it tried, and it does not panic.
    #[test]
    fn an_ineligible_invocation_touches_neither_git_nor_the_clock() {
        let home = tempfile::tempdir().unwrap();
        let run = FakeGit::default();
        let mut ask = Scripted::default();
        for (json, can_ask, in_job) in [
            (true, true, false),
            (false, false, false),
            (false, true, true),
        ] {
            let terminal = if can_ask {
                Terminal::at(80)
            } else {
                Terminal::piped()
            };
            maybe_offer(
                &run,
                &FixedClock(0),
                &place(home.path()),
                &mut ask,
                Style::plain(),
                terminal,
                json,
                in_job,
            );
        }
        assert!(ask.chosen.is_empty());
    }

    /// A machine that has never checked looks at once, at a terminal, and —
    /// behind by two — puts the question with "not now" as the default, and
    /// records that it looked.
    #[test]
    fn a_fresh_machine_behind_by_two_is_asked_and_the_look_is_recorded() {
        let home = tempfile::tempdir().unwrap();
        let run = a_guild_behind_by_two(home.path());
        let mut ask = Scripted::default();

        maybe_offer(
            &run,
            &FixedClock(1_000),
            &place(home.path()),
            &mut ask,
            Style::plain(),
            Terminal::at(80),
            false,
            false,
        );

        assert_eq!(ask.chosen.len(), 1, "{:?}", ask.chosen);
        assert!(
            ask.chosen[0].0.contains("2 commits behind"),
            "{}",
            ask.chosen[0].0
        );
        assert_eq!(offer::last_offer_ms(home.path()), Some(1_000));
    }

    /// **The hard rule, exercised rather than only stated**: answering
    /// nothing — the default a piped stdin, an `esc`, or an unreadable line
    /// all take — must not run `armada guild pull`. The fake `git` has no
    /// more scripted calls after the check itself, so a pull attempted here
    /// panics the test with an empty-queue error.
    #[test]
    fn taking_the_default_never_pulls() {
        let home = tempfile::tempdir().unwrap();
        let run = a_guild_behind_by_two(home.path());
        let mut ask = Scripted::default(); // no choice queued: takes the default

        maybe_offer(
            &run,
            &FixedClock(1_000),
            &place(home.path()),
            &mut ask,
            Style::plain(),
            Terminal::at(80),
            false,
            false,
        );
        // Reaching here without the fake `git`'s queue underflowing *is* the
        // proof: nothing asked it for a fifth call.
    }

    /// Choosing "pull" runs the closure, and only choosing it does —
    /// [`ask_and_maybe_pull`] is tested directly here so this does not have to
    /// reproduce `armada guild pull`'s own call sequence, which already has
    /// its own tests.
    #[test]
    fn choosing_pull_runs_the_pull_closure_and_shows_what_it_did() {
        let mut ask = Scripted {
            choice: Some(2),
            ..Scripted::default()
        };
        let mut calls = 0;
        ask_and_maybe_pull(&mut ask, 2, Style::plain(), Terminal::at(80), || {
            calls += 1;
            Ok(Output::GuildSync(Box::new(Envelope::ok(
                "guild pull",
                None,
                Status::Ready,
                GuildSyncData {
                    remote: Some("git@example.com:me/guild.git".to_string()),
                    ahead: 0,
                    behind: 2,
                    results: Vec::new(),
                    applied: true,
                    headline: None,
                    projected: None,
                },
            ))))
        });

        assert_eq!(calls, 1, "choosing pull must run the pull exactly once");
        assert_eq!(ask.shown.len(), 1, "what the pull did must be shown");
    }

    /// Not choosing "pull" never runs the closure — the same hard rule
    /// [`taking_the_default_never_pulls`] exercises end to end, isolated here
    /// to the one function that makes the choice.
    #[test]
    fn not_choosing_pull_never_runs_the_closure() {
        let mut ask = Scripted::default(); // takes NOT_NOW, the default
        let mut calls = 0;
        ask_and_maybe_pull(&mut ask, 2, Style::plain(), Terminal::at(80), || {
            calls += 1;
            unreachable!("must not be called")
        });
        assert_eq!(calls, 0);
    }

    /// Offline is recorded, and nothing is asked — never a warning on every
    /// invocation (`docs/reserved/009` item 4's hard rule).
    #[test]
    fn offline_is_recorded_and_silent() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("guild").join(".git")).unwrap();
        let run = FakeGit(RefCell::new(vec![Err("no network".to_string())]));
        let mut ask = Scripted::default();

        maybe_offer(
            &run,
            &FixedClock(1_000),
            &place(home.path()),
            &mut ask,
            Style::plain(),
            Terminal::at(80),
            false,
            false,
        );

        assert!(ask.chosen.is_empty());
        assert_eq!(offer::last_offer_ms(home.path()), Some(1_000));
    }

    /// Checked a moment ago: not due to fire again, and in between it must
    /// not touch `git` at all.
    #[test]
    fn not_due_yet_never_calls_git() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("guild").join(".git")).unwrap();
        offer::record(home.path(), 1_000).unwrap();
        let run = FakeGit::default();
        let mut ask = Scripted::default();

        maybe_offer(
            &run,
            &FixedClock(1_500),
            &place(home.path()),
            &mut ask,
            Style::plain(),
            Terminal::at(80),
            false,
            false,
        );

        assert!(ask.chosen.is_empty());
    }
}
