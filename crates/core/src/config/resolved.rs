//! The resolved config: every default materialised, every id derived, every
//! two-spelling key normalised to one.
//!
//! This is what the golden snapshots under `tests/fixtures/*/resolved.json`
//! hold, byte for byte. Two consequences worth knowing before changing
//! anything here:
//!
//! - **Field order is declaration order, and map keys are sorted.** Measured:
//!   `serde_json` writes struct fields in the order they are declared, and
//!   `BTreeMap` writes keys in sorted order — but a `serde_json::Value`
//!   *sorts* object keys, so anything routed through one comes out
//!   alphabetised. Every type here is a struct or a `BTreeMap`, so the
//!   snapshots are stable without a canonicaliser. See `docs/traps.md`.
//! - **Empty containers are omitted; scalar defaults are not.** A snapshot
//!   that spells out `"timeout": 900` makes changing a default a visible diff
//!   in six files, which is the point. A snapshot that spells out
//!   `"secrets": []` on every entry just buries it.

use serde::Serialize;
use std::collections::BTreeMap;

/// A fully resolved `char.yml`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedConfig {
    /// Always 1 in this version of the contract.
    pub version: u32,
    /// Components, keyed by name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub components: BTreeMap<String, ResolvedComponent>,
    /// Repo-local verbs, keyed by name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, ResolvedCommand>,
    /// Declared nested workspaces.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workspaces: Vec<String>,
    /// Secret name → reference. Still a reference; nothing is ever resolved
    /// here, and no value reaches the core at all.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
    /// Provider scheme → the command that prints that kind of secret.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub secret_providers: BTreeMap<String, String>,
}

/// A resolved component.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedComponent {
    /// Where this component's source lives, when it says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// `<root>/**` when `root:` is set, `**` when it is not, and empty when
    /// the component declares no checks — there is then nothing to scope.
    #[serde(rename = "match", skip_serializing_if = "Vec::is_empty")]
    pub match_globs: Vec<String>,
    /// Normalised to a list of steps, each with its shell decision made.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub setup: Vec<ResolvedSetupStep>,
    /// Paths `setup:` created.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_files: Vec<String>,
    /// External resources `setup:` created. Reported, never executed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_release: Vec<String>,
    /// The service this component runs, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<ResolvedRun>,
    /// Checks, keyed by name. Each carries its derived id.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub checks: BTreeMap<String, ResolvedCheck>,
}

/// One resolved `setup:` step.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedSetupStep {
    /// The command.
    pub cmd: String,
    /// Whether it is interpreted by a shell. Default false.
    pub shell: bool,
}

/// A resolved check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedCheck {
    /// `<component>:<check>`, derived. This is the selector an agent uses and
    /// the id `results[]` reports.
    pub id: String,
    /// What to run.
    pub cmd: String,
    /// What to run under `--fix`, when the check declares one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    /// Whether `cmd:` and `fix:` are interpreted by a shell.
    pub shell: bool,
    /// Seconds. Materialised from the machine default when unset.
    pub timeout: u32,
    /// CPU slots, machine-wide.
    pub cost: u32,
    /// `file` or `component`.
    pub scope: Scope,
    /// Machine-wide mutexes, sorted — acquisition order is what makes a
    /// cross-workspace deadlock impossible, so the order is decided here
    /// rather than left to however the config happened to be written.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclusive: Vec<String>,
    /// Prerequisites, classified into components and check ids.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<Need>,
    /// Layered over the inherited environment.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// The compose service this check runs inside, when it does.
    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    pub in_service: Option<String>,
    /// Secret names granted to this check.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
}

/// What a check or service waits for.
///
/// The two forms are told apart by the colon, which is why a component name
/// may never contain one. Classifying at resolve time means nothing downstream
/// re-parses the string and reaches a different answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Need {
    /// A component: the service must be running.
    Component(String),
    /// A check id: that check must have passed in this run.
    Check(String),
}

/// A resolved service.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "driver", rename_all = "lowercase")]
pub enum ResolvedRun {
    /// Resolve → transform → emit, against this file list.
    Compose {
        /// Base plus overrides, in the order compose receives them.
        file: Vec<String>,
        /// The rest of the service definition.
        #[serde(flatten)]
        common: ResolvedService,
    },
    /// Spawned detached in its own process group.
    Command {
        /// What to spawn.
        cmd: String,
        /// How to stop it, when killpg is not what the repo wants.
        #[serde(skip_serializing_if = "Option::is_none")]
        stop: Option<String>,
        /// Whether `cmd:` and `stop:` are interpreted by a shell.
        shell: bool,
        /// The rest of the service definition.
        #[serde(flatten)]
        common: ResolvedService,
    },
}

/// The parts of a service that do not depend on the driver.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedService {
    /// Port name → the port the service itself listens on.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub ports: BTreeMap<String, u16>,
    /// Always populated: an omitted `ready:` is `{none: true}`.
    pub ready: ResolvedReady,
    /// Components that must be running first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<Need>,
    /// Layered over the inherited environment.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Docker selector for containers this service created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owns_containers: Option<String>,
    /// Docker selector for networks this service created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owns_networks: Option<String>,
    /// Docker selector for images this service caused to be built.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owns_images: Option<String>,
    /// Declared port names this service holds.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_ports: Vec<String>,
    /// Paths this service created.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_files: Vec<String>,
    /// External resources. Reported, never executed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_release: Vec<String>,
    /// Secret names granted to this service.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
}

/// A resolved ready-check: exactly one kind, and a deadline.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedReady {
    /// Which kind, and its argument.
    pub kind: ReadyKind,
    /// Seconds.
    pub timeout: u32,
}

/// The five ready-check kinds. This is the enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadyKind {
    /// Ready when a GET returns 2xx.
    Http(String),
    /// Ready when the named port accepts a connection.
    Tcp(String),
    /// Ready when this regex matches the service's stdout.
    Log(String),
    /// Ready when this command exits 0.
    Exec(String),
    /// Ready immediately on spawn.
    None,
}

/// A resolved `commands:` entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedCommand {
    /// The command. Remaining argv passes through to it untouched.
    pub cmd: String,
    /// One line of help, when the entry gives one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Whether `cmd:` is interpreted by a shell.
    pub shell: bool,
    /// Materialised: `pipe` when the entry grants secrets, `inherit`
    /// otherwise, unless the entry says. `--json` overrides it at run time.
    pub stdio: Stdio,
    /// Layered over the inherited environment.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Docker selector, evaluated at `clean` time rather than recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owns_containers: Option<String>,
    /// Docker selector, evaluated at `clean` time rather than recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owns_networks: Option<String>,
    /// Docker selector, evaluated at `clean` time rather than recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owns_images: Option<String>,
    /// Paths this command creates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_files: Vec<String>,
    /// External resources. Reported, never executed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns_release: Vec<String>,
    /// Secret names granted to this entry.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
}

/// What a check runs over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// The changed files, passed as `${files}`.
    File,
    /// The whole component, always.
    Component,
}

/// What a dispatched command's stdio is wired to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stdio {
    /// The child keeps char's terminal. char sees nothing and scrubs nothing.
    Inherit,
    /// char sees the output, and can scrub it.
    Pipe,
}
