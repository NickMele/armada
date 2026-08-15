//! `armada mcp serve` — the toolbelt Helm and a Drone reach Armada through
//! ([`docs/commands/helm/mcp.md`](../../../../docs/commands/helm/mcp.md)).
//!
//! **This layer is thin on purpose.** A command is parse → call into the core →
//! render, and *"the MCP server calls the same functions the CLI does"*
//! (`ARCHITECTURE.md` §1.3). A stateless server is that same shape with a
//! different renderer: the tool's arguments are the parse, [`crate::verbs`] is
//! the core, and [`answer`] is the renderer. **No tool handler decides
//! anything.** A handler that reimplemented a verb would be a second answer to
//! the same question, and the two would disagree the first time either changed.
//!
//! **Two toolbelts, and they are two types rather than one list with a
//! filter.** Helm gets the Fleet and Manifest verbs; a Drone gets three tools
//! and cannot spawn. A worker able to spawn workers is a fork bomb with a
//! budget, and a filter is a thing somebody eventually forgets to apply — so
//! [`Belt::Helm`] and [`Belt::Drone`] are separate handlers with separately
//! written routers, and there is no code path from one to the other.
//!
//! **Stateless.** The base protocol is *"stateless, self-contained requests,
//! per-request capability negotiation"* as of spec revision `2026-07-28`
//! (`docs/traps.md`), so nothing is held between calls: every request rebuilds
//! what it needs from [`World`], which is what the entrypoint already read.
//! Initialization has not vanished — extensions are still negotiated there —
//! but there is no session state of Armada's own.

pub mod answer;
pub mod drone;
pub mod helm;
pub mod world;

use armada_core::envelope::{Envelope, McpData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use rmcp::ServiceExt;

use crate::verbs::Output;
use world::World;

/// Which toolbelt this process serves.
///
/// **Decided by the machine, not by a flag.** A flag is a thing a registration
/// file can get wrong — and getting it wrong in one direction hands a Drone the
/// ability to spawn Drones. `ARMADA_JOB` is set by
/// [`armada_fleet::drone::job_env`] on every child of a Job and by nothing else,
/// so a Drone's own environment answers the question and cannot be silently
/// omitted the way a flag can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Belt {
    /// The orchestrator's: the Fleet verbs and the Manifest verbs.
    Helm,
    /// A Drone's: report, ask, and emit a verdict. **No spawn.**
    Drone,
}

impl Belt {
    /// Which belt this world calls for.
    pub fn decide(world: &World) -> Belt {
        match world.job() {
            Some(_) => Belt::Drone,
            None => Belt::Helm,
        }
    }

    /// The word, in the payload and on the screen.
    pub const fn word(self) -> &'static str {
        match self {
            Belt::Helm => "helm",
            Belt::Drone => "drone",
        }
    }
}

/// Serve until the transport closes.
///
/// **`--stdio` is the only transport, and it is the default.** The server is
/// started by whatever registered it and lives as long as that session — there
/// is no daemon to reach over a socket, and no second lifetime to manage.
///
/// Returns the envelope of a clean shutdown. A transport that would not start is
/// `environment`, which exits **6** — the code
/// [`docs/commands/reference.md`](../../../../docs/commands/reference.md) gives
/// it.
pub fn serve(world: World) -> Result<Output, ArmadaError> {
    let belt = Belt::decide(&world);
    let tools = tool_names(belt);

    // **A current-thread runtime, built here and dropped here.** The verbs
    // underneath are blocking by design — a `docker compose up` is a subprocess
    // — and a stateless server answering one stdio request at a time has
    // nothing to schedule across cores. Building it inside this function rather
    // than putting `#[tokio::main]` on `main` also keeps every other verb
    // free of a runtime it never enters.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ArmadaError {
            class: ErrClass::Environment,
            r#where: "runtime".to_string(),
            message: format!("the server's runtime would not start: {e}"),
            next_action: None,
        })?;

    runtime.block_on(async move {
        let transport = rmcp::transport::stdio();
        match belt {
            Belt::Helm => run(helm::Toolbelt::new(world), transport).await,
            Belt::Drone => run(drone::Toolbelt::new(world), transport).await,
        }
    })?;

    Ok(Output::Mcp(Box::new(Envelope::ok(
        "mcp serve",
        None,
        Status::Ok,
        McpData {
            transport: "stdio".to_string(),
            toolbelt: belt.word().to_string(),
            tools,
        },
    ))))
}

/// Hand one handler to the transport and wait for the peer to go away.
///
/// **Generic over the handler, so the two belts share the lifecycle and nothing
/// else.** Sharing the twenty lines that own the runtime is not the thing
/// §15.2's rule is about; sharing a tool list would be.
async fn run<H>(
    handler: H,
    transport: (tokio::io::Stdin, tokio::io::Stdout),
) -> Result<(), ArmadaError>
where
    H: rmcp::ServerHandler,
{
    let service = handler.serve(transport).await.map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "stdio".to_string(),
        message: format!("the MCP transport failed: {e}"),
        next_action: Some("this server is launched by whatever registered it".to_string()),
    })?;

    // **A peer that hangs up is a clean shutdown, not a failure.** The server's
    // whole lifetime is the session that started it, so the ordinary end of one
    // is the client closing stdin.
    service.waiting().await.map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "stdio".to_string(),
        message: format!("the MCP server did not stop cleanly: {e}"),
        next_action: None,
    })?;
    Ok(())
}

/// The tools a belt carries, for the shutdown envelope and for the help page.
///
/// **Two literal lists.** Deriving them from the routers would make the payload
/// agree with the code by construction and agree with the documented toolbelt by
/// luck; written out, a tool added to one belt and not to
/// [`docs/commands/helm/mcp.md`](../../../../docs/commands/helm/mcp.md) is a
/// diff somebody reads.
pub fn tool_names(belt: Belt) -> Vec<String> {
    let names: &[&str] = match belt {
        Belt::Helm => &helm::TOOLS,
        Belt::Drone => &drone::TOOLS,
    };
    names.iter().map(|name| (*name).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn world(env: &[(&str, &str)]) -> World {
        World {
            cwd: PathBuf::from("/scratch/repo"),
            home: PathBuf::from("/scratch/home"),
            inherited: env
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<String, String>>(),
            exe: PathBuf::from("/scratch/bin/armada"),
            boot_id: "boot".to_string(),
        }
    }

    #[test]
    fn a_session_with_no_job_gets_the_orchestrators_belt() {
        assert_eq!(Belt::decide(&world(&[])), Belt::Helm);
        assert_eq!(Belt::decide(&world(&[("ARMADA_JOB", "")])), Belt::Helm);
    }

    #[test]
    fn a_child_of_a_job_gets_the_drones_belt() {
        assert_eq!(
            Belt::decide(&world(&[("ARMADA_JOB", "tidy-otter")])),
            Belt::Drone
        );
    }

    /// **The rule this file exists to make structural.** Asserted on the lists
    /// as well as on the types, because the lists are what a reader checks the
    /// documentation against.
    #[test]
    fn a_drone_is_offered_nothing_that_spawns() {
        let drone = tool_names(Belt::Drone);
        assert!(!drone.is_empty());
        for tool in &drone {
            assert!(
                !tool.contains("spawn"),
                "a Drone was offered `{tool}`, which is a fork bomb with a budget"
            );
        }
        assert!(tool_names(Belt::Helm).contains(&"fleet.spawn".to_string()));
    }

    /// The two belts are disjoint apart from nothing at all: a tool is either
    /// the orchestrator's or the worker's, and one that drifted into both would
    /// be the filter this design refuses to have.
    #[test]
    fn the_two_belts_share_no_tool() {
        let helm = tool_names(Belt::Helm);
        for tool in tool_names(Belt::Drone) {
            assert!(!helm.contains(&tool), "`{tool}` is on both belts");
        }
    }
}
