//! The parts of a review, and the record of one Fleet accepted. #903.
//!
//! Here rather than in `verification` so `store` and `ipc` read the same record it checks.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Whether Armada is confident in the change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    Confident,
    NotConfident,
}

impl Confidence {
    pub fn as_wire(&self) -> &'static str {
        match self {
            Confidence::Confident => "confident",
            Confidence::NotConfident => "not_confident",
        }
    }

    pub fn from_wire(value: &str) -> Option<Confidence> {
        match value {
            "confident" => Some(Confidence::Confident),
            "not_confident" => Some(Confidence::NotConfident),
            _ => None,
        }
    }
}

/// Where a finding goes: to the person, to a Drone, or alongside for context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bucket {
    NeedsYou,
    SmallFix,
    ForContext,
}

impl Bucket {
    pub fn as_wire(&self) -> &'static str {
        match self {
            Bucket::NeedsYou => "needs_you",
            Bucket::SmallFix => "small_fix",
            Bucket::ForContext => "for_context",
        }
    }

    pub fn from_wire(value: &str) -> Option<Bucket> {
        match value {
            "needs_you" => Some(Bucket::NeedsYou),
            "small_fix" => Some(Bucket::SmallFix),
            "for_context" => Some(Bucket::ForContext),
            _ => None,
        }
    }
}

/// One part of the change, the files in it, and what changed there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Area {
    name: String,
    what: String,
    files: Vec<String>,
}

impl Area {
    pub fn of(name: &str, what: &str, files: &[&str]) -> Area {
        Area {
            name: name.to_string(),
            what: what.to_string(),
            files: files.iter().map(|file| file.to_string()).collect(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn what(&self) -> &str {
        &self.what
    }

    pub fn files(&self) -> &[String] {
        &self.files
    }
}

/// What the tests in one area prove, and how many there are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proves {
    area: String,
    what: String,
    tests: u32,
}

impl Proves {
    pub fn of(area: &str, what: &str, tests: u32) -> Proves {
        Proves {
            area: area.to_string(),
            what: what.to_string(),
            tests,
        }
    }

    pub fn area(&self) -> &str {
        &self.area
    }

    pub fn what(&self) -> &str {
        &self.what
    }

    pub fn tests(&self) -> u32 {
        self.tests
    }
}

/// What happened to a test that was already there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestChange {
    Removed { replaced_by: Option<String> },
    Loosened,
}

impl TestChange {
    pub fn as_wire(&self) -> &'static str {
        match self {
            TestChange::Removed { .. } => "removed",
            TestChange::Loosened => "loosened",
        }
    }

    /// `replaced_by` is kept only on a removal; a loosened test was not replaced.
    pub fn from_wire(value: &str, replaced_by: Option<String>) -> Option<TestChange> {
        match value {
            "removed" => Some(TestChange::Removed { replaced_by }),
            "loosened" => Some(TestChange::Loosened),
            _ => None,
        }
    }
}

/// A test the change removed or loosened, and the reviewer's reason, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedTest {
    pub name: String,
    pub change: TestChange,
    pub why: Option<String>,
}

impl ChangedTest {
    /// No reason, or only whitespace, which a person would read as none.
    pub fn unexplained(&self) -> bool {
        self.why
            .as_deref()
            .map_or(true, |why| why.trim().is_empty())
    }
}

/// Changed code no test reaches, and why that matters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Untested {
    code: String,
    why: String,
}

impl Untested {
    pub fn of(code: &str, why: &str) -> Untested {
        Untested {
            code: code.to_string(),
            why: why.to_string(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn why(&self) -> &str {
        &self.why
    }
}

/// The tests in the change: what they prove, which were touched, and what none reaches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestsInChange {
    pub proves: Vec<Proves>,
    pub changed: Vec<ChangedTest>,
    pub untested: Vec<Untested>,
}

impl TestsInChange {
    pub fn is_empty(&self) -> bool {
        self.proves.is_empty() && self.changed.is_empty() && self.untested.is_empty()
    }
}

/// Something the reviewer found, and why it is there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    bucket: Bucket,
    finding: String,
    why: String,
}

impl Finding {
    pub fn of(bucket: Bucket, finding: &str, why: &str) -> Finding {
        Finding {
            bucket,
            finding: finding.to_string(),
            why: why.to_string(),
        }
    }

    pub fn bucket(&self) -> Bucket {
        self.bucket
    }

    pub fn finding(&self) -> &str {
        &self.finding
    }

    pub fn why(&self) -> &str {
        &self.why
    }
}

/// A review as Fleet accepted it: Fleet's findings first, and the tests given no reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewRecord {
    pub confidence: Confidence,
    pub reasons: Vec<String>,
    pub areas: Vec<Area>,
    pub tests: TestsInChange,
    pub findings: Vec<Finding>,
    pub unexplained_tests: Vec<ChangedTest>,
}
