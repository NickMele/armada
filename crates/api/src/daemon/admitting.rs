//! Who opened the agent's door, and what that caller may reach through it.
//! `#941`.
//!
//! **Placed, never claimed.** Nothing a caller sends says it is Helm: an argv
//! flag, a header or a name in its MCP configuration is something any agent on
//! the machine can copy (`docs/spikes/011-what-can-one-drone-reach.md`). The
//! daemon places the connection against the sessions it started, as
//! `fleet::peer` places a Drone's, so what the door narrows is decided by who
//! holds the socket.
//!
//! **The rule is the daemon's.** This crate cannot name `fleet::helm::may`, so
//! the answer arrives as a [`HelmReach`] the daemon decided row by row, and the
//! door only enforces it.

use ipc::door::{Reachable, DRAFTING, REACHABLE};

use crate::mcp::Caller;

/// The fifth surface: which caller a door call came from.
pub trait Admitting: Send + Sync + 'static {
    /// What the Helm session holding this call's connection may reach.
    ///
    /// **`None` is every other caller**, which the door answers exactly as it
    /// did before a Helm session could be told apart — a person's own agent
    /// session keeps every row the inventory marks `Yes`.
    fn helm_at(&self, caller: Caller) -> Option<HelmReach>;
}

/// What one Helm session may call through the door.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelmReach {
    may: Vec<&'static str>,
    otherwise: &'static str,
}

impl HelmReach {
    /// Decided over [`offerable`] by the daemon's own predicate. `otherwise`
    /// is why anything else is refused, said to the session.
    pub fn deciding(may: impl Fn(&Reachable) -> bool, otherwise: &'static str) -> HelmReach {
        HelmReach {
            may: offerable()
                .filter(|row| may(row))
                .map(|row| row.operation)
                .collect(),
            otherwise,
        }
    }

    pub fn may(&self, operation: &str) -> bool {
        self.may.contains(&operation)
    }

    /// The refusal a call outside the reach is answered with. **It names the
    /// operation**, so a session reads which act was the person's.
    pub fn refusing(&self, operation: &str) -> String {
        format!("`{operation}` is not Helm's to call: {}", self.otherwise)
    }
}

/// Who steered a Drone through `Commands::redirect_drone`.
///
/// **Two, because nothing else steers one**, so Fleet or a Drone recorded as
/// having redirected is not a value this can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redirector {
    /// A person, through Bridge or through their own agent's door session.
    Person,
    /// A Helm session Fleet is hosting, placed by its connection.
    Helm,
}

/// Every row a Helm session could be offered before its daemon decides: what
/// any agent is offered, and the `Drafts only` rows no other agent is.
pub fn offerable() -> impl Iterator<Item = &'static Reachable> {
    REACHABLE.iter().chain(DRAFTING.iter())
}
