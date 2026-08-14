//! Typed, attributed failures — the vocabulary every verb answers in.
//!
//! Owner: `ARCHITECTURE.md` §1.7 for the classes, §1.6 for the exit map. This
//! module is that table encoded once, so a second competing mapping has
//! nowhere to grow.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Which class of failure this is.
///
/// The enum covers **every** non-zero exit. A hole is where a second, competing
/// mapping grows back (`ARCHITECTURE.md` §1.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrClass {
    /// The command itself was wrong — unknown verb or flag. Fix the command.
    BadInvocation,
    /// `armada.yml` is wrong. Fix the config. `next_action` is required here.
    BadConfig,
    /// The underlying tool failed on its own terms. That is a real result.
    ToolFailed,
    /// Armada's own deadline elapsed.
    Timeout,
    /// Cancelled, or the run's holder died. Retryable.
    Aborted,
    /// The machine Armada runs on is broken. Fix the machine, retry unchanged.
    Environment,
    /// Armada broke. Stop; retrying will not help.
    ArmadaBug,
}

impl ErrClass {
    /// The process exit code for this class.
    ///
    /// **There is exactly one mapping: `exit = f(error.class)`, or 0 when there
    /// is no error** (`ARCHITECTURE.md` §1.6). Terminal state never determines
    /// it: `FAILED` is exit 3 when the config was wrong and exit 1 when the
    /// tests were. Signal-derived codes (130, 141) are the stated carve-out and
    /// have no class at all.
    pub const fn exit_code(self) -> u8 {
        match self {
            ErrClass::ToolFailed => 1,
            ErrClass::BadInvocation => 2,
            ErrClass::BadConfig => 3,
            ErrClass::Timeout => 4,
            ErrClass::Aborted => 5,
            ErrClass::Environment => 6,
            ErrClass::ArmadaBug => 70,
        }
    }

    /// Strictness, for aggregating `results[]` into the envelope's one error.
    ///
    /// `armada_bug > environment > bad_config > bad_invocation > timeout >
    /// aborted > tool_failed` (PLAN.md §3.1). The strictly-worse signal wins
    /// because acting on the milder one wastes the time the stricter one was
    /// reporting — a run containing a `TIMEOUT` exits 4, not 1, so a gate
    /// raises a deadline instead of hunting a broken test.
    ///
    /// **`bad_invocation` was missing from that list until phase 1 made this
    /// function total**, and the omission was not cosmetic: a `check` whose
    /// service is not running fails `bad_invocation` (phase 3), so a run
    /// mixing one of those with an ordinary test failure had no defined
    /// maximum. It ranks above `tool_failed` and `timeout` for the reason
    /// `bad_config` does — the caller has to change what they asked for before
    /// any other result means anything — and below `bad_config`, because a
    /// wrong config is wrong for every future invocation while a wrong
    /// invocation is wrong only for this one.
    pub const fn severity(self) -> u8 {
        match self {
            ErrClass::ToolFailed => 0,
            ErrClass::Aborted => 1,
            ErrClass::Timeout => 2,
            ErrClass::BadInvocation => 3,
            ErrClass::BadConfig => 4,
            ErrClass::Environment => 5,
            ErrClass::ArmadaBug => 6,
        }
    }
}

impl fmt::Display for ErrClass {
    /// The documented spelling, which is the one the `--json` payload uses.
    ///
    /// Derived `Debug` would print `BadConfig`, so a human reading the terminal
    /// and an agent reading the envelope would see two different vocabularies
    /// for one enum — and the project's whole thesis is that this is learned
    /// once.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErrClass::BadInvocation => "bad_invocation",
            ErrClass::BadConfig => "bad_config",
            ErrClass::ToolFailed => "tool_failed",
            ErrClass::Timeout => "timeout",
            ErrClass::Aborted => "aborted",
            ErrClass::Environment => "environment",
            ErrClass::ArmadaBug => "armada_bug",
        })
    }
}

/// Where a `bad_config` failure is, in the one grammar `where` uses for that
/// class: `armada.yml:` and then a locator (PLAN.md §3.1, §4.1.1 decision 4).
///
/// Two locators, because a config error arrives in two shapes and a parser
/// cannot give the first one. Both answer the same question — what do I edit —
/// which is why they share a grammar rather than being a third one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWhere {
    /// A key path Armada resolved for itself: `armada.yml:components.api.checks.lint.cmd`.
    Path {
        /// Workspace-relative path of the config file.
        file: String,
        /// Dotted key path into the document.
        path: String,
    },
    /// A 1-indexed source position, for a document that did not parse far
    /// enough to have a key path: `armada.yml:12:7`.
    Location {
        /// Workspace-relative path of the config file.
        file: String,
        /// 1-indexed line.
        line: usize,
        /// 1-indexed column.
        column: usize,
    },
    /// The file and nothing more, when neither is available.
    File {
        /// Workspace-relative path of the config file.
        file: String,
    },
}

impl fmt::Display for ConfigWhere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigWhere::Path { file, path } => write!(f, "{file}:{path}"),
            ConfigWhere::Location { file, line, column } => write!(f, "{file}:{line}:{column}"),
            ConfigWhere::File { file } => write!(f, "{file}"),
        }
    }
}

/// A typed failure: which class, where, and what to do next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmadaError {
    /// The class. The exit code follows from it and from nothing else.
    pub class: ErrClass,
    /// A config path for `bad_config`; a `results[]` id for every other class.
    pub r#where: String,
    /// What went wrong, in one line.
    pub message: String,
    /// Required for `bad_config`, optional elsewhere — that is the one class
    /// where Armada genuinely knows the fix, because it has just validated the
    /// file and knows what it expected (`ARCHITECTURE.md` §1.7).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub next_action: Option<String>,
}

impl ArmadaError {
    /// A `bad_config`, which is the only class phase 1 can produce.
    pub fn bad_config(
        r#where: ConfigWhere,
        message: impl Into<String>,
        next_action: impl Into<String>,
    ) -> Self {
        ArmadaError {
            class: ErrClass::BadConfig,
            r#where: r#where.to_string(),
            message: message.into(),
            next_action: Some(next_action.into()),
        }
    }
}

impl fmt::Display for ArmadaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.r#where, self.message)
    }
}

impl std::error::Error for ArmadaError {}

/// The state a verb ends in, or reports mid-flight.
///
/// One spelling for failure: `FAILED`, never `FAIL` (PLAN.md §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    /// `init` succeeded.
    Ready,
    /// `up` succeeded.
    Up,
    /// `down` succeeded.
    Down,
    /// `clean` succeeded.
    Clean,
    /// `check` succeeded.
    Pass,
    /// `status` answered.
    Ok,
    /// Nothing to do. Exit 0, and not `PARTIAL`.
    Skipped,
    /// Some succeeded, some did not. Never reported by `check`.
    Partial,
    /// Did not achieve its goal.
    Failed,
    /// Stopped from outside: SIGINT, or the acquisition ceiling.
    Aborted,
    /// The run's holder died.
    Dead,
    /// A deadline elapsed.
    Timeout,
    /// Progress: executing. Never terminal, never mapped to an exit code.
    Running,
    /// Progress: not executing; `waiting_on` says why.
    Waiting,
}

impl Status {
    /// Whether this state ends a run.
    ///
    /// `RUNNING` and `WAITING` are progress, not verdicts: a run never ends in
    /// one. Only the three read verbs — `status`, `check --status`, `explain` —
    /// may put one in the envelope's top-level `status` (PLAN.md §3.1).
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Status::Running | Status::Waiting)
    }
}

impl fmt::Display for Status {
    /// One spelling everywhere: `FAILED`, never `FAIL`, and never `Failed`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Status::Ready => "READY",
            Status::Up => "UP",
            Status::Down => "DOWN",
            Status::Clean => "CLEAN",
            Status::Pass => "PASS",
            Status::Ok => "OK",
            Status::Skipped => "SKIPPED",
            Status::Partial => "PARTIAL",
            Status::Failed => "FAILED",
            Status::Aborted => "ABORTED",
            Status::Dead => "DEAD",
            Status::Timeout => "TIMEOUT",
            Status::Running => "RUNNING",
            Status::Waiting => "WAITING",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The terminal-state enum and the error-class enum each have exactly one
    /// spelling, and it is the one the envelope uses. Two vocabularies for one
    /// value is how `FAIL` and `FAILED` coexisted in an earlier draft.
    #[test]
    fn the_human_spelling_is_the_json_spelling() {
        for status in [Status::Failed, Status::Skipped, Status::Ready, Status::Ok] {
            assert_eq!(
                format!("\"{status}\""),
                serde_json::to_string(&status).unwrap()
            );
        }
        for class in [
            ErrClass::BadConfig,
            ErrClass::ArmadaBug,
            ErrClass::ToolFailed,
            ErrClass::Environment,
        ] {
            assert_eq!(
                format!("\"{class}\""),
                serde_json::to_string(&class).unwrap()
            );
        }
    }

    /// The exit map, asserted whole. Nothing else in the suite catches a
    /// silent renumbering: golden snapshots capture stdout, not exit status
    /// (`ARCHITECTURE.md` §1.6).
    #[test]
    fn exit_codes_match_the_one_mapping() {
        assert_eq!(ErrClass::ToolFailed.exit_code(), 1);
        assert_eq!(ErrClass::BadInvocation.exit_code(), 2);
        assert_eq!(ErrClass::BadConfig.exit_code(), 3);
        assert_eq!(ErrClass::Timeout.exit_code(), 4);
        assert_eq!(ErrClass::Aborted.exit_code(), 5);
        assert_eq!(ErrClass::Environment.exit_code(), 6);
        assert_eq!(ErrClass::ArmadaBug.exit_code(), 70);
    }

    #[test]
    fn severity_orders_the_aggregate_the_way_the_envelope_says() {
        let mut classes = [
            ErrClass::BadConfig,
            ErrClass::ToolFailed,
            ErrClass::ArmadaBug,
            ErrClass::Timeout,
            ErrClass::Environment,
            ErrClass::Aborted,
            ErrClass::BadInvocation,
        ];
        classes.sort_by_key(|c| c.severity());
        assert_eq!(
            classes,
            [
                ErrClass::ToolFailed,
                ErrClass::Aborted,
                ErrClass::Timeout,
                ErrClass::BadInvocation,
                ErrClass::BadConfig,
                ErrClass::Environment,
                ErrClass::ArmadaBug,
            ]
        );
    }

    #[test]
    fn failure_has_one_spelling() {
        assert_eq!(
            serde_json::to_string(&Status::Failed).unwrap(),
            "\"FAILED\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Skipped).unwrap(),
            "\"SKIPPED\""
        );
    }

    #[test]
    fn progress_states_are_never_terminal() {
        assert!(!Status::Running.is_terminal());
        assert!(!Status::Waiting.is_terminal());
        assert!(Status::Failed.is_terminal());
        assert!(Status::Skipped.is_terminal());
    }

    #[test]
    fn error_classes_serialize_in_the_documented_spelling() {
        assert_eq!(
            serde_json::to_string(&ErrClass::BadConfig).unwrap(),
            "\"bad_config\""
        );
        assert_eq!(
            serde_json::to_string(&ErrClass::ArmadaBug).unwrap(),
            "\"armada_bug\""
        );
    }

    #[test]
    fn config_where_has_one_grammar_and_two_locators() {
        let path = ConfigWhere::Path {
            file: "armada.yml".into(),
            path: "components.api.checks.lint.cmd".into(),
        };
        assert_eq!(
            path.to_string(),
            "armada.yml:components.api.checks.lint.cmd"
        );

        let loc = ConfigWhere::Location {
            file: "armada.yml".into(),
            line: 12,
            column: 7,
        };
        assert_eq!(loc.to_string(), "armada.yml:12:7");

        // Both carry the `armada.yml:` prefix an agent disambiguates on.
        assert!(path.to_string().starts_with("armada.yml:"));
        assert!(loc.to_string().starts_with("armada.yml:"));
    }
}
