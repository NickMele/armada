//! What a Helm conversation's wire must keep true: a blank message does not
//! decode, and every socket message is told apart by its `message` tag.

use crate::{
    decode, encode, AskHelm, Freshness, HelmAsked, HelmClosed, HelmFresh, HelmMessage, HelmSilence,
    HelmText, Instant, Saw, Shown, TranscriptRow, Voice,
};

#[test]
fn a_message_with_something_in_it_decodes() {
    let asked = decode::<AskHelm>("a message to Helm", br#"{"text":"why did job 12 stall?"}"#)
        .expect("plain data");
    assert_eq!(asked.text.as_str(), "why did job 12 stall?");
}

#[test]
fn a_blank_message_does_not_decode() {
    for body in [&br#"{"text":""}"#[..], br#"{"text":"  \n "}"#, br#"{}"#] {
        assert!(decode::<AskHelm>("a message to Helm", body).is_err());
    }
    assert_eq!(HelmText::said("   "), None);
}

#[test]
fn each_socket_message_carries_its_tag_and_round_trips() {
    let row = Shown::of(TranscriptRow {
        ts: Instant::carried("2026-09-13T09:00:00.000Z"),
        step: None,
        by: Voice::Drone,
        saw: Saw::Said {
            text: "Job 12 is waiting on a person.".to_string(),
        },
    })
    .expect("prose is shown");
    let cases = [
        (
            HelmMessage::Asked(HelmAsked {
                ts: Instant::carried("2026-09-13T09:00:00.000Z"),
                text: "why?".to_string(),
            }),
            r#""message":"asked""#,
        ),
        (HelmMessage::Row(row), r#""message":"row""#),
        (
            HelmMessage::Fresh(HelmFresh {
                ts: Instant::carried("2026-09-13T09:00:00.000Z"),
                because: Freshness::SessionNotFound,
            }),
            r#""because":"session_not_found""#,
        ),
        (
            HelmMessage::Closed(HelmClosed {
                because: HelmSilence::StartedFresh,
            }),
            r#""because":"started_fresh""#,
        ),
    ];
    for (message, carries) in cases {
        let json = encode(&message).expect("plain data");
        assert!(json.contains(carries), "{json} carries {carries}");
        assert_eq!(
            decode::<HelmMessage>("a Helm message", json.as_bytes()).expect("round-trips"),
            message
        );
    }
}
