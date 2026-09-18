//! A Drone reaching for a call outside its toolbelt, and what Armada answers.
//!
//! **Every such call reaches Armada.** A Drone spawns in the asking mode with
//! Armada's permission tool, so the answer is Fleet's and the Job's setting
//! makes it: [`WhenBlocked::RefuseAndHold`] refuses at once,
//! [`WhenBlocked::AskMe`] puts the call to a person, and
//! [`WhenBlocked::AllowAll`] allows it.
//!
//! **Only a command can be allowed from here.** `armada.yml` declares commands
//! and nothing else, so a tool outside the toolbelt is refused under the first
//! two settings, and its row says why. Allow all is a person having already
//! said yes to every call, so a tool is allowed there too.
//!
//! **What `armada.yml` withholds stays withheld under every setting**: a
//! destructive command, and one the harness cannot grant.
//!
//! This half decides and words, and holds nothing. What a Drone is told is
//! drafted in `docs/contracts/agent-prompt.md`, under the permission answer and
//! the permission turn.

mod holding;
mod repository;

pub use holding::{domain_setting, wire_setting, NotPermitted};

use std::time::Duration;

use adapter_traits::PERMISSION_WAIT;
use core_model::{AllowedCommand, Reach, StepId, Timestamp, WhenBlocked};
use ipc::CommandAnswer;
use tokio::sync::oneshot;

/// How long a question is held open for a person: a minute under what the
/// harness waits, so the answer the Drone gets is Fleet's and says why.
pub const HOLD: Duration = Duration::from_secs(PERMISSION_WAIT.as_secs() - 60);

/// How long [`Fleet::permission`](crate::Fleet::permission) holds a question
/// inside the Drone's call before telling it to wait for a turn. The shipped
/// value is [`HOLD`], which the composition root writes out.
///
/// **A fitting rather than the constant it wraps**, so a test can plant a hold
/// it can outlive: four real minutes is not a wait any case can make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PermissionHold(Duration);

impl PermissionHold {
    pub fn of(hold: Duration) -> PermissionHold {
        PermissionHold(hold)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// How long a permission ask may go unanswered before Fleet gives up waiting
/// for a person and reclaims the Job's concurrency slot: ends the Drone,
/// stops the step and escalates the Job as `ask_unanswered`. `#801`.
///
/// **Beside [`PermissionHold`] and not derived from it**, because the two
/// bound the same wait from different vantage points. That one is how long
/// the harness's own HTTP call is held open before the Drone is told to wait
/// for a turn instead — a few minutes, fixed by what the harness itself will
/// wait. This is how long the Job's own slot is held for an answer that may
/// never come, which is a person's patience rather than a process's, and runs
/// on long after the call itself has returned a deny.
///
/// **A fitting rather than a bare constant**, for [`PermissionHold`]'s own
/// reason: a test can plant a limit it can outlive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnansweredAskLimit(Duration);

impl UnansweredAskLimit {
    pub fn of(limit: Duration) -> UnansweredAskLimit {
        UnansweredAskLimit(limit)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// A permission question a person has been asked, about a call this Drone
/// made, and not answered yet.
///
/// **Held on the slot and written to no column**, for
/// `crate::questioning::Question`'s reason: a Fleet that restarts loses the
/// Drone whose call it was.
pub struct Waiting {
    /// The harness's id for the call. What a person's answer names, so an
    /// answer from a window left open across a newer question joins to nothing.
    pub call: String,
    pub step: StepId,
    pub asked_at: Timestamp,
    pub tool: String,
    pub command: String,
    /// Where the answer goes while the tool call is still held open.
    ///
    /// **`None` once the hold has ended**, the Drone having been told to wait
    /// for a turn: the answer then arrives as a [`Permitted`] turn instead.
    ///
    /// It carries [`Answered`] rather than `ipc::CommandAnswer` since 11.5,
    /// because a reject may carry the person's own words and the words have to
    /// reach the Drone inside the call it is still holding open.
    pub reply: Option<oneshot::Sender<Answered>>,
}

/// What a person answered about one command, and their words where a reject
/// carried any.
///
/// **An allow has no field for a note and that is the whole shape of it.** A
/// note is only ever read on a reject — an allow needs no reason, and a Drone
/// told why it was allowed learns nothing it can act on — so the case where one
/// could be carried and dropped is not representable rather than checked for.
///
/// **The rule rides the reach, and only reach matters for what it means.** A
/// person allowing for the job only ever allows the command they ran, so the
/// second field is `None` there by construction; an always-allow may name one
/// of [`always_allow_rules`]'s candidates instead, and `None` there keeps the
/// pre-13.4 meaning — the whole command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answered {
    /// Allowed, this far, and — for [`Reach::Repository`] — the rule a person
    /// chose, where they named one.
    Allowed(Reach, Option<String>),
    /// Rejected, with the person's words where they wrote any.
    Rejected(Option<Note>),
}

impl Answered {
    /// [`Answered::naming`], with no rule — every answer but Always allow, and
    /// an Always allow read off a Fleet from before `#834` that never sent one.
    pub fn of(answer: CommandAnswer, note: Option<&str>) -> Answered {
        Answered::naming(answer, note, None)
    }

    /// A person's answer, read off the wire's three offers, the note and the
    /// rule beside them. **A note sent with an allow, or a rule sent with
    /// anything but Always allow, is dropped here** — the one place that
    /// decides which of the three an answer is.
    pub fn naming(answer: CommandAnswer, note: Option<&str>, rule: Option<&str>) -> Answered {
        match answer {
            CommandAnswer::AllowForJob => Answered::Allowed(Reach::Job, None),
            CommandAnswer::AlwaysAllow => {
                Answered::Allowed(Reach::Repository, rule.map(str::to_string))
            }
            CommandAnswer::Reject => Answered::Rejected(note.and_then(Note::saying)),
        }
    }

    /// The same answer as one of the three offers, for the check that it was
    /// offered and the log line that records what was said. **Derived rather
    /// than carried**, so the two cannot come to disagree.
    pub fn answer(&self) -> CommandAnswer {
        match self {
            Answered::Allowed(Reach::Job, _) => CommandAnswer::AllowForJob,
            Answered::Allowed(Reach::Repository, _) => CommandAnswer::AlwaysAllow,
            Answered::Rejected(_) => CommandAnswer::Reject,
        }
    }
}

/// Why a person said no, in their own words.
///
/// **There is no way to make an empty one.** A field somebody opened and typed
/// nothing into is the same request as no note at all — `redirect_drone`'s rule
/// about a blank note, which this borrows rather than restates — and a heading
/// with nothing under it is what a Drone would otherwise be handed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note(String);

impl Note {
    /// The note, or `None` where there is nothing in it. Trimmed, because the
    /// whitespace is a person's keystrokes and not their reason.
    pub fn saying(note: &str) -> Option<Note> {
        let note = note.trim();
        match note.is_empty() {
            true => None,
            false => Some(Note(note.to_string())),
        }
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What the permission tool answers before anybody is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum First {
    /// A person already allowed this for the Job.
    Allowed,
    /// It cannot be allowed here, whatever a person would say.
    Withheld(Withheld),
    /// The Job refuses and holds.
    NotGranted,
    /// The Job asks, so a person will be.
    Ask,
}

/// Why a call cannot be allowed from Job detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Withheld {
    /// `armada.yml` declares it destructive, under this name.
    Destructive { name: String },
    /// It runs a program one of the Manifest's Checks runs, so Fleet already
    /// runs it under `run_checks` and a Drone reaching past that is spending
    /// its own turns on work it can ask for. **Withheld under every
    /// `when_blocked`**, for [`Withheld::Destructive`]'s reason: no person's
    /// setting widens what the repository already declares. #1174.
    RunsACheck { check: String, program: String },
    /// A tool rather than a command.
    NotACommand { tool: String },
    /// The harness could not grant it: a push, or a command its rules cannot
    /// express. `why` is the harness's own sentence.
    Ungrantable { why: String },
}

impl Withheld {
    /// What a person reads beside the refused row.
    pub fn reason(&self) -> String {
        match self {
            // A person's sentence, drawn as plain text on the row: "job" as
            // Bridge says it, and no Markdown marks for it to show literally.
            Withheld::Destructive { name } => format!(
                "armada.yml declares this destructive, as {name}, and no job runs it unattended"
            ),
            Withheld::RunsACheck { check, program } => format!(
                "{program} is what the {check} check runs, and fleet runs the checks itself"
            ),
            Withheld::NotACommand { tool } => {
                format!("{tool} is a tool, and only a command can be allowed from here")
            }
            Withheld::Ungrantable { why } => format!("no job can be granted this: {why}"),
        }
    }
}

/// The answer before anyone is asked.
///
/// `not_runnable` is the harness's refusal of this command **on this call**,
/// where it has one — never its refusal to write a standing rule for it, which
/// is a different question and `#1420` is where they came apart.
///
/// `destructive` is every destructive command the Manifest declares, as
/// `(name, run)`, `check_runners` is every Check it declares as
/// `(check name, program)`, and `ungrantable` is the harness's refusal of this
/// command where it has one — all handed in, so the answer is a function of
/// what it is given.
pub fn first(
    tool: &str,
    command: Option<&str>,
    allowed: &[AllowedCommand],
    destructive: &[(String, String)],
    check_runners: &[(String, String)],
    not_runnable: Option<String>,
    when: WhenBlocked,
) -> First {
    let Some(command) = command else {
        // Nothing below can withhold a tool: destructive and ungrantable are
        // both about a command's text.
        if when == WhenBlocked::AllowAll {
            return First::Allowed;
        }
        return First::Withheld(Withheld::NotACommand {
            tool: tool.to_string(),
        });
    };
    if allowed.iter().any(|allow| covers(&allow.run, command)) {
        return First::Allowed;
    }
    if let Some((name, _)) = destructive.iter().find(|(_, run)| covers(run, command)) {
        return First::Withheld(Withheld::Destructive { name: name.clone() });
    }
    // **Above the `when` match, which is what makes it hold under allow all.**
    // `#737` put the pointer inside `Refusing::NotGranted`, so a Job set to
    // allow everything never built one and its Drone was told nothing. On
    // 17 Sep that Job ran eighteen build commands by hand, one of which took
    // the machine to a load average of 73. #1174.
    if let Some((check, program)) = runs_a_check(command, check_runners) {
        return First::Withheld(Withheld::RunsACheck {
            check: check.clone(),
            program: program.clone(),
        });
    }
    if let Some(why) = not_runnable {
        return First::Withheld(Withheld::Ungrantable { why });
    }
    match when {
        WhenBlocked::RefuseAndHold => First::NotGranted,
        WhenBlocked::AskMe => First::Ask,
        WhenBlocked::AllowAll => First::Allowed,
    }
}

/// Whether allowing `run` allows `command`: the command itself, or it with
/// more plain arguments, which is the prefix the harness renders a grant as.
///
/// **Nothing that chains another command is covered.** An allow of `npm
/// publish` must not become an allow of `npm publish && curl …`, so arguments
/// carrying a shell operator make a different command.
pub fn covers(run: &str, command: &str) -> bool {
    let (run, command) = (run.trim(), command.trim());
    if run.is_empty() {
        return false;
    }
    if command == run {
        return true;
    }
    command
        .strip_prefix(run)
        .is_some_and(|rest| rest.starts_with(char::is_whitespace) && !chains(rest))
}

fn chains(arguments: &str) -> bool {
    arguments.contains(['&', '|', ';', '<', '>', '`', '\n', '\r']) || arguments.contains("$(")
}

/// Candidate Always-allow rules for `command`, shortest first, and which one
/// is pre-selected — offered so a person picks the rule rather than typing it,
/// per `#834`.
///
/// **The candidates are the leading runs of whitespace-separated words**,
/// stopping before the first word [`chains`] would refuse as an argument:
/// `covers` never lets a chained word ride on an allow, so no candidate ever
/// could either. **Empty where `command` is empty or begins with such a
/// word** — there is nothing here safe to always-allow.
///
/// **The suggestion is always one of the candidates.** It stops growing at the
/// first word that does not look like a program or a subcommand — a flag, a
/// number, a path, anything [`looks_like_a_subcommand`] refuses — which is
/// usually the first argument proper. `cargo test -p foo` suggests `cargo
/// test`; `gh issue view 792 --repo X` suggests `gh issue view`.
pub fn always_allow_rules(command: &str) -> (Vec<String>, Option<String>) {
    let command = command.trim();
    let mut rules = Vec::new();
    let mut suggested = None;
    let mut still_a_subcommand = true;
    let mut word_start = None;
    for (at, ch) in command.char_indices().chain([(command.len(), ' ')]) {
        match (word_start, ch.is_whitespace()) {
            (None, false) => word_start = Some(at),
            (Some(start), true) => {
                let word = &command[start..at];
                if chains(word) {
                    break;
                }
                rules.push(command[..at].to_string());
                still_a_subcommand = still_a_subcommand && looks_like_a_subcommand(word);
                if still_a_subcommand {
                    suggested = rules.last().cloned();
                }
                word_start = None;
            }
            _ => {}
        }
    }
    (rules, suggested)
}

/// Whether `word` could name a program or a subcommand rather than an
/// argument: lowercase, digits, `-`, `_` or `.`, never leading with `-`, never
/// only digits, and never carrying `/` or `=` — the shapes a flag, a number, a
/// path and a `key=value` pair take and a subcommand does not.
fn looks_like_a_subcommand(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && !word.contains(['/', '='])
        && !word.chars().all(|c| c.is_ascii_digit())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

/// Why the permission tool said no, for the words the Drone reads inside the
/// call it made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusing {
    NotGranted,
    /// A person is being asked and has not answered inside the hold.
    Asked,
    /// A person is being asked about another of this Drone's calls.
    AlreadyAsking {
        other: String,
    },
    /// A person said no, and said why where they wrote anything.
    Rejected {
        note: Option<Note>,
    },
    Withheld(Withheld),
    /// Nobody answered before the ask's own bound ran out. The Job has
    /// escalated and this run is ending.
    Unanswered,
}

/// The Check whose runner `command` reaches for, where it reaches for one.
///
/// **Derived from the Manifest rather than named here.** `#737` shipped a
/// hardcoded list of nine programs — `cargo`, `pnpm`, `npm` and six more — and
/// a repository built with `bun` or `gradle` got nothing from it. Every Check's
/// `run`, its `narrow.run` and its `one_test.run` already name the programs
/// this repository's Checks use, so the list is the file's and nobody
/// maintains a second copy.
///
/// **Every token, not the first.** On 17 Sep a Drone reached
/// `npx --yes pnpm@11.6.0 --filter @armada/desktop test`, whose first word is
/// `npx`, after finding `pnpm` was not on its PATH. `bash -c "cargo test"` is
/// the same hole. A token starting with `-` is skipped, so a flag that happens
/// to spell a program name is not one.
fn runs_a_check<'a>(
    command: &str,
    runners: &'a [(String, String)],
) -> Option<&'a (String, String)> {
    let typed: Vec<&str> = command
        .split_whitespace()
        .filter(|token| !token.starts_with('-'))
        .collect();
    // **The Check whose own line this command looks most like.** Matching on
    // the program alone named the first Check declaring that program, so on
    // this repository every `cargo` command was reported as `build` — and a
    // Drone told to ask for `build` when it typed `cargo nextest` would have
    // been told nothing about its tests. Seen for real on job
    // `3-show-what-s-running-in-the-drones-stat`.
    // **Strictly greater, so a tie goes to the Check declared first.**
    // `max_by_key` answers the last of equal keys, which made a command
    // sharing only its program with three Checks report whichever the file
    // happened to write last.
    let mut best: Option<(usize, &(String, String))> = None;
    for runner in runners {
        let shared = shared_prefix(&typed, &runner.1);
        if shared > 0 && best.is_none_or(|(most, _)| shared > most) {
            best = Some((shared, runner));
        }
    }
    if let Some((_, runner)) = best {
        return Some(runner);
    }
    // **A runner reached through a wrapper shares no prefix at all.**
    // `npx --yes pnpm@11.6.0 … test` starts with `npx`, so nothing above can
    // see it; this names the Check by the program hiding further along, which
    // is the hole #1174 was filed for.
    typed.iter().find_map(|token| {
        let named = program_named(token);
        runners
            .iter()
            .find(|(_, run)| run.split_whitespace().next().map(program_named) == Some(named))
    })
}

/// How many leading words `typed` and `run` name in common, comparing the
/// program each word names rather than the word. Zero where the first differs.
fn shared_prefix(typed: &[&str], run: &str) -> usize {
    let declared: Vec<&str> = run
        .split_whitespace()
        .filter(|token| !token.starts_with('-'))
        .collect();
    typed
        .iter()
        .zip(declared.iter())
        .take_while(|(one, other)| program_named(one) == program_named(other))
        .count()
}

/// A token reduced to the program it names: the last path segment, and what
/// stands before a version suffix. `/usr/bin/cargo` and `pnpm@11.6.0` are
/// `cargo` and `pnpm`.
fn program_named(token: &str) -> &str {
    let last = token.rsplit('/').next().unwrap_or(token);
    last.split('@').next().unwrap_or(last)
}

impl Refusing {
    /// The tool's reply. `what` is the command, or the tool where there is none.
    pub fn to_the_drone(&self, what: &str) -> String {
        match self {
            // **`#737`'s pointer moved to `Withheld::RunsACheck`.** It lived
            // here, reachable only by building a refusal, so a Job set to allow
            // everything never fired it. A check runner is now withheld before
            // the setting is consulted at all, which is where the sentence
            // belongs. #1174.
            Refusing::NotGranted => format!(
                "This Job is not granted `{what}`. A person decides whether to allow it. \
                 Do not try to get the same result another way."
            ),
            Refusing::Asked => format!(
                "A person has been asked whether you may run `{what}`, and has not answered \
                 yet. Stop and wait: the answer arrives as your next turn. Do not try another \
                 way to do the same thing meanwhile."
            ),
            Refusing::AlreadyAsking { other } => format!(
                "A person is being asked whether you may run `{other}`. Wait for that answer, \
                 which arrives as your next turn, before reaching for `{what}`."
            ),
            Refusing::Rejected { note } => rejected(what, note.as_ref()),
            Refusing::Withheld(Withheld::RunsACheck { check, .. }) => format!(
                "This Job is not granted `{what}`. It runs what the `{check}` check runs, and \
                 Fleet runs this part's checks itself under `run_checks` — name `{check}` and \
                 the files you changed and the answer comes back in seconds, and asking that \
                 way is not counted against you. Ask for that instead of running the command \
                 yourself."
            ),
            Refusing::Withheld(Withheld::Destructive { .. }) => format!(
                "`{what}` is declared destructive in this repository, and an unattended Job \
                 never runs it. Do not try to get the same result another way."
            ),
            Refusing::Withheld(Withheld::Ungrantable { why }) => format!(
                "`{what}` cannot be granted to a Job: {why}. Do not try to get the same \
                 result another way."
            ),
            Refusing::Withheld(Withheld::NotACommand { .. }) => format!(
                "This Job is not granted `{what}`. Do not try to get the same result another \
                 way."
            ),
            Refusing::Unanswered => format!(
                "Nobody answered whether you may run `{what}` before the job's own limit on \
                 waiting for a person ran out. The job has escalated and this run is ending."
            ),
        }
    }
}

/// The refusal a person's no becomes, and their reason under it where they
/// gave one.
///
/// **Fleet's sentence is unchanged and the note is attributed.** The two are
/// separated and the person's words are introduced as theirs, on their own
/// lines and unquoted — a Drone that read them as Armada's would treat one
/// person's reason as a standing rule, and a note carrying a backtick or a
/// newline would otherwise run into the sentence around it.
fn rejected(what: &str, note: Option<&Note>) -> String {
    let refusal = format!(
        "A person said no to `{what}`. Do not run it, or anything that does the same thing. \
         Carry on without it if the work allows, or ask a question if it cannot be done \
         without it."
    );
    match note {
        None => refusal,
        Some(note) => format!(
            "{refusal}\n\nThis is what the person said, in their own words:\n\n{}",
            note.text()
        ),
    }
}

/// A person's answer to a permission question, as a turn, arriving after the
/// call it was about has returned.
///
/// **One constructor takes free text, and only on a reject.** It did not until
/// 11.5, on `crate::questioning::Answer`'s property and for its reason — the
/// words were Fleet's, around a command the Drone itself ran. What changed is
/// where the reason lives: it is in a person's head exactly when they press
/// reject, and telling them to reject here and redirect from another box spends
/// it. So [`Permitted::rejected`] takes a [`Note`], which cannot be empty, and
/// [`Permitted::allowed`] still takes none — an allow needs no reason, and the
/// text around both is still Fleet's with the person's words attributed inside
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permitted(String);

impl Permitted {
    /// `rule` is read only where `reach` is [`Reach::Repository`], and is what
    /// a person kept for the repository — the whole command, where they named
    /// no rule of its own. **Named apart from `command`** because the two can
    /// now differ: what is kept may be a cut of what the Drone ran, and a
    /// Drone told only the cut would have nothing telling it to run the
    /// command it actually reached for.
    pub fn allowed(command: &str, reach: Reach, rule: Option<&str>) -> Permitted {
        Permitted(match reach {
            Reach::Job => format!(
                "A person allowed `{command}` for this Job. Run it again now; it will not be \
                 refused."
            ),
            Reach::Repository => {
                let declared = rule.unwrap_or(command);
                format!(
                    "A person allowed `{declared}` in this repository. Run `{command}` again \
                     now; it will not be refused."
                )
            }
        })
    }

    pub fn rejected(command: &str, note: Option<&Note>) -> Permitted {
        Permitted(rejected(command, note))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use core_model::{Actor, AllowedCommand, Reach, Timestamp, WhenBlocked};

    use super::{always_allow_rules, covers, first, First, Refusing, Withheld};

    /// The Manifest of a repository whose `format` check narrows to `rustfmt`,
    /// which is the case a list built from `run` alone would miss.
    /// This repository's own shape: several Checks on one program, `build`
    /// declared first, and a `narrow.run` naming a program its `run` does not.
    fn runners() -> Vec<(String, String)> {
        vec![
            (
                "build".to_string(),
                "cargo build --workspace --locked".to_string(),
            ),
            (
                "test".to_string(),
                "cargo nextest run --workspace --exclude acceptance".to_string(),
            ),
            (
                "screens_test".to_string(),
                "pnpm --dir packages/screens exec vitest run".to_string(),
            ),
            ("format".to_string(), "cargo fmt --all --check".to_string()),
            (
                "format".to_string(),
                "rustfmt --check --edition 2021".to_string(),
            ),
        ]
    }

    /// **Seen for real on job `3-show-what-s-running-in-the-drones-stat`.**
    /// Matching on the program alone named the first Check declaring it, so
    /// every `cargo` command was reported as `build` — and a Drone told to ask
    /// for `build` when it typed `cargo nextest` would have been told nothing
    /// about its tests.
    #[test]
    fn the_refusal_names_the_check_the_command_most_looks_like() {
        let named = |command: &str| {
            let decided = first(
                "Bash",
                Some(command),
                &[],
                &[],
                &runners(),
                None,
                WhenBlocked::AllowAll,
            );
            match decided {
                First::Withheld(Withheld::RunsACheck { check, .. }) => check,
                other => panic!("{command}: {other:?}"),
            }
        };
        assert_eq!(named("cargo nextest run -p ipc"), "test");
        assert_eq!(named("cargo build -p ipc"), "build");
        assert_eq!(named("cargo fmt --all --check"), "format");
        assert_eq!(named("rustfmt --check src/lib.rs"), "format");
        assert_eq!(
            named("pnpm --dir packages/screens exec vitest run overview"),
            "screens_test"
        );
        assert_eq!(
            named("npx --yes pnpm@11.6.0 --dir packages/screens exec vitest run"),
            "screens_test",
            "a wrapper shares no prefix, so the program further along names it"
        );
        assert_eq!(
            named("cargo check -p ipc"),
            "build",
            "nothing declares `cargo check`, so the nearest Check on that program takes it"
        );
    }

    /// **`#1174`, and the whole of why `#737` did not hold.** The pointer lived
    /// inside a refusal, so a Job set to allow everything built none and its
    /// Drone was told nothing — eighteen times in one step on 17 Sep. A check
    /// runner is withheld before the setting is read at all.
    #[test]
    fn a_check_runner_is_withheld_under_every_setting_including_allow_all() {
        for when in WhenBlocked::ALL {
            let decided = first(
                "Bash",
                Some("cargo check --tests -p fleet"),
                &[],
                &[],
                &runners(),
                None,
                *when,
            );
            assert!(
                matches!(
                    decided,
                    First::Withheld(Withheld::RunsACheck { ref check, .. }) if check == "build"
                ),
                "under {when:?}: {decided:?}"
            );
        }
    }

    /// The command that actually ran on 17 Sep, whose first word is `npx`.
    #[test]
    fn a_runner_reached_through_a_wrapper_and_a_version_is_still_caught() {
        for command in [
            "npx --yes pnpm@11.6.0 --filter @armada/desktop test",
            "corepack pnpm -C apps/desktop test",
            "/opt/homebrew/bin/cargo nextest run",
        ] {
            let decided = first(
                "Bash",
                Some(command),
                &[],
                &[],
                &runners(),
                None,
                WhenBlocked::AllowAll,
            );
            assert!(
                matches!(decided, First::Withheld(Withheld::RunsACheck { .. })),
                "`{command}` reached past the checks: {decided:?}"
            );
        }
    }

    /// A `narrow.run` names a program the `run` above it does not, and a Drone
    /// reaching for that one is reaching for the same check.
    #[test]
    fn a_program_only_a_narrowed_run_names_is_still_that_checks() {
        let decided = first(
            "Bash",
            Some("rustfmt --check --edition 2021 src/lib.rs"),
            &[],
            &[],
            &runners(),
            None,
            WhenBlocked::AllowAll,
        );
        assert!(
            matches!(
                decided,
                First::Withheld(Withheld::RunsACheck { ref check, .. }) if check == "format"
            ),
            "{decided:?}"
        );
    }

    /// A command no Check runs is not withheld, and under allow all it runs.
    /// Withholding what a repository never declared would be this rule reaching
    /// past what it is for.
    #[test]
    fn a_command_no_check_runs_is_untouched() {
        assert_eq!(
            first(
                "Bash",
                Some("git push origin HEAD"),
                &[],
                &[],
                &runners(),
                None,
                WhenBlocked::AllowAll,
            ),
            First::Allowed
        );
    }

    /// A flag that spells a program name is a flag.
    #[test]
    fn a_flag_is_never_read_as_a_program() {
        assert_eq!(
            first(
                "Bash",
                Some("git log --cargo --pnpm"),
                &[],
                &[],
                &runners(),
                None,
                WhenBlocked::AllowAll,
            ),
            First::Allowed
        );
    }

    /// The refusal says which check covers it and what to call instead — a
    /// Drone told only "no" has nothing to act on, which is `#737`'s point and
    /// still stands.
    #[test]
    fn the_refusal_names_the_check_and_the_call_to_make_instead() {
        let said = Refusing::Withheld(Withheld::RunsACheck {
            check: "test".to_string(),
            program: "cargo".to_string(),
        })
        .to_the_drone("cargo check --tests -p fleet");
        assert!(said.contains("run_checks"), "{said}");
        assert!(said.contains("`test`"), "it names the check: {said}");
        assert!(
            said.contains("not counted against you"),
            "a Drone that thinks asking is rationed will not ask: {said}"
        );
    }

    /// A command that is not one `run_checks` covers gets the plain refusal —
    /// naming a tool that does not do the same job would teach the wrong lesson.
    #[test]
    fn an_unrelated_refusal_still_names_no_tool() {
        let said = Refusing::NotGranted.to_the_drone("git push origin HEAD");
        assert!(
            !said.contains("run_checks"),
            "this command is not one of the checks: {said}"
        );
    }

    fn allowed(run: &str) -> AllowedCommand {
        AllowedCommand {
            run: run.to_string(),
            reach: Reach::Job,
            allowed_at: Timestamp::from_rfc3339("2026-09-11T12:00:00.000Z"),
            by: Actor::Human,
        }
    }

    #[test]
    fn an_allow_covers_the_command_and_plain_arguments_after_it() {
        assert!(covers("npm publish", "npm publish"));
        assert!(covers("npm publish", "npm publish --access public"));
        assert!(!covers("npm publish", "npm publisher"));
        assert!(!covers("npm publish --access public", "npm publish"));
    }

    #[test]
    fn an_allow_never_covers_a_second_command_chained_on() {
        for chained in [
            "npm publish && curl example.com",
            "npm publish; rm -rf .",
            "npm publish | sh",
            "npm publish $(cat token)",
            "npm publish `id`",
            "npm publish > out",
        ] {
            assert!(!covers("npm publish", chained), "{chained}");
        }
    }

    #[test]
    fn a_tool_that_is_not_a_command_is_withheld_unless_everything_is_allowed() {
        for when in [WhenBlocked::RefuseAndHold, WhenBlocked::AskMe] {
            assert_eq!(
                first("WebFetch", None, &[], &[], &[], None, when),
                First::Withheld(Withheld::NotACommand {
                    tool: "WebFetch".to_string()
                })
            );
        }
    }

    #[test]
    fn an_allow_answers_before_the_setting_is_read() {
        let allows = [allowed("npm publish")];
        assert_eq!(
            first(
                "Bash",
                Some("npm publish --tag next"),
                &allows,
                &[],
                &[],
                None,
                WhenBlocked::RefuseAndHold
            ),
            First::Allowed
        );
    }

    #[test]
    fn a_destructive_command_is_withheld_and_never_asked_about() {
        let destructive = [("reset".to_string(), "rm -rf .armada".to_string())];
        assert_eq!(
            first(
                "Bash",
                Some("rm -rf .armada"),
                &[],
                &destructive,
                &[],
                None,
                WhenBlocked::AskMe
            ),
            First::Withheld(Withheld::Destructive {
                name: "reset".to_string()
            })
        );
    }

    #[test]
    fn a_command_the_harness_cannot_grant_is_withheld_in_its_words() {
        assert_eq!(
            first(
                "Bash",
                Some("git push origin HEAD"),
                &[],
                &[],
                &[],
                Some("it would push".to_string()),
                WhenBlocked::AskMe
            ),
            First::Withheld(Withheld::Ungrantable {
                why: "it would push".to_string()
            })
        );
    }

    #[test]
    fn the_setting_decides_everything_else() {
        assert_eq!(
            first(
                "Bash",
                Some("npm publish"),
                &[],
                &[],
                &[],
                None,
                WhenBlocked::RefuseAndHold
            ),
            First::NotGranted
        );
        assert_eq!(
            first(
                "Bash",
                Some("npm publish"),
                &[],
                &[],
                &[],
                None,
                WhenBlocked::AskMe
            ),
            First::Ask
        );
        assert_eq!(
            first(
                "Bash",
                Some("npm publish"),
                &[],
                &[],
                &[],
                None,
                WhenBlocked::AllowAll
            ),
            First::Allowed
        );
    }

    /// A person who chose Allow all said yes to every call, tools included.
    #[test]
    fn allow_all_allows_a_tool_that_is_not_a_command() {
        assert_eq!(
            first("WebFetch", None, &[], &[], &[], None, WhenBlocked::AllowAll),
            First::Allowed
        );
    }

    /// **No setting widens what `armada.yml` withholds.** Destructive and
    /// ungrantable are refused under Allow all exactly as under the others.
    #[test]
    fn allow_all_still_withholds_destructive_and_ungrantable_commands() {
        let destructive = [("reset".to_string(), "rm -rf .armada".to_string())];
        assert_eq!(
            first(
                "Bash",
                Some("rm -rf .armada"),
                &[],
                &destructive,
                &[],
                None,
                WhenBlocked::AllowAll
            ),
            First::Withheld(Withheld::Destructive {
                name: "reset".to_string()
            })
        );
        assert_eq!(
            first(
                "Bash",
                Some("git push origin HEAD"),
                &[],
                &[],
                &[],
                Some("it would push".to_string()),
                WhenBlocked::AllowAll
            ),
            First::Withheld(Withheld::Ungrantable {
                why: "it would push".to_string()
            })
        );
    }

    /// **`#1420`.** A comma is the allowlist's own syntax, so no standing rule
    /// can be written for `sed -n '1,140' file` — and answering one permission
    /// question writes no rule, so the command runs. Under Allow all it runs
    /// without a person; under the other two the person is still asked or the
    /// Job still holds, which is what those settings mean.
    #[test]
    fn a_command_no_rule_can_be_written_for_is_answered_like_any_other() {
        assert_eq!(
            first(
                "Bash",
                Some("sed -n '1,140' f"),
                &[],
                &[],
                &[],
                None,
                WhenBlocked::AllowAll
            ),
            First::Allowed
        );
        assert_eq!(
            first(
                "Bash",
                Some("sed -n '1,140' f"),
                &[],
                &[],
                &[],
                None,
                WhenBlocked::AskMe
            ),
            First::Ask
        );
        assert_eq!(
            first(
                "Bash",
                Some("sed -n '1,140' f"),
                &[],
                &[],
                &[],
                None,
                WhenBlocked::RefuseAndHold
            ),
            First::NotGranted
        );
    }

    /// **The standing question still withholds, and `offers_after` is where it
    /// is asked.** That path hands `first` the harness's refusal to write a
    /// rule, under `AskMe`, so a person is offered no Allow for a command no
    /// rule can be written for — a row for one would fail the next spawn
    /// outright, since the render refuses the whole Drone.
    #[test]
    fn no_standing_allow_is_offered_for_a_command_no_rule_can_be_written_for() {
        assert_eq!(
            first(
                "Bash",
                Some("sed -n '1,140' f"),
                &[],
                &[],
                &[],
                Some("it holds `,`".to_string()),
                WhenBlocked::AskMe,
            ),
            First::Withheld(Withheld::Ungrantable {
                why: "it holds `,`".to_string()
            })
        );
    }

    /// **`#834`'s own example.** The pipe and the redirect are both in the one
    /// word `2>&1`, so the candidates run out there and never reach `| head
    /// -100` at all — and the suggestion stops two words earlier, at the first
    /// argument that is not a subcommand.
    #[test]
    fn the_gh_issue_example_stops_before_the_redirect_and_suggests_three_words() {
        let (rules, suggested) =
            always_allow_rules("gh issue view 792 --repo NickMele/armada 2>&1 | head -100");
        assert_eq!(
            rules,
            vec![
                "gh",
                "gh issue",
                "gh issue view",
                "gh issue view 792",
                "gh issue view 792 --repo",
                "gh issue view 792 --repo NickMele/armada",
            ]
        );
        assert_eq!(suggested.as_deref(), Some("gh issue view"));
    }

    /// A command with no operator at all offers every cut of itself, down to
    /// the whole thing, and suggests as far as the flag.
    #[test]
    fn a_command_with_no_operator_offers_every_cut() {
        let (rules, suggested) = always_allow_rules("npm publish --access public");
        assert_eq!(
            rules,
            vec![
                "npm",
                "npm publish",
                "npm publish --access",
                "npm publish --access public"
            ]
        );
        assert_eq!(suggested.as_deref(), Some("npm publish"));
    }

    /// A command whose very first word chains offers nothing — there is no
    /// cut of it that stops short of the operator.
    #[test]
    fn a_command_starting_with_an_operator_bearing_word_offers_nothing() {
        let (rules, suggested) = always_allow_rules("$(evil) rm -rf .");
        assert!(rules.is_empty(), "{rules:?}");
        assert_eq!(suggested, None);
    }

    /// `-p` is a flag, so the suggestion stops there and `foo` never grows it
    /// further — matching the issue's own second example.
    #[test]
    fn cargo_test_suggests_up_to_the_subcommand_and_no_further() {
        let (rules, suggested) = always_allow_rules("cargo test -p foo");
        assert_eq!(
            rules,
            vec!["cargo", "cargo test", "cargo test -p", "cargo test -p foo"]
        );
        assert_eq!(suggested.as_deref(), Some("cargo test"));
    }

    /// Empty is a command nothing can be always-allowed from.
    #[test]
    fn an_empty_command_offers_nothing() {
        let (rules, suggested) = always_allow_rules("   ");
        assert!(rules.is_empty(), "{rules:?}");
        assert_eq!(suggested, None);
    }
}
