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

use ipc::mcp::{
    ChecksStarted, DeclareScope, DispatchJob, NotRecorded, PermissionAsked, Receipt, SubmitEvidence,
};
use std::sync::atomic::Ordering;

use super::FakeDaemon;
use crate::{PermissionAnswer, Tools};

impl Tools for FakeDaemon {
    /// Allowed while a Job is running and refused otherwise, which is all the
    /// transport is under test for: that the decision reaches the reply.
    async fn permission(
        &self,
        _caller: crate::Caller,
        _asked: PermissionAsked,
    ) -> PermissionAnswer {
        match self.while_working("call to be about") {
            Ok(()) => PermissionAnswer::Allow,
            Err(why) => PermissionAnswer::Deny(why.because),
        }
    }

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

    /// A note taken while a Job is running, answering with who it was for so a
    /// router test can tell the argument arrived. **Who may be addressed** is
    /// `fleet::peers`' and is tested there.
    async fn leave_note(
        &self,
        _caller: crate::Caller,
        note: ipc::mcp::LeaveNote,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("note to be from")?;
        Ok(Receipt {
            word: format!("left for {}", note.to),
        })
    }

    /// A fix asked for while a Job is running, answering with the test it named
    /// so a router test can tell the argument arrived. **The run against main
    /// and the draft** are `fleet::fixing`'s and tested there.
    async fn draft_fix(
        self: std::sync::Arc<Self>,
        _caller: crate::Caller,
        fix: ipc::mcp::DraftFix,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("fix to be asked from")?;
        Ok(Receipt {
            word: format!("drafting a fix for {}", fix.test),
        })
    }

    async fn run_checks(
        self: std::sync::Arc<Self>,
        _caller: crate::Caller,
        _only_what_changed: bool,
    ) -> Result<ChecksStarted, NotRecorded> {
        self.while_working("checks to run")?;
        self.checked.fetch_add(1, Ordering::SeqCst);
        Ok(ChecksStarted)
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

    /// Refused while nothing works, and otherwise refused naming the server,
    /// so a router test can tell the name arrived. **The fake holds no server**
    /// — one instance per Job is `fleet::servers`' and tested there.
    async fn start_server(
        self: std::sync::Arc<Self>,
        _caller: crate::Caller,
        name: String,
    ) -> Result<ipc::mcp::ServerReport, NotRecorded> {
        self.while_working("server to be for")?;
        Err(NotRecorded {
            because: format!("the fake daemon starts no server, `{name}` included"),
        })
    }

    /// Taken while a Job is running, answering with the tool that was called so
    /// a router test can tell the call arrived. **Which step may call which** is
    /// `fleet::work_plan`'s and is tested there.
    async fn change_plan(
        &self,
        _caller: crate::Caller,
        call: ipc::mcp::PlanCall,
    ) -> Result<Receipt, NotRecorded> {
        self.while_working("plan to be about")?;
        Ok(Receipt {
            word: call.tool.to_string(),
        })
    }
}
