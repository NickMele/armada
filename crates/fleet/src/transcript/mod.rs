//! What a Drone said, written down.
//!
//! Fleet's line loop is the only reader of a Drone's stdout — a second reader
//! would starve the thing that makes progress — so every other consumer is fed
//! from that loop rather than from the pipe. [`Tap`] is that seam and [`Taps`]
//! is both things behind it: [`Recording`], the file, and [`Live`], a viewer.
//!
//! # Rows, never the wire shape
//!
//! A row is a `DroneEvent`, which is also the redaction: spike 3's raw opening
//! event carried the operator's home path and whole tool inventory, while
//! `DroneEvent::Started` carries a session, a model and a count.
//!
//! # Neither consumer can hold up the loop
//!
//! The file's queue is `try_send` and the live channel is drop-oldest, so a row
//! either will not take is dropped and counted rather than awaited. That is why
//! a second consumer changes nothing about what Fleet does with the events,
//! which `docs/concepts/observe.md` calls load-bearing.
//!
//! # A Fleet that dies does not come back to this file
//!
//! The writer goes with Fleet, `Fleet::reconcile` never puts a Drone back onto
//! a Job it did not spawn, and every row already taken is flushed. A retry is a
//! second `drone_id`, so nothing appends to a dead Drone's transcript.

mod backfill;
mod row;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use adapter_traits::DroneEvent;
use core_model::{Component, DroneId, Envelope, FieldValue, JobId, Level, StepId, Timestamp, Ulid};
use ipc::TranscriptRow;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

use crate::clock::Clock;
use row::Line;

pub use backfill::{history, HISTORY};

/// How many rows may be waiting to be written.
///
/// An append to a local file drains far faster than an agent emits, so this is
/// a bound on the pathological case rather than a working size.
const QUEUED: usize = 1024;

/// A consumer of Fleet's line loop.
///
/// **Nothing here may block or await.** The loop that calls it is the loop that
/// advances the Job, and a consumer that could hold it up would make watching a
/// Job change its outcome.
pub trait Tap: Send + Sync {
    /// Every event one line decoded to, in order, after the parser has already
    /// taken them.
    fn saw(&self, events: &[DroneEvent]);
}

/// The ids every line written here carries.
///
/// **`drone` is minted at dispatch and lives nowhere else.** `assigned_drone`
/// on the Job record has no event that sets it, so this file name and the Job
/// log line naming it are the only record that the id existed at all — which
/// is why the Job log gets that line before a single row is written.
#[derive(Clone, Debug)]
pub struct Spine {
    pub job: JobId,
    pub drone: DroneId,
    /// The step the Drone was spawned on, which is every row's step: a Drone
    /// belongs to one step and its transcript is that step's. [`StepLabel`] is
    /// what the sinks read, and this seeds it.
    pub step: StepId,
    /// Fleet's own instance. A restart is this value changing.
    pub run: Ulid,
}

/// Which step the rows of this transcript belong to.
///
/// **It does not move, and there is no method that would move it.** It was an
/// `Arc<Mutex<StepId>>` for as long as one process spanned a step boundary: the
/// sinks live on the far side of the reader task, so advancing the step in the
/// slot alone left every row after a Job's first advance claiming the first
/// step. A Drone belongs to a step now — the boundary ends the process, and the
/// next step's rows are the next Drone's transcript, in a file of its own — so
/// there is no moment at which one transcript's rows change step and no lock
/// left to take.
///
/// Still a type rather than a bare [`StepId`], because what it names is not
/// "some step" but the step every row of this file carries, and the sinks are
/// handed it rather than each keeping a copy of the spine.
#[derive(Clone, Debug)]
pub struct StepLabel(StepId);

impl StepLabel {
    fn starting_at(step: StepId) -> StepLabel {
        StepLabel(step)
    }

    /// The step these rows belong to.
    fn now(&self) -> StepId {
        self.0.clone()
    }
}

/// Where a Drone's rows land, under the repository it is working in.
pub fn transcript_of(repo_root: &str, drone: &DroneId) -> PathBuf {
    Path::new(repo_root)
        .join(".armada")
        .join("transcripts")
        .join(format!("{}.jsonl", drone.as_str()))
}

/// Where a Job's log lines land. The path `docs/concepts/log-envelope.md`
/// names, and until now nothing wrote it.
pub fn log_of(repo_root: &str, job: &JobId) -> PathBuf {
    Path::new(repo_root)
        .join(".armada")
        .join("logs")
        .join(format!("{}.jsonl", job.as_str()))
}

/// A transcript being written.
///
/// **There is no `Drop`.** Dropping it drops the sender, which closes the queue
/// and lets the writer finish what it already took; a `Drop` that aborted would
/// throw away exactly the tail somebody would come looking for.
pub struct Recording {
    rows: Sender<TranscriptRow>,
    /// Rows the queue would not take. Read and written down by the writer.
    missed: Arc<AtomicU64>,
    clock: Arc<dyn Clock>,
    /// The step every row from here is stamped with, as it is stamped.
    step: StepLabel,
    writer: JoinHandle<()>,
}

impl Recording {
    /// Open the transcript, and say in the Job's log where it is.
    ///
    /// Both files are opened here rather than inside the writer so that a disk
    /// which will not hold the record fails at dispatch — where the Job can
    /// still be escalated for a person — rather than in a task nobody awaits.
    pub fn of(
        repo_root: &str,
        spine: Spine,
        clock: Arc<dyn Clock>,
    ) -> Result<Recording, io::Error> {
        let at = transcript_of(repo_root, &spine.drone);
        let transcript = appending(&at)?;
        let mut log = appending(&log_of(repo_root, &spine.job))?;
        // Written before any row, and the only thing that makes a file named
        // by a minted id findable from the Job it belongs to.
        write_line(&mut log, &opened(&spine, clock.now(), &at))?;

        let step = StepLabel::starting_at(spine.step.clone());
        let missed = Arc::new(AtomicU64::new(0));
        let (rows, waiting) = mpsc::channel(QUEUED);
        let writer = tokio::spawn(writing(
            File::from_std(transcript),
            File::from_std(log),
            spine,
            step.clone(),
            waiting,
            Arc::clone(&missed),
            Arc::clone(&clock),
        ));
        Ok(Recording {
            rows,
            missed,
            clock,
            step,
            writer,
        })
    }

    /// The label this recording stamps rows with, so the live view and the
    /// file agree on which step a row belongs to.
    pub fn label(&self) -> StepLabel {
        self.step.clone()
    }

    /// Wait for every row already taken to reach the disk.
    ///
    /// For a test and for a deliberate shutdown. The ordinary path drops the
    /// `Recording` and the writer drains on its own.
    pub async fn settled(self) {
        let Recording { rows, writer, .. } = self;
        drop(rows);
        let _ = writer.await;
    }
}

/// A viewer's end of the tee. **Nothing durable** — the file is the record, and
/// this is a copy of it going to whoever is watching right now.
pub struct Live {
    feed: api::Feed,
    /// Its own, because a `Feed` is `api`'s type and cannot hold a `Clock`.
    /// The stamp is the instant Fleet's loop saw the line, as on the file's row.
    clock: Arc<dyn Clock>,
    /// The same label the file's rows carry, so a watcher and the record agree
    /// on which step a row belongs to.
    step: StepLabel,
}

impl Tap for Live {
    /// **Never awaits and never fails.** A viewer that has fallen behind loses
    /// the oldest rows and is told how many; with nobody watching the row is
    /// dropped and nothing is told.
    fn saw(&self, events: &[DroneEvent]) {
        let at = self.clock.now();
        let step = self.step.now();
        for event in events {
            self.feed.offer(row::seen(&at, &step, event));
        }
    }
}

/// Everything one Drone's lines are fanned out to, put together once.
///
/// A type rather than a `Vec` at the call site, so opening the record and
/// opening the view are one act that either happened or did not — dispatch has
/// one failure to handle and cannot start a Drone with half of it.
pub struct Taps {
    each: Vec<Arc<dyn Tap>>,
}

impl Taps {
    /// The durable record and the live view, in that order.
    ///
    /// **The record is opened here** and its failure is the caller's, so a disk
    /// that will not hold a transcript escalates the Job before a Drone exists.
    /// The view cannot fail: a channel nobody is watching is a channel that
    /// drops.
    pub fn opening(
        repo_root: &str,
        spine: Spine,
        clock: Arc<dyn Clock>,
        feed: api::Feed,
    ) -> Result<Taps, io::Error> {
        let recording = Recording::of(repo_root, spine, Arc::clone(&clock))?;
        let step = recording.label();
        Ok(Taps {
            each: vec![Arc::new(recording), Arc::new(Live { feed, clock, step })],
        })
    }

    pub(crate) fn each(self) -> Vec<Arc<dyn Tap>> {
        self.each
    }
}

impl Tap for Recording {
    /// **Never awaits.** A row the queue will not take is dropped and counted,
    /// because the alternative is holding up the loop that advances the Job.
    fn saw(&self, events: &[DroneEvent]) {
        let at = self.clock.now();
        // Read here rather than in the writer: the row belongs to the step that
        // was running when Fleet saw the line, and the writer drains later.
        let step = self.step.now();
        for event in events {
            let row = row::seen(&at, &step, event);
            if self.rows.try_send(row).is_err() {
                self.missed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// Drain the queue onto the disk until it closes, then close the Job's log line.
async fn writing(
    mut transcript: File,
    mut log: File,
    spine: Spine,
    step: StepLabel,
    mut waiting: Receiver<TranscriptRow>,
    missed: Arc<AtomicU64>,
    clock: Arc<dyn Clock>,
) {
    let (mut written, mut lost) = (0u64, 0u64);
    while let Some(row) = waiting.recv().await {
        // Taken as the row is dequeued, so the count lands among the rows it
        // was lost between rather than in a total at the end of the file.
        lost += note_missed(&mut transcript, &step, &missed, &clock).await;
        match append(&mut transcript, &row).await {
            Ok(()) => written += 1,
            // A row the disk refused is as lost as one the queue refused, and
            // counting it as written would put a number on the closing line
            // that the file does not hold.
            Err(_) => lost += 1,
        }
    }
    lost += note_missed(&mut transcript, &step, &missed, &clock).await;
    let _ = write_async(
        &mut log,
        &closed(&spine, &step.now(), clock.now(), written, lost),
    )
    .await;
}

/// Write down what the queue refused, if anything, and say how much.
async fn note_missed(
    transcript: &mut File,
    step: &StepLabel,
    missed: &AtomicU64,
    clock: &Arc<dyn Clock>,
) -> u64 {
    let lost = missed.swap(0, Ordering::Relaxed);
    if lost > 0 {
        let _ = append(transcript, &row::missed(&clock.now(), &step.now(), lost)).await;
    }
    lost
}

/// The line that makes the transcript findable.
fn opened(spine: &Spine, at: Timestamp, transcript: &Path) -> Envelope {
    envelope(spine, &spine.step, at, "drone transcript opened").with_field(
        "transcript",
        FieldValue::Str(transcript.to_string_lossy().into_owned()),
    )
}

/// What the record holds and what it lost, once the pipe has closed.
///
/// `missed` counts a row the queue refused **and** a row the disk refused; only
/// the first can also appear in the file, which is where the two differ.
///
/// **`step_id` is the step the Drone was on when the pipe closed**, which is
/// not the one it opened on for any Job that advanced. The opening line is what
/// carries the step it started under.
fn closed(spine: &Spine, step: &StepId, at: Timestamp, rows: u64, missed: u64) -> Envelope {
    envelope(spine, step, at, "drone transcript closed")
        .with_field("rows", FieldValue::Int(rows as i64))
        .with_field("missed", FieldValue::Int(missed as i64))
}

/// **`msg` never carries an interpolated id.** The spine is fields, which is
/// what any query targets.
fn envelope(spine: &Spine, step: &StepId, at: Timestamp, msg: &str) -> Envelope {
    Envelope::new(at, Level::Info, Component::Fleet, spine.run.clone(), msg)
        .in_job(spine.job.as_ulid().clone())
        .by_drone(spine.drone.as_ulid().clone())
        .at_step(step.as_str())
}

/// Write one Fleet-authored line into a Job's log.
///
/// The Job log is otherwise written by the transcript's own writer task. This
/// is the other author: something Fleet observed about a Job that no Drone
/// event carries — a step editing outside its declared scope is the first.
pub fn note(repo_root: &str, job: &JobId, envelope: &Envelope) -> Result<(), io::Error> {
    let mut log = appending(&log_of(repo_root, job))?;
    write_line(&mut log, envelope)
}

/// Open for appending, making the directory if it is not there.
///
/// Append rather than truncate: a Job log outlives any one Drone, and a
/// transcript is named by an id nothing else will ever be named by.
fn appending(path: &Path) -> Result<std::fs::File, io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

fn write_line(file: &mut std::fs::File, envelope: &Envelope) -> Result<(), io::Error> {
    use std::io::Write;
    file.write_all(encoded(&Line::from(envelope))?.as_bytes())
}

async fn write_async(file: &mut File, envelope: &Envelope) -> Result<(), io::Error> {
    file.write_all(encoded(&Line::from(envelope))?.as_bytes())
        .await?;
    file.flush().await
}

async fn append(file: &mut File, row: &TranscriptRow) -> Result<(), io::Error> {
    file.write_all(encoded(row)?.as_bytes()).await?;
    // Flushed per row, not synced: what this survives is Fleet going away,
    // which is the thing that actually happens, rather than a power cut.
    file.flush().await
}

/// One JSON object and the newline after it, through `ipc`'s encoder rather
/// than a second one here.
fn encoded<T: serde::Serialize>(value: &T) -> Result<String, io::Error> {
    let mut line =
        ipc::encode(value).map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why.why))?;
    line.push('\n');
    Ok(line)
}
