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
        (WhenBlocked::AllowAll, "\"allow_all\""),
    ] {
        assert_eq!(encode(&setting).expect("plain data"), spelled);
        assert_eq!(
            decode::<WhenBlocked>("a setting", spelled.as_bytes()).expect("it reads back"),
            setting
        );
    }
}

/// The third setting reaches `set_when_blocked` as the other two do. **An older
/// peer cannot read it**, which is what made it a major bump: a setting it has
/// no arm for is refused, never read as the nearest one it knows.
#[test]
fn allow_all_is_a_setting_a_person_sends_like_the_other_two() {
    let set = decode::<crate::SetWhenBlocked>("a setting", br#"{"when_blocked":"allow_all"}"#)
        .expect("a setting reads");
    assert_eq!(set.when_blocked, WhenBlocked::AllowAll);
    assert_eq!(
        encode(&set).expect("plain data"),
        r#"{"when_blocked":"allow_all"}"#
    );
    assert!(decode::<WhenBlocked>("a setting", b"\"allow\"").is_err());
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

#[test]
fn each_reach_is_spelled_as_the_domain_spells_it() {
    for (reach, domain, spelled) in [
        (crate::Reach::Job, core_model::Reach::Job, "\"job\""),
        (
            crate::Reach::Repository,
            core_model::Reach::Repository,
            "\"repository\"",
        ),
    ] {
        assert_eq!(encode(&reach).expect("plain data"), spelled);
        assert_eq!(format!("\"{}\"", domain.as_wire()), spelled);
        assert_eq!(crate::Reach::from(domain), reach);
        assert_eq!(
            decode::<crate::Reach>("a reach", spelled.as_bytes()).expect("it reads back"),
            reach
        );
    }
    assert!(decode::<crate::Reach>("a reach", b"\"everywhere\"").is_err());
}

/// An always-allow off the record, as Fleet will convert it. **Every field
/// crosses**, and `run` crosses whole because it is what a removal names.
fn allowed() -> core_model::AllowedCommand {
    core_model::AllowedCommand {
        run: String::from("cargo nextest run -p ipc"),
        reach: core_model::Reach::Repository,
        allowed_at: core_model::Timestamp::from_rfc3339("2026-09-11T09:00:00.000Z"),
        by: core_model::Actor::Human,
    }
}

#[test]
fn an_allowed_command_crosses_whole_from_the_record() {
    let record = allowed();
    let row = crate::AllowedCommandRow::from(&record);
    assert_eq!(row.allowed_at, Instant::from(&record.allowed_at));
    let json = encode(&row).expect("plain data");
    assert!(
        json.contains("\"run\":\"cargo nextest run -p ipc\""),
        "{json}"
    );
    assert!(json.contains("\"reach\":\"repository\""), "{json}");
    assert!(json.contains("\"by\":\"human\""), "{json}");
    assert_eq!(
        decode::<crate::AllowedCommandRow>("an allowed command", json.as_bytes())
            .expect("it reads back"),
        row
    );
}

/// **Always stated on an 11.0 detail, and read as empty where it is not.** A
/// detail with no key is a Job nobody allowed anything on, never a parse error.
#[test]
fn the_allowed_commands_are_stated_and_an_older_detail_reads_as_none() {
    use crate::tests::{detail_of, job};

    let mut detail = detail_of(&job(), &[]);
    let json = encode(&detail).expect("a detail is plain data");
    assert!(json.contains("\"allowed_commands\":[]"), "{json}");
    let older = json.replace(",\"allowed_commands\":[]", "");
    assert!(!older.contains("allowed_commands"), "{older}");
    assert_eq!(
        decode::<crate::JobDetail>("a detail", older.as_bytes()).expect("an older detail reads"),
        detail
    );

    let first = crate::AllowedCommandRow::from(&allowed());
    let second = crate::AllowedCommandRow {
        run: String::from("touch x"),
        reach: crate::Reach::Job,
        ..first.clone()
    };
    detail.allowed_commands = vec![first, second];
    let json = encode(&detail).expect("a detail is plain data");
    let at = |needle: &str| json.find(needle).expect(needle);
    assert!(
        at("cargo nextest run -p ipc") < at("touch x"),
        "oldest first, as Fleet listed them: {json}"
    );
    assert_eq!(
        decode::<crate::JobDetail>("a detail", json.as_bytes()).expect("it reads back"),
        detail
    );
}

/// `set_model`'s body: a name, or a `null` that clears. **A body with no key is
/// refused**, never read as a clear — that would throw away a person's choice
/// and answer 200. A field a newer Bridge adds is ignored, as on every body.
#[test]
fn a_model_is_set_by_name_or_cleared_by_null_and_never_by_silence() {
    let set =
        decode::<crate::SetModel>("a model", br#"{"model":"model-b"}"#).expect("a name reads");
    assert_eq!(set.model.as_deref(), Some("model-b"));
    assert_eq!(encode(&set).expect("plain data"), r#"{"model":"model-b"}"#);

    let cleared =
        decode::<crate::SetModel>("a model", br#"{"model":null}"#).expect("a clear reads");
    assert_eq!(cleared.model, None);
    assert_eq!(
        encode(&cleared).expect("plain data"),
        r#"{"model":null}"#,
        "the clear is stated on the wire, not left out"
    );

    assert!(
        decode::<crate::SetModel>("a model", br#"{}"#).is_err(),
        "a body that says nothing is refused rather than read as a clear"
    );
    assert_eq!(
        decode::<crate::SetModel>("a model", br#"{"model":"model-b","later":true}"#)
            .expect("an unknown field is ignored"),
        set
    );
}

/// A removal names the command as the row carried it, whole.
#[test]
fn an_allow_is_removed_by_the_command_its_row_carries() {
    let row = crate::AllowedCommandRow::from(&allowed());
    let body = crate::RemoveAllowedCommand { run: row.run };
    let json = encode(&body).expect("plain data");
    assert_eq!(json, r#"{"run":"cargo nextest run -p ipc"}"#);
    assert_eq!(
        decode::<crate::RemoveAllowedCommand>("a removal", json.as_bytes()).expect("it reads back"),
        body
    );
    assert!(decode::<crate::RemoveAllowedCommand>("a removal", br#"{}"#).is_err());
}
