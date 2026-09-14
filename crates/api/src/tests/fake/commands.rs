//! Every command, in one list, each delegating to its subject.
//!
//! **A trait implementation cannot be split across files**, so this is the whole
//! `Commands` block and the bodies are inherent methods next door. What that
//! buys is the roster: every command Fleet answers, in the order the inventory
//! names them, on one screen — and three files sized by subject rather than one
//! file sized by the trait.
//!
//! It asserts nothing about the status machine. That machine is `core-model`'s
//! and is tested there against the edge table. What is asserted here is that the
//! route reached the daemon, that the body arrived, and that a refusal comes
//! back as the refusal the transport is supposed to map.
//!
//! [`FakeDaemon::move_to`](super::FakeDaemon) is the one move, and most commands
//! below are a pair of statuses handed to it.

mod gating;
mod proposing;
mod recovering;

use ipc::{
    ChangesRequested, JobExamined, JobForgotten, JobId, JobSummary, ProposeJob, Redispatched,
    WorktreeReclaimed,
};

use super::FakeDaemon;
use crate::{Commands, Refusal};

impl FakeDaemon {
    /// The Job as it stands, or the 404 every fake command gives an id that
    /// names nothing.
    fn unmoved(&self, job_id: &JobId) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        jobs.iter()
            .find(|job| job.id == *job_id)
            .cloned()
            .ok_or_else(|| self.no_such_job(job_id))
    }

    fn no_such_workspace(&self, dir: &str) -> Refusal {
        let said = format!("the fake holds no proposal for `{dir}`");
        Refusal::Unacceptable(ipc::WireError::raised(
            "fake.no_such_workspace",
            said,
            crate::tests::shapes::run_id(),
        ))
    }
}

impl Commands for FakeDaemon {
    async fn propose_from_request(
        &self,
        request: ipc::JobRequest,
        _manifest_id: Option<ipc::ManifestId>,
        by: crate::Redirector,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        self.fake_propose_from_request(request, by).await
    }
    async fn propose_job(
        self: std::sync::Arc<Self>,
        proposal: ProposeJob,
        by: crate::Redirector,
    ) -> Result<JobSummary, Refusal> {
        self.fake_propose_job(proposal, by).await
    }
    async fn stop_proposal(
        &self,
        _proposal_id: ipc::ProposalId,
    ) -> Result<ipc::ProposalStopped, Refusal> {
        self.fake_stop_proposal(_proposal_id).await
    }
    async fn examine_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<JobExamined, Refusal> {
        self.fake_examine_job(job_id).await
    }
    async fn approve_dispatch(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<JobSummary, Refusal> {
        self.fake_approve_dispatch(job_id).await
    }
    async fn approve_review(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<JobSummary, Refusal> {
        self.fake_approve_review(job_id).await
    }
    async fn merge_pull_request(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_merge_pull_request(job_id).await
    }
    async fn resolve_pull_request_conflict(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_resolve_pull_request_conflict(job_id).await
    }
    async fn rerun_failed_checks(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn investigate_failed_checks(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn queue_after_finding(
        &self,
        job_id: JobId,
        _queued: ipc::FindingQueued,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn file_finding_issue(
        &self,
        job_id: JobId,
        _filed: ipc::IssueFiled,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn request_changes(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        self.fake_request_changes(job_id, note).await
    }
    async fn take_up_remarks(
        &self,
        job_id: JobId,
        picked: ipc::RemarksTakenUp,
    ) -> Result<JobSummary, Refusal> {
        self.fake_take_up_remarks(job_id, picked).await
    }
    async fn dismiss_finding(
        &self,
        job_id: JobId,
        dismissed: ipc::FindingDismissed,
    ) -> Result<JobSummary, Refusal> {
        self.fake_dismiss_finding(job_id, dismissed).await
    }
    async fn reject_job(self: std::sync::Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_reject_job(job_id).await
    }
    async fn override_verdict(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        overruling: ipc::Overruled,
    ) -> Result<JobSummary, Refusal> {
        self.fake_override_verdict(job_id, overruling).await
    }
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_rerun_gate(job_id).await
    }
    async fn show_again(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        picked: Option<String>,
    ) -> Result<ipc::ShownAgain, Refusal> {
        self.fake_show_again(job_id, picked).await
    }
    async fn start_run(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _run: ipc::StartRun,
    ) -> Result<ipc::RunUnderway, Refusal> {
        self.runs_nothing(&job_id)
    }
    async fn stop_run(
        &self,
        job_id: JobId,
        _run: ipc::NamedRun,
    ) -> Result<ipc::RunRecord, Refusal> {
        self.runs_nothing(&job_id)
    }
    async fn undo_run(
        &self,
        job_id: JobId,
        _run: ipc::NamedRun,
    ) -> Result<ipc::RunRecord, Refusal> {
        self.runs_nothing(&job_id)
    }
    async fn start_checkout_run(
        self: std::sync::Arc<Self>,
        _run: ipc::StartCheckoutRun,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunUnderway, Refusal> {
        self.runs_nothing_here()
    }
    async fn stop_checkout_run(
        &self,
        _run: ipc::NamedRun,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        self.runs_nothing_here()
    }
    async fn undo_checkout_run(
        &self,
        _run: ipc::NamedRun,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        self.runs_nothing_here()
    }
    async fn start_checkout_verify(
        self: std::sync::Arc<Self>,
        _asked: ipc::StartCheckoutVerify,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutVerify, Refusal> {
        self.runs_nothing_here()
    }
    /// The bytes echoed back as a path so a route test can tell the body
    /// arrived. Nothing is written: what a save does to a file is
    /// `fleet::editing`'s and is tested there against a real one.
    async fn save_manifest_file(
        &self,
        save: ipc::SaveManifestFile,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestSaved, Refusal> {
        Ok(ipc::ManifestSaved {
            path: format!("armada.yml ({} bytes)", save.text.len()),
            at: ipc::Instant::carried("2026-09-12T09:00:00.000Z"),
        })
    }
    /// The edits counted into the answer so a route test can tell the body
    /// arrived. Nothing is placed: what edits do to a file is
    /// `config::amend`'s, and `fleet::amending`'s against a real one.
    async fn edit_manifest(
        &self,
        edit: ipc::EditManifest,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestEdited, Refusal> {
        Ok(ipc::ManifestEdited {
            path: "armada.yml".to_string(),
            at: ipc::Instant::carried("2026-09-12T09:00:00.000Z"),
            text: format!("{}# {} edits\n", edit.read, edit.edits.len()),
            declared: Some(crate::tests::shapes::manifest_declared()),
        })
    }

    /// The fake holds no proposals, so every workspace is one Scan did not
    /// find — the refusal a route test can tell from a missing route.
    async fn edit_manifest_proposal(
        &self,
        asked: ipc::EditManifestProposal,
        _repository: Option<String>,
    ) -> Result<ipc::ManifestProposal, Refusal> {
        Err(self.no_such_workspace(&asked.dir))
    }
    async fn write_manifest_proposal(
        &self,
        asked: ipc::WriteManifestProposal,
        _repository: Option<String>,
    ) -> Result<ipc::ManifestProposal, Refusal> {
        Err(self.no_such_workspace(&asked.dir))
    }

    /// Each field the save names replaces the fake's value. What a save does to
    /// admission is `fleet::limits`' and tested there.
    async fn save_limits(&self, save: ipc::SaveLimits) -> Result<ipc::FleetLimits, Refusal> {
        let mut limits = self.limits.lock().expect("not poisoned");
        if let Some(v) = save.concurrency {
            limits.values.concurrency = v.get();
        }
        if let Some(v) = save.memory_spare_percent {
            limits.values.memory_spare_percent = v.get();
        }
        if let Some(v) = save.disk_floor_gib {
            limits.values.disk_floor_gib = v.get();
        }
        if let Some(v) = save.checks_at_once {
            limits.values.checks_at_once = v.get();
        }
        Ok(*limits)
    }

    /// The one preference name this fake knows; anything else is the 422 a
    /// route test tells apart from a missing route by its code.
    async fn save_preferences(
        &self,
        save: ipc::SavePreference,
    ) -> Result<ipc::Preferences, Refusal> {
        let mut preferences = self.preferences.lock().expect("not poisoned");
        match save.name.as_str() {
            "where_things_are_open" => {
                preferences.where_things_are_open = save.value;
                Ok(*preferences)
            }
            other => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unknown_preference",
                format!("`{other}` is not a preference this build reads"),
                crate::tests::shapes::run_id(),
            ))),
        }
    }

    /// The named row goes, by its exact text; one absent is the 409 every
    /// fake give a name that names nothing.
    async fn add_repository(
        &self,
        asked: ipc::AddRepository,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        Ok(ipc::RepositorySummary {
            root: asked.path,
            records_root: String::from("/records"),
            manifest: None,
        })
    }

    /// Clones nothing: served under `parent`, at the folder a URL of `shop` names.
    async fn clone_repository(
        self: std::sync::Arc<Self>,
        asked: ipc::CloneRepository,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        Ok(ipc::RepositorySummary {
            root: format!("{}/shop", asked.parent),
            records_root: String::from("/records"),
            manifest: None,
        })
    }

    async fn remove_repository_allowed_command(
        &self,
        removing: ipc::RemoveRepositoryAllowedCommand,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        let mut allowed = self.repository_allowed.lock().expect("not poisoned");
        let before = allowed.len();
        allowed.retain(|row| row.run != removing.run);
        if allowed.len() == before {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.no_such_repository_allow",
                format!(
                    "nothing always-allowed for the repository is spelled `{}`",
                    removing.run
                ),
                crate::tests::shapes::run_id(),
            )));
        }
        Ok(ipc::RepositoryAllowedCommands {
            commands: allowed.clone(),
        })
    }

    /// Refused, naming what was asked for, so a route test can tell the body
    /// arrived. Holding a server is `fleet::servers`' and tested there.
    async fn start_server(
        self: std::sync::Arc<Self>,
        asked: ipc::StartServer,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ServerState, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.not_a_server",
            format!(
                "the fake daemon starts no server, `{}` included",
                asked.name
            ),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn stop_server(&self, named: ipc::NamedServer) -> Result<ipc::ServerState, Refusal> {
        Err(Refusal::IllegalMove(ipc::WireError::raised(
            "fleet.no_such_server",
            format!("the fake daemon holds no server called `{}`", named.id),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn raise_cost_cap(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        raise: ipc::CapRaise,
    ) -> Result<JobSummary, Refusal> {
        self.fake_raise_cost_cap(job_id, raise).await
    }
    async fn raise_turn_cap(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        raise: ipc::TurnRaise,
    ) -> Result<JobSummary, Refusal> {
        self.fake_raise_turn_cap(job_id, raise).await
    }
    async fn file_report(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        self.fake_file_report(job_id, filing).await
    }
    async fn kill_drone(self: std::sync::Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_kill_drone(job_id).await
    }
    async fn kill_job(self: std::sync::Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_kill_job(job_id).await
    }
    async fn forget_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<JobForgotten, Refusal> {
        self.fake_forget_job(job_id).await
    }
    async fn reclaim_worktree(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<WorktreeReclaimed, Refusal> {
        self.fake_reclaim_worktree(job_id).await
    }
    async fn delete_branch(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        asked: ipc::DeleteBranch,
    ) -> Result<ipc::BranchDeleted, Refusal> {
        self.fake_delete_branch(job_id, asked).await
    }
    async fn redirect_drone(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _instruction: ipc::Redirection,
        by: crate::Redirector,
    ) -> Result<JobSummary, Refusal> {
        self.fake_redirect_drone(job_id, _instruction, by).await
    }
    async fn add_task(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _add: ipc::AddTask,
    ) -> Result<ipc::WorkPlan, Refusal> {
        self.fake_plan_change(job_id).await
    }
    async fn drop_task(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _drop: ipc::DropTask,
    ) -> Result<ipc::WorkPlan, Refusal> {
        self.fake_plan_change(job_id).await
    }
    async fn answer_question(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        self.fake_answer_question(job_id, _answer).await
    }
    /// The Job back, unmoved: what an answer does is Fleet's slot and store,
    /// and a router test's question is whether the body arrived.
    async fn answer_command(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _answer: ipc::AnswerCommand,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn set_when_blocked(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _setting: ipc::SetWhenBlocked,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn answer_judge(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _answered: ipc::JudgeAnswered,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn set_when_refused(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _setting: ipc::SetWhenRefused,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn set_model(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn set_review_model(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn remove_allowed_command(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _removing: ipc::RemoveAllowedCommand,
    ) -> Result<JobSummary, Refusal> {
        self.unmoved(&job_id)
    }
    async fn restart_step(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
    ) -> Result<JobSummary, Refusal> {
        self.fake_restart_step(job_id, note).await
    }
    async fn redispatch_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<Redispatched, Refusal> {
        self.fake_redispatch_job(job_id).await
    }
}
