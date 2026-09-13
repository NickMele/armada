//! A redirect Helm sent is recorded against Helm, on both moves a redirect can
//! make: the one on the send where a step had stopped, and the one when the
//! Drone answers where none had. `#941`.

use core_model::{Actor, JobId};
use store::Moved;

use crate::tests::redirect::{
    a_drone_that_answers, a_fleet_with, advice, refused, stalled, until_roused, Fixture,
};
use crate::tests::tmp::TempDir;

/// Who the Job's last status move is recorded against.
async fn last_mover(fleet: &Fixture, job: &JobId) -> Actor {
    fleet
        .store()
        .lock()
        .await
        .events_for(job)
        .expect("the Job's log")
        .iter()
        .rev()
        .find_map(|event| match event.moved() {
            Moved::Job { .. } => Some(event.actor()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        })
        .expect("a Job that has moved at least once")
}

#[tokio::test]
async fn a_stalled_job_helm_redirected_is_moved_back_by_helm_when_the_drone_turns() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    fleet
        .redirect(&job, &advice(), api::Redirector::Helm)
        .await
        .expect("a Drone to redirect");
    until_roused(&fleet).await;

    assert_eq!(last_mover(&fleet, &job).await, Actor::Helm);
}

#[tokio::test]
async fn a_stopped_step_helm_redirected_is_handed_back_by_helm() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    fleet
        .redirect(&job, &advice(), api::Redirector::Helm)
        .await
        .expect("a Drone to redirect");

    assert_eq!(last_mover(&fleet, &job).await, Actor::Helm);
}

/// The same act from a person is still a person's.
#[tokio::test]
async fn a_persons_redirect_is_still_recorded_as_theirs() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    fleet
        .redirect(&job, &advice(), api::Redirector::Person)
        .await
        .expect("a Drone to redirect");

    assert_eq!(last_mover(&fleet, &job).await, Actor::Human);
}
