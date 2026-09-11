//! A refused command and what a person answers, as the wire spells them.
//!
//! Nothing generates the DTO types from this crate yet, so these cases are what
//! pin the spellings Bridge mirrors by hand — and both sets are ones Bridge
//! matches on, so a spelling that drifted would be a control that never draws.

use crate::{decode, encode, CommandAnswer, WhenBlocked};

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

/// **Refused, not defaulted.** A peer that sent `allow` does not share this
/// vocabulary, and guessing which of the two allows it meant is guessing
/// whether `armada.yml` gets a commit.
#[test]
fn a_spelling_nobody_offers_is_refused() {
    assert!(decode::<CommandAnswer>("an answer", b"\"allow\"").is_err());
    assert!(decode::<WhenBlocked>("a setting", b"\"ask\"").is_err());
}
