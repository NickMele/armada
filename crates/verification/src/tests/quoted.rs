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
            "\"an action removing every terminal job, wired through the routes\"",
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
        invented("\"Wired Through The Routes — And The Daemon!\"", &shown()),
        None
    );
    assert_eq!(invented("“let n = n - 1”", &shown()), None);
}

/// A model quoting across a cut writes the ellipsis. Each side of it is checked
/// on its own, and the invented one is the side that is named.
#[test]
fn each_side_of_an_elision_is_checked_on_its_own() {
    assert_eq!(
        invented(
            "\"the jobs list gains an action … wired through the routes\"",
            &shown()
        ),
        None
    );
    assert!(invented(
        "\"the jobs list gains an action … and the IPC wiring is not done here\"",
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
    assert_eq!(invented("it stops \"one early\"", &shown()), None);
    assert_eq!(invented("the \"terminal jobs\" case", &shown()), None);
    assert!(invented("\"nothing at all like it\"", &shown()).is_some());
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
