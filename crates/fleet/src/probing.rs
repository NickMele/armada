//! The probes Fleet can run on itself, and the Doctor modules it cannot.
//!
//! **Four rows of Doctor's ten.** The other six are probed from `adapters` and
//! `config`, and Doctor is not built — `docs/concepts/doctor.md`. They are
//! named in the answer rather than left out, because four green rows read as a
//! healthy machine.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FleetHealth, Probe, Unprobed};

use crate::daemon::Fleet;

/// Doctor's three words. `docs/concepts/doctor.md`, *Result vocabulary*.
const PASS: &str = "pass";
const WARN: &str = "warn";
const FAIL: &str = "fail";

fn probe(module: &str, outcome: &str, detail: String) -> Probe {
    Probe {
        module: module.to_string(),
        outcome: outcome.to_string(),
        detail,
    }
}

/// The probes Fleet holds none of, grouped by who owns them.
///
/// **Named by owner rather than one by one.** Which modules those are is
/// `docs/concepts/doctor.md`'s, and spelling them here would put a vendor's
/// name in a crate that is not `adapters`.
fn elsewhere() -> Vec<Unprobed> {
    [
        (
            "adapters",
            "it owns talking to anything outside Armada, and Doctor's four probes of those              live there. `docs/concepts/doctor.md` names them; none is built",
        ),
        (
            "config",
            "it reads and validates its own files, and Doctor's two probes of those live              there. Neither is built, and nothing in this workspace parses a Kit at all",
        ),
        (
            "Bridge",
            "the Armada API row is not a probe: it is a client's own connection state, and              only the client holding it can answer",
        ),
    ]
    .into_iter()
    .map(|(owner, because)| Unprobed {
        owner: owner.to_string(),
        because: because.to_string(),
    })
    .collect()
}

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
    /// `get_health` — every probe Fleet can answer, and every one it cannot.
    ///
    /// **It never refuses.** A probe that could not read is a `fail` row
    /// carrying why; a health check that 500s is the thing being unhealthy.
    pub(crate) async fn health(&self) -> Result<FleetHealth, Refusal> {
        let mut probes = vec![probe(
            "Fleet",
            PASS,
            // Answering is the probe. `docs/concepts/doctor.md` reaches the
            // same row through a pidfile because Doctor asks from outside;
            // from in here, the reply is the evidence.
            format!("answering — this process is run {}", self.run_id().as_str()),
        )];
        probes.push(self.store_probe().await);
        probes.push(self.manifest_probe());
        probes.push(self.machine_probe().await);
        Ok(FleetHealth {
            probes,
            not_probed: elsewhere(),
        })
    }

    /// `armada.db` opens and the Jobs read back. A row that would not load is
    /// a `warn`: the file is readable and something in it is not.
    async fn store_probe(&self) -> Probe {
        match self.every_job().await {
            Ok((loaded, unreadable)) if unreadable.is_empty() => probe(
                "SQLite",
                PASS,
                format!("opens; {} Jobs read back", loaded.jobs.len()),
            ),
            Ok((loaded, unreadable)) => probe(
                "SQLite",
                WARN,
                format!(
                    "opens; {} Jobs read back and {} rows would not",
                    loaded.jobs.len(),
                    unreadable.len()
                ),
            ),
            Err(why) => probe("SQLite", FAIL, why.to_string()),
        }
    }

    /// What Fleet's last re-read of `armada.yml` came to.
    ///
    /// **No reading is a `pass`**, not a gap: Fleet is running on the file it
    /// booted with, which is a configuration in force.
    fn manifest_probe(&self) -> Probe {
        let path = self.manifest().path().display().to_string();
        match self.last_reading() {
            None => probe("Manifest", PASS, format!("{path}, as Fleet booted with it")),
            Some(reading) => match reading.refused {
                None => probe("Manifest", PASS, format!("{path}, re-read and adopted")),
                Some(refused) => probe(
                    "Manifest",
                    WARN,
                    format!(
                        "{path} — the last re-read did not take: {}. The configuration in \
                         force is the one before it",
                        refused.summary
                    ),
                ),
            },
        }
    }

    /// CPU, memory and disk against the thresholds admission itself uses.
    ///
    /// **The cached reading, never a fresh one.** A probe that took three
    /// processes on every call would make a health page cost more than the
    /// thing it reports on, and admission already refreshes this on its own
    /// interval.
    async fn machine_probe(&self) -> Probe {
        let Some(reading) = self.machine_reading().await else {
            // **An unreadable machine admits rather than refuses**, which is
            // `get_capacity`'s rule, so this is not a failure of the fleet.
            return probe(
                "System stats",
                WARN,
                String::from("the machine would not answer; admission is going ahead anyway"),
            );
        };
        let said = format!(
            "cpu {}% in use, memory {}% in use, {} KiB free on the volume",
            reading.cpu().percentage(),
            reading.memory().percentage(),
            reading.disk_free().count()
        );
        match self.headroom().short_of(&reading) {
            None => probe("System stats", PASS, said),
            Some(short) => probe(
                "System stats",
                FAIL,
                format!("{said} — {short}, so nothing new is admitted"),
            ),
        }
    }
}
