//! What Scan found in a repository, before anybody has written an `armada.yml`
//! for it.
//!
//! **Evidence, never a proposal.** Every value here is copied out of a file the
//! repository already had, and each carries that file, because every line of
//! the proposal built on it (#823) must cite one. Which script gates code is a
//! guess, and nothing here makes it: a `test` script is a script named `test`.
//!
//! **What was not read is said, beside what was.** A tool this read has never
//! heard of reads as *not followed*, never as clean — drift's rule, carried
//! over — and a repository that could not be read at all says why in
//! [`RepositoryScan::not_read`] rather than answering with no workspaces.
//!
//! **A port only where a file declares one.** Finding none is the common case,
//! and an empty list is the true answer for it.

use serde::{Deserialize, Serialize};

/// One read-only pass over a checkout, every workspace at once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryScan {
    /// The directory that was read, as Fleet resolved it.
    pub checkout: String,
    /// Every workspace found, the root first and the rest by path. **The root
    /// is always here**: a single-workspace repository's Manifest is the root,
    /// and a monorepo's root holds its own commands.
    pub workspaces: Vec<ScannedWorkspace>,
    /// What the repository's CI jobs run, each with its file and job. **Evidence,
    /// not a Check**: which of these gates code is still the proposal's guess.
    #[serde(default)]
    pub ci_commands: Vec<CiCommand>,
    /// What belongs to no workspace and was not read, and why — a directory
    /// that would not list, a workspace pattern this read does not expand, a
    /// YAML file under a hidden directory. Never evidence of absence.
    pub not_read: Vec<NotRead>,
}

/// One directory that can hold a Manifest of its own, and what its files say.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedWorkspace {
    /// Relative to the checkout, `.` for the root.
    pub dir: String,
    /// The workspace-pattern entries that name this directory. **Empty where
    /// it was found only by holding a manifest of its own**, which is how a
    /// repository of separate products with no workspace file is still read.
    pub declared_by: Vec<WorkspaceGlob>,
    /// How strong this workspace's evidence is — what the picker ticks by.
    pub evidence: EvidenceStrength,
    /// The files that make the directory a package, with the tool that reads
    /// each — `package.json` and `pnpm`'s kin, `Cargo.toml`, `pyproject.toml`.
    pub manifests: Vec<ToolFile>,
    /// Lockfiles, each with the tool that writes it. A shared lockfile is
    /// cited where it sits, and not copied into every member.
    pub lockfiles: Vec<ToolFile>,
    /// What a file names as runnable, by name: `package.json` scripts and
    /// cargo aliases. **Where the file writes them, not what gates code.**
    pub runnables: Vec<Runnable>,
    /// The `[tool.*]` sections a `pyproject.toml` configures.
    pub tools: Vec<ToolSection>,
    /// The services a compose file declares.
    pub services: Vec<ComposeService>,
    /// Ports a file declares. Empty is the ordinary answer.
    pub ports: Vec<DeclaredPort>,
    /// A runnable name every strong sibling declares and this one does not —
    /// the grid's narrow mark, over the batch the picker ticks by default.
    /// Empty for the root, which is not a sibling of the workspaces under it.
    pub missing: Vec<MissingName>,
    /// What this workspace holds that was not read, and why.
    pub not_read: Vec<NotRead>,
}

/// How much a workspace's files say, in the three readings a picker needs.
///
/// **Only a file naming something runnable is strong.** A `Cargo.toml` with no
/// alias names nothing to run, and that `cargo test` would work is convention,
/// which is Proposal's to say and to label as such.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    /// A file here names at least one runnable — a script or an alias.
    Strong,
    /// Files here were read, and none names anything runnable.
    Thin,
    /// Nothing here was readable by this Scan — only files it does not
    /// follow, or files that would not read. **Never clean.**
    NotFollowed,
}

/// One entry of a workspace pattern that names a directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceGlob {
    /// The file the pattern is written in.
    pub file: String,
    /// The entry, verbatim — `apps/*`.
    pub entry: String,
}

/// A file, and the tool it belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFile {
    /// Relative to the checkout.
    pub file: String,
    /// The tool, as a person would name it. **Rendered, never matched on.**
    pub tool: String,
}

/// Something a file names as runnable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runnable {
    /// Relative to the checkout.
    pub file: String,
    /// Where in the file — `scripts.test`, `alias.xtask`.
    pub key: String,
    /// The name it is run by.
    pub name: String,
    /// What it runs, verbatim as the file writes it.
    pub run: String,
}

/// One `[tool.*]` section of a `pyproject.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSection {
    pub file: String,
    /// `tool.pytest`.
    pub key: String,
    /// `pytest`.
    pub tool: String,
}

/// One service a compose file declares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeService {
    pub file: String,
    /// `services.db`.
    pub key: String,
    pub name: String,
}

/// A port a file declares, and the container side of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredPort {
    pub file: String,
    /// `services.web.ports[0]`, `PORT`, `scripts.dev`.
    pub key: String,
    /// What a proposal would name it after — the service, the variable, or
    /// the script.
    pub name: String,
    /// The port inside the container, or the one the process listens on.
    pub container: u16,
}

/// A name every sibling declares, where this workspace does not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingName {
    pub name: String,
    /// Every sibling that declares it, by directory.
    pub declared_in: Vec<String>,
}

/// One command a CI job runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiCommand {
    /// Relative to the checkout.
    pub file: String,
    /// The job, by its id in that file.
    pub job: String,
    /// Where in the file — `jobs.test.steps[2].run`.
    pub key: String,
    /// Verbatim as the file writes it.
    pub run: String,
    /// The matrix cell it was read as, where the job has one: one finding per
    /// command, never one per cell. Rendered, never matched on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
}

/// Something that was not read, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotRead {
    /// Relative to the checkout.
    pub file: String,
    /// As a person would read it. **Rendered, never matched on.**
    pub why: String,
}

/// The part of a `package.json` Scan uses to find workspaces: its
/// `workspaces` key.
///
/// **Not a wire message. A file shape, decoded here because this is the door**
/// — [`PackageScripts`](crate::PackageScripts)'s reason. Scripts are read
/// through that type and never through this one, so Scan and drift cannot come
/// to disagree about what a script is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PackageWorkspaces {
    #[serde(default)]
    pub workspaces: Option<WorkspaceGlobs>,
}

/// Both forms a `package.json` writes its workspaces in.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum WorkspaceGlobs {
    /// `"workspaces": ["apps/*"]`.
    Listed(Vec<String>),
    /// `"workspaces": { "packages": ["apps/*"] }`.
    Nested {
        #[serde(default)]
        packages: Vec<String>,
    },
}
