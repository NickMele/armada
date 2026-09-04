//! The pull request a finished Job opens, assembled from the record.
//!
//! # Nothing a Drone wrote reaches this
//!
//! A Drone's claim is a signal the gate ruled on; the record is what Fleet
//! verified. So every line below comes from a Job's own fields, its step rows
//! and its Check rows — there is no parameter here through which a Drone's
//! sentence could arrive, which is the same shape `landing`'s commit message
//! has and for the same reason.
//!
//! # What a reviewer is told, and what they are told is missing
//!
//! What was asked, what each step's Checks did with the file each one printed
//! into, and — last — what nobody checked. A body that only listed passes reads
//! as a claim that everything is fine.
//!
//! `docs/contracts/agent-copy.md` governs a Drone's own PR copy. This is
//! Fleet's, and it takes that page's structural rules: headings rather than
//! colon reveals, and a caveat a reviewer can act on.

use adapter_traits::{how_the_base_was_found, Base, BaseOnTheRemote, Review};
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
) -> Review {
    let mut body = String::new();
    body.push_str(&what_was_asked(job));
    body.push_str(&what_was_checked(job, checks));
    body.push_str(&what_this_does_not_say(job, base, remote));
    Review::assembled(job.title().as_str(), body)
}

/// The brief the Job was created with, and what it had to satisfy.
fn what_was_asked(job: &Job) -> String {
    let mut out = format!("{}\n\n", job.facts().as_str().trim());
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

/// Every step with its verdict, and every Check under it with its outcome.
fn what_was_checked(job: &Job, checks: &[(StepId, Vec<StepCheck>)]) -> String {
    let mut out = String::from("## What was checked\n\n");
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
fn what_this_does_not_say(job: &Job, base: &Base, remote: &BaseOnTheRemote) -> String {
    let mut out = String::from("## What this does not say\n\n");
    out.push_str(
        "Every line above is something Fleet ran, not something the agent reported. \
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
    out.push_str(&format!(
        "Armada job `{}`, workflow `{}`, branch `{}`, merging into `{}` ({}).\n",
        job.id().as_str(),
        job.workflow_id().as_str(),
        job.branch().map(|b| b.as_str()).unwrap_or("unrecorded"),
        base.name(),
        how_the_base_was_found(base),
    ));
    out
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
