//! The three values an `armada.yml` declares, apart from the walk that reads
//! them.
//!
//! **Types here, the walk next door.** `manifest.rs` was 882 lines against a
//! gate that refuses 900, and what separates cleanly is the seam
//! `docs/practices/rust.md` section 6 names: a value with accessors, and the
//! reasoning about what an absent key means on each one, against the document
//! walk that builds one. Nothing here reads YAML and nothing here can fail.
//!
//! Fields are `pub(super)` because the walk above is the only thing that builds
//! these and every reader outside this crate goes through an accessor — which
//! is where what absence means is written down, and the one place it is.

use core_model::{Covers, Narrowing, Prerequisite};

/// A command a change must pass to land or to advance a step.
///
/// **Armada records how to invoke a tool and never what the tool means.** There
/// is no field here for what the command produces, which tests it runs or how
/// its output should be read: a Check is a command and an exit code, and
/// anything needing the output understood is a Judge question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub(super) run: String,
    pub(super) expect_exit_code: i64,
    pub(super) when: Option<Covers>,
    pub(super) requires: Vec<Prerequisite>,
    pub(super) narrow: Option<Narrowing>,
}

impl Check {
    /// The command line, verbatim as the repo wrote it.
    pub fn run(&self) -> &str {
        &self.run
    }

    /// The code this Check exits with when it is satisfied. **Zero where the
    /// file declares none**, which is every Check ever written here.
    ///
    /// **The Check's own fact, and it lives beside the command rather than on
    /// the step that names it.** A Check that legitimately exits non-zero — a
    /// deliberately failing reproduction, a linter whose clean state is `1` —
    /// knows that about itself, and the repository that wrote the command is
    /// the only place that knows it. Written on a step instead, the same number
    /// is restated in every workflow that gates on the Check, and the day the
    /// command changes they go stale one at a time and silently.
    ///
    /// `i64` and signed, for [`crate::yaml::integer`]'s reason: a shell reports
    /// 128 + N for a signal and nothing here normalises that.
    pub fn expect_exit_code(&self) -> i64 {
        self.expect_exit_code
    }

    /// Which paths this Check covers. **`None` where the file declares no
    /// `when`, and that means always** — never "covers nothing".
    ///
    /// An `Option` rather than an empty [`Covers`] because the two would be one
    /// value with opposite meanings, and [`Covers::of`] has no way to build an
    /// empty one for exactly that reason.
    pub fn when(&self) -> Option<&Covers> {
        self.when.as_ref()
    }

    /// The Commands that run before this Check, **in the order the file names
    /// them**, already resolved to their command lines.
    ///
    /// Empty where the file declares no `requires`, which is every Manifest
    /// written before the key existed. `requires: []` is refused rather than
    /// read as empty, for `when`'s reason — a list with nothing in it is a key
    /// to delete.
    pub fn requires(&self) -> &[Prerequisite] {
        &self.requires
    }

    /// How this Check is run against a subset of the tree. **`None` where the
    /// file declares no `narrow`, and that means it runs whole** — never
    /// "narrows to nothing".
    ///
    /// The second question about paths a Check can be asked, and it is not
    /// [`when`](Check::when)'s: `when` decides whether the Check runs at all,
    /// this decides what it reads once it does. `format` covers every path and
    /// still narrows to the Rust ones, so one key could not carry both.
    pub fn narrow(&self) -> Option<&Narrowing> {
        self.narrow.as_ref()
    }
}

/// A command available to run against the repo, gating nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub(super) run: String,
    pub(super) destructive: bool,
}

impl Command {
    pub fn run(&self) -> &str {
        &self.run
    }

    /// Whether a **Drone** invoking this pauses for approval. It does not gate
    /// a person invoking it by hand, who is already the one triggering it.
    ///
    /// Absent means `false`. The common case is a command that is not
    /// destructive, and a required flag on every entry would be noise on the
    /// many to catch the few.
    pub fn is_destructive(&self) -> bool {
        self.destructive
    }
}

/// A Command that has to run in a worktree before any step does, resolved.
///
/// **Name and command line together, because the two answer different
/// questions and a failure needs both.** The name is what the file wrote and
/// what a person edits; the `run` string is what was executed. A failure
/// reporting only the second reads as an install that broke on its own, which
/// is the mystery this whole key exists to end.
///
/// There is no way to build one but by resolving a `setup.requires` entry
/// against a declared Command, so a caller holding one is holding a name the
/// Manifest declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preparation {
    pub(super) name: String,
    pub(super) run: String,
}

impl Preparation {
    /// The Command's name, as `setup.requires` wrote it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The command line, taken from `commands.<name>.run` at load.
    pub fn run(&self) -> &str {
        &self.run
    }
}
