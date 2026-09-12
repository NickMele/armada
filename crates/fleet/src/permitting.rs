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

pub use holding::{domain_setting, wire_setting, NotPermitted};

use core_model::{AllowedCommand, Reach, StepId, Timestamp, WhenBlocked};
use ipc::CommandAnswer;
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answered {
    /// Allowed, this far.
    Allowed(Reach),
    /// Rejected, with the person's words where they wrote any.
    Rejected(Option<Note>),
}

impl Answered {
    /// A person's answer, read off the wire's three offers and the note beside
    /// them. **A note sent with an allow is dropped here**, at the one place
    /// that decides which of the two an answer is.
    pub fn of(answer: CommandAnswer, note: Option<&str>) -> Answered {
        match answer {
            CommandAnswer::AllowForJob => Answered::Allowed(Reach::Job),
            CommandAnswer::AlwaysAllow => Answered::Allowed(Reach::Repository),
            CommandAnswer::Reject => Answered::Rejected(note.and_then(Note::saying)),
        }
    }

    /// The same answer as one of the three offers, for the check that it was
    /// offered and the log line that records what was said. **Derived rather
    /// than carried**, so the two cannot come to disagree.
    pub fn answer(&self) -> CommandAnswer {
        match self {
            Answered::Allowed(Reach::Job) => CommandAnswer::AllowForJob,
            Answered::Allowed(Reach::Repository) => CommandAnswer::AlwaysAllow,
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
            Withheld::NotACommand { tool } => {
                format!("{tool} is a tool, and only a command can be allowed from here")
            }
            Withheld::Ungrantable { why } => format!("no job can be granted this: {why}"),
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
    if let Some(why) = ungrantable {
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
            Refusing::Rejected { note } => rejected(what, note.as_ref()),
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
         Carry on without it if the task allows, or ask a question if it cannot be done \
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
    fn a_tool_that_is_not_a_command_is_withheld_unless_everything_is_allowed() {
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
        assert_eq!(
            first(
                "Bash",
                Some("npm publish"),
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
            first("WebFetch", None, &[], &[], None, WhenBlocked::AllowAll),
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
                Some("it would push".to_string()),
                WhenBlocked::AllowAll
            ),
            First::Withheld(Withheld::Ungrantable {
                why: "it would push".to_string()
            })
        );
    }
}
