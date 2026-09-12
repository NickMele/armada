//! The port Fleet's own listener binds, claimed from the same lease a Job's
//! span is claimed from.
//!
//! **Fleet's listener was the one port in the system nothing leased.** It came
//! from a constant whose own comment said nothing owned the value, so two
//! Fleets on one machine both bound it and the second stopped at
//! `Address already in use`. It is a claimant now, beside a Job and the main
//! checkout: [`PortClaimant::FleetListener`].
//!
//! **Claimed before the bind, which is why the claim is a free function.** The
//! port is what the listener binds and the claim is what says which port, so
//! it is taken before there is a [`Fleet`] to hang a method on — `armada::serve`
//! opens the store, calls [`claimed_listener_port`], and binds what it hands
//! back. The release below *is* a method, because by shutdown there is one.
//!
//! **A row is never trusted on its own.** A Fleet that died leaves its claim
//! behind, and the bind-and-connect probe is what decides whether the port it
//! names may be taken again — `docs/concepts/fleet.md`, *Claiming*. That is
//! also the whole of what keeps two Fleets apart: they have different homes and
//! therefore different stores, so neither can read the other's claim, and the
//! probe is the only thing that can see a port a live Fleet holds.

use core_model::Timestamp;
use store::{PortClaim, PortClaimant, Store};

use crate::ports::{pick_span, PortProbe, PortRange, PortsRefused};

/// How wide Fleet's own claim is.
///
/// **One, and it is the only claim in the system not sized from a Manifest's
/// `ports:`.** `api::router` serves the event stream, the query and command
/// surface and the MCP endpoint on a single axum listener, so there is one
/// port to hold rather than a span to divide up.
const LISTENER_WIDTH: u16 = 1;

/// The port Fleet's listener should bind, claimed and recorded.
///
/// **Reuse first, then a fresh claim.** A claim already in this store is this
/// store's own Fleet's, left by a start that did not reach its release — a
/// crash, or a refusal after the claim. Where the probe says the port it names
/// is still free it is taken again, which is what makes a crashed Fleet stop
/// blocking the next one. Where something answers on it, the row is released
/// and a fresh span is picked, so the port a live Fleet holds is never
/// double-booked.
///
/// **The range is the Job range.** `docs/concepts/fleet.md`, *The range*: the
/// ceiling is detected from the platform's ephemeral floor so that nothing
/// here hands out a port the kernel will also assign. Fleet's own port comes
/// from that same range rather than from a number chosen in this file.
pub fn claimed_listener_port<P: PortProbe>(
    store: &mut Store,
    range: PortRange,
    probe: &P,
    now: Timestamp,
) -> Result<u16, PortsRefused> {
    if let Some(port) = reusable(store, probe)? {
        return Ok(port);
    }
    // Read after the release above, so a stale row this start just gave back
    // is not counted as occupying the port it named.
    let occupied: Vec<(u16, u16)> = store
        .every_port_claim()
        .map_err(PortsRefused::Database)?
        .iter()
        .map(|claim| (claim.base, claim.width))
        .collect();
    let Some(base) = pick_span(range, LISTENER_WIDTH, &occupied, probe) else {
        return Err(PortsRefused::RangeExhausted {
            width: LISTENER_WIDTH,
        });
    };
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::FleetListener,
            base,
            width: LISTENER_WIDTH,
            claimed_at: now,
        })
        .map_err(PortsRefused::Write)?;
    Ok(base)
}

/// The port an existing claim names, where the probe says it is still free.
///
/// `None` where there is no claim, and `None` where the port it names is held
/// — in that case the row is released here rather than left, because a claim
/// on a port this Fleet is not going to bind is a span the range has lost.
fn reusable<P: PortProbe>(store: &mut Store, probe: &P) -> Result<Option<u16>, PortsRefused> {
    let Some(claim) = store
        .port_span_for_fleet_listener()
        .map_err(PortsRefused::Database)?
    else {
        return Ok(None);
    };
    if probe.free(claim.base) {
        return Ok(Some(claim.base));
    }
    store
        .release_port_span(&PortClaimant::FleetListener)
        .map_err(PortsRefused::Write)?;
    Ok(None)
}

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

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
    /// Give Fleet's own listener port back.
    ///
    /// **Called once, at shutdown, after teardown** — the same moment and from
    /// the same place as
    /// [`released_main_checkout_ports`](Fleet::released_main_checkout_ports),
    /// the composition root, for the same reason: there is no Job whose
    /// transition would carry it.
    ///
    /// **Best-effort.** The process is stopping, and a release that did not
    /// land leaves a row the next start reuses once the probe agrees the port
    /// is free — which is the crashed-Fleet path, already the one this module
    /// is built around.
    pub async fn released_listener_port(&self) {
        let _ = self
            .store()
            .lock()
            .await
            .release_port_span(&PortClaimant::FleetListener);
    }
}
