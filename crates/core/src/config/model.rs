//! The structs `armada.yml` deserializes into.
//!
//! These mirror `crates/core/schema/armada.schema.json`, which is the
//! authoritative statement of the contract. Everything here is
//! `deny_unknown_fields`, so a key the schema does not know is a parse error
//! rather than a silently ignored line — the schema's `additionalProperties:
//! false` and this attribute are the same rule enforced at two entry points,
//! and a fixture that uses a key only one of them knows fails the suite.
//!
//! What these structs deliberately do **not** enforce is anything that needs a
//! second value: a `driver: compose` with no `file:`, a `ready:` with two kind
//! keys. Those are caught in resolution, where the error can carry a key path,
//! and by the schema. Deserialization stays a shape check.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;

/// Deserialize an `env:` block, rejecting values YAML would coerce.
///
/// **Measured: `serde_yaml_ng` deserializes an unquoted scalar straight into a
/// `String`** — `3000` becomes `"3000"`, `true` becomes `"true"`, and `null`
/// becomes the four-character string `"null"` (`docs/traps.md`). The schema
/// says an env value is a string, so without this the two entry points
/// disagree: `DEBUG: null` would load cleanly here, fail `config verify`, and
/// in between put the literal text `null` in a child's environment.
///
/// A newtype with a `visit_str`-only visitor does **not** close it, also
/// measured — the YAML deserializer hands a plain scalar to `visit_str`
/// whatever it looks like. Inspecting the parsed value is the only thing that
/// does.
fn env_block<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: BTreeMap<String, serde_yaml_ng::Value> = BTreeMap::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(name, value)| match value {
            serde_yaml_ng::Value::String(text) => Ok((name, text)),
            other => Err(D::Error::custom(format!(
                "env value for `{name}` is {} — quote it, env values are strings",
                yaml_kind(&other)
            ))),
        })
        .collect()
}

fn yaml_kind(value: &serde_yaml_ng::Value) -> &'static str {
    match value {
        serde_yaml_ng::Value::Null => "null",
        serde_yaml_ng::Value::Bool(_) => "a boolean",
        serde_yaml_ng::Value::Number(_) => "a number",
        serde_yaml_ng::Value::String(_) => "a string",
        serde_yaml_ng::Value::Sequence(_) => "a list",
        serde_yaml_ng::Value::Mapping(_) => "a mapping",
        serde_yaml_ng::Value::Tagged(_) => "a tagged value",
    }
}

/// A parsed `armada.yml` — one section per module.
///
/// **Only `manifest:` exists, and the nesting is there anyway.** Guild is
/// machine-global and never repository content (`ARCHITECTURE.md` §1.9), so it
/// will never appear here; Fleet's repo-level defaults might. The file is
/// nested from the start because the alternative is moving every key in every
/// config in the world the first time a second section is needed, which is the
/// migration this milestone exists to do once rather than twice
/// (`PHASES.md` §8.3).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// What a workspace is and how to operate it.
    pub manifest: Config,
}

/// The `manifest:` section of an `armada.yml`, before defaults and derivation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Required. A config with no version is `bad_config`, since the whole
    /// point of the key is to exist before it is needed.
    pub version: u32,
    /// Named things that may have source to check, a process to run, or both.
    #[serde(default)]
    pub components: BTreeMap<String, Component>,
    /// Repo-local verbs Armada dispatches but does not define.
    #[serde(default)]
    pub commands: BTreeMap<String, CommandEntry>,
    /// Repo-local knowledge, rendered per harness. Armada holds the mechanical
    /// half and a path; it never parses the prose (ARCHITECTURE.md §1.9).
    #[serde(default)]
    pub skills: BTreeMap<String, SkillEntry>,
    /// Paths that are separate workspaces, excluded from this one.
    #[serde(default)]
    pub workspaces: Vec<String>,
    /// Environment variable name → secret reference. A pointer, never a value.
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
    /// URI scheme → the command that prints that kind of secret to stdout.
    #[serde(default)]
    pub secret_providers: BTreeMap<String, SecretProvider>,
}

/// A component: source to check (`checks:`), a process to run (`run:`), or
/// both. Two axes, not two kinds of thing.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    /// Where this component's source lives. Not a working directory.
    #[serde(default)]
    pub root: Option<String>,
    /// Globs, workspace-relative, deciding which changed files select this
    /// component.
    #[serde(default, rename = "match")]
    pub match_globs: Option<Vec<String>>,
    /// What `armada manifest init` runs.
    #[serde(default)]
    pub setup: Option<OneOrMany<SetupStep>>,
    /// What `setup:` created.
    #[serde(default)]
    pub owns: Option<OwnsComponent>,
    /// The process this component runs, if any.
    #[serde(default)]
    pub run: Option<Run>,
    /// The checks this component declares, if any.
    #[serde(default)]
    pub checks: BTreeMap<String, Check>,
}

/// One check. Its id is derived as `<component>:<check>` and never written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// What to run.
    pub cmd: String,
    /// What to run instead under `--fix`.
    #[serde(default)]
    pub fix: Option<String>,
    /// Opt this entry into shell interpretation. Covers `cmd:` and `fix:`.
    #[serde(default)]
    pub shell: Option<bool>,
    /// Seconds. Defaults to the machine's `check_timeout`.
    #[serde(default)]
    pub timeout: Option<u32>,
    /// CPU slots, machine-wide. Default 1.
    #[serde(default)]
    pub cost: Option<u32>,
    /// `file` (default) or `component`.
    #[serde(default)]
    pub scope: Option<String>,
    /// Machine-wide mutexes.
    #[serde(default)]
    pub exclusive: Vec<String>,
    /// Components that must be running, and check ids that must have passed.
    #[serde(default)]
    pub needs: Vec<String>,
    /// Layered over the inherited environment.
    #[serde(default, deserialize_with = "env_block")]
    pub env: BTreeMap<String, String>,
    /// Run inside this compose *service*, via an exec Armada builds itself.
    #[serde(default, rename = "in")]
    pub in_service: Option<String>,
    /// Secret names granted to this entry and nowhere else.
    #[serde(default)]
    pub secrets: Vec<String>,
}

/// A service definition. The `driver:` decides which of the other keys are
/// legal; resolution is where that is decided, so the error can say where.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// `compose` or `command`.
    pub driver: String,
    /// compose only. Base plus override, in the order compose receives them.
    #[serde(default)]
    pub file: Option<Vec<String>>,
    /// command only.
    #[serde(default)]
    pub cmd: Option<String>,
    /// command only. Default is killpg — TERM, grace, KILL.
    #[serde(default)]
    pub stop: Option<String>,
    /// command only.
    #[serde(default)]
    pub shell: Option<bool>,
    /// Port name → the port the service itself listens on.
    #[serde(default)]
    pub ports: BTreeMap<String, u16>,
    /// How Armada decides this service is ready.
    #[serde(default)]
    pub ready: Option<Ready>,
    /// Components that must be running first.
    #[serde(default)]
    pub needs: Vec<String>,
    /// Layered over the inherited environment.
    #[serde(default, deserialize_with = "env_block")]
    pub env: BTreeMap<String, String>,
    /// What this service created.
    #[serde(default)]
    pub owns: Option<OwnsRun>,
    /// Secret names granted to this entry and nowhere else.
    #[serde(default)]
    pub secrets: Vec<String>,
}

/// A ready-check: exactly one kind key, plus an optional timeout.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ready {
    /// Ready when a GET returns 2xx.
    #[serde(default)]
    pub http: Option<String>,
    /// Ready when the named port accepts a connection.
    #[serde(default)]
    pub tcp: Option<String>,
    /// Ready when this regex matches the service's stdout.
    #[serde(default)]
    pub log: Option<String>,
    /// Ready when this command exits 0.
    #[serde(default)]
    pub exec: Option<String>,
    /// Ready immediately on spawn.
    #[serde(default)]
    pub none: Option<bool>,
    /// Seconds. Default 60.
    #[serde(default)]
    pub timeout: Option<u32>,
}

/// A repo-local verb Armada dispatches and does not define.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEntry {
    /// The command. Remaining argv passes through to it untouched.
    pub cmd: String,
    /// One line, for `--help` and the managed AGENTS.md block.
    #[serde(default)]
    pub help: Option<String>,
    /// Opt this entry into shell interpretation.
    #[serde(default)]
    pub shell: Option<bool>,
    /// Layered over the inherited environment.
    #[serde(default, deserialize_with = "env_block")]
    pub env: BTreeMap<String, String>,
    /// `inherit` or `pipe`. Inferred when absent.
    #[serde(default)]
    pub stdio: Option<String>,
    /// A selector Armada evaluates at `clean` time, not a record.
    #[serde(default)]
    pub owns: Option<OwnsCommand>,
    /// Secret names granted to this entry and nowhere else.
    #[serde(default)]
    pub secrets: Vec<String>,
}

/// A named grant plus a pointer to prose (PLAN.md §4.8).
///
/// **Deliberately has no `cmd:`.** A skill with its own command is a
/// `commands:` entry wearing a hat, and if that were the design the honest move
/// would be to delete `skills:` entirely. A skill is a name, a grant and a
/// document; execution stays with the thing that already executes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillEntry {
    /// One line, for listings, MCP responses and generated frontmatter.
    pub summary: String,
    /// Workspace-relative path to the prose. Existence is verified; the
    /// contents are never read by anything in core.
    pub doc: String,
    /// `commands:` keys this skill may invoke. Every name must already be
    /// declared — `config verify` rejects one that is not, which is what stops
    /// a skill smuggling in capability the repository never granted.
    #[serde(default)]
    pub uses: Vec<String>,
    /// The check scope that proves the work landed. Feeds the verdict evidence
    /// rule of PLAN.md §14.3.
    #[serde(default)]
    pub verify: Option<SkillVerify>,
    /// Advisory globs. Not enforced; used for scoping and review.
    #[serde(default)]
    pub touches: Vec<String>,
}

/// What proves a skill's work landed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillVerify {
    /// Check ids or `component:check` selectors, as `check --scope` accepts.
    pub check: Vec<String>,
}

/// A secret provider: a command that prints one secret to stdout.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretProvider {
    /// Must contain `${ref}`, which is substituted as a single argv element
    /// and never through a shell. There is no `shell:` key here.
    pub cmd: String,
}

/// What `run:` created.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnsRun {
    /// A docker filter expression.
    #[serde(default)]
    pub containers: Option<String>,
    /// A docker filter expression.
    #[serde(default)]
    pub networks: Option<String>,
    /// A docker filter expression, for images Armada caused to be built.
    #[serde(default)]
    pub images: Option<String>,
    /// Declared port names this service holds.
    #[serde(default)]
    pub ports: Vec<String>,
    /// Paths removed only by `clean --artifacts`.
    #[serde(default)]
    pub files: Vec<String>,
    /// External resources: recorded, reported, never executed.
    #[serde(default)]
    pub release: Option<OneOrMany<String>>,
}

/// What `setup:` created. No runtime handles, so no containers or ports.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnsComponent {
    /// Paths removed only by `clean --artifacts`.
    #[serde(default)]
    pub files: Vec<String>,
    /// External resources: recorded, reported, never executed.
    #[serde(default)]
    pub release: Option<OneOrMany<String>>,
}

/// What a `commands:` entry creates. A selector, evaluated at `clean` time.
/// No `ports:` — the block is already claimed by `armada manifest init`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnsCommand {
    /// A docker filter expression.
    #[serde(default)]
    pub containers: Option<String>,
    /// A docker filter expression.
    #[serde(default)]
    pub networks: Option<String>,
    /// A docker filter expression.
    #[serde(default)]
    pub images: Option<String>,
    /// Paths removed only by `clean --artifacts`.
    #[serde(default)]
    pub files: Vec<String>,
    /// External resources: recorded, reported, never executed.
    #[serde(default)]
    pub release: Option<OneOrMany<String>>,
}

/// One `setup:` step: a bare command, or an object that can opt into a shell.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SetupStep {
    /// `- bundle install`
    Plain(String),
    /// `- { cmd: "...", shell: true }`
    Object(SetupObject),
}

/// The object spelling of a `setup:` step.
///
/// A separate struct rather than an enum variant so that `deny_unknown_fields`
/// applies: serde has no variant-level form of it, and without it a typo'd key
/// inside a step is silently dropped by the parser while the schema rejects it
/// — the two entry points would then disagree about the same document.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupObject {
    /// The command.
    pub cmd: String,
    /// Whether to interpret it with a shell.
    #[serde(default)]
    pub shell: Option<bool>,
}

/// A key whose value may be written as one item or as a list of them.
///
/// Used only where PLAN.md's own examples show both spellings (`setup:`,
/// `owns.release:`). Resolution normalises it to a list, so nothing downstream
/// ever asks which form was written. Keys where only the list form appears —
/// `file:`, `match:`, `needs:` — stay lists, because a second accepted
/// spelling that no example uses is surface with no demand behind it.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// The single-item spelling.
    One(T),
    /// The list spelling.
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    /// Normalise to a list.
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(item) => vec![item],
            OneOrMany::Many(items) => items,
        }
    }
}
