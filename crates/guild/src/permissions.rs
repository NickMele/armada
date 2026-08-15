//! **What a Drone may do unattended**, read from the guild.
//!
//! The posture itself — the mode, the two lists, and the reasoning for every
//! rule on whichever side it is on — is
//! [`armada_core::fleet::drone::Posture`]. This module is only the half that
//! touches a disk: which file it is read from, and what happens when it is
//! absent or wrong.
//!
//! # Why the guild rather than a constant, a workflow, or the persona
//!
//! **It is a preference, and a preference travels.** The same argument that put
//! the model-tier policy in the guild persona applies here: what you are willing
//! to let an unattended agent do on your machine is a thing you decide once and
//! want on every machine and in every repository, not a thing you re-decide per
//! checkout.
//!
//! **Not a workflow field**, because [`armada_core::fleet::workflow`]'s budget
//! ceilings differ per workflow and this does not. A bug Job and a feature Job
//! spend different amounts and are allowed exactly the same things; putting the
//! posture in four workflow files would be four chances to make three of them
//! disagree, for no question that was ever asked per workflow.
//!
//! **Not the persona**, and that is the sharper distinction. `subagents/helm.md`
//! is prose read by a model; this is argv read by Armada. A permission posture
//! written as prose would be a description of a posture rather than the posture
//! — Helm would have to be trusted to translate it into flags correctly on
//! every spawn, and a model that got it wrong would produce exactly the silent
//! failure this whole change exists to end. The persona is told the file exists
//! so that Helm can point at it; the file is what decides.
//!
//! # Absent is not wrong
//!
//! **A guild with no `permissions.yml` gets the shipped default**, because a
//! user who has never thought about this must still get working Jobs — that is
//! the whole reason [`armada_core::fleet::drone::Posture::default`] is a real
//! posture rather than an empty one.
//!
//! **A guild with a broken one is refused.** Falling back to the default there
//! would run a Drone under a posture the user did not write and does not know
//! about, which is the one outcome worse than not starting: they narrowed
//! something on purpose and Armada widened it back without saying so.

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::fleet::drone::Posture;
use std::path::Path;

/// The file, guild-relative.
pub const FILE: &str = "permissions.yml";

/// The posture a guild describes, or the shipped default if it describes none.
///
/// Three outcomes and each is deliberate:
///
/// | On disk | Result |
/// |---|---|
/// | no file | [`Posture::default`] — the shipped posture, which works with no configuration |
/// | a file that parses and is usable | that posture, **replacing** the default rather than adding to it |
/// | a file that does not parse, or names a mode or a rule the CLI could not carry | [`ErrClass::BadConfig`] naming the file |
///
/// **Replacing rather than merging.** A list you can only add to is a posture
/// whose real contents are written down nowhere — you would have to read the
/// file *and* this crate's constants to know what a Drone may do. Writing the
/// whole thing out is what makes `armada guild ls` able to answer the question.
pub fn read(guild_root: &Path) -> Result<Posture, ArmadaError> {
    let path = guild_root.join(FILE);
    let Ok(body) = std::fs::read_to_string(&path) else {
        return Ok(Posture::default());
    };
    parse(&body).map_err(|why| wrong(&path.display().to_string(), &why))
}

/// The posture a `permissions.yml` body describes.
///
/// **Pure, and separate from [`read`] for the usual reason** —
/// `ARCHITECTURE.md` §1.5. It is also what `armada guild ls` uses to summarise
/// the file without going near a Drone.
pub fn parse(body: &str) -> Result<Posture, String> {
    // **An empty file is the default, not an error.** A user who wants the
    // shipped posture and reached for the file to check has not broken
    // anything by leaving it as they found it.
    if body.trim().is_empty() {
        return Ok(Posture::default());
    }
    let posture: Posture =
        serde_yaml_ng::from_str(body).map_err(|error| collapse(&error.to_string()))?;
    match posture.wrong() {
        Some(why) => Err(why),
        None => Ok(posture),
    }
}

/// The refusal, which names the file rather than the field.
///
/// **`bad_config`, because the correct response is to edit a file and run the
/// same command again** (`ARCHITECTURE.md` §1.7) — the machine is complete and
/// the repository is fine; a preference is written wrong. It is the class
/// [`crate::bundle`] already gives a guild's `plugins.yml` and `mcp.yml`, and
/// this is the third file of that kind.
fn wrong(path: &str, why: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadConfig,
        r#where: path.to_string(),
        message: format!("this guild's Drone permissions cannot be used: {why}"),
        next_action: Some(format!("fix {path}, then retry unchanged")),
    }
}

/// A parser error on one line. Multi-line detail in a `next_action` is detail
/// nobody reads.
fn collapse(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::fleet::drone::{ALLOW, DENY, MODE};

    /// **No file is the shipped posture, and that is the guarantee that makes a
    /// Job work with no configuration at all.** It is also the state every
    /// existing guild is in — `docs/reserved/006` is why a template change does
    /// not reach one.
    #[test]
    fn a_guild_that_says_nothing_gets_a_posture_that_works() {
        let empty = tempfile::tempdir().expect("a temp dir");
        let posture = read(empty.path()).expect("no file is not a failure");
        assert_eq!(posture, Posture::default());
        assert_eq!(posture.mode, MODE);
        assert_eq!(posture.allow.len(), ALLOW.len());
        assert_eq!(posture.deny.len(), DENY.len());
    }

    /// The guild's own words win whole. A narrower allow list is narrower, not
    /// narrower-plus-the-default.
    #[test]
    fn what_the_guild_writes_replaces_the_default_rather_than_adding_to_it() {
        let posture = parse(
            "mode: acceptEdits\n\
             allow:\n  - Read\n\
             deny:\n  - Bash(git push:*)\n",
        )
        .expect("a posture");
        assert_eq!(posture.mode, "acceptEdits");
        assert_eq!(posture.allow, ["Read"]);
        assert_eq!(posture.deny, ["Bash(git push:*)"]);
        assert!(
            !posture.allow.contains(&"Bash".to_string()),
            "the default leaked into a list that replaced it"
        );
    }

    /// A file that names only one of the three keeps the shipped answer for the
    /// other two — the common edit is "deny one more thing", and it should not
    /// require retyping the posture.
    #[test]
    fn a_file_that_answers_one_question_leaves_the_others_as_shipped() {
        let posture = parse("deny:\n  - Bash(rm:*)\n").expect("a posture");
        assert_eq!(posture.mode, MODE);
        assert_eq!(posture.allow.len(), ALLOW.len());
        assert_eq!(posture.deny, ["Bash(rm:*)"]);
    }

    /// An empty file is the default rather than an empty posture — an empty
    /// allow list under `dontAsk` is a Drone that may do nothing at all.
    #[test]
    fn an_empty_file_is_the_default_and_not_a_drone_that_can_do_nothing() {
        for body in ["", "   \n\n", "# a comment and nothing else\n"] {
            assert_eq!(parse(body).unwrap(), Posture::default(), "{body:?}");
        }
    }

    /// **A broken file refuses rather than falling back.** Falling back would
    /// run a Drone under a posture the user did not write, after they narrowed
    /// one on purpose.
    #[test]
    fn a_posture_that_cannot_be_used_is_refused_rather_than_replaced() {
        for (body, expect) in [
            ("mode: yolo\n", "not a permission mode"),
            ("allow:\n  - Edit Write\n", "two rules"),
            ("allow:\n  - --print\n", "reads as a flag"),
            ("mode: [not, a, string]\n", "invalid type"),
            ("allowed:\n  - Edit\n", "unknown field"),
        ] {
            let why = parse(body).expect_err(&format!("{body:?} was accepted"));
            assert!(why.contains(expect), "{body:?} said {why:?}");
            assert!(!why.contains('\n'), "the refusal is one line: {why:?}");
        }
    }

    /// The refusal names the file and the class whose correct response is *edit
    /// it and retry unchanged*.
    #[test]
    fn a_broken_file_is_bad_input_and_names_itself() {
        let guild = tempfile::tempdir().expect("a temp dir");
        std::fs::write(guild.path().join(FILE), "mode: yolo\n").expect("write");
        let error = read(guild.path()).expect_err("a broken posture is refused");
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.r#where.ends_with(FILE), "{}", error.r#where);
        assert!(error.next_action.unwrap().contains("retry unchanged"));
    }

    /// **The file Armada ships parses into the posture Armada compiled in.**
    /// Two hand-written copies of one list is two chances to disagree, and the
    /// disagreement would only ever show up on a machine that had run
    /// `guild init` — which is to say, not on the machine that changed it.
    #[test]
    fn the_template_this_repository_ships_is_the_compiled_in_default() {
        let template = include_str!("../../../templates/guild/permissions.yml");
        assert_eq!(parse(template).expect("the template parses"), Posture::default());
    }
}
