//! Armada's review of a change, as a person reads it on the Job. Since 13.24, #903.
//!
//! Not `review`: `JobDetail.review` is the text Fleet composes for the pull request.

use serde::{Deserialize, Serialize};

/// Whether Armada is confident in the change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Says {
    Confident,
    NotConfident,
}

impl Says {
    pub fn as_wire(&self) -> &'static str {
        match self {
            Says::Confident => "confident",
            Says::NotConfident => "not_confident",
        }
    }
}

/// The review: the verdict and its reasons, the change's areas, its tests, and the findings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobConfidence {
    pub says: Says,
    pub reasons: Vec<String>,
    pub areas: Vec<AreaRow>,
    /// Absent where the change touches no test and the reviewer named nothing untested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<TestsSection>,
    pub needs_you: Vec<FindingRow>,
    pub small_fixes: Vec<FindingRow>,
    pub for_context: Vec<FindingRow>,
    /// Findings a person dismissed, taken out of the three lists above. Since 13.27, #907.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dismissed: Vec<DismissedRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaRow {
    pub name: String,
    pub what: String,
    pub files: Vec<String>,
    /// The area read as a View. Since 13.25, #904; left out where it has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view: Vec<ViewStepRow>,
}

/// One step of a View: a hunk named by its `@@` header, never copied. Since 13.25, #904.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewStepRow {
    pub file: String,
    pub hunk: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tie_to_next: Option<String>,
}

/// Tests in the change. It opens itself where a test was removed or loosened with no reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestsSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened_because: Option<OpenedBecause>,
    pub proves: Vec<ProvesRow>,
    pub changed: Vec<ChangedTestRow>,
    pub untested: Vec<UntestedRow>,
}

/// Why a section opened itself, naming what to look at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpenedBecause {
    TestRemoved { name: String },
    TestLoosened { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvesRow {
    pub area: String,
    pub what: String,
    pub tests: u32,
}

/// A test the change removed or loosened. `flagged` where it was given no reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedTestRow {
    pub name: String,
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    pub flagged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntestedRow {
    pub code: String,
    pub why: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingRow {
    pub finding: String,
    pub why: String,
    /// The code the finding is about, as a View. Since 13.25, #904; left out where it has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view: Vec<ViewStepRow>,
}

/// A finding a person dismissed from the review, and why. Since 13.27, #907.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissedRow {
    pub finding: String,
    pub reason: String,
}

/// `dismiss_finding`'s body: the finding, in the words the review served it in, and why
/// it is wrong. Since 13.27, #907. **A blank reason is refused by Fleet**, a 422, because
/// the reason is what the next review pass is handed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingDismissed {
    pub finding: String,
    pub reason: String,
}

impl JobConfidence {
    pub fn of(record: &core_model::ReviewRecord) -> JobConfidence {
        JobConfidence {
            says: match record.confidence {
                core_model::Confidence::Confident => Says::Confident,
                core_model::Confidence::NotConfident => Says::NotConfident,
            },
            reasons: record.reasons.clone(),
            areas: record
                .areas
                .iter()
                .map(|area| AreaRow {
                    name: area.name().to_string(),
                    what: area.what().to_string(),
                    files: area.files().to_vec(),
                    view: view_rows(area.view()),
                })
                .collect(),
            tests: tests_section(record),
            needs_you: in_bucket(record, core_model::Bucket::NeedsYou),
            small_fixes: in_bucket(record, core_model::Bucket::SmallFix),
            for_context: in_bucket(record, core_model::Bucket::ForContext),
            dismissed: Vec::new(),
        }
    }

    /// The same review, with what a person dismissed taken off the lists and named apart.
    pub fn without(mut self, dismissed: &[core_model::Dismissal]) -> JobConfidence {
        let kept = |row: &FindingRow| !dismissed.iter().any(|gone| gone.finding == row.finding);
        self.needs_you.retain(kept);
        self.small_fixes.retain(kept);
        self.for_context.retain(kept);
        self.dismissed = dismissed
            .iter()
            .map(|gone| DismissedRow {
                finding: gone.finding.clone(),
                reason: gone.reason.clone(),
            })
            .collect();
        self
    }
}

fn tests_section(record: &core_model::ReviewRecord) -> Option<TestsSection> {
    let tests = &record.tests;
    if tests.is_empty() {
        return None;
    }
    let unexplained = |name: &str| {
        record
            .unexplained_tests
            .iter()
            .any(|test| test.name == name)
    };
    Some(TestsSection {
        opened_because: record
            .unexplained_tests
            .first()
            .map(|test| match test.change {
                core_model::TestChange::Removed { .. } => OpenedBecause::TestRemoved {
                    name: test.name.clone(),
                },
                core_model::TestChange::Loosened => OpenedBecause::TestLoosened {
                    name: test.name.clone(),
                },
            }),
        proves: tests
            .proves
            .iter()
            .map(|proves| ProvesRow {
                area: proves.area().to_string(),
                what: proves.what().to_string(),
                tests: proves.tests(),
            })
            .collect(),
        changed: tests
            .changed
            .iter()
            .map(|test| ChangedTestRow {
                name: test.name.clone(),
                change: test.change.as_wire().to_string(),
                replaced_by: match &test.change {
                    core_model::TestChange::Removed { replaced_by } => replaced_by.clone(),
                    core_model::TestChange::Loosened => None,
                },
                why: test.why.clone(),
                flagged: unexplained(&test.name),
            })
            .collect(),
        untested: tests
            .untested
            .iter()
            .map(|untested| UntestedRow {
                code: untested.code().to_string(),
                why: untested.why().to_string(),
            })
            .collect(),
    })
}

fn in_bucket(record: &core_model::ReviewRecord, bucket: core_model::Bucket) -> Vec<FindingRow> {
    record
        .findings
        .iter()
        .filter(|finding| finding.bucket() == bucket)
        .map(|finding| FindingRow {
            finding: finding.finding().to_string(),
            why: finding.why().to_string(),
            view: view_rows(finding.view()),
        })
        .collect()
}

fn view_rows(view: &[core_model::ViewStep]) -> Vec<ViewStepRow> {
    view.iter()
        .map(|step| ViewStepRow {
            file: step.file.clone(),
            hunk: step.hunk.clone(),
            summary: step.summary.clone(),
            tie_to_next: step.tie_to_next.clone(),
        })
        .collect()
}
