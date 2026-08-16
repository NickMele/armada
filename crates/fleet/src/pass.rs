//! **One `armada fleet tick` pass at a time, machine-wide.**
//!
//! # Why this exists at all
//!
//! `020` §1 makes every Drone's `Stop` hook tick the fleet, and `020` §2 makes
//! that tick fleet-wide rather than Job-scoped, because a sweep is the only
//! thing that rescues a Job whose own relay was lost. The cost of that decision
//! is concurrency: on a machine with five Jobs, five exchanges ending in the
//! same second start five passes over the same records.
//!
//! **Two passes gating one step is not a race that loses a write — it is two
//! `claude --resume` calls against one session.** A Job's conversation is one
//! Claude Code session across every step, and resuming it twice is the failure
//! neither process can detect. So the pass is serialised.
//!
//! # Why a lock file and not a lease
//!
//! [`armada_core::lease`] is the claim/lease reducer, and it is deliberately
//! confined to the check scheduler and Manifest's ownership store
//! (`ARCHITECTURE.md` §1.2 — *"elsewhere a plain function is a plain
//! function"*). What that machinery buys is queueing, waiting policies and
//! `waiting_on` reporting; **a tick wants none of them.** A pass that cannot
//! get the lock should not queue — the pass already running is about to do the
//! same work, over the same records, and a second one behind it would only
//! repeat it. Declining is the correct answer, not a slower yes.
//!
//! # Staleness
//!
//! A pass that is SIGKILLed leaves its file behind, and a lock nobody can
//! release would stop the fleet forever — which is the failure `020` §2 is
//! about, reintroduced one layer down. So the file carries the boot it was
//! taken in and the moment it was taken, and a lock older than [`STALE_MS`] is
//! taken over rather than waited on. The number is generous: a pass over a
//! large fleet starts checks and reads transcripts, and stealing a lock from a
//! pass that is merely slow is the thing being avoided.

use armada_core::error::{ArmadaError, ErrClass};
use std::path::{Path, PathBuf};

/// How old a lock has to be before a later pass takes it over.
///
/// **Ten minutes, and it is a staleness bound rather than a deadline.** A pass
/// holds the lock while it starts `armada manifest check --detach` calls and
/// reads transcripts; none of that is slow, but a machine under load can make
/// anything slow, and the cost of stealing early — two Drones on one session —
/// is much worse than the cost of stealing late, which is a fleet that catches
/// up ten minutes after a crash instead of immediately.
pub const STALE_MS: u64 = 600_000;

/// A held pass. **Releases on drop**, including on the error paths — a pass
/// that returned early through `?` and left the lock behind is exactly the
/// stale lock this type exists to make rare.
#[derive(Debug)]
pub struct Pass {
    path: PathBuf,
}

impl Drop for Pass {
    fn drop(&mut self) {
        // Best effort: a lock that outlives its process is [`STALE_MS`]'s
        // problem and not worth failing a finished pass over.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Take the pass, or say who has it.
///
/// `Ok(None)` is **not an error** — it is the ordinary answer when a Drone's
/// `Stop` hook sweeps a fleet another sweep is already halfway through, which
/// on a busy machine is most of them. The caller reports it and returns
/// successfully: nothing was missed, because the pass that holds the lock is
/// walking the same records.
pub fn take(armada_home: &Path, boot_id: &str, now_ms: u64) -> Result<Option<Pass>, ArmadaError> {
    let path = crate::home::tick_lock(armada_home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| unwritable(&path, &error))?;
    }

    // **Read before write, and the read decides.** A lock whose stamp is fresh
    // is somebody else's pass; one whose stamp is stale, unreadable or from
    // another boot names a process that cannot still be running.
    if let Ok(held) = std::fs::read_to_string(&path) {
        if !is_stale(&held, boot_id, now_ms) {
            return Ok(None);
        }
    }

    // `create_new` is not used, deliberately: the file may already exist and be
    // stale, and this is the point at which it is being taken over. The window
    // between the read above and this write is the one two simultaneous *first*
    // sweeps could both pass through, and it is closed by `create_new` below
    // for the case that actually matters — an unheld lock.
    let stamp = format!("{boot_id} {now_ms}\n");
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(_) => {}
        // Somebody created it between the read and here. Either it is fresh —
        // in which case they hold it — or it is the stale one this pass decided
        // to take over, and taking it over is a plain write.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            match std::fs::read_to_string(&path) {
                Ok(held) if !is_stale(&held, boot_id, now_ms) => return Ok(None),
                _ => {}
            }
        }
        Err(error) => return Err(unwritable(&path, &error)),
    }
    std::fs::write(&path, &stamp).map_err(|error| unwritable(&path, &error))?;
    Ok(Some(Pass { path }))
}

/// Whether a lock's stamp names a pass that cannot still be running.
///
/// **A different boot is always stale.** The pid is not recorded and does not
/// need to be: a pass is short-lived, and a lock written before the machine
/// last started belongs to a process that is definitively gone.
fn is_stale(held: &str, boot_id: &str, now_ms: u64) -> bool {
    let mut parts = held.split_whitespace();
    let Some(boot) = parts.next() else {
        // An empty or truncated lock file is a crash mid-write, which is a
        // process that is not running.
        return true;
    };
    if boot != boot_id {
        return true;
    }
    match parts.next().and_then(|at| at.parse::<u64>().ok()) {
        Some(at) => now_ms.saturating_sub(at) >= STALE_MS,
        None => true,
    }
}

fn unwritable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot write {}: {error}", path.display()),
        next_action: Some("check the permissions on ~/.armada/".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordinary case: one pass takes it, a second declines, and the second
    /// gets it once the first is finished with it.
    #[test]
    fn one_pass_holds_it_and_the_next_waits_until_it_is_dropped() {
        let home = tempfile::TempDir::new().unwrap();
        let first = take(home.path(), "boot", 1_000).unwrap();
        assert!(first.is_some(), "the first pass did not get the lock");
        assert!(
            take(home.path(), "boot", 1_100).unwrap().is_none(),
            "two passes held the lock at once"
        );
        drop(first);
        assert!(
            take(home.path(), "boot", 1_200).unwrap().is_some(),
            "the lock was not released"
        );
    }

    /// **Inverted, and it fails without the staleness rule**
    /// (`ARCHITECTURE.md` §2.1.1): a lock a SIGKILLed pass left behind must not
    /// stop the fleet forever, which is the very failure `020` §2 is about.
    #[test]
    fn a_lock_left_behind_by_a_dead_pass_is_taken_over() {
        let home = tempfile::TempDir::new().unwrap();
        let held = take(home.path(), "boot", 0).unwrap();
        std::mem::forget(held); // the pass was SIGKILLed: nothing ran `Drop`.
        assert!(
            take(home.path(), "boot", STALE_MS - 1).unwrap().is_none(),
            "a lock younger than the bound was stolen"
        );
        assert!(
            take(home.path(), "boot", STALE_MS).unwrap().is_some(),
            "a stale lock stopped the fleet"
        );
    }

    /// A lock from a previous boot names a process that cannot exist, whatever
    /// its timestamp says — the same rule a Drone's [`Handle`] is checked under.
    ///
    /// [`Handle`]: armada_core::fleet::job::Handle
    #[test]
    fn a_lock_from_another_boot_is_always_stale() {
        let home = tempfile::TempDir::new().unwrap();
        let held = take(home.path(), "before", 0).unwrap();
        std::mem::forget(held);
        assert!(take(home.path(), "after", 1).unwrap().is_some());
    }

    /// A truncated lock is a crash mid-write, not a live pass.
    #[test]
    fn an_unreadable_lock_is_not_treated_as_held() {
        let home = tempfile::TempDir::new().unwrap();
        std::fs::write(crate::home::tick_lock(home.path()), "").unwrap();
        assert!(take(home.path(), "boot", 5).unwrap().is_some());
        std::fs::write(crate::home::tick_lock(home.path()), "boot notanumber").unwrap();
        assert!(take(home.path(), "boot", 5).unwrap().is_some());
    }
}
