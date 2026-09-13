//! Setup's apparatus for Reach: Scan's findings and the proposals built from them, as
//! they cross the wire. Beside `reach.rs` so neither file carries both halves.

use std::path::Path;

use adapters::ActionsWorkflows;
use config::Manifest;
use fleet::manifest_proposal::{propose, Draft};
use fleet::scanning::scan;
use ipc::{
    CiCommand, ManifestProposal, ProposalEdit, Provenance, RepositoryScan, ScannedWorkspace,
};

use super::{Held, CHECKOUT};

/// A scan as a picker receives it: through `ipc::encode` and back.
pub fn received(repository: &Held) -> RepositoryScan {
    let scanned = scan(CHECKOUT, repository, &ActionsWorkflows);
    let sent = ipc::encode(&scanned).expect("a scan that serialises");
    ipc::decode("a repository scan", sent.as_bytes()).expect("and reads back")
}

/// One CI finding as `(job, key, run, cell)`, beside [`super::CI_RUNS`].
pub fn ci_run(one: &CiCommand) -> (&str, &str, &str, Option<&str>) {
    (&one.job, &one.key, &one.run, one.cell.as_deref())
}

pub fn workspace<'a>(scan: &'a RepositoryScan, dir: &str) -> &'a ScannedWorkspace {
    scan.workspaces
        .iter()
        .find(|one| one.dir == dir)
        .unwrap_or_else(|| panic!("{dir} is a workspace"))
}

/// The storefront's proposals, built from its Scan as it crosses the wire.
pub fn proposals(repository: &Held) -> Vec<Draft> {
    propose(&received(repository))
}

/// A proposal as a sheet receives it: through `ipc::encode` and back.
pub fn as_sent(draft: &Draft) -> ManifestProposal {
    let sent = ipc::encode(&draft.answer()).expect("a proposal that serialises");
    ipc::decode("a manifest proposal", sent.as_bytes()).expect("and reads back")
}

pub fn proposal_at<'a>(proposals: &'a [ManifestProposal], dir: &str) -> &'a ManifestProposal {
    let found = proposals.iter().find(|one| one.dir == dir);
    found.unwrap_or_else(|| panic!("{dir} is proposed"))
}

/// Every line's provenance, with whether its placement is a guess. A port is evidence.
pub fn every_line(proposal: &ManifestProposal) -> Vec<(&Provenance, bool)> {
    let ports = proposal.ports.iter().map(|one| (&one.provenance, false));
    let checks = proposal.checks.iter().map(|one| (&one.provenance, true));
    let commands = proposal.commands.iter().map(|one| (&one.provenance, true));
    let setup = proposal.setup.iter().map(|one| (&one.provenance, true));
    ports.chain(checks).chain(commands).chain(setup).collect()
}

pub fn convention(file: &str, key: Option<&str>) -> Provenance {
    Provenance::Convention {
        file: file.to_string(),
        key: key.map(str::to_string),
    }
}

pub fn read_from(file: &str, key: &str) -> Provenance {
    Provenance::Read {
        file: file.to_string(),
        key: key.to_string(),
    }
}

/// A person's corrections to the shop as a surface sends them, toward the journey's own `e2e`.
pub fn toward_the_journeys_e2e() -> Vec<ProposalEdit> {
    ipc::decode("a person's edits", TOWARD_THE_JOURNEYS_E2E.as_bytes()).expect("edits that decode")
}

/// `test` is re-sent exactly as proposed, so it should keep its citation.
const TOWARD_THE_JOURNEYS_E2E: &str = r#"[
  {"edit": "command", "name": "migrate", "run": "pnpm prisma migrate deploy"},
  {"edit": "command", "name": "seed", "run": "pnpm tsx scripts/seed.ts"},
  {"edit": "check", "name": "e2e", "run": "pnpm playwright test", "requires": ["migrate", "seed"]},
  {"edit": "move", "name": "lint"},
  {"edit": "port", "name": "dev", "container": 3000, "env": "PORT"},
  {"edit": "policy", "key": "auto_merge", "value": "checks-pass"},
  {"edit": "check", "name": "test", "run": "pnpm run test"}
]"#;

/// The text Write would put down, loaded where it would be.
pub fn loads(proposal: &ManifestProposal) -> Manifest {
    let Some(text) = &proposal.text else {
        panic!("{} is refused: {:?}", proposal.file, proposal.refused);
    };
    let at = Path::new(CHECKOUT).join(&proposal.file);
    Manifest::parse(&at, text).unwrap_or_else(|why| panic!("{} loads: {why}", proposal.file))
}

/// The provenance of the Check or Command named `name`.
pub fn provenance_of(proposal: &ManifestProposal, name: &str) -> Provenance {
    for check in &proposal.checks {
        if check.name == name {
            return check.provenance.clone();
        }
    }
    for command in &proposal.commands {
        if command.name == name {
            return command.provenance.clone();
        }
    }
    panic!("no line named {name}")
}

pub fn fault_keys(proposal: &ManifestProposal) -> Vec<&str> {
    let faults = proposal.refused.iter().flat_map(|one| &one.faults);
    faults.map(|one| one.key.as_str()).collect()
}

/// Every file one workspace's findings cite, what it did not read included.
pub fn cited(one: &ScannedWorkspace) -> Vec<&String> {
    let found = one
        .manifests
        .iter()
        .chain(&one.lockfiles)
        .map(|found| &found.file);
    let runnables = one.runnables.iter().map(|found| &found.file);
    let tools = one.tools.iter().map(|found| &found.file);
    let services = one.services.iter().map(|found| &found.file);
    let ports = one.ports.iter().map(|found| &found.file);
    let unread = one.not_read.iter().map(|found| &found.file);
    found
        .chain(runnables)
        .chain(tools)
        .chain(services)
        .chain(ports)
        .chain(unread)
        .collect()
}

/// Every port a scan found, as `(file, key, container)`.
pub fn declared_ports(scan: &RepositoryScan) -> Vec<(&str, &str, u16)> {
    let every = scan.workspaces.iter().flat_map(|one| &one.ports);
    every
        .map(|port| (port.file.as_str(), port.key.as_str(), port.container))
        .collect()
}

/// Each workspace's evidence strength, by directory.
pub fn strengths(scan: &RepositoryScan) -> Vec<(&str, ipc::EvidenceStrength)> {
    scan.workspaces
        .iter()
        .map(|one| (one.dir.as_str(), one.evidence))
        .collect()
}

/// One workspace's runnable, by name.
pub fn runnable<'a>(one: &'a ScannedWorkspace, name: &str) -> &'a ipc::Runnable {
    let found = one.runnables.iter().find(|runnable| runnable.name == name);
    found.unwrap_or_else(|| panic!("{} runs {name}", one.dir))
}
