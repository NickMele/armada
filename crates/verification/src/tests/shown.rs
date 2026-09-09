//! Where a Judge's citation is in the brief, and what part of it holds it.
//!
//! **The other half of [`quoted`](super::quoted).** That module asks whether
//! the words a refusal puts in quotation marks are in what the call was shown,
//! and throws the finding away; this asks where they are and keeps it. The two
//! read one brief through one normalisation, so a span that passes containment
//! and then places nowhere would be two readings disagreeing.
//!
//! **[`located`](super::located) is the same question about the patch.** A
//! gaming flag points into the change; a Judge's citation points into the
//! brief, which is a different document with a different retention.
//!
//! The fixtures are [`judge`](super::judge)'s, like [`answered`](super::
//! answered)'s are — a second brief beside that one would be a second answer to
//! what a Judge is shown.

use core_model::JudgeVerdict;

use crate::{Answered, Printed};

use super::judge::{brief, brief_told, checks, workflow};

/// A no-objection quotes nothing, because it writes nothing to quote out of.
///
/// **Empty is the whole truth, and it is not a gap.** Under `ANSWER_FORMAT` a
/// `met` answer is one line, so there is no prose a citation could be placed
/// in — which is why `JudgeCitations` cannot show what a judge that met a
/// criterion read, and why `given` is the reading that does carry across both
/// verdicts.
#[test]
fn a_no_objection_places_no_citation_because_it_wrote_no_prose() {
    let workflow = workflow();
    let judged = brief(&workflow).read("verdict: met").expect("a pass");
    assert_eq!(
        judged.cited,
        Some(Vec::new()),
        "empty rather than absent: the question was asked and the answer is none"
    );
}

/// A refusal that argues in the Judge's own words places nothing, and is still
/// a refusal.
///
/// **The pair this holds together.** `quoted` deliberately lets an unattributed
/// or short quotation through, because failing an honest refusal costs more
/// than the fabrications it would catch — and the same looseness means a
/// refusal can be complete and place nowhere. An empty list has to mean
/// "nothing quotable" rather than "nothing read", or the screen above it draws
/// a panel that read nothing.
#[test]
fn a_refusal_in_the_judges_own_words_refuses_and_places_nothing() {
    let workflow = workflow();
    let judged = brief(&workflow)
        .read(
            "verdict: not_met\n\
             expected: the reader to stop one row earlier than it does\n\
             produced: a bound that moved with it\n\
             consequence: every caller reads one row too many",
        )
        .expect("a refusal with no quotation in it");
    assert_eq!(judged.verdict, JudgeVerdict::NotMet);
    assert_eq!(
        judged.cited,
        Some(Vec::new()),
        "there is nothing quoted to place, and the refusal stands anyway"
    );
}

/// A quotation out of one Check's output is placed under that Check's name.
///
/// **`checks` and `check:build` are different answers**, and the difference is
/// the whole value of the label: a step with four Checks renders four output
/// blocks, and a citation naming the tier would send a reader to all of them.
#[test]
fn a_quotation_from_a_checks_output_is_placed_under_that_checks_name() {
    let workflow = workflow();
    let judged = brief_told(
        &workflow,
        &[],
        Answered::of(
            &checks(),
            &[Printed {
                check: "build",
                said: "test result: ok. 412 passed; 0 failed",
            }],
        ),
    )
    .read(
        "verdict: not_met\n\
         expected: a suite covering the new bound\n\
         produced: a run that reports \"412 passed; 0 failed\" with no case for it\n\
         consequence: the bound is unverified and the suite says otherwise",
    )
    .expect("a refusal quoting the Check's own output");
    assert_eq!(
        judged
            .cited
            .as_deref()
            .expect("a call records what it placed")
            .iter()
            .map(|citation| citation.region.as_str())
            .collect::<Vec<_>>(),
        vec!["check:build"],
    );
}

/// The digest is a function of the text and nothing else.
///
/// **What it is for is one comparison**: two members of one panel were handed
/// the same object, or they were not. So what it has to be is the same function
/// on both sides of that comparison and different for different text — not
/// collision-resistant, and not stable across releases.
#[test]
fn one_text_digests_one_way_and_two_texts_do_not() {
    let workflow = workflow();
    let question = brief(&workflow).question().to_string();
    assert_eq!(crate::digest(&question), crate::digest(&question));
    assert_ne!(
        crate::digest(&question),
        crate::digest(&format!("{question} "))
    );
}
