//! The tools a Drone calls, served over a real Fleet.
//!
//! **These are not operations.** `crates/ipc/operations.toml` is the authority
//! on the operation set and holds no row for any of them — they come off the
//! tool roster instead, and the difference is who is calling. Every one takes
//! an [`api::Caller`] rather than a [`ipc::JobId`], because the Drone does not
//! know which Job it is; [`Fleet::placed`] reads that off the connection.
//!
//! **Nothing here answers a status code.** A refusal is a [`NotRecorded`],
//! which reaches the model as a 200 carrying `isError` — a 4xx reads to a Drone
//! as a broken server, which is something it stops trying. So neither the
//! redaction `serving` describes nor [`Fleet::refusal`] applies to anything
//! below: what crosses is a [`Receipt`] or a report, never a Job.
//!
//! **Each one converts and maps, and decides nothing.** The binding — which
//! Job, which step, whether the caller was allowed to ask — is made under the
//! slot lock by the `Fleet` method each of these calls.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::mcp::{
    CheckReport, DeclareScope, DispatchJob, NotRecorded, Receipt, RequestScope, SubmitEvidence,
};

use crate::daemon::Fleet;
use crate::wire::told;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Which Job made this call, as the tool's own refusal.
    ///
    /// **Every tool here opens with this**, and none of them takes a Job id:
    /// `Fleet::caller_of` reads the connection, and a caller it cannot place is
    /// told so as a tool error the Drone can read rather than as a 4xx it can
    /// only retry.
    pub(crate) fn placed(&self, caller: &api::Caller) -> Result<core_model::JobId, NotRecorded> {
        self.caller_of(caller).map_err(|why| NotRecorded {
            because: why.to_string(),
        })
    }

    /// The working Drone asking a person something it cannot answer from the
    /// repository. Binding and refusals are `Fleet::ask_question`'s, under the
    /// slot lock. **The receipt says taken, never answered**: what a person
    /// chose arrives as a later turn, which is why this does not block — see
    /// `crate::questioning`.
    pub(crate) async fn asked(
        &self,
        caller: api::Caller,
        asking: ipc::mcp::AskQuestion,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        Fleet::ask_question(self, &job, asking).await?;
        Ok(Receipt {
            word: "asked".to_string(),
        })
    }

    /// The Evidence tool, which is what a step's claim arrives through.
    ///
    /// The binding — which Job, which step, which evidence type — is
    /// `Fleet::record_evidence`'s, under the lock that makes it a single
    /// decision.
    pub(crate) async fn submitted(
        &self,
        caller: api::Caller,
        submission: SubmitEvidence,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        match self.record_evidence(&job, &submission).await {
            Ok(recorded) => Ok(Receipt {
                word: recorded.word().to_string(),
            }),
            Err(why) => Err(told(why)),
        }
    }

    /// Where the working Drone says this step's work will be. The binding —
    /// which Job, which step — is `Fleet::declare_scope`'s, under the slot
    /// lock, and a refusal comes back through the tool so a Drone can read it
    /// and declare again.
    pub(crate) async fn declared(
        &self,
        caller: api::Caller,
        declaration: DeclareScope,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        let declared = Fleet::declare_scope(self, &job, &declaration).await?;
        Ok(Receipt {
            word: declared.word().to_string(),
        })
    }

    /// The working Drone asking the task's own scope to grow. Held open while
    /// a Judge call runs, and **every outcome comes back through the tool** —
    /// a Drone told nothing writes the file anyway.
    pub(crate) async fn widened(
        &self,
        caller: api::Caller,
        request: RequestScope,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        let widened = Fleet::request_scope(self, &job, &request).await?;
        Ok(Receipt {
            word: widened.word().to_string(),
        })
    }

    /// The Drone asking whether its work passes. Which Checks, what they are
    /// run against and what bounds the asking are all `Fleet::run_checks`'s,
    /// under the slot lock that binds them to one step.
    ///
    /// **What comes back is a report and never a verdict.** The step is exactly
    /// where it was when the call arrived, whatever the Checks said.
    pub(crate) async fn checked(&self, caller: api::Caller) -> Result<CheckReport, NotRecorded> {
        let job = self.placed(&caller)?;
        Ok(Fleet::run_checks(self, &job).await?)
    }

    /// The Drone asking for one more Job to exist.
    ///
    /// **What is different is what a success is** — every other tool answers
    /// about the Job the call was made on, and this one answers with the id of
    /// a record that did not exist a moment ago. `crate::sub_dispatch` holds
    /// whether the caller was allowed to ask.
    pub(crate) async fn dispatched(
        &self,
        caller: api::Caller,
        dispatch: DispatchJob,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        match Fleet::sub_dispatch(self, &job, &dispatch).await {
            // The minted id, and nothing else. A Drone needs it to name this
            // Job in a later call's `after`, and it needs nothing else — the
            // Job's state is not knowable yet and a receipt implying it were
            // would be the verdict `Receipt` exists to have no room for.
            Ok(minted) => Ok(Receipt {
                word: minted.as_str().to_string(),
            }),
            Err(why) => Err(why.into()),
        }
    }
}
