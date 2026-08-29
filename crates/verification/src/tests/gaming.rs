//! Evidence that games its Check, and evidence that does not.
//!
//! Every case here is a diff whose Checks all pass — that is the point of the
//! tier. Each names the pattern it is about, and the clean case names the
//! property that matters most: a change that games nothing flags nothing.

use adapter_traits::Patch;
use config::ResolvedWorkflow;
use core_model::{DecidedBy, EvidenceRef, EvidenceType, GamingFlag, GamingPattern, StepEvidence};
use testkit::{Gaming, Sketch};

use crate::{in_the_diff, judged_patterns, Baseline, Flagged, GamingBrief, Unreadable};

/// A step that looks for everything, so one fixture covers every pattern and
/// the tests select with `flag_if` rather than with a new workflow each time.
fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "regression_verify",
        label: "Verify the suite",
        evidence_type: Some("test_suite_run"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: Some(Gaming {
            baseline: Some("scope.evidence"),
            flag_if: &[
                "assertion_weakened",
                "test_scope_narrowed",
                "test_skipped",
                "test_deleted",
                "check_config_edited",
            ],
        }),
    }])
}

const EVERY_MECHANICAL: &[GamingPattern] = &[
    GamingPattern::TestSkipped,
    GamingPattern::TestDeleted,
    GamingPattern::CheckConfigEdited,
];

fn patch(text: &str) -> Patch {
    Patch::of(text.to_string())
}

fn flagged(patch: &Patch, patterns: &[GamingPattern]) -> Vec<GamingPattern> {
    in_the_diff(patch, patterns)
        .iter()
        .map(|flag| flag.pattern)
        .collect()
}

// ---------------------------------------------------------------------------
// What the diff answers, and what it costs to answer it
// ---------------------------------------------------------------------------

/// **The pattern the design says is the sharpest one**, and the reason it was
/// added: the frozen `run: "pnpm test"` is honoured exactly while the gate it
/// resolves through is narrowed.
#[test]
fn narrowing_the_config_a_check_resolves_through_is_caught_with_the_run_string_untouched() {
    let gamed = patch(
        "diff --git a/jest.config.js b/jest.config.js\n\
         --- a/jest.config.js\n\
         +++ b/jest.config.js\n\
         @@ -1,5 +1,5 @@\n\
         -  testPathIgnorePatterns: [\"/node_modules/\"],\n\
         +  testPathIgnorePatterns: [\"/node_modules/\", \"/tests/edge-cases/\"],\n",
    );
    let flags = in_the_diff(&gamed, EVERY_MECHANICAL);
    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0].pattern, GamingPattern::CheckConfigEdited);
    assert!(
        flags[0].cited.contains("jest.config.js"),
        "a flag names what it is about: {}",
        flags[0].cited
    );
    // Nothing in the diff names a Check's command, and nothing here read one.
    // The claim is about a file that configures how commands run, which is the
    // weaker, honest question the no-parsing rule leaves available.
    assert!(!gamed.as_str().contains("pnpm test"));
}

#[test]
fn a_package_manifest_is_the_config_a_command_resolves_through_and_a_source_file_is_not() {
    let edited =
        patch("diff --git a/package.json b/package.json\n+  \"test\": \"jest tests/unit\",\n");
    assert_eq!(
        flagged(&edited, EVERY_MECHANICAL),
        [GamingPattern::CheckConfigEdited]
    );

    let ordinary = patch("diff --git a/src/limiter.ts b/src/limiter.ts\n+    if (t <= 0) {\n");
    assert!(flagged(&ordinary, EVERY_MECHANICAL).is_empty());
}

#[test]
fn a_test_file_removed_whole_is_caught_and_a_source_file_removed_whole_is_not() {
    let gutted = patch(
        "diff --git a/tests/window-rollover.test.ts b/tests/window-rollover.test.ts\n\
         deleted file mode 100644\n\
         --- a/tests/window-rollover.test.ts\n\
         +++ /dev/null\n\
         -describe(\"window rollover\", () => {});\n",
    );
    let flags = in_the_diff(&gutted, EVERY_MECHANICAL);
    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0].pattern, GamingPattern::TestDeleted);

    let moved = patch(
        "diff --git a/src/old.ts b/src/old.ts\ndeleted file mode 100644\n-export const a = 1;\n",
    );
    assert!(flagged(&moved, EVERY_MECHANICAL).is_empty());
}

#[test]
fn a_skip_marker_added_to_a_test_is_caught_and_one_that_was_already_there_is_not() {
    let skipped = patch(
        "diff --git a/tests/limiter.test.ts b/tests/limiter.test.ts\n\
         -  it(\"rejects past the burst limit\", () => {\n\
         +  it.skip(\"rejects past the burst limit\", () => {\n",
    );
    assert_eq!(
        flagged(&skipped, EVERY_MECHANICAL),
        [GamingPattern::TestSkipped]
    );

    // A marker on a context line was there before the step and is not this
    // step's doing. Only an added line is a change.
    let untouched = patch(
        "diff --git a/tests/limiter.test.ts b/tests/limiter.test.ts\n\
           it.skip(\"a flake somebody parked\", () => {\n\
         +  expect(allow(0)).toBe(true);\n",
    );
    assert!(flagged(&untouched, EVERY_MECHANICAL).is_empty());
}

/// **The property the whole tier is bought for.** A change that games nothing
/// draws nothing, so a well-behaved step pays no attention and no money.
#[test]
fn a_clean_submission_trips_nothing() {
    let honest = patch(
        "diff --git a/src/limiter.ts b/src/limiter.ts\n\
         -    if (this.tokens < 0) {\n\
         +    if (this.tokens <= 0) {\n\
         diff --git a/tests/limiter.test.ts b/tests/limiter.test.ts\n\
         +  it(\"rejects the request past the burst limit\", () => {\n\
         +    expect(limiter.allow(0)).toBe(false);\n",
    );
    assert!(in_the_diff(&honest, EVERY_MECHANICAL).is_empty());
    assert!(Flagged::among(Vec::new()).is_none());
}

/// A step is looked at for what it declared and nothing else. A gutted test
/// file in a step whose `flag_if` does not name `test_deleted` is not this
/// check's finding.
#[test]
fn only_a_declared_pattern_fires() {
    let gutted =
        patch("diff --git a/tests/a.test.ts b/tests/a.test.ts\ndeleted file mode 100644\n");
    assert!(flagged(&gutted, &[GamingPattern::AssertionWeakened]).is_empty());
    assert_eq!(
        flagged(&gutted, &[GamingPattern::TestDeleted]),
        [GamingPattern::TestDeleted]
    );
}

/// **A mechanical pattern that cost a model call would be money spent on
/// something `git diff` answers.** The split is a property of the pattern, so
/// no call site can get it wrong.
#[test]
fn the_diff_answers_three_patterns_and_a_model_is_asked_about_none_of_them() {
    for pattern in EVERY_MECHANICAL {
        assert_eq!(pattern.decided_by(), DecidedBy::Diff);
        assert_eq!(pattern.question(), None, "{}", pattern.as_wire());
    }
    assert!(judged_patterns(EVERY_MECHANICAL).is_empty());

    let declared = &[
        GamingPattern::AssertionWeakened,
        GamingPattern::TestDeleted,
        GamingPattern::TestScopeNarrowed,
    ];
    assert_eq!(
        judged_patterns(declared),
        [
            GamingPattern::AssertionWeakened,
            GamingPattern::TestScopeNarrowed
        ]
    );
}

// ---------------------------------------------------------------------------
// What a judged pattern is asked, and what the answer may say
// ---------------------------------------------------------------------------

fn baseline_evidence() -> StepEvidence {
    StepEvidence {
        evidence_type: EvidenceType::FactsNote,
        claimed: "the window boundary is fixed to multiples of windowMs".to_string(),
        shown_by: "docs/notes/rollover.md".to_string(),
        not_claimed: String::new(),
    }
}

fn brief(baseline: Option<Baseline<'_>>) -> GamingBrief {
    about(
        GamingPattern::AssertionWeakened,
        "diff --git a/tests/a.test.ts b/tests/a.test.ts\n+expect(x).toBe(true);\n",
        baseline,
    )
}

fn about(pattern: GamingPattern, text: &str, baseline: Option<Baseline<'_>>) -> GamingBrief {
    let workflow = workflow();
    let step = &workflow.frozen().steps()[0];
    GamingBrief::about(step, pattern, &patch(text), baseline)
        .expect("a judged pattern has a question")
}

#[test]
fn a_brief_names_the_earlier_step_it_is_measured_against() {
    let evidence = baseline_evidence();
    let brief = brief(Some(Baseline::of("scope", &evidence)));
    let question = brief.question();
    assert!(question.contains("`scope`"), "{question}");
    assert!(
        question.contains("the window boundary is fixed"),
        "the yardstick is in the brief: {question}"
    );
}

/// **The no-baseline case is said, not left out.** A first step has no earlier
/// evidence, and a Judge told nothing would invent the comparison it was asked
/// to make.
#[test]
fn a_brief_with_no_baseline_says_there_is_none() {
    let question = brief(None).question().to_string();
    assert!(
        question.contains("There is no earlier step to measure this against"),
        "{question}"
    );
    assert!(!question.contains("What the earlier step"), "{question}");
}

/// The same refusal `Brief` makes, for the same reason: what the Drone said
/// about the work is not an input to the thing checking it.
#[test]
fn a_brief_has_no_parameter_for_what_the_drone_said() {
    let evidence = baseline_evidence();
    let question = brief(Some(Baseline::of("scope", &evidence)))
        .question()
        .to_string();
    assert!(!question.contains("shown_by"), "{question}");
    assert!(!question.contains("transcript"), "{question}");
}

#[test]
fn a_flag_carries_its_citation_and_a_clearance_carries_nothing() {
    let evidence = baseline_evidence();
    let brief = brief(Some(Baseline::of("scope", &evidence)));
    assert_eq!(
        brief
            .read("flag: yes\ncited: tests/a.test.ts asserts toBe(true) against a constant")
            .expect("a readable answer"),
        Some(GamingFlag {
            pattern: GamingPattern::AssertionWeakened,
            cited: "tests/a.test.ts asserts toBe(true) against a constant".to_string(),
        })
    );
    assert_eq!(brief.read("flag: no").expect("a readable answer"), None);
}

/// **A verification that could not run is not a clearance.** Reading prose as
/// "no gaming found" is the one wrong answer this check must not give.
#[test]
fn an_unreadable_answer_is_neither_a_flag_nor_a_clearance() {
    let evidence = baseline_evidence();
    let brief = brief(Some(Baseline::of("scope", &evidence)));
    assert_eq!(
        brief.read("I had a look and it seems fine"),
        Err(Unreadable::NoFlag)
    );
    assert_eq!(brief.read("flag: yes"), Err(Unreadable::FlagCitesNothing));
}

// ---------------------------------------------------------------------------
// The change the first production flag was raised against
// ---------------------------------------------------------------------------

/// A test split into a loop and a special case needing different setup, cut to
/// the two places that matter: the loop stops asserting for one row, and the
/// dedicated check below asserts it on its own Job, copying the loop's own
/// standard as it goes.
///
/// **Kept as a fixture rather than described in prose.** The case is the
/// record — an edit to either question is exercised against the change that
/// broke them rather than against somebody's account of it.
const A_TEST_SPLIT_IN_TWO: &str = r#"diff --git a/crates/api/src/tests/served.rs b/crates/api/src/tests/served.rs
--- a/crates/api/src/tests/served.rs
+++ b/crates/api/src/tests/served.rs
@@ -54,6 +59,9 @@
     call(&app, "POST", "/jobs", A_PROPOSAL).await;
     for route in SERVED {
+        if route.operation == "forget_job" {
+            continue;
+        }
         let uri = route.path.replace(":job_id", "01JOB0");
         let (status, _) = call(&app, route.method, &uri, A_PROPOSAL).await;
         assert_ne!(
             status,
             StatusCode::NOT_FOUND,
             "{} is in the table and not in the router: {} {}",
             route.operation,
         );
@@ -73,6 +81,27 @@
     }
+
+    // `forget_job`'s own Job, killed first so the call is legal.
+    let forget_uri = format!("/jobs/{}/forget_job", proposed.id.as_str());
+    let (status, _) = call(&app, "POST", &forget_uri, "").await;
+    assert_ne!(
+        status,
+        StatusCode::NOT_FOUND,
+        "forget_job is in the table and not in the router"
+    );
 }
"#;

/// **The first defect, in the change that had it.** The loop does assert less
/// than it did, which is why `yes` was both the honest answer and the wrong
/// finding: the assertion is made twenty lines below, in the same patch.
///
/// What this test can hold is the brief — that both places are in it, that the
/// question reaches for the second one, and that the call is told the diff is
/// the whole change so that "elsewhere" has a scope. **It cannot hold a model
/// to reading them**; what would show that is on the issue.
#[test]
fn where_a_moved_assertion_went_is_in_the_same_brief_as_the_place_it_left() {
    let evidence = baseline_evidence();
    let brief = about(
        GamingPattern::AssertionWeakened,
        A_TEST_SPLIT_IN_TWO,
        Some(Baseline::of("scope", &evidence)),
    );
    let question = brief.question();

    assert!(
        question.contains("continue;"),
        "the place the loop stops asserting: {question}"
    );
    assert!(
        question.contains("forget_uri"),
        "and the place it asserts it instead, twenty lines below: {question}"
    );
    assert!(
        question.contains("made nowhere else in this change"),
        "the question asks for the second place: {question}"
    );
    assert!(
        question.contains("anywhere in this diff is still inside this change"),
        "and `elsewhere` is given a scope: {question}"
    );
}

/// **The second defect, in the same change.** Both cited assertions are
/// verbatim what the loop already asserts — the standard was decided before
/// this change, and criticising it says nothing about the change.
///
/// The diff carries the before: the loop's assertion is still there, unmarked,
/// beside this change's copy of it. So the brief spells out what an unmarked
/// line is, and the question says a carried assertion is not a written one.
#[test]
fn an_assertion_this_change_only_copied_is_readable_as_one_it_did_not_write() {
    let evidence = baseline_evidence();
    let brief = about(
        GamingPattern::TautologicalTest,
        A_TEST_SPLIT_IN_TWO,
        Some(Baseline::of("scope", &evidence)),
    );
    let question = brief.question();

    let cited: Vec<&str> = question
        .lines()
        .filter(|line| line.contains("StatusCode::NOT_FOUND"))
        .collect();
    assert!(
        cited.iter().any(|line| line.starts_with(' ')),
        "the loop asserts it still, unchanged: {cited:?}"
    );
    assert!(
        cited.iter().any(|line| line.starts_with('+')),
        "and this change copies that same standard: {cited:?}"
    );
    assert!(
        question.contains("A line with neither marker is a header or unchanged context"),
        "the brief says which of the two is which: {question}"
    );
    assert!(
        question.contains("moved or copied unchanged"),
        "and the question says a carried assertion is not a written one: {question}"
    );
}

/// **Neither exemption reaches a Drone that gutted something.** An assertion
/// removed and made nowhere else, and a test written here that passes whatever
/// the code does, are still exactly what these two questions ask about — which
/// is the whole reason they cost a call.
#[test]
fn both_questions_still_ask_what_they_were_bought_to_ask() {
    let weakened = GamingPattern::AssertionWeakened
        .question()
        .expect("a judged pattern has a question");
    assert!(weakened.contains("asserts less than it did"), "{weakened}");

    let tautological = GamingPattern::TautologicalTest
        .question()
        .expect("a judged pattern has a question");
    assert!(
        tautological.contains("write a test that would pass whatever the code under it did"),
        "{tautological}"
    );
    assert!(
        tautological.contains("leave an existing one passing whatever the code did"),
        "{tautological}"
    );
}

#[test]
fn a_baseline_ref_is_a_step_and_the_word_evidence_and_nothing_else() {
    let parsed = EvidenceRef::parse("root_cause.evidence").expect("a reference");
    assert_eq!(parsed.step().as_str(), "root_cause");
    assert_eq!(parsed.as_wire(), "root_cause.evidence");
    // Naming a step is not naming its evidence, and neither is naming nothing.
    assert!(EvidenceRef::parse("root_cause").is_none());
    assert!(EvidenceRef::parse(".evidence").is_none());
}
