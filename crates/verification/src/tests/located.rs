//! Where a flag says it found what it found.
//!
//! The defect these are written against: job detail promised *what the gaming
//! check found, **and where***, and every flag arrived with an assertion and
//! no file — nothing for a person to open. Each case here names which half of
//! a location it establishes and why the other half is missing, because "some
//! patterns can carry one and some cannot" is the answer, and levelling every
//! flag down to the weakest was the alternative.

use adapter_traits::Patch;
use core_model::{CitedAt, GamingPattern, RepoPath};

use crate::{in_the_diff, Baseline, GamingBrief};

/// Two files, and between them every marker a location has to tell apart: a
/// removed line, an added one, and context on both sides.
///
/// The post-image numbering the cases below assert against, counted out here
/// once so no case has to re-derive it:
///
/// | Line | Marker | Post-image |
/// |---|---|---|
/// | `const outcome = defaultOutcome(result.passed);` | context | 12 |
/// | `if (result.failed.length > 0) setOutcome(…)` | removed | — |
/// | `return outcome;` | context | 13 |
/// | `const rows = collect(result);` | context | 40 |
/// | `if (rows.length === 0) return emptyReport(result);` | added | 41 |
/// | `return render(rows);` | context | 42 |
const A_PATCH: &str = "\
diff --git a/src/outcome.ts b/src/outcome.ts
index 1111111..2222222 100644
--- a/src/outcome.ts
+++ b/src/outcome.ts
@@ -12,4 +12,3 @@ export function summarise(result: Result) {
   const outcome = defaultOutcome(result.passed);
-  if (result.failed.length > 0) setOutcome(result.failed[0]!.outcome);
   return outcome;
diff --git a/src/report.ts b/src/report.ts
index 3333333..4444444 100644
--- a/src/report.ts
+++ b/src/report.ts
@@ -40,2 +40,3 @@ export function report(result: Result) {
   const rows = collect(result);
+  if (rows.length === 0) return emptyReport(result);
   return render(rows);
";

fn patch() -> Patch {
    Patch::of(A_PATCH.to_string())
}

/// Where a gaming answer citing `cited` lands, against [`A_PATCH`].
fn located(cited: &str) -> Option<CitedAt> {
    let workflow = crate::tests::workflow();
    let brief = GamingBrief::about(
        crate::tests::gated(&workflow),
        GamingPattern::AssertionWeakened,
        &patch(),
        None::<Baseline<'_>>,
    )
    .expect("a judged pattern has a question");
    brief
        .read(&format!("flag: yes\ncited: {cited}"), &patch())
        .expect("a readable answer")
        .expect("a flag")
        .at
}

// ---------------------------------------------------------------------------
// What a judged flag can be given
// ---------------------------------------------------------------------------

/// **The case from the screen.** The flag that started this quoted a line the
/// change removed, and the panel had nothing to link to.
///
/// It gets the file and no line, and that is the whole answer rather than a
/// stage on the way to one: the words it quotes are not in `src/outcome.ts`
/// once the change lands, so any number offered would point at whatever now
/// sits there.
#[test]
fn a_citation_quoting_a_removed_line_names_the_file_and_no_line() {
    assert_eq!(
        located(
            "the suite drops \"- if (result.failed.length > 0) \
             setOutcome(result.failed[0]!.outcome);\""
        ),
        Some(CitedAt::in_file(RepoPath::new("src/outcome.ts")))
    );
}

/// A line the change writes is in the file it leaves behind, so it gets a
/// number — and the number is the post-image one, counted from the hunk header
/// across the context lines above it.
#[test]
fn a_citation_quoting_an_added_line_names_the_file_and_the_line() {
    assert_eq!(
        located("it adds \"if (rows.length === 0) return emptyReport(result);\""),
        Some(CitedAt::at_line(RepoPath::new("src/report.ts"), 41))
    );
}

/// Context is unchanged and in both images, so it numbers like an added line —
/// and it is the first line of its hunk, which is what pins the arithmetic to
/// the header rather than to the count of lines seen so far.
#[test]
fn a_citation_quoting_context_numbers_from_the_hunk_header() {
    assert_eq!(
        located("the assertion sits under \"const outcome = defaultOutcome(result.passed);\""),
        Some(CitedAt::at_line(RepoPath::new("src/outcome.ts"), 12))
    );
}

/// **The escape the answer format grants, kept whole.** A citation written
/// without quotation marks is about something the change does *not* do, and an
/// absence has no coordinate — so this answers nothing rather than reaching for
/// the nearest plausible file.
#[test]
fn an_unquoted_citation_has_nowhere_to_point() {
    assert_eq!(
        located("REVIEW.md reports nothing against a diff that removes an assertion"),
        None
    );
}

/// A quotation the patch does not hold is the fabrication `quoted::invented`
/// exists for, and the two readings agree: it is marked unchecked *and* it
/// points nowhere. A location for it would be the worse defect of the two —
/// an uncited flag is unactionable and a wrongly cited one is believed.
#[test]
fn a_citation_quoting_what_the_patch_does_not_hold_points_nowhere() {
    assert_eq!(
        located("the diff shows \"every failed case is silently discarded here\""),
        None
    );
}

/// Under four words a quotation is a term rather than a claim about wording —
/// `quoted::A_CITATION`'s judgement, applied here for a second reason: a short
/// run matches in several places and the first one is not evidence of anything.
#[test]
fn a_quotation_too_short_to_be_a_claim_is_too_short_to_be_a_location() {
    assert_eq!(located("it drops \"result.failed\""), None);
}

// ---------------------------------------------------------------------------
// What the three the diff decides can be given
// ---------------------------------------------------------------------------

/// A skip marker is on a line the change writes, so this is the one mechanical
/// pattern that carries a number.
#[test]
fn a_skipped_test_is_located_at_the_line_the_marker_was_added_on() {
    let patch = Patch::of(
        "\
diff --git a/tests/rollover.test.ts b/tests/rollover.test.ts
@@ -7,2 +7,2 @@ describe(\"rollover\", () => {
   const window = makeWindow();
+  it.skip(\"pins the boundary\", () => {
"
        .to_string(),
    );
    let flags = in_the_diff(&patch, &[GamingPattern::TestSkipped]);
    assert_eq!(
        flags[0].at,
        Some(CitedAt::at_line(RepoPath::new("tests/rollover.test.ts"), 8))
    );
}

/// A file removed whole has no post image, so every line of it is gone and the
/// file is the whole of what can be answered.
#[test]
fn a_deleted_test_is_located_at_the_file_and_never_at_a_line() {
    let patch = Patch::of(
        "\
diff --git a/tests/rollover.test.ts b/tests/rollover.test.ts
deleted file mode 100644
"
        .to_string(),
    );
    let flags = in_the_diff(&patch, &[GamingPattern::TestDeleted]);
    assert_eq!(
        flags[0].at,
        Some(CitedAt::in_file(RepoPath::new("tests/rollover.test.ts")))
    );
}

/// The finding is that the file was edited at all, so naming one of its lines
/// would narrow a claim that is about the whole of it.
#[test]
fn an_edited_check_config_is_located_at_the_file_it_is_about() {
    let patch = Patch::of(
        "\
diff --git a/package.json b/package.json
@@ -4,1 +4,1 @@
-    \"test\": \"vitest run\",
+    \"test\": \"vitest run src/only\",
"
        .to_string(),
    );
    let flags = in_the_diff(&patch, &[GamingPattern::CheckConfigEdited]);
    assert_eq!(
        flags[0].at,
        Some(CitedAt::in_file(RepoPath::new("package.json")))
    );
}
