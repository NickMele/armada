//! What counts as a quotation, and what counts as finding it.
//!
//! The cases in [`judge`](super::judge) are about the verdict. These are about
//! the reading underneath it — where the check has to be lenient so that an
//! honestly-worded refusal survives it, and where it must not be.

use crate::quoted::invented;

/// The material one call was shown, in the shape a brief has it: wrapped,
/// indented, and with the yardstick above the work.
fn shown() -> String {
    String::from(
        "What earlier steps established, which this work is measured against:\n\n  \
         `scope` established: the jobs list gains an action removing every\n  \
         terminal job, wired through the routes and the daemon\n\n\
         The change, as a diff:\n\n\
         +    let n = n - 1;\n",
    )
}

#[test]
fn a_quotation_that_is_in_the_material_is_not_invented() {
    assert_eq!(
        invented(
            "the note says \"wired through the routes and the daemon\"",
            &shown()
        ),
        None
    );
}

/// The brief wraps and indents, and a model quoting across a wrap writes one
/// line. An exact byte match would fail on where the source broke a line, which
/// is a fact about formatting rather than about the answer.
#[test]
fn a_quotation_spanning_a_line_break_in_the_source_is_found() {
    assert_eq!(
        invented(
            "the note reads \"an action removing every terminal job, wired through the routes\"",
            &shown()
        ),
        None
    );
}

/// Punctuation and case are not what is being checked. An invented sentence is
/// caught by its words; a smart quote or an em dash where the source had a
/// hyphen is not a fabrication.
#[test]
fn punctuation_and_case_do_not_make_a_quotation_invented() {
    assert_eq!(
        invented(
            "the note says \"Wired Through The Routes — And The Daemon!\"",
            &shown()
        ),
        None
    );
    assert_eq!(invented("the diff shows “let n = n - 1”", &shown()), None);
}

/// A model quoting across a cut writes the ellipsis. Each side of it is checked
/// on its own, and the invented one is the side that is named.
#[test]
fn each_side_of_an_elision_is_checked_on_its_own() {
    assert_eq!(
        invented(
            "the note says \"the jobs list gains an action … wired through the routes\"",
            &shown()
        ),
        None
    );
    assert!(invented(
        "the note says \"the jobs list gains an action … and the IPC wiring is not done here\"",
        &shown()
    )
    .is_some());
}

#[test]
fn a_sentence_that_is_in_none_of_it_is_named_back() {
    let span = invented(
        "as scope notes \"Implementation, tests, and the IPC/UI wiring itself \
         are not done\"",
        &shown(),
    )
    .expect("a quotation that is in nothing it was shown");
    assert!(span.starts_with("Implementation, tests"), "{span}");
}

/// Under four words a quotation mark is emphasis or a term, and the check does
/// not read it as a claim about how something is worded.
#[test]
fn a_short_quotation_is_never_invented() {
    assert_eq!(invented("the note calls it \"one early\"", &shown()), None);
    assert_eq!(
        invented("the note describes the \"terminal jobs\" case", &shown()),
        None
    );
    assert!(invented("the note says \"nothing at all like it\"", &shown()).is_some());
}

/// A quotation mark with nothing closing it opens nothing — there is no span to
/// check, and inventing a boundary for it would invent the claim too.
#[test]
fn an_unclosed_quotation_mark_is_not_a_quotation() {
    assert_eq!(
        invented("the note says \"and then it trails off here", &shown()),
        None
    );
    assert_eq!(invented("\"\"", &shown()), None);
}

// ------------------------------------------ what a quotation is attributed to

/// **The two honest refusals this check stopped**, written back verbatim. Both
/// are the Judge's own wording in quotation marks — one a standard it wanted
/// met, the other its own paraphrase of the facts it had been shown — and
/// neither says the words are anywhere. There is nothing here for containment
/// to check, so it does not check it.
#[test]
fn an_unattributed_quotation_is_the_judges_own_wording() {
    for own in [
        "\"A plan addressing all five variants as stated: NotResumable, \
         NoDroneToRedirect, DroneStillThere, WorktreeGone and NoStepStopped, each \
         currently answering 500 and requiring 409, with clear evidence they all \
         reach the catch-all\"",
        "\"these three variants are unmapped in the match statement at serving.rs \
         and fall through to the catch-all 500 handler\"",
    ] {
        assert_eq!(invented(own, &shown()), None, "{own}");
    }
}

/// And the same words, once a source is named for them. Nothing about the span
/// changed; what changed is that the answer now claims it is somewhere, which
/// is a claim that can be wrong.
#[test]
fn the_same_words_attributed_to_a_source_are_checked() {
    assert!(invented(
        "the note says \"these three variants are unmapped in the match statement\"",
        &shown()
    )
    .is_some());
}

/// **Attribution is read from the run-up, not from the field.** A model that
/// names a source for one quotation must not have that vouch for the next one,
/// or the check is satisfied by writing one honest citation first.
#[test]
fn attribution_does_not_carry_from_one_quotation_to_the_next() {
    let honest_then_own = "the note says \"wired through the routes and the daemon\", \
                           and the bar is \"a plan addressing every terminal job in one pass\"";
    assert_eq!(invented(honest_then_own, &shown()), None);
    // The second span is not simply going unread: attribute it and it fails.
    let honest_then_invented = "the note says \"wired through the routes and the daemon\", \
                                and the diff shows \"a plan addressing every terminal job \
                                in one pass\"";
    assert!(invented(honest_then_invented, &shown()).is_some());
}

/// The same cut at a sentence boundary, for the answer that names a source and
/// then states a standard. Two sentences, one attributed and one not.
#[test]
fn attribution_does_not_carry_across_a_sentence() {
    assert_eq!(
        invented(
            "the note says the wiring is done. The bar is \"a plan addressing every \
             terminal job in one pass\"",
            &shown()
        ),
        None
    );
}

/// A gaming citation points at a hunk rather than at something said — *the
/// suite drops "…"* — and that is attribution too: it claims the words are in
/// the diff. The gaming path's own cases are in [`gaming`](super::gaming); this
/// is the reading they rest on.
#[test]
fn words_a_source_is_said_to_have_dropped_are_attributed_to_it() {
    assert!(invented(
        "the suite drops \"the rollover window is pinned to a whole multiple\"",
        &shown()
    )
    .is_some());
}
