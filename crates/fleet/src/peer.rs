//! Which Drone is on the other end of a connection.
//!
//! # The Drone is not asked, and there is no scheme in which it could be
//!
//! The three tools take no job id **by name** — `ipc::mcp::tools` refuses a call
//! that invents one — because a value a Drone supplies is a value a Drone chose.
//! With one working slot Fleet needed no other answer: there was one caller, so
//! nothing had to ask who. With several there is, and every scheme in which the
//! *Drone holds* its identity was measured and none survives:
//! `docs/spikes/011-what-can-one-drone-reach.md` puts `Write` and
//! `Bash(cargo …:*)` together and gets arbitrary native code at the operator's
//! uid, which reads any same-uid file, any same-uid process's argv **and its
//! environment**, and opens any socket. A port per Drone, a socket path per
//! Drone, a token in a file, in the environment or in argv: all five are as
//! private as same-uid isolation on macOS, which is to say not private.
//!
//! **So the identity is the connection.** `docs/spikes/010` spawned two Drones
//! against one listener and one config file and told them apart by the process
//! at the far end of each connection — including a Drone that skipped the tool
//! entirely and `curl`ed a Job name of its choosing, where the payload said one
//! thing and the transport said another and the transport was right. A shell
//! cannot give a process an ancestry it does not have, so the property needs no
//! confinement, which matters because `docs/scope.md` declines to build one.
//!
//! # The port pair, and why the obvious lookup is wrong every time
//!
//! `docs/spikes/012-peer-identity-under-concurrency.md` is the measurement that
//! decides the shape of this file.
//!
//! **A local port number is not unique on a host.** Two processes may each hold
//! port 24101 as long as they are talking to different places. So a lookup keyed
//! on the peer's port alone — `lsof -nP -i TCP:<port>`, which is what spike 10
//! used — names the wrong live pid **deterministically**, whenever it happens to
//! scan the impostor first. It is not a rare race, it is the wrong question, and
//! a test that passes against it proves nothing.
//!
//! **The pair (local port, foreign port) is the right one.** `insi_fport` sits
//! four bytes from `insi_lport` in the same kernel record, and matching both was
//! right in every ordering the reproduction was run in. That is what
//! [`PeerOf::holds`] matches, and the pair test in `crate::tests::peer` is what
//! would fail if it stopped.
//!
//! # And it asks the pids Fleet already holds, rather than the machine
//!
//! Spike 12 timed the same answer four ways. `lsof -i TCP:<port>` is 64ms and
//! scans every process on the machine; bounding it to five pids saves a third
//! and no more, because what it costs is a process spawn and a kernel table
//! walk. `netstat` prints no loopback TCP rows at all on darwin 27 and is not a
//! route. `proc_pidfdinfo` over the pids the caller already holds is **22µs** —
//! between 240 and 2,900 times cheaper, with no subprocess — and it is the only
//! one of the four that can match the pair.
//!
//! `crate::process` asks `ps` and gives its reason: the check there is one
//! Bridge has to be able to make too. Nothing outside this process asks this
//! question, so that argument does not reach here, and this crate already
//! carries `libc` for `setsid`.
//!
//! # Failing absent, never wrong
//!
//! Spike 12 engineered 384 connections to lose their peer and **not one came
//! back naming another process**: a peer that does not outlive the lookup is
//! unidentifiable as *nothing*, because a dead process holds no socket and the
//! port it held is not yet anyone else's. That is the direction this has to fail
//! in, and it is why [`NotACaller`] is a refusal rather than a guess.
//!
//! It is also why the lookup is on the tool call rather than at accept: a call
//! being served is a connection that is open, so the deadline spike 12 measured
//! — about 2ms for this route — cannot be missed by the caller that matters.
//!
//! # A Drone that goes round the tool is refused rather than traced
//!
//! Spike 10's bypass — `curl` from the Drone's own Bash — arrives from a pid
//! whose *ancestry* runs back to the Drone but which is not the Drone. This asks
//! only about the pids Fleet spawned, so such a call matches nothing and is
//! refused. That is narrower than the ancestry walk the spike demonstrated and
//! it needs no walk: the sanctioned road is the tool, and a call that did not
//! come down the CLI's own connection is not one.

use std::collections::BTreeMap;

use api::Caller;
use core_model::JobId;

/// Why a call could not be attributed to a Drone.
///
/// **One variant, because there is one answer.** Whether the peer exited, or was
/// never a Drone, or was a `curl` the Drone started, the fact Fleet can state is
/// the same: nothing it spawned holds that connection. Splitting it would invite
/// a caller to treat one of them as softer, and none of them is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotACaller;

impl std::fmt::Display for NotACaller {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(
            "this call did not arrive on a connection held by a Drone this Fleet started, \
             so there is no Job to record it against",
        )
    }
}

impl std::error::Error for NotACaller {}

/// Which process is working which Job.
///
/// **A second place a pid is held, and it earns its keep.** The pid is also
/// inside the working slot, on the [`DroneSession`](crate::DroneSession) — and
/// it cannot be read from there to answer this question, because reading it
/// means taking that slot's lock, and the lookup does not know yet *which* slot
/// to take. Awaiting each of them in turn would put one Drone's tool call behind
/// another Drone's `cargo nextest`, which is the single working slot arriving
/// back under a new name.
///
/// It is written where the record's own `assigned_drone` is written — the spawn
/// puts a row in, the departure takes it out — so the two cannot drift without
/// a departure having gone unrecorded, which `crate::boundary` already refuses
/// to let happen quietly.
#[derive(Debug, Default)]
pub struct Drones(BTreeMap<JobId, u32>);

impl Drones {
    /// A Drone started on this Job, as this process.
    pub fn arrived(&mut self, job: &JobId, pid: u32) {
        self.0.insert(job.clone(), pid);
    }

    /// The Drone on this Job has gone.
    pub fn left(&mut self, job: &JobId) {
        self.0.remove(job);
    }

    /// Every Drone this Fleet is holding, as pid and Job.
    pub fn each(&self) -> Vec<(JobId, u32)> {
        self.0
            .iter()
            .map(|(job, pid)| (job.clone(), *pid))
            .collect()
    }
}

/// Whether a process holds a particular TCP connection.
///
/// **A trait so the matching can be tested against something other than the
/// kernel**, and implemented once — [`Kernel`] — because there is one right
/// answer and it is a syscall.
pub trait PeerOf: Send + Sync {
    /// Whether `pid` holds a TCP socket whose local port is `from` and whose
    /// foreign port is `to`.
    ///
    /// **Both, never one.** A local port is not unique on a host; see this
    /// module's header for the measurement, and for what matching on one alone
    /// gets wrong.
    fn holds(&self, pid: u32, from: u16, to: u16) -> bool;
}

/// Which Job a call belongs to, over the Drones handed in.
///
/// `None` where no Drone holds that connection — see [`NotACaller`], and see
/// the header for why absent is the only failure this can have.
pub fn attributed(
    caller: &Caller,
    served_on: u16,
    drones: &[(JobId, u32)],
    peers: &dyn PeerOf,
) -> Option<JobId> {
    let from = caller.port()?;
    drones
        .iter()
        .find(|(_, pid)| peers.holds(*pid, from, served_on))
        .map(|(job, _)| job.clone())
}

pub use self::kernel::Kernel;

#[cfg(target_vendor = "apple")]
mod kernel;

#[cfg(not(target_vendor = "apple"))]
mod kernel {
    /// Nothing to ask. Armada is a macOS application — see
    /// `docs/contracts/system-architecture.md` — and this exists so the
    /// workspace still builds where somebody is reading it rather than running
    /// it. **It answers `false` to everything**, which makes every call
    /// unattributable, which is the refusal rather than a wrong Job.
    #[derive(Debug, Default)]
    pub struct Kernel;

    impl super::PeerOf for Kernel {
        fn holds(&self, _pid: u32, _from: u16, _to: u16) -> bool {
            false
        }
    }
}
