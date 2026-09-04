//! The Evidence tool's transport, and the property it holds instead of skew
//! tolerance.
//!
//! A Drone is spawned by the Fleet it reports to, so there is no version to be
//! lenient about. **A field the tool does not take is refused by name**, because
//! dropping one would leave a Drone believing something it sent was read.

/// The four methods the real client's own server log names, read back as what
/// each one is. A method outside them is answered rather than dropped, because
/// silence on this seam is a Drone that waits forever.
#[test]
fn the_four_methods_read_as_themselves() {
    use crate::mcp::{read, Incoming};

    assert!(matches!(
        read(br#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#),
        Incoming::Handshake { revision, .. } if revision == "2025-03-26"
    ));
    assert!(matches!(
        read(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
        Incoming::Nothing
    ));
    assert!(matches!(
        read(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#),
        Incoming::Tools { .. }
    ));
    assert!(matches!(
        read(br#"{"jsonrpc":"2.0","id":2,"method":"resources/list"}"#),
        Incoming::NoSuchMethod { named, .. } if named == "resources/list"
    ));
    assert!(matches!(read(b"{"), Incoming::Unreadable { .. }));
}

/// A call id may be a string or a number, and a server that re-typed one would
/// answer a call the client cannot match to its request.
#[test]
fn a_call_id_comes_back_the_type_it_arrived_as() {
    use crate::mcp::{answer, read, Answered, Incoming};

    for id in ["7", "\"7\""] {
        let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#);
        let Incoming::Ping { id: carried } = read(body.as_bytes()) else {
            panic!("a ping");
        };
        let answered = answer(Answered::Ping { id: carried }).expect("plain data");
        assert!(answered.contains(&format!("\"id\":{id}")), "{answered}");
    }
}

/// **A tool takes the fields it takes and refuses another by name.** Dropping
/// one would leave a Drone believing something it sent was read.
#[test]
fn a_field_the_tool_does_not_take_is_named_rather_than_dropped() {
    use crate::mcp::{read, Incoming, NotAnArgument};

    let called = read(
        br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"c","shown_by":"s","not_claimed":"","source":"human"}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::NotASubmission {
            why: NotAnArgument::NotAField { ref named, .. },
            ..
        } if named == "source"
    ));
}

/// Empty is a legal `not_claimed` and absent is not, and the difference
/// survives the wire: one reads as a submission, the other as a refusal.
#[test]
fn an_empty_not_claimed_reads_and_an_absent_one_refuses() {
    use crate::mcp::{read, Incoming, NotAnArgument};

    let full = read(
        br#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"c","shown_by":"s","not_claimed":""}}}"#,
    );
    assert!(matches!(
        full,
        Incoming::Submit { ref submission, .. } if submission.not_claimed.is_empty()
    ));

    let short = read(
        br#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"c","shown_by":"s"}}}"#,
    );
    assert!(matches!(
        short,
        Incoming::NotASubmission {
            why: NotAnArgument::Missing {
                field: "not_claimed"
            },
            ..
        }
    ));
}

/// The transcript row's bytes are the file's bytes, both ways. `flatten` over
/// an internally tagged enum is the one shape here serde could get wrong, and
/// the file `#102` already writes is what a round trip has to agree with.
#[test]
fn a_transcript_row_reads_back_as_what_was_written() {
    let row = crate::TranscriptRow {
        ts: crate::Instant::carried("2026-08-26T09:00:00.000Z"),
        step: Some(crate::StepId::carried("implement")),
        by: crate::Voice::Drone,
        saw: crate::Saw::Called {
            tool: "Bash".to_string(),
            call: "toolu_1".to_string(),
            detail: "cargo build --workspace".to_string(),
            truncated: false,
            detail_length: Some(23),
            whole: None,
        },
    };
    let json = crate::encode(&row).expect("a row encodes");
    assert_eq!(
        json,
        r#"{"ts":"2026-08-26T09:00:00.000Z","step":"implement","by":"drone","event":"called","tool":"Bash","call":"toolu_1","detail":"cargo build --workspace","truncated":false,"detail_length":23}"#
    );
    let back: crate::TranscriptRow = crate::decode("row", json.as_bytes()).expect("a row decodes");
    assert_eq!(back, row);
}

/// A file written before rows carried a step still reads. **Nothing relabels
/// it** — which step produced it is not recoverable, and `None` says so rather
/// than naming the step the Drone was spawned on.
#[test]
fn a_row_from_before_the_step_was_recorded_still_decodes_and_says_it_does_not_know() {
    let old = r#"{"ts":"2026-08-26T09:00:00.000Z","event":"said","text":"one"}"#;
    let back: crate::TranscriptRow = crate::decode("row", old.as_bytes()).expect("a row decodes");
    assert_eq!(back.step, None);
    assert_eq!(
        back.by,
        crate::Voice::Drone,
        "and a file written before the voice existed is a Drone's, which is what \
         the default says rather than guesses"
    );
    let again = crate::encode(&back).expect("it re-encodes");
    assert!(
        !again.contains("step"),
        "an absent step is written back absent rather than as a null or a guess: {again}"
    );
}

/// A row the design withholds cannot be put on the wire, and cannot be read off
/// it either — the narrowing is the type, not a check somewhere.
#[test]
fn a_withheld_row_has_no_constructor_and_no_decoder() {
    let quota = crate::TranscriptRow {
        ts: crate::Instant::carried("2026-08-26T09:00:00.000Z"),
        step: Some(crate::StepId::carried("implement")),
        by: crate::Voice::Drone,
        saw: crate::Saw::QuotaMoved {
            window: "five_hour".to_string(),
            status: "warning".to_string(),
        },
    };
    let json = crate::encode(&quota).expect("the file still holds it");
    assert!(crate::Shown::of(quota).is_none());
    let read: Result<crate::Shown, _> = crate::decode("shown", json.as_bytes());
    assert!(read.is_err());
}

/// One message, nested tags and all.
#[test]
fn a_turn_message_carries_a_row_under_its_own_tag() {
    let shown = crate::Shown::of(crate::TranscriptRow {
        ts: crate::Instant::carried("2026-08-26T09:00:00.000Z"),
        step: Some(crate::StepId::carried("implement")),
        by: crate::Voice::Drone,
        saw: crate::Saw::Said {
            text: "reading the file".to_string(),
        },
    })
    .expect("prose is shown");
    let message = crate::TurnMessage::Row(shown);
    let json = crate::encode(&message).expect("a message encodes");
    let back: crate::TurnMessage = crate::decode("turn", json.as_bytes()).expect("it decodes");
    assert_eq!(back, message);
}

/// **A declaration is not a submission**, and the transport says so: a call of
/// the scope tool reaches a different variant, so nothing downstream can
/// mistake a plan for a report.
#[test]
fn a_scope_call_reads_as_a_declaration_and_not_as_evidence() {
    use crate::mcp::{read, DeclareScope, Incoming};

    let called = read(
        br#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"declare_scope",
            "arguments":{"context_paths":["docs","crates/config/src"]}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::Declare {
            declaration: DeclareScope { ref context_paths },
            ..
        } if context_paths == &["docs".to_string(), "crates/config/src".to_string()]
    ));
}

/// Declaring nothing is a legal answer — a part that changes nothing has said
/// so — and it is a different answer from not calling at all.
#[test]
fn an_empty_path_list_is_a_declaration_rather_than_a_refusal() {
    use crate::mcp::{read, DeclareScope, Incoming};

    let called = read(
        br#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"declare_scope",
            "arguments":{"context_paths":[]}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::Declare {
            declaration: DeclareScope { ref context_paths },
            ..
        } if context_paths.is_empty()
    ));
}

/// A string where a list belongs is refused with the thing to do about it,
/// because a Drone that sent one path as text believes it declared one path.
#[test]
fn a_path_list_that_is_not_a_list_is_refused_by_name() {
    use crate::mcp::{read, Incoming, NotAnArgument};

    let called = read(
        br#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"declare_scope",
            "arguments":{"context_paths":"docs"}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::NotASubmission {
            why: NotAnArgument::NotAList {
                field: "context_paths"
            },
            ..
        }
    ));
}

/// **A tool that takes nothing is called with nothing**, and a client is
/// entitled to omit the `arguments` member entirely rather than send an empty
/// object. Both readings are the same call, and the arm that answers them runs
/// before the arguments are looked for at all.
#[test]
fn a_checks_call_reads_the_same_with_no_arguments_and_with_none() {
    use crate::mcp::{read, Incoming};

    let omitted =
        read(br#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"run_checks"}}"#);
    let empty = read(
        br#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"run_checks",
            "arguments":{}}}"#,
    );
    assert!(matches!(omitted, Incoming::RunChecks { .. }), "{omitted:?}");
    assert!(matches!(empty, Incoming::RunChecks { .. }), "{empty:?}");
}

/// **A Drone cannot choose which bar it is measured against**, so an invented
/// Check name has no field to arrive in — and it is refused by name rather than
/// dropped, because a Drone that named one believed it would be honoured.
#[test]
fn a_checks_call_naming_a_check_is_refused_and_told_who_decides() {
    use crate::mcp::{read, Incoming};

    let called = read(
        br#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"run_checks",
            "arguments":{"check":"tests"}}}"#,
    );
    let Incoming::NotASubmission { why, .. } = called else {
        panic!("a field this tool does not take is refused as a tool error");
    };
    let said = why.to_string();
    assert!(said.contains("`check` is not a field"), "{said}");
    assert!(
        said.contains("when this task was approved"),
        "and is told who settled it: {said}"
    );
}

/// The refusal for a fourth tool names every real one, so a Drone that guessed
/// is told what it may call rather than only that it guessed.
#[test]
fn a_tool_that_is_neither_names_both() {
    use crate::mcp::{read, Incoming};

    let called = read(
        br#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"ReportFindings",
            "arguments":{}}}"#,
    );
    let Incoming::NotASubmission { why, .. } = called else {
        panic!("a tool nobody serves is refused as a tool error");
    };
    let said = why.to_string();
    assert!(
        said.contains("submit_evidence")
            && said.contains("declare_scope")
            && said.contains("request_scope")
            && said.contains("run_checks"),
        "{said}"
    );
}

/// The two scope tools are two tools. One says where this part's work will be
/// and is replaced by calling it again; this one asks the task's own scope to
/// grow and is answered. A call of either reaching the other's variant would
/// make a plan correction cost a Judge call, or a request for scope cost
/// nothing and decide nothing.
#[test]
fn a_request_for_scope_reads_as_a_request_and_not_as_a_declaration() {
    use crate::mcp::{read, Incoming, RequestScope};

    let called = read(
        br#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"request_scope",
            "arguments":{"paths":["crates/store/src/schema.rs"],
            "reason":"the column the fix needs is declared there"}}}"#,
    );
    let Incoming::Widen {
        request: RequestScope { paths, reason },
        ..
    } = called
    else {
        panic!("a scope request is its own variant");
    };
    assert_eq!(paths, ["crates/store/src/schema.rs".to_string()]);
    assert_eq!(reason, "the column the fix needs is declared there");
}

/// Empty is a legal declaration and is not a legal request. The difference is
/// that one is an answer — this part changes nothing — and the other spends
/// the one ask a part gets on nothing at all.
#[test]
fn a_request_for_no_paths_is_refused_by_name() {
    use crate::mcp::{read, Incoming, NotAnArgument};

    let called = read(
        br#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"request_scope",
            "arguments":{"paths":[],"reason":"I need more room"}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::NotASubmission {
            why: NotAnArgument::AskedForNothing,
            ..
        }
    ));
}

/// The reason is the whole of what the decision is made on beyond the paths,
/// and it is what a person reads beside a refusal. A blank one is refused where
/// the Drone can still fix it.
#[test]
fn a_request_with_no_reason_is_refused_where_the_drone_can_fix_it() {
    use crate::mcp::{read, Incoming, NotAnArgument};

    let called = read(
        br#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"request_scope",
            "arguments":{"paths":["crates/store"],"reason":"  "}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::NotASubmission {
            why: NotAnArgument::Blank { field: "reason" },
            ..
        }
    ));
}

/// No field for a path to hand back, and none for a criterion. Both are
/// properties of the argument type rather than checks, and this is the shape
/// they take on the wire: a Drone that sends either is told the field does not
/// exist rather than having it quietly dropped.
#[test]
fn a_request_cannot_narrow_and_cannot_raise_its_own_bar() {
    use crate::mcp::{read, Incoming, NotAnArgument};

    for field in ["paths_removed", "acceptance_criteria"] {
        let body = format!(
            r#"{{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{{"name":"request_scope",
               "arguments":{{"paths":["crates/store"],"reason":"why","{field}":["x"]}}}}}}"#
        );
        let called = read(body.as_bytes());
        let Incoming::NotASubmission {
            why: NotAnArgument::NotAField { named, .. },
            ..
        } = called
        else {
            panic!("`{field}` is not a field of this tool");
        };
        assert_eq!(named, field);
    }
}
