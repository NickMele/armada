//! An `AgentHarness` that renders something harmless.
//! **Faithful: the seam.** It takes a real `DroneSpawnConfig`, answers with a
//! real `Launch` and records what it was asked for, so a test can start a Drone
//! end to end through `fleet::drone::start` and read back what was rendered.
//! What a test wants to ask of the real harness is a question about a `Launch`.
//!
//! **Not faithful: the argument list.** It renders a program that is not an
//! agent, because a test must never start one — a real spawn costs money, needs
//! a network and needs a credential, and a suite with any of those in it is a
//! suite people stop running. Whether the *real* argument list carries the
//! flags confining a Drone is asserted in `adapters`, with no process at all.
//!
//! **Not faithful: reading.** It does not decode — turning a line of a vendor's
//! stream into an event is that vendor's schema, and belongs to `adapters`,
//! the only crate permitted to read untyped bytes on this path.
//! [`FakeHarness::read`] answers with the line as prose unless a test scripted
//! something, which is enough for a caller that only needs *an* event.
//!
//! **The shell adds three variables of its own.** Every constructor below that
//! reports something runs `/bin/sh`, and a POSIX shell sets `PWD`, `SHLVL` and
//! `_` for itself after it starts. A test asserting on what a Drone got has to
//! say so rather than count lines, and a program adding nothing would have to
//! print and exit, which races the first turn's write. The constructors are
//! named for behaviour, not for the program: a test about a Drone that dies
//! before it is told anything should say that, not `/usr/bin/true`.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, DroneEvent, DroneSpawnConfig, Launch};

/// A harness that renders a harmless program.
///
/// `Mutex` rather than `RefCell` for the reason `FakeVcs` gives: a Fleet is
/// `Sync`, so the seams it holds have to be.
#[derive(Debug)]
pub struct FakeHarness {
    program: String,
    args: Vec<String>,
    refusing: Option<&'static str>,
    scripted: BTreeMap<String, Vec<DroneEvent>>,
    rendered: Mutex<Vec<Launch>>,
    configured: Mutex<Vec<DroneSpawnConfig>>,
}

impl FakeHarness {
    /// A Drone that holds its input open and echoes every turn it is given.
    ///
    /// The default, because it is the one that stays alive long enough for a
    /// test to speak to it.
    pub fn that_listens() -> FakeHarness {
        FakeHarness::running("/bin/cat", &[])
    }

    /// A Drone that is gone before Fleet can write to it.
    ///
    /// The case v1 could not survive: with SIGPIPE at its default the write
    /// that follows killed the parent instead of returning an error.
    pub fn that_exits_immediately() -> FakeHarness {
        FakeHarness::running("/usr/bin/true", &[])
    }

    /// A Drone that prints its own environment, then reads its first turn and
    /// exits.
    ///
    /// The only way to answer "what did the child actually get" from outside
    /// it — nothing else in the workspace can tell an environment that was
    /// built from one that was inherited.
    ///
    /// **It reads before it exits, deliberately.** A program that printed and
    /// exited would race the first turn's write, and the test would pass or
    /// fail on pipe buffering rather than on the thing it is about.
    pub fn that_reports_its_environment() -> FakeHarness {
        FakeHarness::running("/bin/sh", &["-c", "env; read _"])
    }

    /// A Drone that prints its own working directory, then reads and exits.
    pub fn that_reports_where_it_is() -> FakeHarness {
        FakeHarness::running("/bin/sh", &["-c", "pwd; read _"])
    }

    /// A Drone that reads its first turn, prints it back, and exits.
    ///
    /// Distinct from [`FakeHarness::that_listens`], which never exits: a test
    /// that wants to read the whole of what arrived needs the far end to close,
    /// and one that wants a session still alive needs it not to.
    pub fn that_echoes_its_first_turn() -> FakeHarness {
        FakeHarness::running(
            "/bin/sh",
            &["-c", "IFS= read -r line; printf '%s\\n' \"$line\""],
        )
    }

    /// Some other program. Named `running` rather than `at`, so a reader can
    /// tell a fake standing in for a Drone from the real harness's own
    /// constructor.
    pub fn running(program: &str, args: &[&str]) -> FakeHarness {
        FakeHarness {
            program: String::from(program),
            args: args.iter().map(|a| String::from(*a)).collect(),
            refusing: None,
            scripted: BTreeMap::new(),
            rendered: Mutex::new(Vec::new()),
            configured: Mutex::new(Vec::new()),
        }
    }

    /// A harness that will not render at all, standing in for an `armada.yml`
    /// whose declared command cannot be expressed as a rule.
    pub fn refusing(standing_in_for: &'static str) -> FakeHarness {
        FakeHarness {
            refusing: Some(standing_in_for),
            ..FakeHarness::that_listens()
        }
    }

    /// What one line means, for a test that needs a specific event out of it.
    pub fn reading(mut self, line: &str, as_events: Vec<DroneEvent>) -> FakeHarness {
        self.scripted.insert(String::from(line), as_events);
        self
    }

    /// Every launch this harness produced, in order. **The whole point of the
    /// fake**: a test asserts on what Fleet asked for rather than on what a
    /// process did.
    pub fn rendered(&self) -> Vec<Launch> {
        self.rendered.lock().expect("not poisoned").clone()
    }

    /// Every config this harness was handed, in order.
    ///
    /// **Kept beside [`FakeHarness::rendered`] rather than derived from it**: a
    /// [`Launch`] carries the directory and the environment and nothing else,
    /// so the model, the toolbelt and the MCP file — everything a real harness
    /// spells into argv — are unreachable from one. A test asking which model a
    /// step's Drone was started as has nowhere else to look.
    pub fn configured(&self) -> Vec<DroneSpawnConfig> {
        self.configured.lock().expect("not poisoned").clone()
    }
}

impl AgentHarness for FakeHarness {
    type Error = FakeHarnessRefused;

    fn render(&self, config: &DroneSpawnConfig) -> Result<Launch, FakeHarnessRefused> {
        if let Some(standing_in_for) = self.refusing {
            return Err(FakeHarnessRefused { standing_in_for });
        }
        self.configured
            .lock()
            .expect("not poisoned")
            .push(config.clone());
        let launch = Launch::rendered(config, &self.program, self.args.clone());
        self.rendered
            .lock()
            .expect("not poisoned")
            .push(launch.clone());
        Ok(launch)
    }

    fn read(&self, line: &str) -> Vec<DroneEvent> {
        match self.scripted.get(line) {
            Some(events) => events.clone(),
            None => vec![DroneEvent::Said {
                text: String::from(line),
            }],
        }
    }
}

/// The fake's refusal. One variant, because a test only needs the seam to fail,
/// not to fail in a particular way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FakeHarnessRefused {
    pub standing_in_for: &'static str,
}

impl fmt::Display for FakeHarnessRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "refused, standing in for {}", self.standing_in_for)
    }
}

impl Error for FakeHarnessRefused {}
