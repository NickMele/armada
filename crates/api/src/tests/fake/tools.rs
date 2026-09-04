//! What the fake does with a Drone's tool call.
//!
//! **The `Tools` block, in its own file because the trait is three traits**,
//! and the one of the three whose caller is not Bridge. Every method takes a
//! [`Caller`](crate::Caller) it does not read: which Job a connection belongs
//! to is `fleet::peer`'s answer and is tested there, and a router test's
//! question is whether the arguments arrive and a receipt comes back.
//!
//! [`FakeDaemon::while_working`](super::FakeDaemon) is the only refusal, and it
//! is the only one this crate can honestly raise — the rest is the working slot
//! and belongs to Fleet.

use ipc::mcp::{CheckReport, DeclareScope, DispatchJob, NotRecorded, Receipt, SubmitEvidence};
use std::sync::atomic::Ordering;

use super::FakeDaemon;
use crate::tests::shapes;
use crate::Tools;

impl Tools for FakeDaemon {
    /// The Evidence tool, faked down to what the transport is under test for:
    /// a submission is taken while a Job is running and refused otherwise.
    ///
    /// **The fake names no Job either.** The trait has no parameter for one, so
    /// a fake that wanted to accept evidence for a Job of the caller's choosing
    /// could not express it — which is the binding this crate is able to assert
    /// about, the rest being Fleet's working slot and asserted there.
    /// The caller is taken and not read. **A fake daemon has no processes to
    /// place one against** — which Job a connection belongs to is
    /// `fleet::peer`'s answer and is tested there, and a router test is about
    /// the route rather than about the attribution.
    async fn submit_evidence(
        &self,
        _caller: crate::Caller,
        submission: SubmitEvidence,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("submission to be against")?;
        self.submitted
            .lock()
            .expect("not poisoned")
            .push(submission);
        Ok(Receipt {
            word: "recorded".to_string(),
        })
    }

    async fn declare_scope(
        &self,
        _caller: crate::Caller,
        declaration: DeclareScope,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("declaration to be about")?;
        self.declared
            .lock()
            .expect("not poisoned")
            .push(declaration);
        Ok(Receipt {
            word: "declared".to_string(),
        })
    }

    /// A request for more scope. **No Judge here** — what a real Fleet does
    /// with one is `fleet::widening`'s and is tested there.
    async fn request_scope(
        &self,
        _caller: crate::Caller,
        request: ipc::mcp::RequestScope,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("request to be about")?;
        self.requested.lock().expect("not poisoned").push(request);
        Ok(Receipt {
            word: "widened".to_string(),
        })
    }

    /// A question taken, refused on the one thing this daemon sees: nothing is
    /// being worked. **The receipt says taken, never answered** — what a person
    /// chose arrives in the Drone's session, which no fake has.
    async fn ask_question(
        &self,
        _caller: crate::Caller,
        asking: ipc::mcp::AskQuestion,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("question to be about")?;
        self.asked.lock().expect("not poisoned").push(asking);
        Ok(Receipt {
            word: "asked".to_string(),
        })
    }

    async fn run_checks(&self, _caller: crate::Caller) -> Result<CheckReport, NotRecorded> {
        self.while_working("checks to run")?;
        self.checked.fetch_add(1, Ordering::SeqCst);
        Ok(shapes::check_report())
    }

    /// One minted id, and the call recorded. **The fake decides nothing about
    /// whether the caller was allowed to ask** — that is `fleet::sub_dispatch`,
    /// and a router test's question is whether the arguments arrive and the id
    /// comes back.
    async fn dispatch_job(
        &self,
        _caller: crate::Caller,
        dispatch: DispatchJob,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("Jobs would belong to")?;
        self.dispatched.lock().expect("not poisoned").push(dispatch);
        Ok(Receipt {
            word: "01M0DISPATCHEDCHILD0000000".to_string(),
        })
    }
}
