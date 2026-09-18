//! The record a person carries when a Helm answer is bad. `#1367`.
//!
//! **A stand-in host writes the thread through the real fold**, so what the
//! record holds is what a session's own stream became, not a hand-built list.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::DroneEvent;
use api::Conversations;
use ipc::{AskHelm, Cursor, EventTally, EventsSince, HelmDebugSaid, HelmText, ManifestId};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::helm::{Carried, Carry, Carrying, Heard, Hosting};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A session that answers one message with a call, a refusal, a sentence and a
/// turn's end — the four kinds of line a record is read for.
struct StandIn;

impl Hosting for StandIn {
    fn carry<'a>(&'a self, _carry: Carry, heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(async move {
            heard.heard(&[
                DroneEvent::Started {
                    session: "a-session".to_string(),
                    model: "stand-in".to_string(),
                    mcp_servers: 19,
                },
                DroneEvent::Called {
                    tool: "Bash".to_string(),
                    call: "toolu_1".to_string(),
                    detail: adapter_traits::CallDetail::of("pnpm test"),
                },
                DroneEvent::Refused {
                    tool: "Write".to_string(),
                    call: "toolu_2".to_string(),
                    because: "the person refused it".to_string(),
                },
                DroneEvent::Said {
                    text: "I cannot answer that.".to_string(),
                    by: adapter_traits::Speaker::Drone,
                },
                DroneEvent::Ended {
                    turns: 3,
                    cost_micros: 24_300,
                    refusals: 1,
                },
            ]);
            Carried::Answered {
                session: "a-session".to_string(),
            }
        }) as Pin<Box<dyn Future<Output = _> + Send>>
    }

    fn model(&self) -> String {
        String::from("a-model")
    }

    fn running(&self) -> Vec<u32> {
        Vec::new()
    }
}

fn hosted(home: &TempDir) -> Arc<Hosted> {
    Arc::new(
        Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
            .hosting_helm_on(Arc::new(StandIn)),
    )
}

/// One message, answered, with the record taken once the reply is in.
async fn answered(fleet: &Arc<Hosted>, text: &str) {
    Arc::clone(fleet)
        .ask_helm(
            AskHelm {
                text: HelmText::said(text).expect("not blank"),
                context: None,
            },
            None,
        )
        .await
        .expect("the message is taken");
    for _ in 0..500 {
        if !fleet.observe_helm(None).await.expect("observed").replying {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the reply never settled");
}

#[tokio::test]
async fn the_record_holds_the_brief_the_roster_the_thread_and_what_the_turn_cost() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    answered(&fleet, "why did you refuse that").await;

    let record = fleet
        .get_helm_debug_info(None)
        .await
        .expect("a record for the repository Fleet started in");

    assert!(
        record.brief.contains("You are Helm, in Armada."),
        "the brief as it was sent: {}",
        record.brief
    );
    assert!(
        record.tools.contains(&"list_jobs".to_string()),
        "the door's roster, by name: {:?}",
        record.tools
    );
    assert_eq!(record.model, "a-model");
    assert_eq!(record.servers, Some(19), "what the session came up holding");
    assert_eq!(record.session.as_deref(), Some("a-session"));
    assert_eq!(record.cut, 0, "nothing was left out of four lines");

    let said: Vec<&HelmDebugSaid> = record.thread.iter().map(|line| &line.said).collect();
    assert!(
        matches!(said[0], HelmDebugSaid::Asked { text } if text.text == "why did you refuse that"),
        "what was asked, first: {said:?}"
    );
    assert!(
        said.iter()
            .any(|one| matches!(one, HelmDebugSaid::Called { tool, .. } if tool == "Bash")),
        "the call it made: {said:?}"
    );
    assert!(
        said.iter()
            .any(|one| matches!(one, HelmDebugSaid::Refused { tool, .. } if tool == "Write")),
        "the call it was refused: {said:?}"
    );
    assert!(
        said.iter().any(|one| matches!(
            one,
            HelmDebugSaid::Ended {
                turns: 3,
                cost_micros: 24_300,
                refusals: 1
            }
        )),
        "what the turn cost, and how many turns it took: {said:?}"
    );
}

#[tokio::test]
async fn the_record_reports_the_poll_the_session_was_answered() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let manifest_id = fleet
        .get_helm_debug_info(None)
        .await
        .expect("a record")
        .manifest_id;

    assert!(
        fleet
            .get_helm_debug_info(None)
            .await
            .expect("a record")
            .polled
            .is_none(),
        "a session that has not polled claims no window"
    );

    fleet
        .helm_polled(
            ManifestId::carried(manifest_id.as_str()),
            EventsSince {
                from: Cursor::at(41),
                upto: Cursor::at(58),
                kinds: vec![EventTally {
                    kind: "job.state_changed".to_string(),
                    count: 9,
                }],
                missed: None,
            },
        )
        .await;

    let polled = fleet
        .get_helm_debug_info(None)
        .await
        .expect("a record")
        .polled
        .expect("the window its last poll was answered");
    assert_eq!(polled.from.position(), 41);
    assert_eq!(polled.upto.position(), 58);
    assert_eq!(polled.kinds[0].count, 9);
}
