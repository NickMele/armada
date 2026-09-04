//! The boundaries nothing lifts, and the reason each one is one.
//!
//! # Why this is a constant and not a key
//!
//! A step's `exclude_paths` is the *other* tier — a boundary somebody drew
//! before anybody read the code, which a Judge may lift. This list is what no
//! answer reaches, so it must not be stated anywhere a Drone can edit: every
//! entry lives inside the repository a Drone has a worktree of, and a list
//! naming them from inside it could be edited by the thing it denies.
//!
//! # What earns a place, and what does not
//!
//! Secrets, and what decides how the work is checked and judged.
//! `core_model::GamingPattern::CheckConfigEdited` is the argument: a model will
//! honour a frozen `run:` string exactly while narrowing what it runs.
//!
//! **`Cargo.toml`, `package.json` and a Makefile are not here**, though that
//! same check flags all three. Ordinary work edits them constantly, and a
//! boundary that stopped a Job adding a dependency would refuse the work
//! rather than protect anything. There a Judge reads a flag; here nothing is
//! asked. `#417` names the two gaps: a forge's CI directory, which
//! `no_vendor_literal_outside_adapters` keeps out of this crate, and a
//! repository's own entry, which needs a workflow schema key.

use core_model::{under, RepoPath};

/// How a boundary's name matches a path segment.
///
/// **Two rules, and the second exists for one entry.** `.env` is a family —
/// `.env`, `.env.local`, `.env.production` — where `.git` is not: `.gitignore`
/// and `.gitattributes` are ordinary files that ordinary work edits, and a
/// prefix rule applied to `.git` would refuse them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    /// The segment is exactly this.
    Exactly,
    /// The segment is this, or begins with it.
    OrAnythingAfterIt,
}

/// One boundary nothing lifts.
struct Boundary {
    /// Matched against every segment of a repository-relative path.
    name: &'static str,
    reach: Reach,
    /// Directories inside this boundary that it does not reach.
    ///
    /// **Not an exception a caller can supply** — it is compiled in beside the
    /// boundary it narrows, and no signature in this module carries it. What it
    /// says is what the boundary *is*, not what somebody may do about it, and
    /// [`BOUNDARIES`] holds one entry that needs one.
    ///
    /// The test is the one this file already applies to `Cargo.toml`: a
    /// boundary that stops the work Armada itself directs a Drone to do is
    /// refusing the work rather than protecting anything.
    but_not: &'static [&'static str],
    /// Why nothing lifts it. **Reaches the Drone and the person**, so it says
    /// what the file is rather than that a rule exists.
    because: &'static str,
}

/// The whole of it. **Four entries, and each one is a thing that decides what
/// is true about the work rather than a thing the work is about.**
const BOUNDARIES: &[Boundary] = &[
    Boundary {
        name: ".env",
        reach: Reach::OrAnythingAfterIt,
        but_not: &[],
        because: "it holds secrets",
    },
    Boundary {
        name: ".git",
        reach: Reach::Exactly,
        but_not: &[],
        because: "it holds the repository's own machinery, including the hooks \
                  a check runs through",
    },
    Boundary {
        // **`.armada/artifacts/` is the one directory under here a Drone is
        // told to write to**, by `mechanical_checks[].target` in seven of the
        // shipped workflows, and Fleet opens exactly that path at the gate to
        // put in the Judge's brief. The boundary without this exception refuses
        // every deliverable step in the fleet.
        //
        // It did not bite until `#431` because this repository's `.gitignore`
        // holds `.armada/*`, so the artifact never enters a diff and the gate
        // never saw it. A repository that does not ignore it would have met
        // this the first time a `plan` step submitted — which is the reason the
        // exception is stated here rather than left to every repository's
        // ignore file to imply.
        name: ".armada",
        reach: Reach::Exactly,
        but_not: &[".armada/artifacts"],
        because: "it holds the workflow definitions, which say what this step is \
                  judged by",
    },
    Boundary {
        name: "armada.yml",
        reach: Reach::Exactly,
        but_not: &[],
        because: "it is the manifest of checks this work is measured by",
    },
];

/// A path under a boundary nothing lifts, carrying why.
///
/// **The reason travels with the path** rather than being looked up again by
/// whoever renders the refusal. A Drone told only that a path is refused asks
/// again in other words; a Drone told the file holds secrets does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forbidden {
    path: RepoPath,
    because: &'static str,
}

impl Forbidden {
    pub fn path(&self) -> &RepoPath {
        &self.path
    }

    /// Why nothing lifts it, as a clause that follows the path.
    pub fn because(&self) -> &'static str {
        self.because
    }
}

impl core::fmt::Display for Forbidden {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "`{}` — {}", self.path.as_str(), self.because)
    }
}

/// Whether this path falls under a boundary nothing lifts.
///
/// **There is no argument that turns it off.** No scope, no workflow and no
/// verdict is in this signature, which is the whole property: a caller holding
/// a path and wanting to know cannot supply anything that changes the answer.
pub fn forbidden(path: &RepoPath) -> Option<Forbidden> {
    let path = path.as_str();
    BOUNDARIES
        .iter()
        .find(|boundary| {
            !boundary
                .but_not
                .iter()
                .any(|sanctioned| under(sanctioned, path))
                && path
                    .split('/')
                    .filter(|segment| !segment.is_empty())
                    .any(|segment| match boundary.reach {
                        Reach::Exactly => segment == boundary.name,
                        Reach::OrAnythingAfterIt => segment.starts_with(boundary.name),
                    })
        })
        .map(|boundary| Forbidden {
            path: RepoPath::new(path),
            because: boundary.because,
        })
}

/// Every path in `paths` that falls under one, in the order they were read.
///
/// Empty is the ordinary answer, and the caller reads it as "nothing here is
/// absolute" rather than as "there was nothing to check".
pub fn forbidden_among<'a>(paths: impl IntoIterator<Item = &'a RepoPath>) -> Vec<Forbidden> {
    paths.into_iter().filter_map(forbidden).collect()
}

/// The same answer over a footprint alone — the paths Fleet read out of the
/// worktree, with no declaration beside them.
///
/// **This is the half a step that declared nothing can still be asked.**
/// `InScope::resolved` answers the absolute tier over the declaration and the
/// footprint together, which needs a declaration to have been made;
/// `#431` is what a step with none is owed, and it is this. Same
/// [`forbidden`] underneath, so there is one list and one predicate.
pub fn out_of_bounds(changed: &[String]) -> Vec<Forbidden> {
    changed
        .iter()
        .filter_map(|path| forbidden(&RepoPath::new(path)))
        .collect()
}

/// What a refusal says when a path is under one of these, **written once**.
///
/// Both tiers of caller render it: the resolution that found it beside a
/// declaration, and the floor that found it in a footprint with no declaration
/// at all. Two spellings of this sentence would be the absolute refusal reading
/// as two different rules depending on which door the path came through.
pub(crate) fn reaches(paths: &[Forbidden]) -> String {
    let mut said = String::from("the step reaches ");
    for (n, found) in paths.iter().enumerate() {
        if n > 0 {
            said.push_str(", ");
        }
        said.push_str(&found.to_string());
    }
    said.push_str(", which nothing here can allow");
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(path: &str) -> Option<Forbidden> {
        forbidden(&RepoPath::new(path))
    }

    #[test]
    fn a_dotenv_family_member_anywhere_in_the_tree_is_absolute() {
        assert!(at(".env").is_some());
        assert!(at(".env.production").is_some());
        assert!(at("apps/desktop/.env.local").is_some());
    }

    #[test]
    fn the_files_that_say_what_runs_and_what_judges_are_absolute() {
        assert!(at("armada.yml").is_some());
        assert!(at(".armada/workflows/bug.json").is_some());
        assert!(at(".git/hooks/pre-commit").is_some());
    }

    /// The prefix rule is `.env`'s alone. A change that adds a build directory
    /// to `.gitignore` is ordinary work, and a boundary that refused it would
    /// be refusing the work rather than protecting anything.
    #[test]
    fn a_file_that_merely_begins_like_git_is_ordinary() {
        assert_eq!(at(".gitignore"), None);
        assert_eq!(at(".gitattributes"), None);
        assert_eq!(at("crates/config/src/environment.rs"), None);
    }

    /// The gaming check flags an edit to these and a Judge reads the flag.
    /// Refusing them outright would stop a Job adding a dependency, which is
    /// the ordinary case rather than the adversarial one.
    #[test]
    fn the_build_configuration_the_gaming_check_flags_is_not_absolute() {
        assert_eq!(at("Cargo.toml"), None);
        assert_eq!(at("package.json"), None);
        assert_eq!(at("Makefile"), None);
    }

    /// The directory the workflows send a deliverable to is the work, not the
    /// machinery. A boundary that refused it would fail every `plan`, `scope`,
    /// `draft`, `read`, `assess` and `roll_up` step in the fleet.
    #[test]
    fn the_directory_a_drone_is_told_to_write_its_deliverable_to_is_ordinary() {
        assert_eq!(at(".armada/artifacts/plan.md"), None);
        assert_eq!(at(".armada/artifacts/roll-up.md"), None);
        assert!(at(".armada/artifacts").is_none());
    }

    /// The exception is that directory and nothing beside it. A Job's own log
    /// and a Drone's scratch directory are records of the run, and a step that
    /// edits the record of itself is what the boundary is for.
    #[test]
    fn the_rest_of_the_armada_directory_is_still_absolute() {
        assert!(at(".armada/workflows/bug.json").is_some());
        assert!(at(".armada/artifacts-of-mine/plan.md").is_some());
        assert!(at(".armada/some-job-id/notes.md").is_some());
    }

    #[test]
    fn a_footprint_is_answered_without_a_declaration_beside_it() {
        let changed = vec![
            String::from("crates/fleet/src/gate.rs"),
            String::from(".env.local"),
        ];
        let found = out_of_bounds(&changed);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path(), &RepoPath::new(".env.local"));
        assert!(out_of_bounds(&[String::from("docs/plan.md")]).is_empty());
    }

    #[test]
    fn the_reason_travels_with_the_path() {
        let found = at("apps/desktop/.env.local").expect("a boundary");
        assert_eq!(found.path(), &RepoPath::new("apps/desktop/.env.local"));
        assert_eq!(found.because(), "it holds secrets");
        assert_eq!(
            found.to_string(),
            "`apps/desktop/.env.local` — it holds secrets"
        );
    }

    #[test]
    fn nothing_absolute_in_a_list_is_an_empty_answer() {
        let paths = [RepoPath::new("src/lib.rs"), RepoPath::new("docs/plan.md")];
        assert!(forbidden_among(paths.iter()).is_empty());
    }
}
