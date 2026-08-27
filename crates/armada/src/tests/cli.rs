//! What was typed, and what a refusal says back.
//!
//! A refusal is read by a person at a terminal who is about to retype the
//! line, so what is asserted here is mostly what the message names — the verbs
//! that exist, the flag that was meant, the arguments that had nowhere to go.

use crate::cli::{read, Fault, Verb};

fn asked(line: &str) -> Result<Verb, crate::cli::Misread> {
    read(line.split_whitespace().map(str::to_string))
}

fn said(line: &str) -> String {
    asked(line).expect_err("this line is refused").to_string()
}

#[test]
fn the_four_verbs_parse() {
    assert_eq!(
        asked("serve"),
        Ok(Verb::Serve { repository: None }),
        "no path means the working directory"
    );
    assert_eq!(
        asked("serve /repos/armada"),
        Ok(Verb::Serve {
            repository: Some("/repos/armada".into())
        })
    );
    assert_eq!(
        asked("check build"),
        Ok(Verb::Check {
            name: "build".to_string()
        })
    );
    assert_eq!(
        asked("run fmt"),
        Ok(Verb::Run {
            name: "fmt".to_string()
        })
    );
    assert_eq!(
        asked("clean"),
        Ok(Verb::Clean {
            everything: false,
            force: false
        })
    );
    assert_eq!(
        asked("clean --all"),
        Ok(Verb::Clean {
            everything: true,
            force: false
        })
    );
}

/// **Two flags because they are two questions.** `--all` clears this machine's
/// store; `--force` deletes work nobody has taken. Either alone, or both.
#[test]
fn force_and_all_are_separate_answers_and_compose() {
    assert_eq!(
        asked("clean --force"),
        Ok(Verb::Clean {
            everything: false,
            force: true
        })
    );
    assert_eq!(
        asked("clean --all --force"),
        Ok(Verb::Clean {
            everything: true,
            force: true
        })
    );
}

/// **`check` and `run` stay two verbs.** There is no flag on either that turns
/// it into the other, and nothing here parses one.
#[test]
fn there_is_no_flag_that_turns_one_verb_into_the_other() {
    let refused = said("check --command fmt");
    assert!(refused.contains("`--command` is a flag this verb does not take"));
}

#[test]
fn a_verb_that_does_not_exist_is_answered_with_the_ones_that_do() {
    let refused = said("chekc");
    for verb in ["serve", "check", "run", "clean"] {
        assert!(refused.contains(verb), "the verbs are named: {refused}");
    }
}

#[test]
fn asking_for_nothing_gets_the_usage() {
    let refused = read(Vec::<String>::new()).expect_err("nothing is not a verb");
    assert_eq!(refused.faults, vec![Fault::NothingAsked]);
    assert!(refused.to_string().contains("armada serve"));
}

/// There is no default Check. A verb that ran something when nothing was named
/// is a verb that runs the wrong thing on a typo.
#[test]
fn check_and_run_are_named_at() {
    assert!(said("check").contains("needs the name of one thing the Manifest declares"));
    assert!(said("run").contains("needs the name of one thing the Manifest declares"));
}

#[test]
fn help_is_a_verb_and_a_flag() {
    for spelling in ["help", "--help", "-h"] {
        assert_eq!(asked(spelling), Ok(Verb::Help));
    }
}

/// Every fault, never the first one — the same rule `config` holds for a
/// Manifest, because one correction should fix the whole line.
#[test]
fn a_line_with_two_things_wrong_names_both() {
    let refused = asked("clean --wat extra").expect_err("two faults");
    assert_eq!(refused.faults.len(), 2, "{refused}");
    let said = refused.to_string();
    assert!(said.contains("--wat"));
    assert!(said.contains("extra"));
}

/// `clean` acts where you are standing. A path would let somebody clean a
/// repository they are not looking at, which is the shape the branch mistake
/// had.
#[test]
fn clean_takes_no_path() {
    assert!(said("clean /somewhere/else").contains("nowhere to put"));
}

#[test]
fn a_flag_that_does_not_exist_names_the_one_that_does() {
    assert!(said("clean --everything").contains("it takes `--all`, `--force`"));
}
