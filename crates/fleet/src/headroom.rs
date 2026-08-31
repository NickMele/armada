//! What the machine has left, and whether it is enough to start another Drone.
//!
//! **This is the pre-spawn half and only that half.** A Job that has not
//! started and is short of headroom stays `queued` and the Board says
//! `waiting_on_resources`; a Job that exhausts CPU or memory *while running*
//! has nowhere to queue back to and escalates as `resource_exhausted`.
//! `docs/concepts/fleet.md` separates the two, `escalation-triggers.toml` types
//! the second, and nothing here raises it.
//!
//! # Three signals, and disk is the one that has bitten
//!
//! CPU and memory are what `settings.toml`'s headroom row names. Disk is here
//! from a measured failure rather than from symmetry: a volume filled during a
//! parallel agent run, 220 GB across 74 worktrees, and three agents died at
//! zero bytes free holding uncommitted work. Nothing saw it coming and the
//! first symptom was `ENOSPC`. A Job cuts a worktree and runs a build, so its
//! disk cost is the one of the three that is predictable in advance.
//!
//! Quota is **not** a fourth. `docs/spikes/005-what-does-a-job-cost.md` settled
//! it: the stream's rate-limit event carries a window and a status and no
//! quantity, so there is no number to hold a Job back against.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// What share of one resource is spoken for, in whole percent.
///
/// **Not capped at 100.** A machine with more runnable work than it has cores
/// is past its capacity, and rounding that to "full" would lose the difference
/// between busy and hopeless.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InUse(u32);

impl InUse {
    pub const fn percent(percent: u32) -> InUse {
        InUse(percent)
    }

    /// What is left. Never below none, whatever the reading was.
    pub const fn spare(&self) -> Spare {
        Spare(100u32.saturating_sub(self.0))
    }

    pub const fn percentage(&self) -> u32 {
        self.0
    }
}

/// The least share of a resource that must be spare before another Drone
/// starts. `cpu-mem-headroom-threshold-for-spawning` in
/// `crates/config/settings.toml`, which is where the row is argued.
///
/// **Its own type rather than a second [`InUse`]**, so a threshold cannot be
/// compared against a reading without one of them being turned into the other
/// first. The turning happens once, in [`Headroom::short_of`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spare(u32);

impl Spare {
    pub const fn percent(percent: u32) -> Spare {
        Spare(percent)
    }

    pub const fn percentage(&self) -> u32 {
        self.0
    }
}

/// A quantity of disk, held as bytes and named in the unit it was decided in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes(u64);

impl Bytes {
    pub const fn gibibytes(gibibytes: u64) -> Bytes {
        Bytes(gibibytes * 1024 * 1024 * 1024)
    }

    pub const fn kibibytes(kibibytes: u64) -> Bytes {
        Bytes(kibibytes * 1024)
    }

    pub const fn count(&self) -> u64 {
        self.0
    }
}

/// How stale a machine reading may be before it is taken again.
///
/// **The `fleet-health-check-resource-poll-interval` row**, spent as a
/// freshness bound rather than as a second timer. A background poll and a
/// lazily refreshed reading are the same number and the same staleness; the
/// bound costs nothing while Fleet is idle, and takes its reading at the moment
/// somebody asks rather than a fraction of an interval before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Polling(Duration);

impl Polling {
    pub const fn every(interval: Duration) -> Polling {
        Polling(interval)
    }

    pub const fn interval(&self) -> Duration {
        self.0
    }
}

/// What the machine had left, at one moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    cpu: InUse,
    memory: InUse,
    disk_free: Bytes,
}

impl Reading {
    pub const fn of(cpu: InUse, memory: InUse, disk_free: Bytes) -> Reading {
        Reading {
            cpu,
            memory,
            disk_free,
        }
    }

    pub const fn cpu(&self) -> InUse {
        self.cpu
    }

    pub const fn memory(&self) -> InUse {
        self.memory
    }

    pub const fn disk_free(&self) -> Bytes {
        self.disk_free
    }
}

/// Which resource the machine has too little of.
///
/// **Not a `QueuedReason`.** `job-statuses.toml` gives `queued` exactly three
/// labels — `blocked_by_dependency`, `waiting_on_resources`, `none` — and every
/// one of these folds to the second. This is the operator's distinction, not
/// the Board's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Short {
    Cpu,
    Memory,
    Disk,
}

impl std::fmt::Display for Short {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Short::Cpu => "cpu",
            Short::Memory => "memory",
            Short::Disk => "disk",
        })
    }
}

/// How much of the machine has to be free before another Drone starts.
///
/// **No `Default`**, for [`Concurrency`](crate::Concurrency)'s reason: the
/// numbers are a decision somebody made and wrote down, and a type that
/// supplies them lets a caller not make it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Headroom {
    spare: Spare,
    disk: Bytes,
}

impl Headroom {
    /// One share for CPU and memory, and an absolute floor for disk.
    ///
    /// **Disk is not a share, and that is the finding.** A fraction of the
    /// volume is the wrong unit for a cost that is absolute: a tenth of a 4 TB
    /// disk holds 400 GB in reserve against a worktree that needs three, and a
    /// tenth of a 60 GB disk leaves less than one build. What a Job needs is a
    /// number of gigabytes, so that is what it is held against.
    pub const fn of(spare: Spare, disk: Bytes) -> Headroom {
        Headroom { spare, disk }
    }

    /// Which resource is too short to start another Drone on, or `None` where
    /// there is enough of all three.
    ///
    /// **Disk first.** All three refuse the same dispatch, so the order decides
    /// only which one is named — and disk is the one that has actually run out
    /// here, the one a person can act on, and the one whose exhaustion destroys
    /// work rather than slowing it.
    pub fn short_of(&self, reading: &Reading) -> Option<Short> {
        if reading.disk_free() < self.disk {
            return Some(Short::Disk);
        }
        if reading.cpu().spare() < self.spare {
            return Some(Short::Cpu);
        }
        if reading.memory().spare() < self.spare {
            return Some(Short::Memory);
        }
        None
    }
}

/// Where a reading comes from.
///
/// **A seam so a test can plant one**, exactly as `peers` is: the shipped
/// answer is [`TheMachine`], and a fixture has no machine it can hold still.
///
/// **`None` is admit, not refuse.** A machine that cannot be read must not hold
/// every Job back for ever with nothing saying why — that is a Fleet that looks
/// dead. The concurrency bound still holds, and a failed reading is retried at
/// the next poll rather than at the next ask.
pub trait Machine: Send + Sync {
    fn read(&self) -> Option<Reading>;
}

/// The machine Fleet is running on, read by asking the shell.
///
/// # Why it shells out, and why there is no fourth platform difference
///
/// `crate::process` sets the precedent and records the argument: `ps` is one
/// spelling on both platforms, needs no `unsafe` and no platform crate, and is
/// a spelling the Node side can run too. All three readings here hold to it —
/// `uptime`, `ps -A -o %mem=` and `df -P -k` are POSIX and behave the same on
/// darwin and on Linux.
///
/// The alternatives do not. `vm_stat` is darwin's and `free` is Linux's;
/// `sysinfo` is a platform crate in the crate that spawns Drones; and
/// `crate::peer::kernel` is the workspace's third platform dependency and is
/// counted as such in `docs/contracts/adapters.md`. Headroom is a number that
/// is approximate by nature, so nothing here could have justified being the
/// fourth.
pub struct TheMachine {
    volume: PathBuf,
    cores: usize,
}

impl TheMachine {
    /// Read the volume `volume` sits on. **Fleet's repository root**, because
    /// every worktree is cut beneath it — the disk that fills is the one the
    /// work is on, not the one the daemon's binary is on.
    pub fn watching(volume: impl Into<PathBuf>) -> TheMachine {
        TheMachine {
            volume: volume.into(),
            cores: std::thread::available_parallelism()
                .map(|cores| cores.get())
                .unwrap_or(1),
        }
    }
}

impl Machine for TheMachine {
    fn read(&self) -> Option<Reading> {
        let cpu = load_in_use(&said(&mut Command::new("uptime"))?, self.cores)?;
        let memory = memory_in_use(&said(Command::new("ps").args(["-A", "-o", "%mem="]))?)?;
        let free = disk_free(&said(
            Command::new("df").args(["-P", "-k"]).arg(&self.volume),
        )?)?;
        Some(Reading::of(cpu, memory, free))
    }
}

/// What a command said, or nothing where it did not run or did not succeed.
fn said(command: &mut Command) -> Option<String> {
    let out = command.output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The one-minute load average as a share of the cores there are.
///
/// **The one-minute figure and not the five.** It is the one that reflects a
/// Check that started thirty seconds ago; the cost is that it also lags a
/// Drone Fleet has just spawned by up to a minute, which at a bound of two
/// means the second Drone is admitted against a reading that does not yet
/// include the first.
///
/// Parsed off the end of `uptime` rather than out of the middle, because the
/// line differs: darwin writes `load averages: 1.2 1.1 1.0` and Linux writes
/// `load average: 1.20, 1.10, 1.00`. The last three fields are the averages on
/// both, and a trailing comma is the only other difference.
pub(crate) fn load_in_use(said: &str, cores: usize) -> Option<InUse> {
    let fields: Vec<&str> = said.split_whitespace().collect();
    let first_of_three = fields.len().checked_sub(3)?;
    let one_minute: f64 = fields[first_of_three].trim_end_matches(',').parse().ok()?;
    if !one_minute.is_finite() || one_minute < 0.0 {
        return None;
    }
    let cores = cores.max(1) as f64;
    Some(InUse::percent((one_minute / cores * 100.0).round() as u32))
}

/// Physical memory held by processes, summed off `ps -A -o %mem=`.
///
/// **It over-reads, and the threshold is set knowing that.** A page shared
/// between processes is counted once per process, so the sum runs well above
/// what the operating system itself would call used: on one reading here it
/// said 64% against darwin's own 26%. What it is good for is direction — it
/// rises as memory fills and never the other way — so it is a backstop rather
/// than the signal that ordinarily fires. That was the price of not putting
/// `vm_stat` and `free` behind a platform fork.
///
/// **An unreadable line is skipped, not fatal.** `ps` walks a live process
/// table and a process that exits mid-walk can leave a short one.
pub(crate) fn memory_in_use(said: &str) -> Option<InUse> {
    let mut total = 0.0f64;
    let mut seen = 0usize;
    for line in said.lines() {
        if let Ok(share) = line.trim().parse::<f64>() {
            total += share;
            seen += 1;
        }
    }
    (seen > 0).then(|| InUse::percent(total.round().max(0.0) as u32))
}

/// Bytes free on the volume, off the fourth field of `df -P -k`.
///
/// `-P` is what makes the fourth field the fourth field: without it a long
/// device name wraps onto its own line and the columns move. `-k` fixes the
/// block size at 1024 on both platforms, which darwin otherwise takes from the
/// environment.
pub(crate) fn disk_free(said: &str) -> Option<Bytes> {
    let last = said.lines().rfind(|line| !line.trim().is_empty())?;
    let available: u64 = last.split_whitespace().nth(3)?.parse().ok()?;
    Some(Bytes::kibibytes(available))
}
