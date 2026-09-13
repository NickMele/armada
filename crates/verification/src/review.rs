//! A review as its Drone wrote it, the faults Fleet refuses in it, and what Fleet adds.
//!
//! The parts are `core-model`'s, so what this checks is the record `store` and `ipc` read.

use core_model::{Area, Bucket, ChangedTest, Confidence, Finding, ReviewRecord, TestsInChange};

/// Why Fleet would not take a review. Every one names what to fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewRefused {
    FileInNoArea { path: String },
    TestNotInDiff { name: String },
    FindingWithNoReason { finding: String },
    ViewStepNotInDiff { file: String, hunk: String },
}

impl core::fmt::Display for ReviewRefused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReviewRefused::FileInNoArea { path } => {
                write!(
                    f,
                    "`{path}` changed and no area names it. Put it in an area"
                )
            }
            ReviewRefused::TestNotInDiff { name } => write!(
                f,
                "`{name}` is named as a changed test and the diff never touches it"
            ),
            ReviewRefused::FindingWithNoReason { finding } => write!(
                f,
                "\"{finding}\" gives no reason. Say why it is there, or leave it out"
            ),
            ReviewRefused::ViewStepNotInDiff { file, hunk } => write!(
                f,
                "View names `{hunk}` in `{file}`, and the diff holds no such hunk there. \
                 Name a hunk by its @@ header, as git wrote it"
            ),
        }
    }
}

impl std::error::Error for ReviewRefused {}

/// A review as its Drone wrote it, before Fleet has checked it against the change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    confidence: Confidence,
    reasons: Vec<String>,
    areas: Vec<Area>,
    tests: TestsInChange,
    findings: Vec<Finding>,
}

impl Review {
    pub fn written(
        confidence: Confidence,
        reasons: &[&str],
        areas: Vec<Area>,
        tests: TestsInChange,
        findings: Vec<Finding>,
    ) -> Review {
        Review {
            confidence,
            reasons: reasons.iter().map(|reason| reason.to_string()).collect(),
            areas,
            tests,
            findings,
        }
    }

    pub fn confidence(&self) -> Confidence {
        self.confidence
    }

    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }

    pub fn areas(&self) -> &[Area] {
        &self.areas
    }

    pub fn tests(&self) -> &TestsInChange {
        &self.tests
    }

    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Check the review accounts for the change, refusing every fault by name in one pass.
    ///
    /// `changed` is every file the diff touches; `diff` is its text, where each named test must appear.
    pub fn against(
        &self,
        changed: &[&str],
        diff: &str,
    ) -> Result<AcceptedReview, Vec<ReviewRefused>> {
        let mut refused = Vec::new();
        for path in changed {
            let named = self
                .areas
                .iter()
                .any(|area| area.files().iter().any(|file| file == path));
            if !named {
                refused.push(ReviewRefused::FileInNoArea {
                    path: path.to_string(),
                });
            }
        }
        for test in &self.tests.changed {
            if !diff.contains(test.name.as_str()) {
                refused.push(ReviewRefused::TestNotInDiff {
                    name: test.name.clone(),
                });
            }
        }
        for finding in &self.findings {
            if finding.why().trim().is_empty() {
                refused.push(ReviewRefused::FindingWithNoReason {
                    finding: finding.finding().to_string(),
                });
            }
        }
        let views = self.areas.iter().flat_map(Area::view);
        for step in views.chain(self.findings.iter().flat_map(Finding::view)) {
            if !changed.contains(&step.file.as_str()) || !hunk_in(diff, &step.file, &step.hunk) {
                refused.push(ReviewRefused::ViewStepNotInDiff {
                    file: step.file.clone(),
                    hunk: step.hunk.clone(),
                });
            }
        }
        if refused.is_empty() {
            Ok(AcceptedReview::of(self.clone()))
        } else {
            Err(refused)
        }
    }
}

/// Whether `hunk`, a `@@` header, heads a hunk in `file`'s part of the patch. #904.
///
/// Matched from its start, so a header given without the function name after it still reads.
fn hunk_in(diff: &str, file: &str, hunk: &str) -> bool {
    let hunk = hunk.trim();
    if !hunk.starts_with("@@") {
        return false;
    }
    let file_header = format!(" b/{file}");
    let mut in_file = false;
    for line in diff.lines() {
        if line.starts_with("diff --") {
            in_file = line.ends_with(&file_header);
        } else if in_file && line.starts_with(hunk) {
            return true;
        }
    }
    false
}

/// A review Fleet checked against the change, with what Fleet adds to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedReview {
    review: Review,
    findings: Vec<Finding>,
}

impl AcceptedReview {
    /// A test removed or loosened with no reason needs the person, first.
    fn of(review: Review) -> AcceptedReview {
        let mut findings: Vec<Finding> = review
            .tests
            .changed
            .iter()
            .filter(|test| test.unexplained())
            .map(|test| {
                Finding::of(
                    Bucket::NeedsYou,
                    &format!(
                        "`{}` was {} with no reason given",
                        test.name,
                        test.change.as_wire()
                    ),
                    "A test taken out or weakened is the reviewer's to explain",
                )
            })
            .collect();
        findings.extend(review.findings.iter().cloned());
        AcceptedReview { review, findings }
    }

    pub fn review(&self) -> &Review {
        &self.review
    }

    /// Fleet's findings first, then the reviewer's, in the order written.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// The tests removed or loosened with no reason, in the order the review named them.
    pub fn unexplained_tests(&self) -> Vec<&ChangedTest> {
        self.review
            .tests
            .changed
            .iter()
            .filter(|test| test.unexplained())
            .collect()
    }

    /// The record `store` keeps and `ipc` sends.
    pub fn recorded(&self) -> ReviewRecord {
        ReviewRecord {
            confidence: self.review.confidence,
            reasons: self.review.reasons.clone(),
            areas: self.review.areas.clone(),
            tests: self.review.tests.clone(),
            findings: self.findings.clone(),
            unexplained_tests: self.unexplained_tests().into_iter().cloned().collect(),
        }
    }
}
