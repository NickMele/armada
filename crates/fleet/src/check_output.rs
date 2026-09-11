//! What a Check produced, written down and read back: the output to a file,
//! the row to the store, and the file to whoever opens that row.
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
//! `.armada/checks/<job-id>/<step-id>.<attempt>.<ordinal>.log`, which is the
//! row's key exactly: `job_step_checks` is keyed by Job, step, attempt and
//! ordinal, and every one of the four is here.
//!
//! The attempt was missing while nothing could run a step twice. Now a failed
//! Check hands the step back, so a second run wrote over the first run's files
//! while `store` kept both runs' rows — leaving the first attempt's row
//! pointing at the second attempt's output, which is worse than pointing at
//! nothing. A path that is the whole key cannot do that.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Output;
use core_model::{Attempt, JobId, StepCheck, StepId};
use store::Attempted;

use verification::Submission;

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
        handle: &str,
        step: &StepId,
        attempt: Attempt,
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
            &self.host().records_root,
            handle,
            step,
            attempt,
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

    /// Write down which gaming patterns the step's evidence tripped.
    ///
    /// **Only where something was flagged.** A step whose gaming check found
    /// nothing has nothing to say and nothing to clear; the writer replaces a
    /// step's rows whole, so an empty write on an ordinary pass would erase the
    /// finding a resubmission had not answered.
    ///
    /// Without this the escalation says the evidence is suspect and not what
    /// about it, which is the whole content of the finding — the same defect an
    /// uncited refusal would be.
    pub(crate) async fn recorded_gaming(
        &self,
        job_id: &JobId,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        let Some(flagged) = ruling.flagged() else {
            return Ok(());
        };
        self.store()
            .lock()
            .await
            .record_step_gaming_flags(job_id, step, flagged.cited(), &self.now())
            .map_err(Adrift::Writing)
    }

    /// Write down the evidence the gate ruled on.
    ///
    /// **Written whatever the ruling**, for the reason a Check result is: a
    /// step that submitted and was refused and a step that submitted nothing
    /// are different facts. It is skipped only where nothing was ruled on —
    /// a submission of the wrong kind spends nothing and records nothing.
    ///
    /// A later step's gaming check reads this as its baseline, which is why it
    /// goes to disk rather than staying in the daemon: a baseline that
    /// evaporates on restart would take the check quietly with it.
    pub(crate) async fn recorded_evidence(
        &self,
        job_id: &JobId,
        step: &StepId,
        submission: &Submission,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        if matches!(ruling, Ruling::NotWhatTheStepAsked(_)) {
            return Ok(());
        }
        self.store()
            .lock()
            .await
            .record_step_evidence(job_id, step, &submission.recorded(), &self.now())
            .map_err(Adrift::Writing)
    }
}

/// Where one step's Check output lives, under this repository's own share
/// of Fleet's data directory.
pub fn checks_dir(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("checks")
        .join(handle)
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
    records_root: &str,
    handle: &str,
    step: &StepId,
    attempt: Attempt,
    checks: &[StepCheck],
    output: &[(String, Output)],
) -> Vec<StepCheck> {
    keeping(
        records_root,
        handle,
        step,
        attempt,
        checks,
        output,
        RECORDED,
    )
}

/// The same, for a run the Drone asked for rather than one the gate made.
///
/// **A different file name, deliberately.** `crate::dry_run` writes no row, so
/// a dry run using [`RECORDED`]'s path would overwrite output the record points
/// at with output nothing points at — and a person opening the log named on a
/// step's Check row would be reading a run that decided nothing.
///
/// The attempt and the ordinal key it to the step's Check on this run, so a
/// second dry run inside one attempt overwrites the first. The files and the
/// attempt have one lifetime, which is what stops the directory growing with
/// every ask — and what keeps a reattempt's dry runs apart from the ones
/// before it, the same reason the gate's own path carries the attempt.
pub fn kept_dry(
    records_root: &str,
    handle: &str,
    step: &StepId,
    attempt: Attempt,
    checks: &[StepCheck],
    output: &[(String, Output)],
) -> Vec<StepCheck> {
    keeping(records_root, handle, step, attempt, checks, output, DRY)
}

/// Write each Check's output for a run against a commit, and put its path on
/// the row.
///
/// **Under `commits/`, beside the Job directories rather than inside one.** The
/// run belongs to the commit and not to whichever Job noticed the merge — `#474`
/// — so filing it under that Job's id would put one commit's answer somewhere a
/// second Job merging into the same commit would never look. A Job id is a
/// ULID, so the literal collides with nothing.
///
/// The ordinal alone is the file name, because the row's key is the commit and
/// the ordinal and nothing else: there is no step here, and no attempt — a
/// commit is proved once.
pub fn kept_for_a_commit(
    records_root: &str,
    at_commit: &str,
    checks: &[StepCheck],
    output: &[(String, Output)],
) -> Vec<StepCheck> {
    let relative = format!(".armada/checks/commits/{at_commit}");
    let dir = Path::new(records_root).join(&relative);
    if !one_component(at_commit) || std::fs::create_dir_all(&dir).is_err() {
        return checks.to_vec();
    }
    checks
        .iter()
        .enumerate()
        .map(|(ordinal, check)| {
            let Some((_, printed)) = output.iter().find(|(name, _)| name == &check.name) else {
                return check.clone();
            };
            let name = format!("{ordinal}.log");
            let mut kept = check.clone();
            kept.output_path = write(&dir, &name, printed).then(|| format!("{relative}/{name}"));
            kept
        })
        .collect()
}

/// Where one Check writes its log while the gate runs it, as the absolute file
/// and the repository-relative path a surface is handed.
///
/// **A third name beside the recorded one and the dry run's**, for `kept_dry`'s
/// reason: the recorded file is written from the ruling and its row points at
/// it, and a live log written under that name would be overwritten by it — or,
/// on a gate that never ruled, left where a row would take it for the record.
/// The attempt and the ordinal are in it for the reason they are in both of
/// the others, so a re-gate of one attempt writes over its own live logs and
/// never over another run's.
///
/// `None` where the step id is not one path component or the directory will
/// not open, which is `keeping`'s answer to both.
pub(crate) fn live_file(
    repo_root: &str,
    handle: &str,
    step: &StepId,
    attempt: Attempt,
    ordinal: usize,
) -> Option<(PathBuf, String)> {
    let dir = writable(repo_root, handle)?;
    let name = file_name(step, attempt, ordinal, LIVE)?;
    Some((dir.join(&name), format!(".armada/checks/{handle}/{name}")))
}

/// What a gate run's file name carries between the step and the ordinal:
/// nothing.
const RECORDED: &str = "";
/// What a dry run's carries.
const DRY: &str = "dry.";
/// What a live log's carries.
const LIVE: &str = "live.";

fn keeping(
    records_root: &str,
    handle: &str,
    step: &StepId,
    attempt: Attempt,
    checks: &[StepCheck],
    output: &[(String, Output)],
    infix: &str,
) -> Vec<StepCheck> {
    let Some(dir) = writable(records_root, handle) else {
        return checks.to_vec();
    };
    checks
        .iter()
        .enumerate()
        .map(|(ordinal, check)| {
            let Some((_, printed)) = output.iter().find(|(name, _)| name == &check.name) else {
                return check.clone();
            };
            let Some(name) = file_name(step, attempt, ordinal, infix) else {
                return check.clone();
            };
            let mut kept = check.clone();
            kept.output_path =
                write(&dir, &name, printed).then(|| format!(".armada/checks/{}/{name}", handle));
            kept
        })
        .collect()
}

fn writable(records_root: &str, handle: &str) -> Option<PathBuf> {
    let dir = checks_dir(records_root, handle);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// The file name for one Check of one step.
///
/// **`None` where the step id is not a single path component.** A step id is
/// text a workflow author typed and nothing validates it, so one holding a
/// separator would put the file somewhere other than the directory named above.
/// The output is then not kept and the row says so by having no path.
fn file_name(step: &StepId, attempt: Attempt, ordinal: usize, infix: &str) -> Option<String> {
    let id = step.as_str();
    one_component(id).then(|| format!("{id}.{attempt}.{infix}{ordinal}.log"))
}

/// Whether an id can stand as one path component.
///
/// **One predicate, two callers.** `crate::asked` names files after a step id
/// and a criterion id under the same rule and for the same reason; a second
/// spelling of it here and there is how the two would come to disagree about
/// which id is safe.
pub fn one_component(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains('\0')
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

/// How many lines of a Check's output one read carries.
///
/// **The tail, for the reason `checks_runner` captures the tail**: a test
/// runner prints its failures last, and a runaway command prints forever, so
/// keeping the beginning keeps the part nobody opened the Job for.
const A_READING: usize = 2_000;

/// How many bytes of those lines one read carries.
///
/// A second bound beside the line count, because one line can be a whole
/// minified bundle. The window loses its oldest lines to stay under it, which
/// keeps this read's cost a property of the code rather than of whatever a
/// Check printed.
const MOST: usize = 256 * 1024;

/// One Check's output, read back out of the file its row points at.
///
/// **Beside the writing, because both halves depend on the one shape above.**
/// A name the writer stops producing is a name this stops resolving, and the
/// two would drift the first time they were a crate apart.
///
/// **The id is the file's own name and never a path the caller composed.**
/// `kept` is the last component of a row's `output_path`, and it is resolved
/// against the rows this Job actually holds before anything is opened — so an
/// id naming no row of this Job reaches no file, whatever it spells. That is
/// what makes reading a file at a caller's word safe here: the caller does not
/// name a file, it names a row, and Fleet says where that row's file is.
///
/// `None` where no row of this Job kept an output under that name: a Job whose
/// `.armada` directory was reclaimed, an id that was never one, or a file that
/// will not open. Not an error — the caller decides what to say about it,
/// exactly as `transcript::arguments` leaves that decision to `serving`.
///
/// **The file is counted as it is read.** `total_lines` is exact even where the
/// window is not the whole, which is what lets `from_line` be the file's own
/// numbering rather than the window's: a reader citing line 1,940 means the
/// file's 1,940th line.
pub fn kept_output(
    records_root: &str,
    kept: &str,
    ran: &[Attempted<Vec<StepCheck>>],
) -> Option<ipc::CheckOutput> {
    let (attempt, name, path) = named(kept, ran)?;
    let file = std::fs::File::open(Path::new(records_root).join(&path)).ok()?;
    let bytes = file.metadata().map(|at| at.len()).unwrap_or_default();

    // A window over a stream rather than the file in memory: the oldest line
    // goes as soon as the window is over either bound, so what this holds is
    // bounded by the two constants above and by nothing a Check printed.
    let mut window: VecDeque<String> = VecDeque::new();
    let mut held = 0usize;
    let mut total = 0u32;
    let mut first = 1u32;
    for line in BufReader::new(file).lines() {
        // A line that will not decode is where the reading stops. Skipping it
        // would renumber every line after it, and this read's whole claim is
        // that its numbering is the file's.
        let Ok(line) = line else { break };
        total = total.saturating_add(1);
        held += line.len();
        window.push_back(line);
        while window.len() > A_READING || (held > MOST && window.len() > 1) {
            held -= window
                .pop_front()
                .map(|gone| gone.len())
                .unwrap_or_default();
            first = first.saturating_add(1);
        }
    }

    Some(ipc::CheckOutput {
        attempt,
        name,
        path,
        from_line: first,
        total_lines: total,
        bytes,
        whole: first == 1,
        lines: window.into(),
    })
}

/// Which recorded run of which Check kept a file under this name, and where.
///
/// **The record is the allowlist.** Nothing else decides whether a path may be
/// opened: a row of this Job holds it or the answer is `None`, which is why
/// there is no separate guard for `..`, for a leading separator, or for
/// anything else a caller might spell — none of them can match a path Fleet
/// wrote. [`one_component`] is belt on top of that, and it is the same
/// predicate the writing half uses.
fn named(kept: &str, ran: &[Attempted<Vec<StepCheck>>]) -> Option<(u32, String, String)> {
    if !one_component(kept) {
        return None;
    }
    ran.iter().find_map(|group| {
        group.record.iter().find_map(|check| {
            let path = check.output_path.as_deref()?;
            (path.rsplit('/').next() == Some(kept))
                .then(|| (group.attempt.number(), check.name.clone(), path.to_string()))
        })
    })
}
