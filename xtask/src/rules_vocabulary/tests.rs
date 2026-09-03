//! The frame the gate and the generator share, proved from the gate's side.
//!
//! The rule itself is a file comparison and the repository can only be in one
//! state at a time — the agreeing one — so what is worth testing here is the
//! part that can be wrong without anything looking wrong: the NUL frame. A
//! misread frame pairs a path with the wrong text and reports every file stale,
//! or worse, reads one pair out of three and gates a third of what it should.

use super::*;

/// The frame the generator writes: a NUL after every field, including the last.
fn framed(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(path, text)| format!("{path}\0{text}\0"))
        .collect()
}

#[test]
fn a_run_reads_back_as_the_pairs_it_wrote() {
    let out = framed(&[("a/one.ts", "export const A = 1;\n"), ("b/two.ts", "")]);
    assert_eq!(
        parse(&out).unwrap(),
        vec![
            ("a/one.ts".to_string(), "export const A = 1;\n".to_string()),
            ("b/two.ts".to_string(), String::new()),
        ]
    );
}

/// The whole reason the frame is NUL-separated rather than line- or
/// length-delimited: a generated module is mostly newlines and quotes.
#[test]
fn a_body_full_of_newlines_and_quotes_stays_one_field() {
    let body = "// GENERATED\n\nexport const X = { verb: \"Needs you\" };\n";
    let parsed = parse(&framed(&[("a/one.ts", body)])).unwrap();
    assert_eq!(parsed, vec![("a/one.ts".to_string(), body.to_string())]);
}

#[test]
fn an_odd_number_of_fields_is_refused_rather_than_paired_off() {
    let out = "a/one.ts\0export const A = 1;\0b/two.ts\0";
    assert!(parse(out).unwrap_err().contains("pairs of path and text"));
}

#[test]
fn a_generator_that_emitted_nothing_is_refused() {
    assert!(parse("").unwrap_err().contains("gates no files"));
}

/// The rule runs against the repository it is checked into, which is the only
/// place the three real outputs exist. A green here says the checked-in files
/// are what the registries say today.
#[test]
fn the_repository_agrees_with_its_own_registries() {
    let report = the_generated_vocabulary_says_what_the_registries_say(&crate::repo_root());
    let complaints: Vec<&String> = report
        .findings
        .iter()
        .map(|f| match f {
            crate::Finding::Fail(what) | crate::Finding::Warn(what) => what,
        })
        .collect();
    assert!(complaints.is_empty(), "{complaints:#?}");
}

/// The one blank the vocabulary sanctions, spelled as the generated module
/// spells its keys.
///
/// `escalated` renders whichever escalation reason is set and never its own
/// name, because nobody says a Job escalated at step 3. It is the only variant
/// on any rendered vocabulary that is meant to reach a surface with no word.
const RENDERS_ITS_REASON: &str = "\"escalated\"";

/// **No variant a surface draws reaches it as a wire spelling.**
///
/// The design contract says this map is one artifact with a test asserting
/// every variant has copy, so a new reason cannot ship without any. Until now
/// the generator only *counted* a missing verb: eight vocabularies had rows
/// that said nothing and two had no rows at all, and each was found by a person
/// reading `check_config_edited` or `nothing_writing` off a screen in mono.
/// Issue #110.
///
/// It reads the emitted module rather than the registry because the emitted
/// module is the set a surface can actually reach — a vocabulary nothing wants
/// has no rows here and is not this test's business, and the rule above already
/// holds the two in agreement.
#[test]
fn every_variant_a_surface_renders_has_a_word() {
    let module = crate::repo_root().join("packages/components/src/generated/vocabulary.ts");
    let text = std::fs::read_to_string(&module).expect("the generated vocabulary");

    let wordless: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("verb: null"))
        .filter(|line| !line.trim_start().starts_with(RENDERS_ITS_REASON))
        .collect();

    assert!(
        wordless.is_empty(),
        "these render as their wire spelling at a person, and each needs a `verb` in \
         crates/core-model/domain/enum-verbs.toml:\n{wordless:#?}"
    );
}

/// A vocabulary the generator wants and the registry has no rows for emits an
/// empty map, which the test above cannot see — there is no variant to key a
/// `verb: null` on. This is the other half of the same claim.
#[test]
fn no_vocabulary_a_surface_renders_is_empty() {
    let module = crate::repo_root().join("packages/components/src/generated/vocabulary.ts");
    let text = std::fs::read_to_string(&module).expect("the generated vocabulary");

    let empty: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("Rendering | undefined>> = {};"))
        .collect();

    assert!(
        empty.is_empty(),
        "these have no rows in crates/core-model/domain/enum-verbs.toml at all, so every \
         variant of them renders as its wire spelling:\n{empty:#?}"
    );
}
