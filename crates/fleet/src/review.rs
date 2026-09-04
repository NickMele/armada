//! The pull request a finished Job opens, assembled from the record.
//!
//! # Nothing a Drone wrote reaches this
//!
//! A Drone's claim is a signal the gate ruled on; the record is what Fleet
//! verified. Every line below comes from a Job's own fields, its step rows and
//! its Check rows — no parameter here can carry a Drone's sentence, which is
//! `landing`'s commit message's shape and for its reason.
//!
//! # The four headings, and the one Fleet fills differently
//!
//! `docs/contracts/agent-copy.md` fixes the shape of every pull request body
//! that leaves Armada — why, outcome, risks, evidence — and it is a Drone's
//! contract. This is Fleet's body and it keeps the same four, so a reviewer
//! reading a queue finds the same section in the same place whoever wrote it.
//!
//! **Outcome is the one Fleet cannot write in a Drone's terms.** A Drone knows
//! what its change does; Fleet has only what it verified, and admitting a
//! Drone's sentence is what this file exists to prevent. So that section says
//! what the record proves — which files changed — and says nothing read them
//! for meaning. A heading held open honestly beats one dropped, which reads as
//! a body that forgot it.
//!
//! Risks come before evidence: a body ending on its passes reads as a claim
//! that everything is fine.

use adapter_traits::{how_the_base_was_found, Base, BaseOnTheRemote, Changed, Review};
use core_model::{Job, StepCheck, StepId, StepState};

/// The pull request for a Job that finished.
///
/// `checks` is what the store holds against the Job, step by step, in the order
/// the store returned. A step with no rows is a step whose Checks were never
/// run, and it is named as one rather than left out.
pub(crate) fn review_of(
    job: &Job,
    checks: &[(StepId, Vec<StepCheck>)],
    base: &Base,
    remote: &BaseOnTheRemote,
    changed: &Changed,
) -> Review {
    let mut body = String::new();
    body.push_str(&why_it_was_needed(job));
    body.push_str(&what_came_of_it(changed));
    body.push_str(&the_risks(job, base, remote));
    body.push_str(&the_evidence(job, checks));
    body.push_str(&the_record(job, base));
    Review::assembled(job.title().as_str(), body)
}

/// The brief the Job was created with, and what it had to satisfy.
///
/// **A person's own words, which is why they may be quoted whole.** The rule
/// this file keeps is that nothing a *Drone* wrote reaches a pull request; the
/// brief came from whoever asked for the work.
fn why_it_was_needed(job: &Job) -> String {
    let mut out = format!(
        "## Why was the change needed?\n\n{}\n\n",
        job.facts().as_str().trim()
    );
    if !job.acceptance_criteria().is_empty() {
        out.push_str("## What it had to satisfy\n\n");
        for criterion in job.acceptance_criteria() {
            out.push_str(&format!(
                "- {} ({})\n",
                criterion.text.trim(),
                criterion.source.as_wire()
            ));
        }
        out.push('\n');
    }
    out
}

/// What the Job changed, which is as much of an outcome as the record holds.
///
/// **Paths and not a count of lines.** `WorkProduct::counted_files` is the
/// reading that carries `+94 −31` and it costs the patch that would render the
/// diff — measured at 90ms over 414 files — and it is already spent once, on
/// the transition that ends a Job. `changed_files` is a delta walk under a
/// microsecond, and a reviewer about to open the diff wants the shape of the
/// change rather than its arithmetic.
///
/// **A Job that changed nothing says so.** An empty section under a heading
/// reads as a body that gave up; the sentence is a fact, and an unusual one.
fn what_came_of_it(changed: &Changed) -> String {
    let mut out = String::from("## What is the outcome of the change\n\n");
    if changed.is_empty() {
        out.push_str("This Job changed no file. Its work, if it did any, is not in the tree.\n\n");
        return out;
    }
    out.push_str(&format!(
        "{} changed:\n\n",
        match changed.len() {
            1 => String::from("One file"),
            many => format!("{many} files"),
        }
    ));
    for file in changed.files() {
        out.push_str(&format!(
            "- `{}` — {}\n",
            file.path(),
            file.change().as_wire()
        ));
    }
    out.push_str(
        "\nWhat the change *does* is in the commits. Nothing in this repository reads a \
         diff for meaning, so Fleet does not describe one.\n\n",
    );
    out
}

/// Every step with its verdict, and every Check under it with its outcome.
fn the_evidence(job: &Job, checks: &[(StepId, Vec<StepCheck>)]) -> String {
    let mut out = String::from("## Checks Evidence\n\n");
    for row in job.steps() {
        let label = job
            .workflow()
            .step(row.step_id())
            .map(|step| step.label())
            .unwrap_or_else(|| row.step_id().as_str());
        out.push_str(&format!("**{label}** — {}\n", said(row.state())));
        let ran = checks
            .iter()
            .find(|(step, _)| step == row.step_id())
            .map(|(_, ran)| ran.as_slice())
            .unwrap_or(&[]);
        if ran.is_empty() {
            out.push_str("- no Check ran against this step\n");
        }
        for check in ran {
            out.push_str(&one_check(check));
        }
        out.push('\n');
    }
    out
}

fn one_check(check: &StepCheck) -> String {
    let mut line = format!("- `{}` — {}", check.name, check.outcome.as_wire());
    if let Some(produced) = &check.produced {
        line.push_str(&format!(", {}", produced.trim()));
    }
    if let Some(path) = &check.output_path {
        line.push_str(&format!(" ([output]({path}))"));
    }
    line.push('\n');
    line
}

/// The caveat, which is the part a reviewer needs and the diff cannot supply.
///
/// It carries the cost, the reason, and what to do — the three things
/// `agent-copy.md` says a caveat needs to be actionable rather than skimmed.
fn the_risks(job: &Job, base: &Base, remote: &BaseOnTheRemote) -> String {
    let mut out = String::from("## Risks\n\n");
    out.push_str(
        "Every line below is something Fleet ran, not something the agent reported. \
         What no Check covered is not covered here either — a Check is a command and \
         an exit code, and nothing in this repository reads a Check's output for \
         meaning.\n\n",
    );
    if !unchecked(job).is_empty() {
        out.push_str(&format!(
            "These acceptance criteria are not a Check and nothing mechanical \
             confirmed them:\n\n{}\n\n",
            unchecked(job)
                .iter()
                .map(|text| format!("- {text}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if let Some(line) = what_the_base_carries(base, remote) {
        out.push_str(&line);
    }
    out
}

/// Where this came from, under no heading. **Last, and not a section**: it is
/// how a reader joins the pull request back to the Job, and it answers nothing
/// the four headings ask.
fn the_record(job: &Job, base: &Base) -> String {
    format!(
        "Armada job `{}`, workflow `{}`, branch `{}`, merging into `{}` ({}).\n",
        job.id().as_str(),
        job.workflow_id().as_str(),
        job.branch().map(|b| b.as_str()).unwrap_or("unrecorded"),
        base.name(),
        how_the_base_was_found(base),
    )
}

/// What the branch carries that this Job did not write, and what to do about it.
///
/// **The files in a pull request are the thing a reviewer trusts least when
/// they are wrong**, and a base ahead of its remote makes every one of its own
/// commits look like this Job's work. So the caveat names the count, names the
/// remedy, and says what the diff becomes afterwards — `agent-copy.md`'s rule
/// that a caveat is something a reviewer can act on.
///
/// A base *behind* its remote is said in the same line and is a different
/// problem: nothing here is wrong, the rebase used a reading somebody has since
/// moved past.
fn what_the_base_carries(base: &Base, remote: &BaseOnTheRemote) -> Option<String> {
    let BaseOnTheRemote::Apart {
        remote,
        ahead,
        behind,
    } = remote
    else {
        return None;
    };
    let mut out = String::new();
    if *ahead > 0 {
        out.push_str(&format!(
            "**This pull request carries {} nobody asked this Job for** — on `{}` on the \
             machine Fleet runs on and not on `{remote}`, so already on the branch before \
             the Job started, and counted as this Job's work by the diff below. Push `{}` \
             and the diff becomes what the Job actually changed.\n\n",
            commits(*ahead),
            base.name(),
            base.name(),
        ));
    }
    if *behind > 0 {
        out.push_str(&format!(
            "`{remote}` holds {} that `{}` has not got, so this branch was brought up to \
             a base somebody has since moved past.\n\n",
            commits(*behind),
            base.name(),
        ));
    }
    Some(out)
}

fn commits(n: usize) -> String {
    match n {
        1 => String::from("one commit"),
        _ => format!("{n} commits"),
    }
}

/// The criteria a person wrote as a judgement rather than as a command.
fn unchecked(job: &Job) -> Vec<String> {
    job.acceptance_criteria()
        .iter()
        .filter(|criterion| criterion.source.as_wire() != "check")
        .map(|criterion| criterion.text.trim().to_string())
        .collect()
}

/// A step's state, as a sentence rather than a wire value.
fn said(state: StepState) -> &'static str {
    match state {
        StepState::Advanced => "advanced",
        StepState::AwaitingHuman => "is waiting for a person",
        StepState::NotStarted => "never started",
        StepState::Retrying => "was being reattempted",
        StepState::Running => "was still running",
        StepState::Stopped => "stopped with its retries spent",
    }
}
