//! Whether a guild file is still readable by the thing that reads it.
//!
//! **This is the half of `guild edit` that makes it a verb rather than a
//! shortcut for `$EDITOR`.** `RESERVED_GUILD_VERBS` reserved the name as *open a
//! guild file, validate it, commit it*, and the middle word is the contract: a
//! workflow that no longer parses, committed and pushed, is a workflow that
//! fails on the next machine for a reason git will happily replicate.
//!
//! # It refuses to commit; it does not refuse to save
//!
//! An edit that does not validate stays on disk and is left **uncommitted**.
//! Losing somebody's work because a colon was in the wrong place would be the
//! worse failure of the two, and git already holds the previous version — so
//! `git -C ~/.armada/guild checkout <path>` is the undo, and the refusal names
//! it. What does not happen is the broken version reaching `push`.
//!
//! # Only what has a reader
//!
//! A workflow is checked against the schema Fleet parses it with; `settings.json`
//! against JSON; `plugins.yml` and `mcp.yml` against YAML. **Markdown is checked
//! only for being there**, because nothing parses prose and a validator that
//! invented rules for `voice.md` would be a validator inventing your voice.

use crate::inventory::Kind;

/// What a file is, once it has been read successfully.
///
/// The string is printed on the row that reports the edit, because *what it
/// turned out to be* is the confirmation a person wants after saving: `4 steps,
/// plan, approval, implement, land` says the edit landed the way it was meant
/// to in a way that `ok` does not.
pub type Reading = String;

/// Read a guild file the way the thing that consumes it will.
///
/// `Ok` carries what it turned out to be; `Err` carries why it cannot be
/// committed, in the words the reader needs to fix it.
pub fn check(kind: Kind, name: &str, body: &str) -> Result<Reading, String> {
    if body.trim().is_empty() {
        return Err("it is empty".to_string());
    }
    match kind {
        Kind::Workflow => {
            let workflow = armada_core::fleet::workflow::parse(body, name)
                .map_err(|error| flatten(&error.message))?;
            Ok(format!(
                "{} steps, {}",
                workflow.steps.len(),
                workflow
                    .steps
                    .iter()
                    .map(|step| step.id.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        Kind::Settings => match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(map)) => Ok(format!("{} settings", map.len())),
            Ok(_) => Err("settings.json has to be an object".to_string()),
            Err(error) => Err(flatten(&error.to_string())),
        },
        Kind::Plugins | Kind::Mcp => match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(body) {
            Ok(serde_yaml_ng::Value::Mapping(_)) => Ok("read".to_string()),
            Ok(_) => Err(format!("{name} has to be a mapping")),
            Err(error) => Err(flatten(&error.to_string())),
        },
        // **Nothing parses prose.** A `voice.md` that is there is a `voice.md`
        // that works; the only failure it has is being empty, which is checked
        // above for every kind.
        Kind::Memory | Kind::Skill | Kind::Subagent | Kind::Hook => {
            Ok(crate::inventory::plural(body.lines().count(), "line"))
        }
        // Reachable only if a caller ignores `Kind::editable`, and it answers
        // rather than panicking — the schema is Armada's, and an edit to it is
        // an edit to what every workflow is judged by.
        Kind::Schema => Err("the workflow schema is Armada's, not yours".to_string()),
    }
}

/// A parser's complaint on one line.
///
/// `serde` reports a line and a column and often a newline of its own, and a
/// multi-line message in a table cell breaks the row into several — the same
/// reason the listing collapses its details.
fn flatten(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_workflow() -> String {
        crate::starters::all()
            .into_iter()
            .find(|starter| starter.path.ends_with("bug.yml"))
            .expect("bug is a starter")
            .body
            .to_string()
    }

    /// **The starter workflows validate**, which is the check that keeps this
    /// module honest: a validator stricter than what `guild init` writes would
    /// refuse every first edit.
    #[test]
    fn every_starter_workflow_passes_the_check_that_guards_editing_it() {
        for starter in crate::starters::all() {
            let Some(name) = starter.path.strip_prefix("workflows/") else {
                continue;
            };
            if name == crate::inventory::SCHEMA {
                continue;
            }
            let read = check(Kind::Workflow, name, starter.body)
                .unwrap_or_else(|why| panic!("the starter `{name}` does not validate: {why}"));
            assert!(read.contains("steps"), "{read}");
        }
    }

    /// **A workflow that does not parse is refused**, which is the whole reason
    /// `guild edit` is a verb: the broken version must not reach `push`.
    #[test]
    fn a_workflow_that_no_longer_parses_is_refused() {
        let broken = a_workflow().replace("steps:", "steps");
        let why = check(Kind::Workflow, "bug.yml", &broken).unwrap_err();
        assert!(!why.is_empty());
        assert!(!why.contains('\n'), "a complaint fits one row: {why:?}");
    }

    /// An emptied file is refused whatever its kind. A `voice.md` with nothing
    /// in it is a `voice.md` that says nothing, and it is far more often a
    /// mis-save than an intention.
    #[test]
    fn an_emptied_file_is_refused_whatever_it_is() {
        for kind in [Kind::Memory, Kind::Skill, Kind::Workflow, Kind::Settings] {
            assert_eq!(check(kind, "x", "   \n").unwrap_err(), "it is empty");
        }
    }

    /// **Nothing parses prose.** Markdown is checked for being there and for
    /// nothing else — a validator with opinions about `voice.md` would be a
    /// validator with opinions about your voice.
    #[test]
    fn markdown_is_checked_for_being_there_and_nothing_else() {
        assert_eq!(
            check(Kind::Memory, "voice.md", "150 words maximum.\n").unwrap(),
            "1 line"
        );
        assert!(check(Kind::Skill, "SKILL.md", "not: valid: yaml: at: all").is_ok());
    }

    /// Structured files are checked as the reader that consumes them reads
    /// them, and a broken one names what is wrong rather than that something is.
    #[test]
    fn structured_files_are_read_the_way_their_readers_read_them() {
        assert_eq!(
            check(Kind::Settings, "settings.json", "{\"model\":\"opus\"}").unwrap(),
            "1 settings"
        );
        assert!(check(Kind::Settings, "settings.json", "{oops").is_err());
        assert!(check(Kind::Settings, "settings.json", "[1,2]")
            .unwrap_err()
            .contains("object"));
        assert!(check(Kind::Plugins, "plugins.yml", "enabled:\n  - a\n").is_ok());
        assert!(check(Kind::Mcp, "mcp.yml", "- a\n- b\n")
            .unwrap_err()
            .contains("mapping"));
    }

    /// The schema is Armada's. Editing it to make an invalid workflow pass is
    /// the one edit that would defeat every check above.
    #[test]
    fn the_schema_is_not_yours_to_edit() {
        assert!(check(Kind::Schema, crate::inventory::SCHEMA, "{}").is_err());
    }
}
