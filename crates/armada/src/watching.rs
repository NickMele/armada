//! Noticing that `armada.yml` changed, and reading it again.
//!
//! # Why the file is polled rather than subscribed to
//!
//! macOS's native answer is FSEvents, and it is a *directory tree* subscription:
//! a watch on the repository root delivers every write beneath it. Beneath this
//! root are `target/` and `.armada/worktrees/`, so a single Drone running
//! `cargo build` would deliver tens of thousands of events to a filter looking
//! for one filename. Watching the file itself instead is the other trap — the
//! subscription follows the inode, and the common save on macOS is a write to a
//! temp file and a rename over the target, which leaves the watch attached to a
//! file nothing will ever write to again.
//!
//! [`notify::PollWatcher`] has neither problem. It re-resolves the path each
//! round, so an atomic replace is a change like any other, and it costs one
//! `stat` of one file every [`POLL`] — a number this module states rather than
//! a load it cannot bound. `notify` is depended on with no default features for
//! that reason: the native backends are not compiled in.
//!
//! # A save is rarely one event
//!
//! An editor writes twice, or truncates and writes, or writes-and-renames. Each
//! shape produces more than one reading, and one of the readings can be of a
//! half-written file. So nothing is read until the path has sat still for
//! [`SETTLE`], and then it is read once. See [`settling`].

use std::path::Path;
use std::time::Duration;

use config::{Adopted, LoadError, Reloads};
use notify::{Config, PollWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// How often the Manifest is stat-ed.
///
/// **One `stat` of one file, and that is the whole cost of this feature at
/// rest.** A second is far below the interval at which a person edits a file
/// and far above the interval at which one is written, so it neither wastes a
/// syscall nor delays the next Job.
const POLL: Duration = Duration::from_secs(1);

/// How long the file must sit still before it is read.
///
/// **Longer than one poll round**, so a save that a poll splits across two
/// rounds — a remove and then a create, which is what a rename over the target
/// looks like to a poller — settles into one read rather than two, the first of
/// them against a file that is not there.
const SETTLE: Duration = Duration::from_millis(1_500);

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
    mut said: impl FnMut(Result<Adopted, LoadError>) + Send + 'static,
) -> Result<Watching, notify::Error> {
    let (changed, arrived) = mpsc::unbounded_channel();
    let mut watcher = PollWatcher::new(
        move |event: notify::Result<notify::Event>| {
            // **A watch error is a change**, not a reason to stop: a poll round
            // that could not stat the file is exactly the moment the file is
            // being replaced, and the settle window turns both readings into
            // one read of whatever is there afterwards.
            if event.map(|seen| touches(&seen)).unwrap_or(true) {
                let _ = changed.send(());
            }
        },
        Config::default().with_poll_interval(POLL),
    )?;
    // **The file, not the directory.** A poller re-resolves the path each round,
    // so it survives the replace that would defeat an inode subscription, and
    // watching the directory instead would put every file in the repository root
    // through this filter for nothing.
    watcher.watch(reloads.path(), RecursiveMode::NonRecursive)?;
    let settling = tokio::spawn(settling(arrived, SETTLE, move || said(reloads.reread())));
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

/// Read once per burst of events, after the burst has stopped.
///
/// **A separate function taking a channel and a closure**, so the debounce can
/// be tested without a filesystem, a watcher or a clock that has to be waited
/// on. `crate::tests::watching` drives it directly.
///
/// **Nothing is read when the channel closes.** That is the watcher being
/// dropped, which is Fleet stopping — adopting a value nothing will go on to
/// use would be work done on the way out.
async fn settling(
    mut arrived: mpsc::UnboundedReceiver<()>,
    quiet: Duration,
    mut apply: impl FnMut() + Send + 'static,
) {
    while arrived.recv().await.is_some() {
        loop {
            match tokio::time::timeout(quiet, arrived.recv()).await {
                // Another reading inside the window: the save is still going on.
                Ok(Some(())) => continue,
                Ok(None) => return,
                // The window passed with nothing further. Read it.
                Err(_) => break,
            }
        }
        apply();
    }
}

/// What the daemon says on its console about one re-read.
///
/// **Fleet has no log of its own** — every `core_model::Envelope` in this
/// workspace is written into one Job's transcript, and a Manifest reload
/// belongs to no Job. So this is the daemon's console, which is where every
/// other Fleet-level fact in `crate::serve` goes.
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
