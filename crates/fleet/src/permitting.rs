//! A Drone reaching for a call outside its toolbelt, and what Armada answers.
//!
//! **Every such call reaches Armada.** A Drone spawns in the asking mode with
//! Armada's permission tool, so the answer is Fleet's and the Job's setting
//! makes it: [`WhenBlocked::RefuseAndHold`] refuses at once, and
//! [`WhenBlocked::AskMe`] puts the call to a person.
//!
//! **Only a command can be allowed from here.** `armada.yml` declares commands
//! and nothing else, so a tool outside the toolbelt is refused whatever the
//! setting, and its row says why.
//!
//! This half decides and words, and holds nothing. What a Drone is told is
//! drafted in `docs/contracts/agent-prompt.md`, under the permission answer and
//! the permission turn.

mod holding;

pub use holding::{domain_setting, wire_setting, NotPermitted};

use core_model::{AllowedCommand, Reach, StepId, Timestamp, WhenBlocked};
use tokio::sync::oneshot;

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
    pub reply: Option<oneshot::Sender<ipc::CommandAnswer>>,
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
            Withheld::Destructive { name } => format!(
                "armada.yml declares this destructive, as `{name}`, and no task runs it unattended"
            ),
            Withheld::NotACommand { tool } => {
                format!("{tool} is a tool, and only a command can be allowed from here")
            }
            Withheld::Ungrantable { why } => format!("no task can be granted this: {why}"),
        }
    }
}

/// The answer before anyone is asked.
///
/// `destructive` is every destructive command the Manifest declares, as
/// `(name, run)`, and `ungrantable` is the harness's refusal of this command
/// where it has one — both handed in, so the answer is a function of what it
/// is given.
pub fn first(
    tool: &str,
    command: Option<&str>,
    allowed: &[AllowedCommand],
    destructive: &[(String, String)],
    ungrantable: Option<String>,
    when: WhenBlocked,
) -> First {
    let Some(command) = command else {
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
    if let Some(why) = ungrantable {
        return First::Withheld(Withheld::Ungrantable { why });
    }
    match when {
        WhenBlocked::RefuseAndHold => First::NotGranted,
        WhenBlocked::AskMe => First::Ask,
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
    /// A person said no.
    Rejected,
    Withheld(Withheld),
}

impl Refusing {
    /// The tool's reply. `what` is the command, or the tool where there is none.
    pub fn to_the_drone(&self, what: &str) -> String {
        match self {
            Refusing::NotGranted => format!(
                "This task is not granted `{what}`. A person decides whether to allow it. \
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
            Refusing::Rejected => rejected(what),
            Refusing::Withheld(Withheld::Destructive { .. }) => format!(
                "`{what}` is declared destructive in this repository, and an unattended task \
                 never runs it. Do not try to get the same result another way."
            ),
            Refusing::Withheld(Withheld::Ungrantable { why }) => format!(
                "`{what}` cannot be granted to a task: {why}. Do not try to get the same \
                 result another way."
            ),
            Refusing::Withheld(Withheld::NotACommand { .. }) => format!(
                "This task is not granted `{what}`. Do not try to get the same result another \
                 way."
            ),
        }
    }
}

fn rejected(what: &str) -> String {
    format!(
        "A person said no to `{what}`. Do not run it, or anything that does the same thing. \
         Carry on without it if the task allows, or ask a question if it cannot be done \
         without it."
    )
}

/// A person's answer to a permission question, as a turn, arriving after the
/// call it was about has returned.
///
/// **No constructor takes free text**, the property `crate::questioning::Answer`
/// has and for its reason: the words are Fleet's, around a command the Drone
/// itself ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permitted(String);

impl Permitted {
    pub fn allowed(command: &str, reach: Reach) -> Permitted {
        Permitted(match reach {
            Reach::Job => format!(
                "A person allowed `{command}` for this task. Run it again now; it will not be \
                 refused."
            ),
            Reach::Repository => format!(
                "A person allowed `{command}` in this repository. It is declared in armada.yml \
                 on your branch, in a commit of its own; leave that change as it is. Run the \
                 command again now; it will not be refused."
            ),
        })
    }

    pub fn rejected(command: &str) -> Permitted {
        Permitted(rejected(command))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use core_model::{Actor, AllowedCommand, Reach, Timestamp, WhenBlocked};

    use super::{covers, first, First, Withheld};

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
    fn a_tool_that_is_not_a_command_is_withheld_whatever_the_setting() {
        for when in [WhenBlocked::RefuseAndHold, WhenBlocked::AskMe] {
            assert_eq!(
                first("WebFetch", None, &[], &[], None, when),
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
                None,
                WhenBlocked::AskMe
            ),
            First::Ask
        );
    }
}
