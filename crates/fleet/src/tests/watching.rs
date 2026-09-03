//! What a watched call says about itself while it is out, and what stopping one
//! does.
//!
//! **The claim these hold down.** A person dispatching a Job waits on a model
//! call with nothing on screen but an elapsed count, and an elapsed count
//! cannot tell a model thinking hard from a harness that never reached the
//! vendor. Those take opposite decisions — one is worth waiting out and the
//! other never will be — so the wait has to say which it is looking at.
//!
//! **A real process, a real pipe, a real reader.** `FakeJudge::render_watched`
//! scripts a shell that prints the lines the vendor's own stream prints, with a
//! beat between them, and the reader under test is the shipped one:
//! `FakeJudge::heard` delegates to `HeadlessAgent`'s. What is faked is the
//! model and nothing else — a suite that faked the reader could pass against a
//! format the shipped client cannot parse, which is the one failure a fake at
//! this seam must not be able to hide.
//!
//! The stop cases are the reason the script sleeps. A call that finished before
//! anybody could ask it to stop proves nothing about stopping.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{Ask, CallProgress, Environment, Model};
use testkit::FakeJudge;

use crate::judging::{watched, CallFailed, JudgeBudget};

/// A budget nothing in here is meant to reach. The scripted call takes about
/// two hundred milliseconds; anything approaching this is the test hanging.
fn generous() -> JudgeBudget {
    JudgeBudget::of(Duration::from_secs(10))
}

fn ask() -> Ask {
    Ask::put(
        Model::named("haiku").expect("a model"),
        "which workflow fits: the parser crashes on an empty file",
        Environment::default(),
    )
    .expect("an ask")
}

/// Everything the call reported, in the order it reported it.
#[derive(Clone, Default)]
struct Reported(Arc<Mutex<Vec<CallProgress>>>);

impl Reported {
    fn telling(&self) -> impl Fn(CallProgress) + Send + Sync + use<> {
        let held = Arc::clone(&self.0);
        move |progress| held.lock().expect("not poisoned").push(progress)
    }

    fn seen(&self) -> Vec<CallProgress> {
        self.0.lock().expect("not poisoned").clone()
    }
}

/// A future that never resolves. **The ordinary case**: nobody asked this call
/// to stop, so the arm that stops it must never be taken.
async fn nobody_stops() {
    std::future::pending::<()>().await
}

/// The whole claim, in order. Started, then at the vendor, then thinking — and
/// the answer still comes back, because progress is a courtesy and must not
/// change what the call returns.
#[tokio::test]
async fn a_watched_call_reports_getting_started_before_it_answers() {
    let judge = FakeJudge::saying("verdict: met");
    let reported = Reported::default();

    let said = watched(
        &judge,
        &ask(),
        generous(),
        &reported.telling(),
        nobody_stops(),
    )
    .await
    .expect("the call answered");

    assert_eq!(said.trim(), "verdict: met");
    let seen = reported.seen();
    assert_eq!(
        seen.first(),
        Some(&CallProgress::Started),
        "the harness announcing itself is the first thing a caller learns, and \
         the one that separates a hung harness from a slow model: {seen:?}"
    );
    assert!(
        seen.contains(&CallProgress::Requesting),
        "the moment the question reached the vendor is what makes the elapsed \
         count mean something: {seen:?}"
    );
    assert!(
        seen.iter()
            .any(|progress| matches!(progress, CallProgress::Thinking { .. })),
        "a call that is thinking says so, with how much of it there has been: \
         {seen:?}"
    );
}

/// **The answer is not assembled from the progress.** A caller that lost every
/// reading gets the same answer a beat later, which is what keeps a courtesy
/// from becoming a second authority on what the call said.
#[tokio::test]
async fn what_the_call_answers_does_not_depend_on_anybody_reading_it() {
    let judge = FakeJudge::saying("verdict: not_met\nexpected: a test\nproduced: none");

    let said = watched(&judge, &ask(), generous(), &|_| {}, nobody_stops())
        .await
        .expect("the call answered");

    assert_eq!(
        said.trim(),
        "verdict: not_met\nexpected: a test\nproduced: none",
        "a multi-line answer survives the stream whole; it is read off the \
         line the turn arrives on rather than reassembled from frames"
    );
}

/// A call that ends badly is still a failed call, and **still not a stop**.
/// The two are different variants because a person deciding not to wait and a
/// call the vendor refused take different sentences.
#[tokio::test]
async fn a_call_that_fails_is_a_failure_and_not_a_stop() {
    let judge = FakeJudge::that_fails("the quota");

    let failed = watched(&judge, &ask(), generous(), &|_| {}, nobody_stops())
        .await
        .expect_err("the call failed");

    assert!(
        matches!(failed, CallFailed::Refused { .. }),
        "a non-zero exit is a refused call: {failed:?}"
    );
}

/// The control the whole feature exists for. **Stopping wins over the budget**
/// — a person who has decided not to wait is not made to wait out the rest of
/// Fleet's two minutes.
#[tokio::test]
async fn a_call_somebody_stops_ends_at_stopped() {
    let judge = FakeJudge::saying("verdict: met");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    // After the first readings and before the answer, which is where a person
    // deciding is actually standing.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(60)).await;
        let _ = stop.send(());
    });

    let failed = watched(
        &judge,
        &ask(),
        // Long enough that reaching it would mean the stop did nothing.
        JudgeBudget::of(Duration::from_secs(30)),
        &|_| {},
        async {
            let _ = stopped.await;
        },
    )
    .await
    .expect_err("the call was stopped");

    assert_eq!(
        failed,
        CallFailed::Stopped,
        "stopping is its own outcome and never a fault: a surface drawing it \
         as one would tell somebody Armada broke when they decided"
    );
}

/// A budget that expires is still `TimedOut`, and stopping did not replace it.
/// The two arms answer different questions — nobody decided here.
#[tokio::test]
async fn a_watched_call_still_times_out_when_nobody_stops_it() {
    let judge = FakeJudge::saying("verdict: met");

    let failed = watched(
        &judge,
        &ask(),
        // Shorter than the scripted stream's own beats.
        JudgeBudget::of(Duration::from_millis(20)),
        &|_| {},
        nobody_stops(),
    )
    .await
    .expect_err("the budget expired");

    assert_eq!(failed, CallFailed::TimedOut);
}
