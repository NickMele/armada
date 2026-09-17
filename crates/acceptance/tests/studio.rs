//! Studio's claim: **I can work something out on a Studio — notes from using
//! the app, what a scout read, what I pasted in — turn what holds up into a
//! Job, and come back later to see how I got there.** The apparatus is
//! [`bench::studio`].
//!
//! **Almost none of it is carried yet, and a green run here says so rather
//! than hiding it.** No crate holds a Studio, a node or an edge, so the claim
//! is the table below, in the order a person meets it. The change that builds
//! a row adds its assertion to this file, made against a value that has been
//! through [`ipc::encode`] and back as `board.rs`'s are, and takes the row out.
//!
//! | Step of the claim | Carried by |
//! |---|---|
//! | 1. A Studio is created for a repository, and its nodes, edges and positions cross the wire | #1285 |
//! | 2. A Run node is started from it and ends failed, and keeps its log's tail and result past retention | #1289. **What it meets:** a checkout run today is `ipc::CheckoutRunRecord`, whose end is an exit code and a sentence documented as unhued — there is no run state for a Run node to alias to a Job status |
//! | 3. A Note is captured, and nothing writes to it afterwards | #1290 |
//! | 4. A scout's Finding arrives Frozen, listing every file and source it read | #1292 |
//! | 5. An issue from the repository's forge is read in, and a Contradiction appears | #1293 |
//! | 6. Two Notes are clustered, written up as an Issue draft, and dispatched, and a Job node stands at the gate | #1291. **The dispatch's far half is asserted below**: an Issue draft's text alone reaches the Job proposer, and the Job it proposes is told all of it |
//! | 7. After a restart the Studio reads back with every node where it was left | #1285. The restart itself touches a file, so it is `store`'s own test; what comes here is the record decoding back through the wire |
//! | Helm's unasked proposals, and the acts it takes only on a person's ask | #1288 |
//!
//! | Not proved here | Why not |
//! |---|---|
//! | That a proposer reading the draft chooses well | Choosing is a model's, and this file calls none. The answer below is written by the test |
//! | That the proposed Job is created at `awaiting_approval` | `fleet::drafting`'s conversion from a proposal to a Job is `pub(crate)`, so building one here would assert what the test built. `fleet`'s own tests create one |
//! | That a reopened Studio is read-only until Continue | A Bridge state; #1287's mock browser test proves it |
//! | Anything a person sees | Nothing here renders. The whiteboard is #1286 and #1287 |

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use fleet::{Brief, Proposal};

use bench::studio::{
    an_issue_draft, held, one_job_under, received_request, DRAFT_TITLE, FIRST_NOTE, SECOND_NOTE,
};

/// Step 6's far half: **an Issue draft is dispatched from its text, through the
/// Job proposer, and the Job it becomes is told the whole of it.**
/// `docs/concepts/studio.md`, *Promotion*: filing the issue anywhere is
/// optional, so nothing between the draft and the proposer may need it filed.
///
/// **The failure this is against is a lossy hop.** A write-up is made from the
/// Notes feeding it, and a dispatch that summarised, trimmed or re-fetched the
/// draft on the way would hand a Drone something other than what the person
/// saw. So the draft's text goes over the wire as a request, the proposer is
/// asked with it, and a one-Job answer reads back with the draft as that Job's
/// brief, both Notes' words in it.
#[test]
fn an_issue_drafts_text_alone_reaches_the_proposer_and_the_job_is_told_all_of_it() {
    let draft = an_issue_draft();
    let received = received_request(&draft);
    assert_eq!(
        received.request, draft,
        "the draft crosses the wire verbatim, with no issue filed to point at"
    );

    let held = held();
    let brief = Brief::about(&received.request, &held);
    assert!(
        brief.question().contains(&draft),
        "the proposer is asked with the draft as the person wrote it up: {}",
        brief.question()
    );

    let workflow = held.keys().next().expect("the bench holds a workflow");
    let Ok(Proposal::Resolved(jobs)) = brief.read(&one_job_under(workflow), &held) else {
        panic!("an answer naming a held workflow and a title is a proposal");
    };
    assert_eq!(
        jobs.len(),
        1,
        "one draft, written up as one thing, is one Job"
    );
    let job = &jobs[0];
    assert_eq!(job.title, DRAFT_TITLE, "the Job keeps the draft's title");
    assert_eq!(
        job.brief, draft,
        "what the Job's Drone is told is the whole draft, not a part of it"
    );
    for note in [FIRST_NOTE, SECOND_NOTE] {
        assert!(
            job.brief.contains(note),
            "both Notes the draft was written up from reach the Drone: {note}"
        );
    }
}
