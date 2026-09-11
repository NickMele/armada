//! A span of ports per Job, claimed at worktree cut and released when the Job
//! ends. `docs/concepts/fleet.md`, *Ports*.
//!
//! **What this module owns, and what it hands off.** Sizing a claim from a
//! Manifest's declared ports, probing a candidate before it is handed out, and
//! resolving `${port.NAME}` in a Command string are all here. Persisting the
//! claim is `store::ports`'; rewriting a compose document's published ports is
//! the compose adapter's, out of scope — see the concept page's own *Compose*
//! section.
//!
//! **The wrong call is unspeakable rather than checked**, the pattern
//! `docs/practices/rust.md` names: [`PortClaimant`] is `store`'s enum, not two
//! nullable fields a caller here could set both or neither of, and
//! [`PortRange::of`] takes every one of its three numbers so a caller cannot
//! build a range with a granule of zero and a division by it later.

use std::collections::BTreeMap;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use config::Manifest;
use store::{PortClaim, PortClaimant};

/// The range a Job's port span is claimed from, and the unit its width rounds
/// up to. `settings.port-range-base`, `settings.port-range-ceiling` and
/// `settings.port-block-granule` in `crates/config/settings.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortRange {
    base: u16,
    ceiling: u16,
    granule: u16,
}

impl PortRange {
    /// Every field named at once. **No `Default` and no setter** — a range
    /// with a granule of zero would divide by it below, and a caller building
    /// one field at a time could stop before naming it.
    pub const fn of(base: u16, ceiling: u16, granule: u16) -> PortRange {
        PortRange {
            base,
            ceiling,
            granule,
        }
    }

    pub fn base(&self) -> u16 {
        self.base
    }

    pub fn ceiling(&self) -> u16 {
        self.ceiling
    }

    pub fn granule(&self) -> u16 {
        self.granule
    }
}

/// The platform's ephemeral port floor, minus one — `net.inet.ip.portrange.first`
/// via `sysctl` on macOS, `net.ipv4.ip_local_port_range` in `/proc` on Linux —
/// or [`FALLBACK_CEILING`] where the read fails.
///
/// **Never the value nobody measured.** `docs/concepts/fleet.md`, *The range*:
/// detection supplies the default and an explicit `settings.port-range-ceiling`
/// still wins, so this is only ever the composition root's fallback for that
/// setting, never read again once the daemon is up — `lifetime = "Daemon
/// start"` in `crates/config/settings.toml`.
pub fn detect_ceiling() -> u16 {
    platform_ceiling().unwrap_or(FALLBACK_CEILING)
}

/// One below Linux's ephemeral floor. **A constant that stores the answer, not
/// the rule** — see [`detect_ceiling`]'s own doc for why it is the fallback
/// and never the default.
const FALLBACK_CEILING: u16 = 32_767;

#[cfg(target_os = "macos")]
fn platform_ceiling() -> Option<u16> {
    // SAFETY: `sysctlbyname` writes at most `size_of::<libc::c_int>()` bytes
    // through `oldp`, and `len` is set to that size before the call and
    // passed by mutable reference, which is the contract that bounds the
    // write to the local's own size. `name` is a NUL-terminated C string built
    // from a literal, so no user input reaches this call. Nothing is read
    // afterwards but `first`, and only when the call reported success and a
    // full `c_int`'s worth of bytes.
    #[allow(unsafe_code)]
    unsafe {
        let name = c"net.inet.ip.portrange.first";
        let mut first: libc::c_int = 0;
        let mut len = std::mem::size_of::<libc::c_int>();
        let asked = libc::sysctlbyname(
            name.as_ptr(),
            (&mut first as *mut libc::c_int).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        );
        if asked == 0 && len == std::mem::size_of::<libc::c_int>() && first > 1 {
            Some((first - 1) as u16)
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn platform_ceiling() -> Option<u16> {
    let read = std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range").ok()?;
    let first = read.split_whitespace().next()?;
    let first: u32 = first.parse().ok()?;
    u16::try_from(first.checked_sub(1)?).ok()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn platform_ceiling() -> Option<u16> {
    None
}

/// A claim's width, sized from how many ports a Manifest declares and rounded
/// up to the granule. **Zero stays zero** — a repository that declares no
/// `ports:` claims nothing, rather than a Job everywhere else paying for a
/// granule's worth of ports it never names.
pub fn rounded_width(declared: usize, granule: u16) -> u16 {
    if declared == 0 || granule == 0 {
        return declared.min(u16::MAX as usize) as u16;
    }
    let declared = declared.min(u16::MAX as usize) as u16;
    let remainder = declared % granule;
    if remainder == 0 {
        declared
    } else {
        declared - remainder + granule
    }
}

/// Whether a span of `width` starting at `base` is entirely inside the range
/// and does not overlap any already-claimed span.
fn fits(range: PortRange, base: u16, width: u16, occupied: &[(u16, u16)]) -> bool {
    let Some(top) = base.checked_add(width - 1) else {
        return false;
    };
    if base < range.base() || top > range.ceiling() {
        return false;
    }
    !occupied
        .iter()
        .any(|&(other_base, other_width)| overlaps(base, width, other_base, other_width))
}

fn overlaps(base: u16, width: u16, other_base: u16, other_width: u16) -> bool {
    let end = base.saturating_add(width);
    let other_end = other_base.saturating_add(other_width);
    base < other_end && other_base < end
}

/// Whether a port answers to a connect after this process could bind it.
///
/// **Load-bearing, not defensive.** `docs/concepts/fleet.md`, *Claiming*: a
/// bind can succeed under `SO_REUSEADDR` while something is still tearing
/// down, so the connect is what tells "nothing is here" from "something is
/// still finishing" — teardown that silently failed, teardown never declared,
/// and a process outside every tree are otherwise indistinguishable.
pub trait PortProbe {
    /// `true` where the port is free to hand out.
    fn free(&self, port: u16) -> bool;
}

/// The real probe: bind, then try to connect, on loopback.
pub struct BindConnectProbe;

impl PortProbe for BindConnectProbe {
    fn free(&self, port: u16) -> bool {
        let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) else {
            return false;
        };
        let addr = match listener.local_addr() {
            Ok(addr) => addr,
            Err(_) => return false,
        };
        // A short timeout: nothing is expected to answer, and the ordinary
        // case is a fast refusal rather than a wait.
        let answered = TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok();
        drop(listener);
        !answered
    }
}

/// The first span of `width` in `range`, stepping by the granule, every port
/// in it probed free. `None` where nothing in the range fits.
pub fn pick_span<P: PortProbe>(
    range: PortRange,
    width: u16,
    occupied: &[(u16, u16)],
    probe: &P,
) -> Option<u16> {
    if width == 0 {
        return None;
    }
    let mut base = range.base();
    loop {
        if base.checked_add(width - 1).map(|top| top > range.ceiling()) != Some(false) {
            return None;
        }
        if fits(range, base, width, occupied)
            && (base..=base.saturating_add(width - 1)).all(|port| probe.free(port))
        {
            return Some(base);
        }
        base = match base.checked_add(range.granule().max(1)) {
            Some(next) => next,
            None => return None,
        };
    }
}

/// `ARMADA_PORT_<NAME>` — the name uppercased, and anything that is not
/// `[A-Z0-9_]` after uppercasing turned into `_`. **Every declared port name
/// gets one of these, `env` or not**, per `docs/concepts/manifest.md`, *Ports*:
/// "`env` exists because a command string covers one channel and there are
/// more" — this is the guaranteed form.
pub fn armada_port_env_name(name: &str) -> String {
    let mut out = String::from("ARMADA_PORT_");
    for ch in name.chars() {
        let upper = ch.to_ascii_uppercase();
        if upper.is_ascii_alphanumeric() || upper == '_' {
            out.push(upper);
        } else {
            out.push('_');
        }
    }
    out
}

/// Replace every `${port.NAME}` in `text` with the number `ports` holds for
/// `NAME`. **Plain substitution, not shell expansion** —
/// `checks_runner::run`'s own header rules that out for a Command string, and
/// this is the same rule one layer up: the replacement happens before the
/// string ever reaches a splitter that does not interpret `$`.
///
/// A `${port.NAME}` naming a port `ports` does not hold is left exactly as
/// written — the Manifest declares its own `ports:`, so an unresolved
/// reference is a typo to be read off the rendered command, not a value this
/// function has anything else to offer.
pub fn resolve_ports(text: &str, ports: &BTreeMap<String, u16>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${port.") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "${port.".len()..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match ports.get(name) {
                    Some(port) => out.push_str(&port.to_string()),
                    None => out.push_str(&rest[start..start + "${port.".len() + end + 1]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Why a claim could not be made.
#[derive(Debug)]
pub enum PortsRefused {
    /// Two ports in one Manifest declare the same `env`. Refused at claim
    /// time rather than at load, per `docs/concepts/manifest.md`, *Ports*: the
    /// check is a Job's whole Manifest set, and a config-load check scoped to
    /// one file cannot see across the set that matters.
    CollidingEnv { name: String },
    /// Every candidate span in the range was either occupied or failed the
    /// probe. **Never guessed past** — `docs/concepts/fleet.md`'s "never guess
    /// high" is the ceiling's rule and this is its consequence at claim time:
    /// a range with nothing left in it refuses rather than handing out a span
    /// the kernel might also assign.
    RangeExhausted { width: u16 },
    /// The store would not say what is occupied. Nothing was spawned.
    Database(store::LoadAllError),
    /// The claim itself would not write. Nothing was spawned.
    Write(store::WriteError),
}

impl std::fmt::Display for PortsRefused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortsRefused::CollidingEnv { name } => {
                write!(out, "two declared ports both name `env: {name}`")
            }
            PortsRefused::RangeExhausted { width } => {
                write!(out, "no span of {width} free ports was left in the range")
            }
            PortsRefused::Database(cause) => {
                write!(out, "what is already claimed could not be read: {cause}")
            }
            PortsRefused::Write(cause) => write!(out, "the claim would not write: {cause}"),
        }
    }
}

impl std::error::Error for PortsRefused {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PortsRefused::Database(cause) => Some(cause),
            PortsRefused::Write(cause) => Some(cause),
            PortsRefused::CollidingEnv { .. } | PortsRefused::RangeExhausted { .. } => None,
        }
    }
}

/// The declared ports of one Manifest, each named and given an
/// `ARMADA_PORT_<NAME>` plus its own `env` where it declares one. `None` where
/// two ports collide on the same `env`.
///
/// **Names, not numbers.** What a claim resolves those names to is a later
/// question — this only says which variable names a claim, once made, has to
/// fill in.
pub fn env_names(manifest: &Manifest) -> Result<BTreeMap<String, Vec<String>>, PortsRefused> {
    let mut seen = BTreeMap::new();
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for port_name in manifest.port_names() {
        let declared = manifest.port(&port_name).expect("just listed");
        let mut vars = vec![armada_port_env_name(&port_name)];
        if let Some(env) = declared.env() {
            if let Some(earlier) = seen.insert(env.to_string(), port_name.clone()) {
                let _ = earlier;
                return Err(PortsRefused::CollidingEnv {
                    name: env.to_string(),
                });
            }
            vars.push(env.to_string());
        }
        names.insert(port_name, vars);
    }
    Ok(names)
}

/// Every variable a claimed span sets, resolved to the numbers `ports` maps
/// each declared name to.
pub fn env_vars(
    names: &BTreeMap<String, Vec<String>>,
    ports: &BTreeMap<String, u16>,
) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    for (name, keys) in names {
        let Some(port) = ports.get(name) else {
            continue;
        };
        for key in keys {
            vars.push((key.clone(), port.to_string()));
        }
    }
    vars
}

/// The claimed span, read back as a name-to-port map — what
/// [`resolve_ports`] and [`env_vars`] both need and neither can build on its
/// own, since a claim carries only a base and a width and the names come from
/// the Manifest, in the order it declares them.
pub fn port_map(manifest: &Manifest, claim: &PortClaim) -> BTreeMap<String, u16> {
    manifest
        .port_names()
        .into_iter()
        .enumerate()
        .filter(|(offset, _)| (*offset as u32) < u32::from(claim.width))
        .map(|(offset, name)| (name, claim.base + offset as u16))
        .collect()
}

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Job;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
    /// Claim this Job's port span, sized from its Manifest's `ports:` and
    /// rounded to the granule. **A no-op where the Manifest declares none** —
    /// most repositories, and every fixture that plants no `ports:` of its
    /// own.
    ///
    /// Called once, at worktree cut — `crate::dispatch`'s own `create_worktree`
    /// call is the only one in the workspace, which is what makes "claim once
    /// per worktree" a property of the call site rather than of a record this
    /// method has to keep.
    ///
    /// **Escalates before returning, the same shape `crate::preparing::prepared`
    /// takes**: nothing was spawned, so `not_configurable` reads correctly —
    /// the Manifest's own `ports:` would not resolve to a usable span.
    pub(crate) async fn claimed_ports(&self, job: &Job) -> Result<(), Adrift> {
        if let Err(cause) = self.try_claim_ports(job).await {
            self.move_job(
                job,
                core_model::Target::Escalated(core_model::EscalationTrigger::NotConfigurable),
                core_model::Actor::Fleet,
            )
            .await?;
            return Err(Adrift::PortsRefused {
                job: job.id().clone(),
                cause,
            });
        }
        Ok(())
    }

    async fn try_claim_ports(&self, job: &Job) -> Result<(), PortsRefused> {
        let declared = self.manifest().port_names();
        if declared.is_empty() {
            return Ok(());
        }
        env_names(self.manifest())?;
        let width = rounded_width(declared.len(), self.port_range().granule());
        let occupied: Vec<(u16, u16)> = self
            .store()
            .lock()
            .await
            .every_port_claim()
            .map_err(PortsRefused::Database)?
            .iter()
            .map(|claim| (claim.base, claim.width))
            .collect();
        let Some(base) = pick_span(self.port_range(), width, &occupied, &BindConnectProbe) else {
            return Err(PortsRefused::RangeExhausted { width });
        };
        self.store()
            .lock()
            .await
            .claim_port_span(&PortClaim {
                claimant: PortClaimant::Job(job.id().clone()),
                base,
                width,
                claimed_at: self.now(),
            })
            .map_err(PortsRefused::Write)?;
        Ok(())
    }

    /// This Job's declared port names, resolved to the numbers its claim
    /// holds. Empty where it declared none, or claimed none.
    pub(crate) async fn port_map(&self, job: &Job) -> BTreeMap<String, u16> {
        let claim = self
            .store()
            .lock()
            .await
            .port_span_for_job(job.id())
            .ok()
            .flatten();
        match claim {
            Some(claim) => port_map(self.manifest(), &claim),
            None => BTreeMap::new(),
        }
    }

    /// Every variable this Job's claimed span sets: `ARMADA_PORT_<NAME>` and
    /// any declared `env`, for every process Fleet spawns in the worktree.
    pub(crate) async fn port_env(&self, job: &Job) -> Vec<(String, String)> {
        let names = match env_names(self.manifest()) {
            Ok(names) => names,
            // Already refused at claim time, which is upstream of every spawn
            // — a Job that reached one holds a valid claim or none at all.
            Err(_) => return Vec::new(),
        };
        let ports = self.port_map(job).await;
        env_vars(&names, &ports)
    }

    /// Release this Job's port span, if it holds one. **Best-effort**, the
    /// same reading [`crate::footprint::Fleet::kept_footprint`] gives a
    /// terminal Job's footprint: the move has already landed, and a Job that
    /// ended is over whether or not its span could be released cleanly. A
    /// claim a release missed is still reachable — `forget_job` takes it with
    /// the rest of the record.
    pub(crate) async fn released_ports(&self, job: &Job) {
        let _ = self
            .store()
            .lock()
            .await
            .release_port_span(&PortClaimant::Job(job.id().clone()));
    }
}
