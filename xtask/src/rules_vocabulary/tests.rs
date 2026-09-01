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
