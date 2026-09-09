//! `evidence:`, the third thing a repository declares beside its Checks and
//! its Commands.
//!
//! **A repository says how it shows its work, and Armada grows no capture
//! stack.** `checks:` holds no opinion about cargo versus pnpm; this holds none
//! about Playwright versus anything else. Five keys — how to serve the thing,
//! how to know it is up, what runs one spec, where the frames land, and the
//! short list of paths nothing may ever visit — and every one of them is a
//! command line or a path the repository wrote. Nothing here knows what a
//! browser is.
//!
//! **Not a route list.** The section says how a spec is run, never which
//! screens exist: nobody hand-writes a hundred of those, and a list that had to
//! be maintained beside the app would be stale on the first screen anybody
//! added. What is captured is scoped by the spec a Drone wrote, which is code
//! that lands in the diff and can be read against the frame it produced.
//!
//! **A file of its own for [`super::policies`]' reason, and one more**:
//! `manifest.rs` is at 887 lines against a gate that refuses 900, and this is
//! a value with accessors plus the walk that builds it — the seam
//! `docs/practices/rust.md` section 6 names.

use core_model::RepoPath;
use serde_yaml_ng::Value;

use super::Manifest;
use crate::error::{BadTarget, Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys M1 reads inside `evidence`.
pub(super) const EVIDENCE_KEYS: &[&str] = &["serve", "ready", "run", "frames", "never"];

/// Where a spec's path goes in [`Harness::run`]. The same two characters
/// `checks.<name>.narrow.each` substitutes with, and deliberately so: one
/// spelling of "the value goes here" across the whole file.
const SUBSTITUTION: &str = "{}";

/// How a repository shows what its work looks like.
///
/// **Four commands and a boundary, and not one of them is Armada's.** A
/// repository that serves itself with one command and runs a spec with another
/// has said everything Fleet needs; a repository that needs a pipeline writes a
/// script and names the script, which is `checks-runner`'s rule and holds here
/// for its reason — there is no shell, so `run` cannot pipe or chain.
///
/// Fields are `pub(super)` for [`super::declared`]'s reason: the walk below is
/// the only thing that builds one, and every reader outside this crate goes
/// through an accessor, which is where what a value means is written down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Harness {
    pub(super) serve: String,
    pub(super) ready: String,
    pub(super) run: String,
    pub(super) frames: RepoPath,
    pub(super) never: Vec<String>,
}

impl Harness {
    /// The command that serves the thing being shown. **Long-running on
    /// purpose** — it is started, held for the run, and ended after it.
    ///
    /// **It names its own port, and Armada does not assign one.**
    /// `crates/config/settings.toml` carries three port rows and nothing
    /// implements them, so a Fleet handing out a port would be inventing the
    /// half of that design nobody has built. A repository already knows which
    /// port its own preview server takes, and writing it here keeps the one
    /// place it is spelled inside the repository that owns it.
    pub fn serve(&self) -> &str {
        &self.serve
    }

    /// The command that exits zero once [`serve`](Harness::serve) is up.
    ///
    /// **Required, and there is no sleep to fall back on.** A duration would
    /// be a number somebody guessed against a machine they were not using, and
    /// the failure it produces is a frame of a blank page that looks exactly
    /// like a frame of a broken one. A command that answers is the only reading
    /// that distinguishes them.
    pub fn ready(&self) -> &str {
        &self.ready
    }

    /// The command that runs one spec, with `{}` where the spec's path goes.
    ///
    /// **Refused without the substitution**, for `narrow.each`'s reason: a
    /// template with nowhere to put the value runs the same command whichever
    /// spec was named, which is a capture scoped to nothing and saying so
    /// nowhere.
    pub fn run(&self) -> &str {
        &self.run
    }

    /// The command with one spec's path substituted in.
    ///
    /// **Here rather than at the caller**, because the substitution is what
    /// makes the template a command and there must be one spelling of it. A
    /// caller composing the string itself would be a second place the two
    /// characters are known.
    pub fn running(&self, spec: &str) -> String {
        self.run.replace(SUBSTITUTION, spec)
    }

    /// Where the frames land, relative to the repository root.
    ///
    /// **The repository's directory and never Armada's.** A harness writes
    /// where its own tooling was configured to write, and a path Fleet imposed
    /// would be one every repository had to reconfigure its runner to match.
    /// Fleet reads this directory after the run and keeps what it finds.
    pub fn frames(&self) -> &RepoPath {
        &self.frames
    }

    /// The paths a spec must never visit, in the order the file names them.
    ///
    /// **Empty means the repository named none**, which is a legitimate answer
    /// and not a gap: a read-only demo has nothing to forbid. `never: []` is
    /// refused rather than read as empty, for `requires`' reason — a list with
    /// nothing in it is a key to delete.
    ///
    /// **Armada does not enforce it and says so.** Nothing here drives a
    /// browser, so there is no navigation to intercept; what reads this is the
    /// block a Drone writing the spec is given, and the reviewer reading the
    /// spec against it. That is weaker than an interception and it is the
    /// honest shape — the alternative is a key that reads as a guarantee and
    /// is a comment.
    pub fn never(&self) -> &[String] {
        &self.never
    }
}

/// `evidence:`, read.
///
/// **Every key is refused on its own and the walk continues**, so a section
/// with three faults is one edit. A refused value leaves the section unbuilt
/// and the refusal is already in `out` — no Manifest carrying one loads at all.
pub(super) fn read(value: &Value, out: &mut Vec<Refusal>) -> Option<Harness> {
    let mut table = Table::open("evidence", value, out)?;
    let serve = table
        .required("serve", out)
        .and_then(|value| yaml::text(&table.at("serve"), value, out));
    let ready = table
        .required("ready", out)
        .and_then(|value| yaml::text(&table.at("ready"), value, out));
    let run_key = table.at("run");
    let run = table
        .required("run", out)
        .and_then(|value| yaml::text(&run_key, value, out))
        .and_then(|written| match written.contains(SUBSTITUTION) {
            true => Some(written),
            false => {
                out.push(Refusal::new(&run_key, Fault::NothingToSubstitute));
                None
            }
        });
    let frames_key = table.at("frames");
    let frames = table
        .required("frames", out)
        .and_then(|value| yaml::text(&frames_key, value, out))
        .and_then(|written| where_frames_land(&frames_key, written, out));
    // Values a person reads, not paths this crate resolves: a route is the
    // app's own vocabulary and Armada has no way to check one. `never: []` is
    // refused by `yaml::list`, and an absent key is a repository that forbids
    // nothing.
    let never = table
        .optional("never")
        .and_then(|value| yaml::list(&table.at("never"), value, out))
        .map(|items| {
            items
                .into_iter()
                .filter_map(|(key, item)| yaml::text(&key, item, out))
                .collect()
        })
        .unwrap_or_default();
    table.close(EVIDENCE_KEYS, out);
    Some(Harness {
        serve: serve?,
        ready: ready?,
        run: run?,
        frames: frames?,
        never,
    })
}

/// A `frames` that names one directory inside the worktree, or a refusal
/// saying which of the three ways it does not.
///
/// **Three of `artifact_target`'s four, and the fourth is inverted.** A glob
/// cannot name a directory to read, an absolute path is not in the worktree the
/// run happens in, and a path climbing out of it reaches the machine rather
/// than the checkout — each is a harness that fails on every Job in the same
/// way, and each is cheaper to catch here than after a server has been started.
/// [`BadTarget::ADirectory`] is not among them, because a directory is exactly
/// what this names.
fn where_frames_land(at: &str, written: String, out: &mut Vec<Refusal>) -> Option<RepoPath> {
    let why = if written.contains('*') || written.contains('?') {
        Some(BadTarget::Globbed)
    } else if written.starts_with('/') {
        Some(BadTarget::Absolute)
    } else if written.split('/').any(|segment| segment == "..") {
        Some(BadTarget::Escapes)
    } else {
        None
    };
    match why {
        None => Some(RepoPath::new(written.trim_end_matches('/'))),
        Some(why) => {
            out.push(Refusal::new(
                at,
                Fault::NotAnArtifactPath {
                    value: written,
                    why,
                },
            ));
            None
        }
    }
}

/// The accessor, **here rather than beside the file's other getters**, for
/// [`super::policies`]' reason: the key, what it holds, its refusals and the
/// answer a caller gets are one file rather than four hundred lines apart.
impl Manifest {
    /// How this repository shows its work, or `None` where it does not say.
    ///
    /// **Absent is not a default and there is nothing to fall back to.** A
    /// repository that declares no harness cannot run a `visual` step, which is
    /// what `ResolvedWorkflow::resolve` refuses before anything is dispatched —
    /// so absence reaches a Job as a refusal a person reads, never as a capture
    /// that quietly produced nothing.
    ///
    /// **Not behind the live cell**, unlike the two policies: every workflow
    /// was resolved against this at daemon start, so a save that added or
    /// removed the section is reported as needing a restart rather than adopted
    /// under Jobs that were resolved without it. `crate::live` carries the rule
    /// and `drone.exclude_paths` is the other key on the same side of it.
    pub fn harness(&self) -> Option<&Harness> {
        self.harness.as_ref()
    }
}
