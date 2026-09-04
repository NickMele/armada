//! Noticing that `armada.yml` changed, and reading it again.
//!
//! # Why the file is polled rather than subscribed to
//!
//! macOS's native answer is FSEvents, and it is a *directory tree*
//! subscription: a watch on the repository root delivers every write beneath
//! it. Beneath this root are `target/` and `.armada/worktrees/`, so one Drone
//! running `cargo build` would put tens of thousands of events through a filter
//! looking for one filename. Subscribing to the file instead is the other trap
//! — the subscription follows the inode, and the ordinary save is a write to a
//! temp file and a rename over the target, which leaves the watch attached to a
//! file nothing will write to again.
//!
//! [`notify::PollWatcher`] has neither problem. It re-resolves the path each
//! round, so an atomic replace is a change like any other, and it costs one
//! read of one small file every [`POLL`] — a number this module states rather
//! than a load it cannot bound. What a save looks like to it was measured
//! rather than assumed: see [`SETTLE`] for the shapes, and [`watch_every`] for
//! why the comparison is of contents and not of metadata.

use std::path::Path;
use std::time::Duration;

use config::{Adopted, LoadError, Moved, Reloads};
use core_model::Timestamp;
use ipc::{ManifestFault, ManifestMoved, ManifestReading, ManifestRefused};
use notify::{Config, PollWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// How often the Manifest is read and compared.
///
/// **One read of one small file, and that is the whole cost of this at rest.**
/// A second is far below the interval at which a person edits a file and far
/// above the interval at which one is written.
const POLL: Duration = Duration::from_secs(1);

/// How long the file must sit still before it is read.
///
/// **A save is not one event.** Four shapes, driven against this watcher on
/// macOS 27 and APFS:
///
/// | Save | What arrives |
/// |---|---|
/// | Write in place | one `Modify(Data)` |
/// | Write beside and rename over | one `Modify(Data)`, and the watch survives |
/// | Truncate, then write | two, **the first of an empty file** |
/// | Delete, then create | `Remove`, then `Create`, a stat error before each |
///
/// So this is longer than one poll round: a save split across two rounds
/// settles into one read rather than two, the first of them against a file that
/// is empty or not there.
const SETTLE: Duration = Duration::from_millis(1_500);

/// How long a burst may hold the read off, however busy the file is.
///
/// **A debounce with no ceiling is a read that a steady stream of events never
/// reaches**, which would be `#430`'s own defect arriving by a longer road. Ten
/// seconds is far beyond any save and far below the minute the issue describes
/// somebody waiting.
const LATEST: Duration = Duration::from_secs(10);

/// A live watch on one `armada.yml`. **Dropping it stops the watch**, which is
/// why the composition root holds it for as long as it serves.
pub struct Watching {
    /// Held only to keep it alive. Dropping the watcher closes the channel,
    /// which is what ends [`settling`].
    _watcher: PollWatcher,
    _settling: tokio::task::JoinHandle<()>,
}

/// Watch `reloads`'s file, and say what each re-read came to.
///
/// **`said` is handed the refusal too.** A file that no longer parses leaves
/// the last good configuration in force — `Reloads::reread` guarantees that —
/// and the whole value of noticing is that somebody is told, so the caller gets
/// both answers rather than only the good one.
pub fn watch(
    reloads: Reloads,
    said: impl FnMut(Result<Adopted, LoadError>) + Send + 'static,
) -> Result<Watching, notify::Error> {
    watch_every(reloads, POLL, SETTLE, said)
}

/// The same, at stated intervals. **A seam so a test can drive a real watcher
/// over a real file in well under a second** — the shipped numbers are
/// [`POLL`] and [`SETTLE`], and what a test about a rename is about is that the
/// rename is noticed and read once, not what the two constants are.
/// **Contents are compared, not metadata, and that was measured.** notify
/// compares metadata by default and its write-time comparison is at
/// whole-second resolution: a save 250ms after the previous write to the same
/// file produced no event at all, and the identical save 1.5s later produced
/// one. Two saves inside one second with a poll round between them would leave
/// the second invisible until a third. The file is a few kilobytes.
pub(crate) fn watch_every(
    reloads: Reloads,
    poll: Duration,
    settle: Duration,
    mut said: impl FnMut(Result<Adopted, LoadError>) + Send + 'static,
) -> Result<Watching, notify::Error> {
    let (changed, arrived) = mpsc::unbounded_channel();
    let mut watcher = PollWatcher::new(
        move |event: notify::Result<notify::Event>| {
            // **A stat error is not a change.** The poller reports one on every
            // round while the file is absent, and every real transition arrives
            // as an `Ok` beside it — the table above is that measurement. Taking
            // the errors as changes would mean a file deleted and left deleted
            // produced an event every round, so the settle window would never
            // close and the one thing worth saying would never be said.
            if event.as_ref().map(touches).unwrap_or(false) {
                let _ = changed.send(());
            }
        },
        Config::default()
            .with_poll_interval(poll)
            .with_compare_contents(true),
    )?;
    // **The file, not the directory.** A poller re-resolves the path each round,
    // so it survives the replace that would defeat an inode subscription, and
    // watching the directory instead would put every file in the repository root
    // through this filter for nothing.
    watcher.watch(reloads.path(), RecursiveMode::NonRecursive)?;
    let settling = tokio::spawn(settling(arrived, settle, LATEST, move || {
        said(reloads.reread())
    }));
    Ok(Watching {
        _watcher: watcher,
        _settling: settling,
    })
}

/// Whether an event is about the file being watched.
///
/// A poller only reports the path it was given, so this is belt-and-braces
/// against a future backend that reports siblings — and it is why the name is
/// compared rather than the whole path, which a rename reports differently on
/// each platform.
fn touches(event: &notify::Event) -> bool {
    event.paths.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| name == std::ffi::OsStr::new(crate::setup::MANIFEST))
    })
}

/// Read once per burst of events, after the burst has stopped — or at `latest`
/// after it started, whichever comes first.
///
/// **A separate function taking a channel and a closure**, so the debounce can
/// be tested without a filesystem, a watcher, or a clock anybody has to wait
/// on. `crate::tests::watching` drives it directly under paused time.
///
/// **Nothing is read when the channel closes.** That is the watcher being
/// dropped, which is Fleet stopping — adopting a value nothing will go on to
/// use would be work done on the way out.
pub(crate) async fn settling(
    mut arrived: mpsc::UnboundedReceiver<()>,
    quiet: Duration,
    latest: Duration,
    mut apply: impl FnMut() + Send + 'static,
) {
    while arrived.recv().await.is_some() {
        let by = tokio::time::Instant::now() + latest;
        loop {
            let until = by.min(tokio::time::Instant::now() + quiet);
            match tokio::time::timeout_at(until, arrived.recv()).await {
                // Another reading inside the window: the save is still going on,
                // unless it has been going on longer than anybody should wait.
                Ok(Some(())) if tokio::time::Instant::now() < by => continue,
                Ok(None) => return,
                // The window passed, or the ceiling did. Read it.
                Ok(Some(())) | Err(_) => break,
            }
        }
        apply();
    }
}

/// What the daemon says on its console about one re-read.
///
/// **Kept now that the same fact reaches Bridge.** `armada fleet start` in a
/// terminal is a real way to run this, and somebody watching that window should
/// not lose what it used to tell them because a second surface learned it. See
/// [`reading`] for the half that crosses the wire.
pub fn say(read: Result<Adopted, LoadError>, file: &Path) {
    match read {
        Ok(adopted) if adopted.is_quiet() => {}
        Ok(adopted) => {
            for moved in adopted.moved() {
                println!(
                    "{} changed: {moved} — from the next step boundary",
                    file.display()
                );
            }
            for frozen in adopted.at_restart() {
                // Named rather than swallowed. Somebody who edited `checks:`
                // under a running Fleet is owed the reason nothing happened,
                // which is the whole defect one section over.
                eprintln!(
                    "{} changed `{}`, which this Fleet read at start and will not read again \
                     until it is restarted",
                    file.display(),
                    frozen.as_str()
                );
            }
        }
        Err(why) => {
            // **The fleet keeps running on the last good configuration.** One
            // mistyped number is not grounds for stopping every Job.
            eprintln!("{why}");
            eprintln!("the configuration in force is unchanged; correct the file and save again");
        }
    }
}

/// One re-read, in the vocabulary the wire carries.
///
/// **The conversion lives here because this is where both halves are in
/// scope.** `docs/practices/protocol.md` puts a domain-to-DTO conversion at the
/// Fleet boundary and keeps `ipc` types out of the crates below it; `config`
/// knows nothing about a wire and `ipc` knows nothing about a Manifest, so the
/// composition root that holds both is where somebody has to decide, field by
/// field, what a person on another machine's screen gets to see.
///
/// **What is deliberately not carried is the file's contents.** A refusal names
/// keys and says what is wrong with each; it never quotes the document. An
/// `armada.yml` can hold a private base branch or a command line, and the whole
/// point of a DTO is that a new field in it is a decision rather than whatever
/// serde does by default.
///
/// The instant is passed in rather than read, which is [`ipc::Instant`]'s own
/// rule: nothing in that crate produces one.
pub fn reading(read: &Result<Adopted, LoadError>, file: &Path, at: Timestamp) -> ManifestReading {
    ManifestReading {
        path: file.display().to_string(),
        at: ipc::Instant::from(&at),
        moved: match read {
            Ok(adopted) => adopted.moved().iter().map(moved).collect(),
            // **A refusal moved nothing**, which is the fact beside the reason:
            // the previous values are still running.
            Err(_) => Vec::new(),
        },
        at_restart: match read {
            Ok(adopted) => adopted
                .at_restart()
                .iter()
                .map(|frozen| frozen.as_str().to_string())
                .collect(),
            Err(_) => Vec::new(),
        },
        refused: read.as_ref().err().map(refused),
    }
}

/// One live key's move. Both ends travel, so a message can say what it was
/// rather than that something was.
fn moved(moved: &Moved) -> ManifestMoved {
    ManifestMoved {
        key: moved.key.as_str().to_string(),
        before: moved.before,
        after: moved.after,
    }
}

/// Why a read did not take.
///
/// **`summary` is `LoadError`'s own sentence and is never rebuilt from
/// `faults`.** A file that is not YAML at all has no keys to attribute anything
/// to, and the parser's error is what carries the line and column — so the
/// prose is what gets somebody to the line, and the key list is what gets them
/// to the fields once there is a document to have fields.
fn refused(why: &LoadError) -> ManifestRefused {
    ManifestRefused {
        summary: why.to_string(),
        faults: why
            .refusals()
            .iter()
            .map(|refusal| ManifestFault {
                key: refusal.key.clone(),
                fault: refusal.fault.to_string(),
            })
            .collect(),
    }
}
