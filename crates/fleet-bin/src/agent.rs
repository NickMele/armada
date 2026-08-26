//! What a Drone is started as: which binary, and which model.
//!
//! **One reader, because they are one missing piece.** `ARMADA_AGENT_BINARY`
//! and the model were both "nothing reads configuration yet", and both are
//! answered the same way — a default the adapter states, an environment
//! variable that overrides it, and the reading done here at the composition
//! root before anything is bound. Nothing below Fleet reads either.
//!
//! # The default is the adapter's, and this file cannot spell it
//!
//! `crates/config/settings.toml` gives the AgentHarness binary path as Machine
//! scope, `Daemon start`, defaulting to the CLI as installed and found on
//! `PATH`. **That name is a vendor's**, so it lives in `adapters` and nowhere
//! else: this module asks for [`HeadlessAgent::on_path`] and never learns what
//! comes back. A composition root carrying the string would be the adapter
//! boundary leaking into the one crate that assembles everything.
//!
//! # An override, not a requirement
//!
//! [`AGENT_BINARY`] names a binary for a machine that put one somewhere
//! unusual. Unset is the ordinary case and is **not** a refusal.
//!
//! Set-and-wrong is the case worth failing on, because it is somebody having
//! tried to point Fleet at something. It is probed here — with the operator at
//! the terminal, before a port is taken and before a runtime file is published
//! — rather than at the first Drone, where it reads as a Job that will not
//! start. The unset default is deliberately **not** probed: that would refuse
//! the ordinary case on a machine whose `PATH` differs from a Drone's, and
//! whether the agent is installed is Doctor's question rather than a start-up
//! precondition.

use std::error::Error;
use std::fmt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

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

/// The harness to assemble Fleet with: the settings default, or the override,
/// which is refused if nothing runnable is there.
///
/// `path` is the `PATH` a **Drone** is given rather than this process's, which
/// is what makes the probe answer the question actually being asked: not
/// whether the operator can run it, but whether a Drone will find it.
pub fn agent_binary(named: Option<String>, path: &str) -> Result<HeadlessAgent, NoSuchAgent> {
    let Some(named) = named else {
        return Ok(HeadlessAgent::on_path());
    };
    if runnable(&named, path) {
        return Ok(HeadlessAgent::at(named));
    }
    Err(NoSuchAgent {
        named,
        path: String::from(path),
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

/// The named agent binary is not there. **Raised before the bind.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoSuchAgent {
    /// What was named. Carried so the message can print it — a refusal that
    /// does not name the thing it refused sends the reader to their shell.
    named: String,
    /// What was searched, where the name was a bare one.
    path: String,
}

impl fmt::Display for NoSuchAgent {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "{AGENT_BINARY} names `{}`, and there is nothing runnable there",
            self.named
        )?;
        if !self.named.contains('/') {
            write!(out, " on a Drone's PATH ({})", self.path)?;
        }
        out.write_str(
            ". It is an override — unset it to use the agent CLI as installed, \
             or name a path that exists. A Fleet started on a wrong name would \
             fail at the first Drone instead of here",
        )
    }
}

impl Error for NoSuchAgent {}
