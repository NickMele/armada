//! Which lines of a brief are block headings, which are not, and what kind of
//! section each heading is.
//!
//! **Its own module rather than a case in `briefing`, because the subject is
//! not the wording.** That module asserts the structure the Agent Prompt
//! Contract requires. This asserts the one fact about the assembled text that
//! only the assembler holds: which of its lines a reader on the far side of the
//! wire may draw as a heading, and — since `ipc::BlockKind` — what kind of
//! section it opens.
//!
//! **Nothing here asserts that a heading is upper case**, and the omission is
//! the point. Capitals are one of the two guesses `ipc::Saw::Instructed`'s
//! `headings` exists to replace, and a test that pinned them would put the
//! guess back in as a rule.
//!
//! No case pins a list of indices either. A list would break on every block the
//! contract adds and would say nothing about whether an index landed on a
//! heading.

use core_model::{EvidenceType, StepEvidence, StepId};
use ipc::BlockKind;
use verification::TheBaseMoved;

use crate::briefing::{first_turn, resuming_turn, Brief, Opening, Stopped, BASELINE};
use crate::crossing::{Crossed, Produced};
use crate::tests::briefing::{a_job, a_workflow};

/// The brief a fresh Drone on `implement` opens with.
fn opening() -> Brief {
    first_turn(
        &a_job(),
        &a_workflow(),
        &StepId::new("implement"),
        &Crossed::nothing(),
    )
    .expect("a prompt")
}

/// The lines a brief names as headings, read out of the text they index.
fn headed(brief: &Brief) -> Vec<String> {
    let lines: Vec<&str> = brief.as_str().split('\n').collect();
    brief
        .headings()
        .iter()
        .map(|at| {
            lines
                .get(*at)
                .unwrap_or_else(|| panic!("line {at} of {} lines", lines.len()))
                .to_string()
        })
        .collect()
}

/// Every heading, paired with the kind `briefing` stamped it with, in order.
fn kinded(brief: &Brief) -> Vec<(String, BlockKind)> {
    headed(brief)
        .into_iter()
        .zip(brief.kinds().iter().copied())
        .collect()
}

/// **The one invariant a peer decodes by.** `ipc::Saw::Instructed`'s doc says a
/// mismatched pair of lengths is unreadable and must fall back to plain
/// headings — this module is the one writer of both, so it never produces the
/// pair that fallback exists for.
#[test]
fn every_heading_has_exactly_one_kind() {
    for brief in [opening(), a_restart(), a_reconciled_brief()] {
        assert_eq!(
            brief.headings().len(),
            brief.kinds().len(),
            "a kind for every heading and no more: {}",
            brief.as_str()
        );
    }
}

/// The rules that read the same on every Job, folded shut on Bridge's design
/// and still present here.
#[test]
fn what_is_the_same_on_every_job_is_named_standing() {
    let pairs = kinded(&opening());
    assert_eq!(
        pairs.first().map(|(line, kind)| (line.as_str(), *kind)),
        Some(("FILES YOU WRITE FOR YOURSELF", BlockKind::Standing)),
        "the first heading after the unheaded baseline is the standing rule \
         about a Drone's own notes: {pairs:?}"
    );
}

/// The rail — where this step sits among the workflow's — is a list Bridge
/// already holds the data for, and is named apart from a Job's own facts.
#[test]
fn the_rail_is_named_as_the_step_list() {
    let pairs = kinded(&opening());
    assert!(
        pairs
            .iter()
            .any(|(line, kind)| line == "WHERE YOU ARE" && *kind == BlockKind::Steps),
        "the rail is `Steps`, not `AboutThisJob`: {pairs:?}"
    );
}

/// What was asked reads as a fact about this Job, open rather than folded.
#[test]
fn the_job_brief_is_named_as_about_this_job() {
    let pairs = kinded(&opening());
    assert!(
        pairs
            .iter()
            .any(|(line, kind)| line == "JOB BRIEF" && *kind == BlockKind::AboutThisJob),
        "what was asked is a fact about this Job: {pairs:?}"
    );
}

/// A restart's reason is a fact about this attempt, and so is the branch a
/// rebase leaves a Drone on — never `Standing`, because neither reads the same
/// on every Job.
#[test]
fn why_this_attempt_and_the_branch_are_named_as_about_this_job() {
    let restart_pairs = kinded(&a_restart());
    assert_eq!(
        restart_pairs
            .last()
            .map(|(line, kind)| (line.as_str(), *kind)),
        Some(("WHY THIS PART IS BEING DONE AGAIN", BlockKind::AboutThisJob)),
        "{restart_pairs:?}"
    );

    let branch_pairs = kinded(&a_reconciled_brief());
    assert_eq!(
        branch_pairs
            .last()
            .map(|(line, kind)| (line.as_str(), *kind)),
        Some(("THE BRANCH YOU ARE ON", BlockKind::AboutThisJob)),
        "{branch_pairs:?}"
    );
}

/// A restart's extra block, for [`why_this_attempt_and_the_branch_are_named_as_about_this_job`].
fn a_restart() -> Brief {
    resuming_turn(
        &a_job(),
        &a_workflow(),
        &StepId::new("implement"),
        &Stopped::default(),
        &Crossed::nothing(),
    )
    .expect("a prompt")
}

/// The rebase's block, for the same test.
fn a_reconciled_brief() -> Brief {
    let moved = TheBaseMoved::BroughtUpToDate {
        base: String::from("main"),
        commits: 4,
    };
    Opening::fresh()
        .turn(
            &a_job(),
            &a_workflow(),
            &StepId::new("implement"),
            Some(&moved),
        )
        .expect("a prompt")
}

/// **What makes the marking usable on the far side.** A block heading is
/// written as its own line with a blank one under it, so grouping the text at
/// its blank lines leaves every heading in a block of its own — and a surface
/// can draw that block as a heading without splitting anything or deciding
/// anything. An index landing mid-block would be a heading a renderer either
/// drew with its body or dropped.
#[test]
fn every_line_a_brief_names_stands_alone_in_its_block() {
    let brief = opening();
    let lines: Vec<&str> = brief.as_str().split('\n').collect();
    assert!(
        !brief.headings().is_empty(),
        "the brief a step opens with is assembled out of headed blocks"
    );
    for at in brief.headings() {
        let line = lines[*at];
        assert!(!line.trim().is_empty(), "a heading is not a blank line");
        assert_eq!(line.trim(), line, "a heading carries no indent: {line:?}");
        assert_eq!(
            lines.get(at + 1).map(|next| next.trim()),
            Some(""),
            "a blank line follows every heading: {line:?}"
        );
    }
}

/// **The half a reader taking the first line of each block would get wrong.**
/// The baseline opens with a sentence about the worktree and it is the first
/// line of the first block, so the cheap rule marks it — and a brief opens with
/// a heading that is really prose.
#[test]
fn the_baseline_s_first_line_is_not_a_heading() {
    let opens_with = BASELINE
        .split('\n')
        .next()
        .expect("the baseline has a first line");
    assert!(
        !headed(&opening()).contains(&opens_with.to_string()),
        "the baseline is prose all the way down: {opens_with:?}"
    );
}

/// **The other half of the same guess, in the middle of the text.** What the
/// part before produced sits inside the rail's block, under the rail's own
/// heading, and the blank line above it starts a block whose first line is
/// prose. So position is wrong here too, and a renderer that fell back to it
/// on any block but the first would draw this sentence as a heading.
#[test]
fn a_block_that_opens_with_a_sentence_is_not_named() {
    let workflow = a_workflow();
    let recorded = vec![(
        StepId::new("implement"),
        StepEvidence {
            evidence_type: EvidenceType::FactsNote,
            claimed: String::from("the log reader stops one line later"),
            shown_by: String::from("read.rs:41"),
            not_claimed: String::new(),
        },
    )];
    let at = StepId::new("summarise");
    let crossed = Crossed::nothing().and_produced(Produced::before(&workflow, &at, &recorded));
    let brief = first_turn(&a_job(), &workflow, &at, &crossed).expect("a prompt");
    assert!(
        brief.as_str().contains("What part 1 produced:"),
        "the boundary handed the block across: {}",
        brief.as_str()
    );
    assert!(
        !headed(&brief)
            .iter()
            .any(|line| line.starts_with("What part")),
        "and it stays body"
    );
}

/// A restart appends one more block after the blocks a fresh spawn has, and an
/// index counted against the wrong string would be off by that block's length.
#[test]
fn a_restart_s_extra_block_is_named_where_it_lands() {
    let brief = resuming_turn(
        &a_job(),
        &a_workflow(),
        &StepId::new("implement"),
        &Stopped::default(),
        &Crossed::nothing(),
    )
    .expect("a prompt");
    assert_eq!(
        headed(&brief).last().map(String::as_str),
        Some("WHY THIS PART IS BEING DONE AGAIN"),
        "the block a restart appends is the last one named"
    );
    assert_eq!(
        opening().headings().len() + 1,
        brief.headings().len(),
        "and it is the only block a restart adds"
    );
}

/// The rebase's block is appended last of all, after the brief a spawn already
/// assembled, so its index is counted against a string that has grown once.
/// `Opening::turn` is the one funnel every spawn goes through.
#[test]
fn the_branch_block_is_named_at_the_end_of_the_whole_brief() {
    let moved = TheBaseMoved::BroughtUpToDate {
        base: String::from("main"),
        commits: 4,
    };
    let brief = Opening::fresh()
        .turn(
            &a_job(),
            &a_workflow(),
            &StepId::new("implement"),
            Some(&moved),
        )
        .expect("a prompt");
    assert_eq!(
        headed(&brief).last().map(String::as_str),
        Some("THE BRANCH YOU ARE ON"),
        "the last block of the turn is the last heading named"
    );
}
