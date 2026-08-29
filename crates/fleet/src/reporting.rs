//! Filing what a person knows went wrong, with the Job's own record attached.
//!
//! **Collected, not captured.** v1 built a ring buffer of CLI output because it
//! had nothing else to attach — a command had run, printed, and the scrollback
//! was gone. Every line of the record below is already written down, and this
//! reads it: the transitions, the verdicts with what each cited, the flags, the
//! Checks, what each step claimed, what the worktree held. A Job persists, so
//! the question the ring buffer answered does not arise. What does not exist
//! until somebody types it is [`Filed::said`].
//!
//! **Rendered once and stored, not joined.** Every row read here belongs to the
//! Job, and `armada clean` takes the Job and all of them away, so a report
//! pointing at them would go blank exactly when it was most needed. The same
//! rendering is what a Drone would be handed as facts — a Drone cannot reach
//! the issue tracker either way — so the bundling is solved once.
//!
//! **Every string goes through the redactor by construction.** [`Rendering`] is
//! the only thing that appends, and each of its methods scrubs, so a section
//! added later is scrubbed because there is no way to write one that is not.
//!
//! **Choosing the fields is the act the wire's `From` impls are.** This renders
//! from domain records, so a field added to `core_model::Job` has to be written
//! in here to appear — the cost `crates/ipc` pays in the other direction, and
//! for the same reason: a record put on a page nobody redacted is a redaction
//! decision nobody made.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId};
use ipc::{Claim, CriterionId, StepId};
use store::Report;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::mint::Mint;
use crate::redaction::Redactor;

/// What a person filed, with the emptiness already refused.
///
/// A type of its own for `crate::overruling::Overruling`'s reason: the sentence
/// is required, and a type that cannot hold a blank one is what makes that a
/// property rather than a check somebody remembers to make. **And it is not
/// enough on its own** — the first override in this repository carries the
/// reason `probe`, which is non-blank and says nothing — which is why
/// [`Filed::claim`] is a closed set and is what a count reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filed {
    claim: Claim,
    said: String,
    scope: Option<(StepId, CriterionId)>,
}

impl Filed {
    /// `None` where the sentence is blank, or where half a criterion scope was
    /// sent: a criterion id is unique inside a step, and one arriving without
    /// its step names every attempt at once.
    pub fn saying(
        claim: Claim,
        said: &str,
        step_id: Option<StepId>,
        criterion_id: Option<CriterionId>,
    ) -> Option<Filed> {
        let said = said.trim();
        if said.is_empty() {
            return None;
        }
        let scope = match (step_id, criterion_id) {
            (Some(step), Some(criterion)) => Some((step, criterion)),
            (None, None) => None,
            _ => return None,
        };
        Some(Filed {
            claim,
            said: said.to_string(),
            scope,
        })
    }

    pub fn claim(&self) -> Claim {
        self.claim
    }

    pub fn said(&self) -> &str {
        &self.said
    }

    /// The verdict being disputed, where the report is about one.
    pub fn scope(&self) -> Option<(&StepId, &CriterionId)> {
        self.scope
            .as_ref()
            .map(|(step, criterion)| (step, criterion))
    }
}

/// What is known about whether the Judge has been right, as counts.
///
/// **Counts and not a rate.** Dividing the disputes by the refusals would put
/// every Job nobody read into the denominator, and an unread Job is not a pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counted {
    pub refusals_recorded: u32,
    pub refusals_disputed: u32,
    pub passes_disputed: u32,
    pub reports_filed: u32,
}

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
    /// File a report on one Job. **Nothing about the Job changes.**
    ///
    /// No status moves, no step moves, no Drone is spoken to and no Job is
    /// proposed. A report is a record of what a person concluded, and an act
    /// that also moved the Job would make disagreeing with a verdict a way of
    /// getting past one — which is `override_verdict`, a different act with a
    /// different refusal.
    ///
    /// The Job is loaded first, so an id naming nothing is a 404 rather than a
    /// report about a Job that never existed.
    pub async fn file_report(&self, job_id: &JobId, filed: &Filed) -> Result<Report, Adrift> {
        let job = self.load(job_id).await?;
        let record = self.record_of(&job).await?;
        let report = Report {
            report_id: self.mint().ulid().as_str().to_string(),
            filed_at: self.now(),
            // Only a person files one today. The column is `origin` and not a
            // boolean because the day Fleet files its own, that is a value.
            origin: ipc::ReportOrigin::Human.as_wire().to_string(),
            claim: filed.claim().as_wire().to_string(),
            job_id: job.id().clone(),
            job_title: self.redactor().scrub(job.title().as_str()),
            step_id: filed.scope().map(|(step, _)| step.to_domain()),
            criterion_id: filed.scope().map(|(_, id)| id.to_domain()),
            // The person's own words, scrubbed like everything else: a sentence
            // is the likeliest place a pasted token arrives.
            said: self.redactor().scrub(filed.said()),
            record,
        };
        self.store()
            .lock()
            .await
            .record_report(&report)
            .map_err(Adrift::Writing)?;
        Ok(report)
    }

    /// Every report filed, newest first, and the counts they are read beside.
    ///
    /// **Not scoped to a Job**, because a report outlives one: a listing
    /// reachable only through a Job would lose exactly the reports whose Job
    /// has been cleaned up.
    pub async fn reports(&self) -> Result<(Vec<Report>, Counted), Adrift> {
        let store = self.store().lock().await;
        let filed = store.reports().map_err(Adrift::Reading)?;
        let by_claim = store.reports_by_claim().map_err(Adrift::Reading)?;
        let counted = |claim: Claim| {
            by_claim
                .iter()
                .find(|(spelling, _)| spelling == claim.as_wire())
                .map(|(_, count)| *count)
                .unwrap_or(0)
        };
        let counts = Counted {
            refusals_recorded: store.refusals_recorded().map_err(Adrift::Reading)?,
            refusals_disputed: counted(Claim::WronglyRefused),
            passes_disputed: counted(Claim::WronglyPassed),
            reports_filed: filed.len() as u32,
        };
        Ok((filed, counts))
    }

    /// The Job's own record, rendered.
    ///
    /// Read in the order a person reads it in: what the Job was, how it got to
    /// where it stopped, what each gate said, what the work claimed, and what
    /// it changed. **The patch is not here** — it is however large the work is,
    /// and the file list is what says where to look; a reader who wants the
    /// bytes has the branch, which is named above them.
    async fn record_of(&self, job: &Job) -> Result<String, Adrift> {
        let store = self.store().lock().await;
        let events = store
            .events_for(job.id())
            .map_err(|why| Adrift::Reading(store::LoadJobError::Unreadable(why)))?;
        let checks = store.step_checks(job.id()).map_err(Adrift::Reading)?;
        let judged = store.step_judgments(job.id()).map_err(Adrift::Reading)?;
        let flagged = store.step_gaming_flags(job.id()).map_err(Adrift::Reading)?;
        let claimed = store.step_evidence(job.id()).map_err(Adrift::Reading)?;
        let footprint = store.footprint(job.id()).map_err(Adrift::Reading)?;
        let plans = store.step_plans(job.id()).map_err(Adrift::Reading)?;
        drop(store);

        let redactor = self.redactor();
        let mut out = Rendering::through(&redactor);
        out.heading("What the job was");
        out.field("Job", job.id().as_str());
        out.field("Title", job.title().as_str());
        out.field("Status", job.status().as_wire());
        out.field("Workflow", job.workflow().name());
        out.field("Model", job.model().as_str());
        if let Some(branch) = job.branch() {
            out.field("Branch", branch.as_str());
        }
        if let Some(step) = job.current_step_id() {
            out.field("Current step", step.as_str());
        }
        out.blank();

        out.heading("What it was told");
        out.body(job.facts().as_str());
        for criterion in job.acceptance_criteria() {
            out.bullet(&format!(
                "{} — {}",
                criterion.criterion_id.as_str(),
                criterion.text
            ));
        }
        out.blank();

        out.heading("Where each step got to");
        for step in job.steps() {
            let verdict = match step.last_verdict() {
                Some(core_model::StepVerdict::Failed(why)) => {
                    format!("failed({})", why.as_wire())
                }
                Some(other) => other.as_wire().to_string(),
                None => "no verdict".to_string(),
            };
            out.bullet(&format!(
                "{} — {}, {verdict}",
                step.step_id().as_str(),
                step.state().as_wire()
            ));
        }
        out.blank();

        out.heading("What the judge said");
        for (step, judgments) in &judged {
            for judgment in judgments {
                out.bullet(&format!(
                    "{} / {} — {}",
                    step.as_str(),
                    judgment.criterion_id.as_str(),
                    judgment.verdict.as_wire()
                ));
                out.detail("expected", judgment.expected.as_deref());
                out.detail("produced", judgment.produced.as_deref());
                out.detail("consequence", judgment.consequence.as_deref());
            }
        }
        if judged.is_empty() {
            out.body("The judge answered nothing on this job.");
        }
        out.blank();

        out.heading("What the gaming check flagged");
        for (step, flags) in &flagged {
            for flag in flags {
                out.bullet(&format!(
                    "{} — {} cites {}",
                    step.as_str(),
                    flag.pattern.as_wire(),
                    flag.cited
                ));
            }
        }
        if flagged.is_empty() {
            out.body("Nothing was flagged.");
        }
        out.blank();

        out.heading("What the checks did");
        for (step, runs) in &checks {
            for run in runs {
                out.bullet(&format!(
                    "{} / {} — {}",
                    step.as_str(),
                    run.name,
                    run.outcome.as_wire()
                ));
                out.detail("expected", run.expected.as_deref());
                out.detail("produced", run.produced.as_deref());
                out.detail("output", run.output_path.as_deref());
            }
        }
        if checks.is_empty() {
            out.body("No check was run.");
        }
        out.blank();

        out.heading("What the drone claimed");
        for (step, evidence) in &claimed {
            out.bullet(&format!(
                "{} — {}",
                step.as_str(),
                evidence.evidence_type.as_wire()
            ));
            out.detail("claimed", Some(&evidence.claimed));
            out.detail("shown by", Some(&evidence.shown_by));
            out.detail("not claimed", Some(&evidence.not_claimed));
        }
        if claimed.is_empty() {
            out.body("No step submitted evidence.");
        }
        out.blank();

        out.heading("What it changed");
        match &footprint {
            None => out.body(
                "No footprint was recorded. The job had not stopped when this was written, \
                 or it finished before fleet wrote them down.",
            ),
            Some(recorded) => {
                out.field("Read at", recorded.recorded_at.as_str());
                for file in &recorded.files {
                    // Marked only where a plan exists to be outside of.
                    let covered =
                        plans.is_empty() || plans.iter().any(|plan| plan.paths.covers(file.path()));
                    let outside = match covered {
                        true => "",
                        false => " — outside every declared plan",
                    };
                    out.bullet(&format!(
                        "{} — {}{outside}",
                        file.change().as_wire(),
                        file.path()
                    ));
                }
                if recorded.files.is_empty() {
                    out.body("The worktree was read and held no change.");
                }
            }
        }
        out.blank();

        // A step that never declared is absent here rather than empty.
        out.heading("What each step said it would change");
        for plan in &plans {
            out.bullet(&format!("{} — run {}", plan.step_id.as_str(), plan.attempt));
            for path in plan.paths.paths() {
                out.detail("path", Some(path.as_str()));
            }
            if plan.paths.is_empty() {
                out.detail("path", Some("none — it promised to touch nothing"));
            }
        }
        if plans.is_empty() {
            out.body(
                "No step declared a plan, so nothing above is marked. That is not the same \
                 as every path being inside one.",
            );
        }
        out.blank();

        out.heading("Every move it made");
        for event in &events {
            out.bullet(&crate::reporting::moved(event));
        }
        if events.is_empty() {
            out.body("The job was created and has not moved.");
        }
        out.blank();

        // Said last, where a reader has the record above it, and said at all
        // because an override's own words are not in the store: `overruling`
        // writes them to the job's log, which this record cannot read.
        out.heading("What this record does not carry");
        out.body(
            "The patch, which is on the branch named above. The drone's turns, which are in \
             the job's transcripts. The words on any override of a verdict, which are in \
             fleet's own log rather than in the job record.",
        );
        Ok(out.finish())
    }

    /// The scrubber every string in a report passes through.
    fn redactor(&self) -> Redactor {
        Redactor::with_home(&self.host().home)
    }
}

/// One recorded move, as a line.
///
/// **The trigger is on the line where the log carried one**, because "the step
/// stopped" and "the step stopped on `gate_failure`" are the difference between
/// a timeline and a diagnosis.
fn moved(event: &store::RecordedEvent) -> String {
    let at = event.at().as_str();
    let seq = event.seq();
    match event.moved() {
        store::Moved::Job { to, reason } => format!(
            "{seq} {at} — job {} to {}{}",
            event.under().as_wire(),
            to.as_wire(),
            reason
                .as_wire()
                .map(|named| format!(", {named}"))
                .unwrap_or_default()
        ),
        store::Moved::Step {
            step_id,
            from,
            to,
            why,
        } => format!(
            "{seq} {at} — step {} {} to {}{}",
            step_id.as_str(),
            from.as_wire(),
            to.as_wire(),
            why.map(|trigger| format!(", {}", trigger.as_wire()))
                .unwrap_or_default()
        ),
        store::Moved::Drone { drone_id, presence } => format!(
            "{seq} {at} — drone {} {}",
            drone_id.as_str(),
            presence.as_wire()
        ),
    }
}

/// The only thing that writes a report's record.
///
/// **Every method scrubs.** That is the whole reason this is a type rather than
/// a `String` and a `push_str`: a section added later cannot be written without
/// passing through the redactor, so the guarantee is structural instead of
/// being a rule in a comment nobody reads at the moment they add a heading.
struct Rendering<'a> {
    redactor: &'a Redactor,
    out: String,
}

impl<'a> Rendering<'a> {
    fn through(redactor: &'a Redactor) -> Rendering<'a> {
        Rendering {
            redactor,
            out: String::new(),
        }
    }

    fn heading(&mut self, text: &str) {
        self.out.push_str("## ");
        self.write(text);
        self.out.push('\n');
    }

    fn field(&mut self, label: &str, value: &str) {
        self.out.push_str("- **");
        self.write(label);
        self.out.push_str("**: ");
        self.write(value);
        self.out.push('\n');
    }

    fn bullet(&mut self, text: &str) {
        self.out.push_str("- ");
        self.write(text);
        self.out.push('\n');
    }

    /// A field beneath a bullet, where the record has one. **Absent stays
    /// absent** — an empty line under a verdict reads as a Judge that cited
    /// nothing, which is a different and much worse fact.
    fn detail(&mut self, label: &str, value: Option<&str>) {
        let Some(value) = value.filter(|text| !text.trim().is_empty()) else {
            return;
        };
        self.out.push_str("  - ");
        self.write(label);
        self.out.push_str(": ");
        self.write(value);
        self.out.push('\n');
    }

    fn body(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        self.write(text);
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    /// The one place text reaches the record, and therefore the one place the
    /// redactor has to run.
    fn write(&mut self, text: &str) {
        self.out.push_str(&self.redactor.scrub(text));
    }

    fn finish(self) -> String {
        self.out
    }
}
