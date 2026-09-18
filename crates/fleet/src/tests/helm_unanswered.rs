//! What a person reads when Helm does not answer.
//!
//! **The sentence is Fleet's, whole.** Both surfaces that draw a `why` used to
//! prepend their own lead-in, so the reply-budget timeout read `No reply came.
//! no reply came within 900 seconds, so the session was ended` in the thread
//! and `no reply came: no reply came within…` in the session record.
//! `crates/fleet/src/helm/unanswered.rs` owns the copy and says which contract
//! line settled it. The two readings are asserted where they are drawn —
//! `packages/screens/src/helm-thread.test.ts` and the `Unanswered` story in
//! `packages/components/src/compositions/HelmRecord/HelmRecord.stories.tsx` —
//! against the same strings this file pins.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::DroneEvent;
use adapters::HeadlessAgent;

use crate::drone::HostPaths;
use crate::helm::{unanswered, Carried, Carry, Heard, Hosting, ProcessHost, REPLY_BUDGET};
use crate::tests::tmp::TempDir;

/// The lead-ins the two surfaces used to add. **Nothing Fleet writes may need
/// one**: a sentence that reads as a fragment without one is a sentence a
/// surface will start framing again.
const LEAD_INS: &[&str] = &["No reply came. ", "no reply came: "];

/// Nothing is heard on any of these paths, and that is asserted rather than
/// ignored — a row before a session starts would be a second defect.
struct Deaf;

impl Heard for Deaf {
    fn heard(&self, events: &[DroneEvent]) {
        assert!(
            events.is_empty(),
            "nothing is heard before a session starts"
        );
    }
}

/// One message carried to `script` under `budget`, and the `why` it failed
/// with. A real process through the real host.
async fn why(script: &str, budget: Duration) -> String {
    let home = TempDir::new();
    let at = home.path().join("stand-in.sh");
    std::fs::write(&at, script).expect("the stand-in is written");
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
    let home_dir = home.path().to_string_lossy().to_string();
    let host = Arc::new(
        ProcessHost::new(
            HeadlessAgent::at(at.to_string_lossy()),
            "stand-in",
            home.path().join("helm-mcp.json"),
            HostPaths {
                path: "/usr/bin:/bin",
                home: &home_dir,
                user: "someone",
            },
        )
        .answering_within(budget),
    );
    let carried = host
        .carry(
            Carry {
                directory: home_dir,
                turn: "why did 77 stop".to_string(),
                resuming: None,
            },
            &Deaf,
        )
        .await;
    match carried {
        Carried::Failed { why } => why,
        other => panic!("the stand-in was meant to fail, and gave {other:?}"),
    }
}

/// **The reply budget ran out** — the reading the doubling was reported on,
/// pinned at the budget a person actually meets.
#[test]
fn the_reply_budget_running_out_reads_as_one_sentence() {
    let said = unanswered::nothing_came_back(REPLY_BUDGET);
    assert_eq!(
        said,
        "no reply came within 900 seconds, so the session was ended"
    );
    assert_eq!(said.matches("no reply came").count(), 1);
}

/// **The session never started**, on each of the four things a process can
/// leave behind. The template this replaced read `the session never started
/// (): ` where it left neither.
#[test]
fn a_session_that_never_started_reads_as_one_sentence() {
    let readings = [
        (
            ("exit status: 1", "the model provider refused this key"),
            "the session never started: the model provider refused this key (exit status: 1)",
        ),
        (
            ("", "the model provider refused this key"),
            "the session never started: the model provider refused this key",
        ),
        (
            ("exit status: 127", ""),
            "the session never started, and left nothing behind saying why (exit status: 127)",
        ),
        (
            ("", ""),
            "the session never started, and left nothing behind saying why",
        ),
    ];
    for ((exited, complaint), reads) in readings {
        assert_eq!(unanswered::never_started(exited, complaint), reads);
    }
}

/// **The store would not answer about the conversation's session.** An error
/// under `fleet.helm_session_unreadable` rather than a thread message, and the
/// one path the two surfaces never draw — it was a fragment for the same
/// reason the rest were.
#[test]
fn a_store_that_will_not_answer_reads_as_one_sentence() {
    assert_eq!(
        unanswered::stored_session_unreachable("database is locked"),
        "the conversation's stored session could not be reached: database is locked"
    );
}

/// **Every sentence stands alone.** A reading that ends with a stop or a
/// dangling colon, or that carries a surface's old lead-in, is one that is
/// about to be framed again by whoever draws it.
#[test]
fn no_failure_needs_a_surface_to_finish_it() {
    let every = [
        unanswered::nothing_came_back(REPLY_BUDGET),
        unanswered::never_started("exit status: 1", "nothing on the socket"),
        unanswered::no_session_started("no model was named"),
        unanswered::door_unconfigured("Permission denied (os error 13)"),
        unanswered::would_not_start("stand-in.sh", "No such file or directory"),
        unanswered::lost_before_held(),
        unanswered::message_not_encoded("a string is not valid JSON"),
        unanswered::message_not_written("Broken pipe (os error 32)"),
        unanswered::session_would_not_read("database is locked"),
        unanswered::lost_session_not_forgotten("database is locked"),
        unanswered::session_not_kept("database is locked"),
        unanswered::no_session_to_resume(),
        unanswered::stored_session_unreachable("database is locked"),
    ];
    for said in every {
        assert!(!said.ends_with('.'), "`{said}` ends with a stop");
        assert!(!said.ends_with(':'), "`{said}` trails off");
        assert!(
            !said.contains("(): "),
            "`{said}` has an empty bracket in it"
        );
        for lead_in in LEAD_INS {
            assert!(!said.contains(lead_in), "`{said}` carries a lead-in");
        }
        assert!(
            said.matches("no reply came").count() <= 1,
            "`{said}` says it twice"
        );
    }
}

/// The never-started path end to end — the wiring, not the builder.
#[tokio::test]
async fn a_stand_in_that_exits_without_a_session_says_so_whole() {
    let said = why(
        "#!/bin/sh\nread -r turn\necho 'the model provider refused this key' >&2\nexit 1\n",
        Duration::from_secs(20),
    )
    .await;
    assert_eq!(
        said,
        "the session never started: the model provider refused this key (exit status: 1)"
    );
}

/// The budget end to end. Two seconds rather than fifteen minutes, so what is
/// asserted here is the shape; the seconds a person reads are pinned above.
#[tokio::test]
async fn a_stand_in_that_never_answers_says_the_session_was_ended() {
    let said = why(
        "#!/bin/sh\nread -r turn\nsleep 60\n",
        Duration::from_secs(2),
    )
    .await;
    assert_eq!(
        said,
        "no reply came within 2 seconds, so the session was ended"
    );
}
