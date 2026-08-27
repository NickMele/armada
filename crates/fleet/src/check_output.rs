//! What a Check produced, written down: the output to a file, the row to the
//! store.
//!
//! # A file, with the path on the row
//!
//! The same call the Drone transcript got, for the same reason: a large
//! artifact with its own retention profile lives on disk and the record holds a
//! reference. `Ruling::Failed` carried the bytes in memory and they died with
//! the process, so a failed Check showed its exit code and never its output.
//!
//! # The path is a function of the row's key, and nothing else
//!
//! `.armada/checks/<job-id>/<step-id>.<ordinal>.log`, where the ordinal is the
//! Check's position in the step. `record_step_checks` replaces a step's rows
//! whole on a rerun, so a second run overwrites the same files — the file and
//! the row it belongs to have one lifetime. A path carrying a run id would
//! leave output no row points at, which is a directory that grows and a
//! question nobody can answer.

use std::io::Write;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Output;
use core_model::{JobId, StepCheck, StepId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::Ruling;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Write down what each of the step's declared Checks did — **and what
    /// each of them printed.**
    ///
    /// The output goes to a file and the row keeps the path, for the reason
    /// this module gives. **No event and nothing published**, like the branch:
    /// a Check running is not a transition, it is the evidence one was derived
    /// from. A ruling that ran nothing writes nothing, so a resubmission of the
    /// wrong kind does not clear the last real run.
    pub(crate) async fn recorded_checks(
        &self,
        job_id: &JobId,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        if ruling.checks().is_empty() {
            return Ok(());
        }
        let printed: Vec<(String, checks_runner::Output)> = ruling
            .output()
            .iter()
            .map(|kept| (kept.check.clone(), kept.output.clone()))
            .collect();
        let checks = kept(
            &self.host().repo_root,
            job_id,
            step,
            ruling.checks(),
            &printed,
        );
        self.store()
            .lock()
            .await
            .record_step_checks(job_id, step, &checks, &self.now())
            .map_err(Adrift::Writing)
    }

    /// Write down what the Judge said, where it said anything.
    ///
    /// **Written even when nothing was refused.** A step the Judge cleared and
    /// a step the Judge never ran on are different facts, and only the record
    /// can tell them apart.
    pub(crate) async fn recorded_judgments(
        &self,
        job_id: &JobId,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        if ruling.judged().is_empty() {
            return Ok(());
        }
        self.store()
            .lock()
            .await
            .record_step_judgments(job_id, step, ruling.judged(), &self.now())
            .map_err(Adrift::Writing)
    }
}

/// Where one step's Check output lives, under the repository it ran in.
pub fn checks_dir(repo_root: &str, job: &JobId) -> PathBuf {
    Path::new(repo_root)
        .join(".armada")
        .join("checks")
        .join(job.as_str())
}

/// Write each Check's output and put its path on the row.
///
/// **Takes the rows and the output separately because they are different
/// lengths**: `diff_nonempty` is a declared Check that runs no command, so it
/// has a row and nothing to write. They are matched by the Check's name, which
/// is the one label both halves already carry.
///
/// A write that fails leaves the row's path absent rather than failing the
/// ruling. The output is what a person reads afterwards; refusing to record a
/// verdict because a log file would not open would lose the verdict as well.
pub fn kept(
    repo_root: &str,
    job: &JobId,
    step: &StepId,
    checks: &[StepCheck],
    output: &[(String, Output)],
) -> Vec<StepCheck> {
    let Some(dir) = writable(repo_root, job) else {
        return checks.to_vec();
    };
    checks
        .iter()
        .enumerate()
        .map(|(ordinal, check)| {
            let Some((_, printed)) = output.iter().find(|(name, _)| name == &check.name) else {
                return check.clone();
            };
            let Some(name) = file_name(step, ordinal) else {
                return check.clone();
            };
            let mut kept = check.clone();
            kept.output_path = write(&dir, &name, printed)
                .then(|| format!(".armada/checks/{}/{name}", job.as_str()));
            kept
        })
        .collect()
}

fn writable(repo_root: &str, job: &JobId) -> Option<PathBuf> {
    let dir = checks_dir(repo_root, job);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// The file name for one Check of one step.
///
/// **`None` where the step id is not a single path component.** A step id is
/// text a workflow author typed and nothing validates it, so one holding a
/// separator would put the file somewhere other than the directory named above.
/// The output is then not kept and the row says so by having no path.
fn file_name(step: &StepId, ordinal: usize) -> Option<String> {
    let id = step.as_str();
    let plain = !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains('\0');
    plain.then(|| format!("{id}.{ordinal}.log"))
}

/// Both streams in one file, each behind a marker line.
///
/// They were captured separately and cannot be honestly interleaved, so the
/// markers say which is which. Nothing parses this — it is read with `cat`.
fn write(dir: &Path, name: &str, printed: &Output) -> bool {
    let Ok(mut file) = std::fs::File::create(dir.join(name)) else {
        return false;
    };
    let mut written = writeln!(file, "--- stdout ---").is_ok();
    written &= file.write_all(printed.stdout.as_bytes()).is_ok();
    written &= writeln!(file, "\n--- stderr ---").is_ok();
    written &= file.write_all(printed.stderr.as_bytes()).is_ok();
    if printed.truncated {
        // Said out loud, because a reader treating a cut-off log as complete
        // will conclude the failure is not in it.
        written &= writeln!(
            file,
            "\n--- one or both streams were longer than the capture limit ---"
        )
        .is_ok();
    }
    written
}
