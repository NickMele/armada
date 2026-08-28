//! Which binary a Drone is started as, and which model each caller runs on.
//!
//! **One reader, because they are one missing piece.** Each was "nothing reads
//! configuration yet", and each is answered the same way — a default the
//! adapter states, an environment variable that overrides it, read here before
//! anything is bound. Nothing below Fleet reads any of them.
//!
//! # The default is the adapter's, and this file cannot spell it
//!
//! `crates/config/settings.toml` gives the AgentHarness binary path as Machine
//! scope, `Daemon start`, defaulting to the CLI as installed on `PATH`. **That
//! name is a vendor's**, so it lives in `adapters`: this module asks
//! [`HeadlessAgent`] for it and never learns what comes back.
//!
//! # Both names are probed, not just the override
//!
//! Probing only [`AGENT_BINARY`] produced the failure this file exists to move
//! earlier: Fleet bound a port, published a runtime file, accepted a Job,
//! showed it on the Board and died at spawn with *no such file or directory* —
//! a condition knowable before the bind. Same shape as the empty model that was
//! accepted, stored, shown and refused at dispatch.
//!
//! The probe uses the `PATH` a **Drone** is given, and the refusal prints it.
//! `which` succeeding in the operator's shell is a different question, and a
//! message that did not say so cost half an hour.

use std::error::Error;
use std::fmt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use adapter_traits::{Model, SpawnConfigRefused};
use adapters::HeadlessAgent;

/// The environment variable naming the headless agent CLI.
///
/// **Read at the composition root and nowhere below it.** Everything under
/// Fleet is handed the harness it was assembled with.
pub const AGENT_BINARY: &str = "ARMADA_AGENT_BINARY";

/// The environment variable naming the model a Job gets when it names none.
///
/// The same shape as [`AGENT_BINARY`]: an override for a machine with an
/// opinion, unset being the ordinary case. It is **not** probed — whether a
/// model name is one this account may use is a question only the vendor can
/// answer, and asking it would put a network call before the bind.
pub const MODEL: &str = "ARMADA_MODEL";

/// The environment variable naming the model a Judge call is made on.
///
/// A second variable rather than a reuse of [`MODEL`], because the two are
/// opposite dials: the Drone's is what does the work and the Judge's is the
/// cheap one checking it. A machine that raised one by raising the other would
/// pay Drone prices on every criterion.
pub const JUDGE_MODEL: &str = "ARMADA_JUDGE_MODEL";

/// The environment variable naming the model a Job proposer call is made on.
///
/// A third variable and not a third pattern: same shape as [`JUDGE_MODEL`],
/// same adapter-stated default, read here and nowhere below Fleet. It is its
/// own dial because it fires on every dispatch rather than on every criterion,
/// and the two are raised for different reasons.
pub const PROPOSER_MODEL: &str = "ARMADA_PROPOSER_MODEL";

/// What a step naming no model of its own is judged by.
///
/// **`crates/config/settings.toml` decides it and this module still never
/// learns the spelling.** The `judge-model` row carries `haiku` as of Aug 2026,
/// and `adapters` derives the value from its own roster rather than naming it
/// twice — so the default arrives from there, as it always did. What changed is
/// that it is now a decision rather than a stand-in; [`model_choices`] is the
/// arrangement that is still provisional.
pub fn judge_model(named: Option<String>) -> Result<Model, SpawnConfigRefused> {
    resolved(named, HeadlessAgent::judge_model())
}

/// What a dispatch request is read by when nothing names one.
///
/// `crates/config/settings.toml`'s `job-proposer-model` row reads `undecided`,
/// so the default is the adapter's — the same stand-in [`judge_model`] is in,
/// and this module never learns the spelling of either.
pub fn proposer_model(named: Option<String>) -> Result<Model, SpawnConfigRefused> {
    resolved(named, HeadlessAgent::proposer_model())
}

/// An override where one is set and not blank, and the adapter's default
/// otherwise. **One reader for both dials**, because a second copy is how the
/// two would come to disagree about what an empty variable means.
fn resolved(named: Option<String>, default: &str) -> Result<Model, SpawnConfigRefused> {
    match named.map(|named| named.trim().to_string()) {
        Some(named) if !named.is_empty() => Model::named(&named),
        _ => Model::named(default),
    }
}

/// The harness to assemble Fleet with: the settings default, or the override.
/// **Either one is refused if nothing runnable is there.**
///
/// `path` is the `PATH` a **Drone** is given rather than this process's, which
/// is what makes the probe answer the question actually being asked: not
/// whether the operator can run it, but whether a Drone will find it.
pub fn agent_binary(named: Option<String>, path: &str) -> Result<HeadlessAgent, NoSuchAgent> {
    let overridden = named.is_some();
    let harness = match named {
        Some(named) => HeadlessAgent::at(named),
        None => HeadlessAgent::on_path(),
    };
    if runnable(harness.program(), path) {
        return Ok(harness);
    }
    Err(NoSuchAgent {
        // Read back through the adapter rather than written here. The default's
        // spelling is a vendor's and this crate may not carry it.
        named: harness.program().to_string(),
        path: String::from(path),
        overridden,
    })
}

/// The models a Job may name, and the one it gets when it names none.
///
/// **`crates/config/settings.toml` supplies neither.** The two rows that should
/// — `kit-level-allowed-default-models-list`, "the set that Default model per
/// Job type and Judge model select from", and `default-model-per-job-type` —
/// carry no `default` value, unlike `agentharness-binary-path-and-version-pin`
/// beside them, which is why the binary above could be resolved from
/// configuration and this cannot. So the roster and the default come from
/// `adapters`, the boundary allowed to know a vendor's spellings, and this
/// module never learns what comes back. That is a stand-in and the settings
/// rows are the answer; it is reported as such.
///
/// An override joins the roster rather than replacing it, and leads it: naming
/// a model Fleet should use is also saying it is a model this machine has, and
/// a picker that could not offer the configured default would be a picker that
/// disagrees with the value it starts on.
pub fn model_choices(named: Option<String>) -> ipc::ModelChoices {
    let mut models: Vec<String> = HeadlessAgent::models()
        .iter()
        .map(|model| (*model).to_string())
        .collect();
    let default = match named {
        Some(named) if !named.trim().is_empty() => {
            let named = named.trim().to_string();
            if !models.contains(&named) {
                models.insert(0, named.clone());
            }
            named
        }
        _ => HeadlessAgent::default_model().to_string(),
    };
    ipc::ModelChoices { models, default }
}

/// Whether something runnable is there under that name.
///
/// Two spellings, because they are two different questions. A name with a
/// separator in it is a path and is answered by looking at that one place; a
/// bare name is answered by the `PATH`, in order, the way the exec that
/// eventually runs it will.
fn runnable(named: &str, path: &str) -> bool {
    if named.contains('/') {
        return executable(Path::new(named));
    }
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .any(|dir| executable(&Path::new(dir).join(named)))
}

/// A file, with an execute bit somebody has. Directories are not runnable and a
/// file with no execute bit is a name that will fail at spawn — which is the
/// failure this probe exists to move earlier.
fn executable(at: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(at) else {
        return false;
    };
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

/// The agent binary is not there. **Raised before the bind.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoSuchAgent {
    /// What was named. Carried so the message can print it — a refusal that
    /// does not name the thing it refused sends the reader to their shell.
    named: String,
    /// What was searched, where the name was a bare one.
    path: String,
    /// Whether [`AGENT_BINARY`] chose the name. The two cases need opposite
    /// answers: unset the variable, or install the CLI.
    overridden: bool,
}

impl NoSuchAgent {
    /// The `PATH` that was searched. **Named in every message**: `which`
    /// succeeding in the operator's shell says nothing about a Drone's `PATH`,
    /// and a refusal that omitted it is what made this take half an hour.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for NoSuchAgent {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.overridden {
            true => write!(
                out,
                "{AGENT_BINARY} names `{}`, and there is nothing runnable there",
                self.named
            )?,
            false => write!(
                out,
                "the agent CLI `{}` is not installed anywhere a Drone would find it",
                self.named
            )?,
        }
        if !self.named.contains('/') {
            write!(out, " — a Drone's PATH is {}", self.path)?;
        }
        match self.overridden {
            true => out.write_str(
                ". It is an override — unset it to use the agent CLI as installed, \
                 or name a path that exists",
            ),
            false => out.write_str(
                ". Install it, or set ARMADA_AGENT_BINARY to where it already is. \
                 A Fleet started without it would take a port, publish a runtime \
                 file, accept a Job and fail at the first Drone",
            ),
        }
    }
}

impl Error for NoSuchAgent {}
