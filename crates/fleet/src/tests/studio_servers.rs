//! A server started from a Studio, and what its node keeps when it ends.
//! `#1345`.
//!
//! **Real processes, on `crate::tests::servers`' own fixture**, for that
//! module's reason: whether a port answers, whether a second ask gets the
//! instance already up, and what a server printed before it fell over are none
//! of them things a fake stands in for.

use std::sync::Arc;

use api::{Redirector, Studios};
use ipc::{
    CreateStudio, ServerPhase, StartStudioServer, Studio, StudioNodeContent, StudioPosition,
    StudioRunHeld,
};

use crate::servers::Unservable;
use crate::tests::servers::{a_fleet_serving, exited, serving, Fixture};
use crate::tests::tmp::TempDir;

async fn a_studio(fleet: &Fixture) -> Studio {
    fleet
        .create_studio(
            CreateStudio {
                name: Some("The app, running".to_string()),
            },
            None,
        )
        .await
        .expect("created in the repository Fleet starts in")
}

fn asked(name: &str) -> StartStudioServer {
    StartStudioServer {
        name: name.to_string(),
        position: StudioPosition { x: 40, y: 80 },
        produced_by: None,
    }
}

fn node_on<'a>(studio: &'a Studio, node_id: &ipc::StudioNodeId) -> &'a StudioNodeContent {
    &studio
        .nodes
        .iter()
        .find(|node| &node.id == node_id)
        .expect("the node that was made")
        .content
}

/// **A server started from a Studio is a Run node holding the live instance,
/// and a second ask gets the one already up.**
///
/// The failure this is against is a Studio starting a second copy of a server
/// that is already serving — which is what one instance per checkout rules out
/// everywhere else, and what a node holding a copy of a state rather than an
/// id would hide.
#[tokio::test]
async fn a_server_started_from_a_studio_holds_the_live_instance_and_a_second_ask_gets_it() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let studio = a_studio(&fleet).await;
    let mut watching = events.subscribe();

    let started = Arc::clone(&fleet)
        .start_studio_server(
            studio.id.clone(),
            asked("storybook"),
            Redirector::Person,
            None,
        )
        .await
        .expect("it starts in the main checkout");
    assert!(!started.already_up, "nothing was serving before this");
    assert_eq!(started.server.job_id, None, "no Job: the main checkout's");
    assert!(
        !started.server.ports.is_empty(),
        "its ports are the checkout's own span, not the Studio's"
    );
    assert_eq!(
        node_on(&started.studio, &started.node_id),
        &StudioNodeContent::Run {
            run_id: started.server.id.clone(),
            held: Some(StudioRunHeld::Server),
            kept: None,
        },
        "a reference to the instance, and no copy of anything about it"
    );
    serving(&mut watching, &started.server.id).await;

    let again = Arc::clone(&fleet)
        .start_studio_server(
            studio.id.clone(),
            asked("storybook"),
            Redirector::Person,
            None,
        )
        .await
        .expect("the one already up");
    assert!(again.already_up, "no second copy was started");
    assert_eq!(
        again.server.id, started.server.id,
        "the same instance answered"
    );
    assert_eq!(
        fleet.server_list().servers.len(),
        1,
        "one instance per checkout, however many Studios ask"
    );

    let stopped = fleet
        .stopped_server(&started.server.id)
        .await
        .expect("it stops");
    assert!(stopped.stopped);
    assert_eq!(stopped.phase, ServerPhase::Exited);
}

/// **A server that exits on its own reads as a failure with its exit code, and
/// the node keeps what it printed.**
///
/// The failure this is against is a Studio outliving what it points at, one
/// holder over from `#1289`: a server's result is Fleet's memory and a restart
/// takes it, so the node has to keep it the instant the server ends rather
/// than waiting for a sweep that may never come.
#[tokio::test]
async fn a_server_that_exits_on_its_own_is_kept_on_its_node_as_a_failure_with_its_log() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let studio = a_studio(&fleet).await;
    let mut watching = events.subscribe();

    let started = Arc::clone(&fleet)
        .start_studio_server(
            studio.id.clone(),
            asked("falls_over"),
            Redirector::Person,
            None,
        )
        .await
        .expect("it starts");
    exited(&mut watching, &started.server.id).await;

    let studio = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("the Studio reads back");
    let StudioNodeContent::Run { held, kept, .. } = node_on(&studio, &started.node_id) else {
        panic!("still a Run node");
    };
    assert_eq!(held, &Some(StudioRunHeld::Server), "and still a server's");
    let kept = kept
        .as_ref()
        .expect("the server ended, so its node kept it");
    assert_eq!(kept.name, "falls_over");
    assert_eq!(kept.exit_code, Some(7), "how it ended");
    assert!(
        !kept.stopped,
        "nobody stopped it, which is what makes it a failure"
    );
    assert!(
        kept.lines.iter().any(|line| line == "about to fall over"),
        "what it printed before it went, kept on the node: {:?}",
        kept.lines
    );
}

/// **A Command that declares no `serve` is not startable as a server**, and a
/// server is not runnable as a Check — the two refusals that keep each name on
/// the operation that can hold it.
#[tokio::test]
async fn a_command_is_not_a_server_and_a_server_is_not_a_run() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let studio = a_studio(&fleet).await;

    let refused = Arc::clone(&fleet)
        .start_studio_server(studio.id.clone(), asked("fmt"), Redirector::Person, None)
        .await
        .expect_err("`fmt` runs and exits");
    let api::Refusal::Unacceptable(why) = refused else {
        panic!("a 422");
    };
    assert!(
        why.message.contains("fmt"),
        "the refusal names it: {}",
        why.message
    );
    assert!(
        matches!(
            Arc::clone(&fleet)
                .hold_server(
                    crate::servers::Place::Checkout(crate::checkouts::Checkout::main(
                        fleet.first()
                    )),
                    "fmt",
                    ipc::StartedBy::Person
                )
                .await,
            Err(Unservable::IsACommand { .. })
        ),
        "and it is a Command, not a name nothing declares"
    );

    let run = Arc::clone(&fleet)
        .start_studio_run(
            studio.id.clone(),
            ipc::StartStudioRun {
                name: String::from("storybook"),
                workspace: None,
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a server is held, not run");
    let api::Refusal::Unacceptable(why) = run else {
        panic!("a 422");
    };
    assert!(
        why.message.contains("start_studio_server"),
        "and it says which operation does start one: {}",
        why.message
    );

    let studio = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("it reads back");
    assert!(
        studio.nodes.is_empty(),
        "a refused start leaves no node behind, either way round"
    );
}
