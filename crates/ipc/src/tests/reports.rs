//! The report's two closed sets, and the one place they could drift.
//!
//! [`Claim`] and [`ReportOrigin`] each spell themselves twice — once for serde,
//! which is what crosses the wire, and once in `as_wire`, which is what is
//! stored and grouped on. Two spellings of one value is exactly what this
//! workspace calls a second vocabulary, and here it is unavoidable: `store` may
//! not depend on `ipc`, so the stored text has to be produced by a method. The
//! test below is what makes the pair a pair — a variant renamed on one side and
//! not the other fails here rather than in a count that quietly splits into
//! two rows.

use crate::{decode, encode, Claim, FileReport, ReportOrigin, StepId};

#[test]
fn every_claim_is_spelled_the_same_way_by_serde_and_by_as_wire() {
    for claim in Claim::ALL {
        let encoded = encode(claim).expect("a claim encodes");
        assert_eq!(
            encoded,
            format!("\"{}\"", claim.as_wire()),
            "{claim:?} crosses the wire as one thing and is stored as another"
        );
        assert_eq!(
            Claim::from_wire(claim.as_wire()),
            Some(*claim),
            "what is stored reads back"
        );
    }
}

#[test]
fn every_origin_is_spelled_the_same_way_by_serde_and_by_as_wire() {
    for origin in ReportOrigin::ALL {
        let encoded = encode(origin).expect("an origin encodes");
        assert_eq!(encoded, format!("\"{}\"", origin.as_wire()));
        assert_eq!(ReportOrigin::from_wire(origin.as_wire()), Some(*origin));
    }
}

/// Only two of the three claims say anything about the Judge. A wrong pass and
/// a wrong refusal are both verdicts disputed; Armada misbehaving is not a
/// verdict at all, and counting it as one would put the dry-run case into a
/// number about the Judge.
#[test]
fn only_a_disputed_verdict_counts_toward_the_judge() {
    assert!(Claim::WronglyRefused.disputes_a_verdict());
    assert!(Claim::WronglyPassed.disputes_a_verdict());
    assert!(!Claim::ArmadaMisbehaved.disputes_a_verdict());
}

/// A claim outside the set is refused rather than defaulted, which is what
/// keeps the count over a closed vocabulary.
#[test]
fn a_claim_the_set_does_not_hold_is_refused() {
    let sent = r#"{"claim":"the_judge_is_biased","said":"it refused twice"}"#;
    assert!(decode::<FileReport>("a report", sent.as_bytes()).is_err());
}

/// The criterion scope is optional and absent by omission, not by `null`: a
/// report about the whole Job carries neither field.
#[test]
fn a_report_about_the_whole_job_carries_no_scope() {
    let filing = FileReport {
        claim: Claim::ArmadaMisbehaved,
        said: "the dry run said it created a worktree and created none".to_string(),
        step_id: None,
        criterion_id: None,
    };

    let encoded = encode(&filing).expect("a filing encodes");

    assert!(!encoded.contains("step_id"), "and it was: {encoded}");
    assert!(!encoded.contains("criterion_id"));
    assert_eq!(
        decode::<FileReport>("a report", encoded.as_bytes()).expect("it reads back"),
        filing
    );
}

/// The scope, where there is one, crosses as the pair — the step and the
/// criterion, because a criterion id is unique inside a step and not across a
/// Job that retried one.
#[test]
fn a_disputed_verdict_names_its_step_and_its_criterion() {
    let filing = FileReport {
        claim: Claim::WronglyRefused,
        said: "the quoted sentence exists in no scope note and in no submission".to_string(),
        step_id: Some(StepId::carried("implement")),
        criterion_id: Some(crate::CriterionId::carried("no_behaviour_beyond_scope")),
    };

    let encoded = encode(&filing).expect("a filing encodes");

    assert_eq!(
        decode::<FileReport>("a report", encoded.as_bytes()).expect("it reads back"),
        filing
    );
}
