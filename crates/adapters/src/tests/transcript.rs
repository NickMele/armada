//! Reading a real Drone's output, against real Drone output.
//!
//! # The captures are the specification
//!
//! Every stream read here is a byte-for-byte capture from a spike run, checked
//! into `docs/spikes/`. A decoder tested against fixtures somebody wrote to
//! match the decoder proves the decoder agrees with itself; these were written
//! by the CLI, before this file existed, and one of them is a run that reported
//! success having done nothing.
//!
//! **They are also the drift alarm.** When the stream's shape changes, the case
//! that fails is a line the capture holds and the decoder no longer understands
//! — which surfaces as `Unreadable`, by design, rather than as an event that
//! quietly stopped arriving.
//!
//! One thing about the captures is not byte-for-byte and it is stated where it
//! matters: the operator's own inventory in the opening event is replaced by a
//! count. That is the field this file asserts on, and the count is what the
//! confinement claim rests on, so nothing here is weakened by it.

use std::path::PathBuf;

use adapter_traits::{CallDetail, DroneEvent};

use crate::transcript::read;

fn capture(name: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/spikes")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{} is a checked-in capture: {why}", path.display()))
        .lines()
        .map(String::from)
        .collect()
}

fn all_of(name: &str) -> Vec<DroneEvent> {
    capture(name).iter().flat_map(|line| read(line)).collect()
}

#[test]
fn a_whole_real_run_reads_with_nothing_unreadable() {
    let events = all_of("003-transcript.ndjson");
    let unreadable: Vec<&DroneEvent> = events.iter().filter(|e| e.is_unreadable()).collect();
    assert!(
        unreadable.is_empty(),
        "a real run must read end to end: {unreadable:?}"
    );
    assert!(
        events.len() >= capture("003-transcript.ndjson").len(),
        "a line can be more than one event and is never fewer than one"
    );
}

#[test]
fn the_session_opening_says_how_many_servers_the_drone_actually_came_up_with() {
    // The reading that can contradict the flag Armada passed, taken from inside
    // the session rather than from the argument list Armada wrote.
    let started = all_of("003-transcript.ndjson")
        .into_iter()
        .find(|event| matches!(event, DroneEvent::Started { .. }))
        .expect("a run opens");

    let DroneEvent::Started {
        session,
        model,
        mcp_servers,
    } = started
    else {
        unreachable!()
    };
    assert!(!session.is_empty());
    assert!(!model.is_empty());
    assert_eq!(mcp_servers, 1, "the capture carries the count as one entry");
}

#[test]
fn a_denied_call_is_an_event_and_not_an_absence() {
    // Three independent places carry a denial, so nothing here has to infer one
    // from a tool call with no result.
    let events = all_of("003-transcript-denial.ndjson");
    let refusals: Vec<&DroneEvent> = events.iter().filter(|e| e.is_a_refusal()).collect();
    assert!(!refusals.is_empty(), "a denial is never silent here");

    let DroneEvent::Refused { tool, because, .. } = refusals[0] else {
        unreachable!()
    };
    assert_eq!(tool, "Bash");
    assert!(!because.is_empty(), "the harness's own wording is carried");
}

#[test]
fn the_run_that_reported_success_having_done_nothing_carries_its_refusals() {
    // The finding that decides what a gate may read. Exit 0, `is_error` false,
    // a success subtype and a polite final message — and `permission_denials`
    // is the cheap contradiction. There is no field on `Ended` for the other
    // four, so no gate can read one.
    let ended = all_of("003-transcript-denial.ndjson")
        .into_iter()
        .find(|event| matches!(event, DroneEvent::Ended { .. }))
        .expect("a run ends");

    let DroneEvent::Ended { refusals, .. } = ended else {
        unreachable!()
    };
    assert!(
        refusals > 0,
        "the run that did nothing must be distinguishable from the one that did"
    );
}

#[test]
fn the_terminating_event_carries_the_cost_as_an_integer() {
    let ended = all_of("003-transcript.ndjson")
        .into_iter()
        .find(|event| matches!(event, DroneEvent::Ended { .. }))
        .expect("a run ends");

    assert_eq!(
        ended,
        DroneEvent::Ended {
            turns: 5,
            cost_micros: 117_761,
            refusals: 0,
        }
    );
}

#[test]
fn a_run_with_an_injected_turn_reads_end_to_end_too() {
    // The stream from the injection spike: it carries a replayed user message,
    // whose content is a plain string rather than a block list, and it emits
    // two terminating events in one process.
    let events = all_of("004-transcript-idle-session.ndjson");
    assert!(
        !events.iter().any(|e| e.is_unreadable()),
        "a replayed turn must not read as unreadable"
    );
    let endings = events
        .iter()
        .filter(|e| matches!(e, DroneEvent::Ended { .. }))
        .count();
    assert!(
        endings > 1,
        "one process emitted more than one terminating event, which is why an \
         ending is a turn boundary and not a lifetime"
    );
}

#[test]
fn a_line_that_is_not_json_is_an_event_rather_than_a_dropped_line() {
    let read = read("this is not a transcript at all");
    assert_eq!(read.len(), 1);
    let DroneEvent::Unreadable { line, why } = &read[0] else {
        panic!("{read:?}")
    };
    assert_eq!(line, "this is not a transcript at all");
    assert!(!why.is_empty(), "the reason is carried, not invented later");
}

#[test]
fn a_truncated_line_is_unreadable_and_says_so() {
    // What a Drone killed mid-write leaves behind. Reading it as "nothing
    // happened" is the failure this variant exists to prevent.
    let read = read(r#"{"type":"assistant","message":{"content":[{"type":"tool"#);
    assert!(read[0].is_unreadable(), "{read:?}");
}

#[test]
fn a_blank_line_is_unreadable_rather_than_skipped() {
    assert!(read("   ")[0].is_unreadable());
}

#[test]
fn an_unreadable_line_is_kept_short() {
    let long = format!("{{not json{}", "x".repeat(4_000));
    let DroneEvent::Unreadable { line, .. } = &read(&long)[0] else {
        panic!("expected an unreadable line")
    };
    assert!(line.len() < 400, "a runaway line is not copied whole");
}

#[test]
fn a_kind_this_vocabulary_has_no_name_for_is_reported_by_kind() {
    let read = read(r#"{"type":"something_new","payload":1}"#);
    assert_eq!(
        read,
        vec![DroneEvent::Unrecognised {
            kind: String::from("something_new")
        }]
    );
}

#[test]
fn a_turn_carrying_two_tool_calls_is_two_events() {
    let read = read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"Read"},
             {"type":"tool_use","id":"b","name":"Edit"}]}}"#,
    );
    assert_eq!(
        read,
        vec![
            DroneEvent::Called {
                tool: String::from("Read"),
                call: String::from("a"),
                detail: CallDetail::none(),
            },
            DroneEvent::Called {
                tool: String::from("Edit"),
                call: String::from("b"),
                detail: CallDetail::none(),
            },
        ],
        "answering with only the first would drop work the Drone did"
    );
}

/// `Bash · toolu_01Haa…` twenty-two times reads the same whether the Drone ran
/// `ls` or `rm -rf`. These are the three shapes the approved transcript design
/// shows, read out of the real stream's own keys.
#[test]
fn a_call_carries_what_it_did() {
    let read = read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"Bash",
              "input":{"command":"cargo build --workspace","timeout":60000}},
             {"type":"tool_use","id":"b","name":"Read",
              "input":{"file_path":"/tmp/repo/src/settings.rs"}},
             {"type":"tool_use","id":"c","name":"Edit",
              "input":{"file_path":"/tmp/repo/reducer.rs",
                       "old_string":"one\ntwo","new_string":"a\nb\nc"}}]}}"#,
    );
    let shown: Vec<&str> = read
        .iter()
        .map(|event| match event {
            DroneEvent::Called { detail, .. } => detail.text(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        shown,
        vec![
            "cargo build --workspace",
            "/tmp/repo/src/settings.rs",
            "/tmp/repo/reducer.rs +3 -2",
        ]
    );
}

/// A `Write` argument is a whole file. The bound is the type's and not this
/// decoder's, so what is asserted here is that a row says it was cut rather
/// than leaving a reader to guess from a trailing character.
#[test]
fn a_runaway_argument_is_cut_and_the_row_says_so() {
    let long = "x".repeat(4_000);
    let read = read(&format!(
        r#"{{"type":"assistant","message":{{"content":[
             {{"type":"tool_use","id":"a","name":"Bash","input":{{"command":"{long}"}}}}]}}}}"#
    ));
    let DroneEvent::Called { detail, .. } = &read[0] else {
        panic!("{read:?}")
    };
    assert!(detail.truncated(), "a cut row says it was cut");
    assert!(detail.text().len() < 400, "{}", detail.text().len());
}

/// A heredoc is many lines and a row is one. Collapsing is what keeps a
/// transcript readable at Drone speed.
#[test]
fn a_multi_line_command_collapses_to_one_line() {
    let read = read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"Bash",
              "input":{"command":"cat <<EOF\n  one\n  two\nEOF"}}]}}"#,
    );
    let DroneEvent::Called { detail, .. } = &read[0] else {
        panic!("{read:?}")
    };
    assert_eq!(detail.text(), "cat <<EOF one two EOF");
    assert!(!detail.truncated());
}

/// Spike 3's capture had to be scrubbed by hand before this repository could
/// hold it, and what needed removing was the operator's home path. A row naming
/// a file the Drone read would carry it again on every call.
#[test]
fn a_path_under_a_home_directory_does_not_carry_the_operator_s_name() {
    for path in [
        "/Users/user/Development/armada/src/lib.rs",
        "/home/user/Development/armada/src/lib.rs",
    ] {
        let read = read(&format!(
            r#"{{"type":"assistant","message":{{"content":[
                 {{"type":"tool_use","id":"a","name":"Read","input":{{"file_path":"{path}"}}}}]}}}}"#
        ));
        let DroneEvent::Called { detail, .. } = &read[0] else {
            panic!("{read:?}")
        };
        assert_eq!(detail.text(), "~/Development/armada/src/lib.rs");
    }
}

/// A tool whose arguments this vocabulary has no name for is still a call. An
/// empty detail says "nothing to show" without the decoder guessing at a shape.
#[test]
fn a_call_whose_arguments_have_no_name_here_is_still_a_call() {
    let read = read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"TodoWrite","input":{"todos":[]}}]}}"#,
    );
    let DroneEvent::Called { detail, tool, .. } = &read[0] else {
        panic!("{read:?}")
    };
    assert_eq!(tool, "TodoWrite");
    assert_eq!(detail, &CallDetail::none());
}

/// The captures are the specification, and the largest argument in the stream
/// is a `Write`'s file body. It has no field on `ToolInput`, so no row can
/// carry one — asserted against the real capture rather than a written fixture.
#[test]
fn a_real_write_call_carries_its_path_and_not_its_content() {
    let written = all_of("004-transcript-during-tool-call.ndjson")
        .into_iter()
        .filter_map(|event| match event {
            DroneEvent::Called { tool, detail, .. } if tool == "Write" => Some(detail),
            _ => None,
        })
        .next()
        .expect("the capture writes a file");
    assert!(written.text().ends_with("POKED2.txt"), "{written:?}");
    assert!(!written.text().contains("POKED\""), "{written:?}");
}

#[test]
fn a_turn_with_nothing_armada_names_in_it_is_still_an_event() {
    let read = read(r#"{"type":"assistant","message":{"content":[{"type":"a_new_block"}]}}"#);
    assert_eq!(read.len(), 1);
    assert!(
        matches!(read[0], DroneEvent::Unrecognised { .. }),
        "{read:?}"
    );
}

/// The turn that made the removal invisible: the Drone thought, then reached
/// for a tool, and what shipped answered with the call alone. The reasoning was
/// synthesised into a row only when the turn had nothing else in it, so the
/// commonest turn in the stream lost it silently.
#[test]
fn a_turn_that_reasoned_and_then_called_a_tool_still_says_it_reasoned() {
    let read = read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"thinking","thinking":"a long look at the problem","signature":"abc"},
             {"type":"tool_use","id":"a","name":"Read",
              "input":{"file_path":"/tmp/repo/src/lib.rs"}}]}}"#,
    );
    let kinds: Vec<&str> = read
        .iter()
        .filter_map(|event| match event {
            DroneEvent::Unrecognised { kind } => Some(kind.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(kinds.len(), 1, "the turn reasoned and says so: {read:?}");
    assert!(
        matches!(read[1], DroneEvent::Called { .. }),
        "the call keeps its own row and its order: {read:?}"
    );

    // Distinguishable from a turn that held nothing, which is the row a reader
    // already learns nothing from — and never the working itself.
    let empty = read_kind(r#"{"type":"assistant","message":{"content":[]}}"#);
    assert_ne!(kinds[0], empty);
    assert!(!kinds[0].contains("a long look"), "{kinds:?}");
}

/// A turn's working arrives as several blocks. The fact a reader is owed is
/// about the turn, and a row each would add volume to a fold that already reads
/// as noise.
#[test]
fn a_turn_that_reasoned_twice_says_so_once() {
    let read = read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"thinking","thinking":"one"},
             {"type":"redacted_thinking","data":"opaque"},
             {"type":"text","text":"done"},
             {"type":"thinking","thinking":"two"}]}}"#,
    );
    let reasoning = read
        .iter()
        .filter(|event| matches!(event, DroneEvent::Unrecognised { .. }))
        .count();
    assert_eq!(reasoning, 1, "{read:?}");
    assert_eq!(read.len(), 2, "the prose keeps its own row: {read:?}");
}

/// The captures are the specification, and spike 4's is a real run whose turns
/// carry reasoning beside their calls. Asserted against the stream rather than
/// against a fixture written to match this decoder.
#[test]
fn a_real_run_that_reasoned_leaves_a_row_saying_so() {
    let kinds: Vec<String> = all_of("004-transcript-during-tool-call.ndjson")
        .into_iter()
        .filter_map(|event| match event {
            DroneEvent::Unrecognised { kind } => Some(kind),
            _ => None,
        })
        .collect();
    assert!(
        kinds.iter().any(|kind| kind.contains("reasoning")),
        "the capture holds thinking blocks: {kinds:?}"
    );
}

fn read_kind(line: &str) -> String {
    let read = read(line);
    let DroneEvent::Unrecognised { kind } = &read[0] else {
        panic!("{read:?}")
    };
    kind.clone()
}

#[test]
fn a_quota_window_moving_is_read_and_is_not_about_this_job() {
    let read = read(
        r#"{"type":"rate_limit_event","rate_limit_info":
             {"status":"allowed","rateLimitType":"five_hour"}}"#,
    );
    assert_eq!(
        read,
        vec![DroneEvent::QuotaMoved {
            window: String::from("five_hour"),
            status: String::from("allowed"),
        }]
    );
}
