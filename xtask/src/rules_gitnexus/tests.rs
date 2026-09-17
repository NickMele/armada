//! The rule's own negative tests, against text written here rather than the
//! skills on disk, which can only be in one state at a time.

use super::*;
use crate::Finding;

/// Whether the text would fail the gate, and every finding it produced.
fn run(text: &str) -> (bool, Vec<String>) {
    let mut report = Report::new("test");
    check("gitnexus-guide/SKILL.md", text, &mut report);
    let found = report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect();
    (report.failed(), found)
}

#[test]
fn the_pinned_script_reports_nothing() {
    assert_eq!(
        run("> If the index is stale, run `pnpm gitnexus:index` in terminal.\n"),
        (false, Vec::new())
    );
}

#[test]
fn an_inline_bare_analyze_is_refused_and_names_the_line_and_the_fix() {
    let (failed, found) =
        run("intro\n> If stale, run `node .gitnexus/run.cjs analyze` in terminal.\n");
    assert!(failed);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("SKILL.md:2"), "{}", found[0]);
    assert!(found[0].contains(FIX), "{}", found[0]);
}

#[test]
fn a_fenced_bare_analyze_is_refused() {
    let (failed, _) = run("```bash\nnpx gitnexus@latest analyze\n```\n");
    assert!(failed);
}

#[test]
fn a_flag_that_skips_the_block_lets_it_through() {
    for span in [
        "`gitnexus analyze --index-only`",
        "`node .gitnexus/run.cjs analyze --skip-agents-md --skip-skills`",
        "`node .gitnexus/run.cjs analyze --watch`",
    ] {
        assert_eq!(run(span), (false, Vec::new()), "{span}");
    }
}

#[test]
fn analyze_in_prose_or_after_another_tool_is_not_an_invocation() {
    assert_eq!(
        run("Run a one-shot `analyze` when needed, or `cargo analyze` elsewhere.\n"),
        (false, Vec::new())
    );
}
