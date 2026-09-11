//! A refused command and what a person answers, as the wire spells them.
//!
//! Nothing generates the DTO types from this crate yet, so these cases are what
//! pin the spellings Bridge mirrors by hand — and both sets are ones Bridge
//! matches on, so a spelling that drifted would be a control that never draws.

use crate::{decode, encode, CommandAnswer, CommandInFlight, Instant, StepId, WhenBlocked};

/// A Drone held on one shell command, with all three answers on offer.
fn waiting() -> CommandInFlight {
    CommandInFlight {
        call: String::from("toolu_01"),
        step_id: StepId::carried("implement"),
        asked_at: Instant::carried("2026-09-11T09:00:00.000Z"),
        tool: String::from("Bash"),
        detail: String::from("cargo nextest run -p ipc"),
        truncated: false,
        length: Some(24),
        offers: vec![
            CommandAnswer::AllowForJob,
            CommandAnswer::AlwaysAllow,
            CommandAnswer::Reject,
        ],
    }
}

/// The call id is what an answer names and the offers are what it may say, so
/// both cross whole and in order.
#[test]
fn a_waiting_command_crosses_with_the_answers_a_person_may_give() {
    let json = encode(&waiting()).expect("plain data");
    assert!(json.contains("\"call\":\"toolu_01\""), "{json}");
    assert!(
        json.contains("\"offers\":[\"allow_for_job\",\"always_allow\",\"reject\"]"),
        "{json}"
    );
    assert_eq!(
        decode::<CommandInFlight>("a waiting command", json.as_bytes()).expect("it reads back"),
        waiting()
    );
}

/// An argument Fleet could not measure carries no length, which is a different
/// answer from an argument of no length.
#[test]
fn an_unmeasured_command_carries_no_length_rather_than_nought() {
    let unmeasured = CommandInFlight {
        length: None,
        ..waiting()
    };
    let json = encode(&unmeasured).expect("plain data");
    assert!(!json.contains("\"length\""), "absent, never null: {json}");
    assert_eq!(
        decode::<CommandInFlight>("a waiting command", json.as_bytes()).expect("it reads back"),
        unmeasured
    );
}

#[test]
fn each_setting_is_spelled_as_the_operations_name_it() {
    for (setting, spelled) in [
        (WhenBlocked::RefuseAndHold, "\"refuse_and_hold\""),
        (WhenBlocked::AskMe, "\"ask_me\""),
    ] {
        assert_eq!(encode(&setting).expect("plain data"), spelled);
        assert_eq!(
            decode::<WhenBlocked>("a setting", spelled.as_bytes()).expect("it reads back"),
            setting
        );
    }
}

#[test]
fn each_answer_is_spelled_as_the_operations_name_it() {
    for (answer, spelled) in [
        (CommandAnswer::AllowForJob, "\"allow_for_job\""),
        (CommandAnswer::AlwaysAllow, "\"always_allow\""),
        (CommandAnswer::Reject, "\"reject\""),
    ] {
        assert_eq!(encode(&answer).expect("plain data"), spelled);
        assert_eq!(
            decode::<CommandAnswer>("an answer", spelled.as_bytes()).expect("it reads back"),
            answer
        );
    }
}

/// The two bodies Bridge sends, read as `api` reads them — and a field a newer
/// Bridge adds does not break the parse, which is the minor-skew rule.
#[test]
fn the_two_bodies_read_as_sent_and_ignore_what_they_do_not_know() {
    let answered = decode::<crate::AnswerCommand>(
        "an answer",
        br#"{"call":"toolu_01","answer":"always_allow","later":true}"#,
    )
    .expect("an answer reads");
    assert_eq!(
        answered,
        crate::AnswerCommand {
            call: String::from("toolu_01"),
            answer: CommandAnswer::AlwaysAllow,
        }
    );

    let set = decode::<crate::SetWhenBlocked>("a setting", br#"{"when_blocked":"ask_me"}"#)
        .expect("a setting reads");
    assert_eq!(set.when_blocked, WhenBlocked::AskMe);
    assert!(
        decode::<crate::SetWhenBlocked>("a setting", br#"{}"#).is_err(),
        "a body that sets nothing is refused rather than read as the default"
    );
}

/// **Refused, not defaulted.** A peer that sent `allow` does not share this
/// vocabulary, and guessing which of the two allows it meant is guessing
/// whether `armada.yml` gets a commit.
#[test]
fn a_spelling_nobody_offers_is_refused() {
    assert!(decode::<CommandAnswer>("an answer", b"\"allow\"").is_err());
    assert!(decode::<WhenBlocked>("a setting", b"\"ask\"").is_err());
}
