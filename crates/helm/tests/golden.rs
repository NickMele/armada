//! One golden snapshot per verb, in `tests/golden/`.
//!
//! **There is deliberately no `--update-snapshots` flag.** These catch the one
//! thing nothing else does — **key renames** — because every other test asserts
//! on value objects from the pure core, so renaming `verdict` to `status` in
//! the renderer breaks no test, runs fine locally, and surfaces in someone
//! else's repo. An auto-update flag turns that into "regenerate, commit, don't
//! read", which is the failure mode. Regenerating by hand is annoying enough to
//! make you look, and looking is the point.
//!
//! The three ways golden tests rot, and what is done about each here:
//!
//! | Failure | What this file does |
//! |---|---|
//! | auto-update reflex | no flag; a mismatch prints the payload and you write the file |
//! | nondeterminism | `now` is injected, the workspace path is passed in, and ids and paths are redacted |
//! | over-coverage | one canonical snapshot per verb, never per test case |

mod support;

use armada_core::ctx::{Clock, Ctx, Run, RunOutput, RunRequest, SpawnError};
use armada_core::error::ArmadaError;
use armada_core::registry::{OwnedKind, OwnedRow};
use armada_core::scope::Lens;
use armada_helm::verbs::Output;
use armada_helm::{app, verbs};
use armada_manifest::discovery;
use armada_manifest::net::RealFetch;
use armada_manifest::process::RealRun;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use support::Machine;

/// A clock that never moves, so `claimed_at` is a constant rather than a
/// redaction.
struct FrozenClock;

impl Clock for FrozenClock {
    fn wall_rfc3339(&self) -> String {
        "2026-08-09T14:02:11Z".to_string()
    }
    /// The same instant as the string above, so a run id minted here is a
    /// constant too — a snapshot containing one must not need redacting.
    fn wall_ms(&self) -> u64 {
        1_786_284_131_000
    }
    fn mono(&self) -> u64 {
        1_000
    }
    fn sleep_until(&self, _mono_ms: u64) {}
}

/// Real, except that docker answers "nothing here".
///
/// Not a shortcut: without it a snapshot would contain whatever Armada-labelled
/// resources happen to exist on the machine running the suite, reported as
/// belonging to a foreign namespace — so the file would differ between two
/// correct machines. Everything else, including git and the setup steps, is the
/// real thing.
struct NoDocker;

impl Run for NoDocker {
    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
        if request.argv.first().map(String::as_str) == Some("docker") {
            return Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
            });
        }
        RealRun.call(request)
    }

    fn call_with_tick(
        &self,
        request: &RunRequest,
        tick: &mut dyn FnMut(),
    ) -> Result<RunOutput, SpawnError> {
        if request.argv.first().map(String::as_str) == Some("docker") {
            return self.call(request);
        }
        RealRun.call_with_tick(request, tick)
    }
}

/// Golden runs are serialised, and the reason is a measurement rather than
/// caution: **a bind probe is itself a bind.** Two of these tests probing port
/// 5460 at the same moment — each with its own `$HOME`, so each claiming the
/// same first block — make one of them see the other's probe and report
/// `CONFLICT`. Observed, and it is a property of the probe rather than of the
/// test: `docs/traps.md` records it.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .canonicalize()
        .expect("tests/golden exists")
}

/// Compare, and on a mismatch print the payload so a human can write the file.
fn assert_golden(name: &str, actual: &str) {
    let path = golden_dir().join(format!("{name}.json"));
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        actual,
        expected,
        "\n{} is out of date.\n\nThere is no update flag: read the payload below, decide \
         whether the change is intended, and write it into the file yourself. A field added \
         does not bump schema_version; a field removed or retyped does.\n\n--- actual ---\n{}",
        path.display(),
        actual
    );
}

/// Everything that varies between two correct runs, replaced by a stable name.
struct Redactions {
    root: PathBuf,
    workspace: String,
    project: Option<String>,
}

impl Redactions {
    fn apply(&self, json: &str) -> String {
        let mut out = json
            .replace(&self.root.display().to_string(), "/scratch/repo")
            .replace(&self.workspace, "<workspace>");
        if let Some(project) = &self.project {
            out = out.replace(project, "<project>");
        }
        // **The run id, and only the run id.** It carries a wall reading —
        // frozen here — mixed with this process's entropy, which is not, so it
        // differs between two correct runs. A `duration_ms` does *not*: every
        // reading in a payload comes from the injected clock, so a frozen one
        // makes durations a constant, and redacting them would throw away a
        // real assertion. Measured by trying: the first version of this
        // redaction rewrote `init`'s stable `0` and broke its snapshot.
        out = redact_field(&out, "\"run_id\": \"", "\"", "<run>");
        out = redact_run_paths(&out);
        out
    }
}

/// Replace what sits between a prefix and the next terminator.
fn redact_field(json: &str, prefix: &str, terminator: &str, with: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let mut rest = json;
    while let Some(at) = rest.find(prefix) {
        let (before, after) = rest.split_at(at + prefix.len());
        out.push_str(before);
        match after.find(terminator) {
            Some(end) => {
                out.push_str(with);
                rest = &after[end..];
            }
            None => {
                rest = after;
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A log path carries the run id in the middle of it.
fn redact_run_paths(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let mut rest = json;
    while let Some(at) = rest.find(".armada/run/") {
        let (before, after) = rest.split_at(at + ".armada/run/".len());
        out.push_str(before);
        match after.find('/') {
            Some(end) => {
                out.push_str("<run>");
                rest = &after[end..];
            }
            None => {
                rest = after;
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

struct Scenario {
    machine: Machine,
    repo: PathBuf,
    redactions: Redactions,
}

fn scenario(config: &str) -> Scenario {
    let machine = Machine::new();
    let repo = machine.repo("main", config);
    let workspace = discovery::resolve(&RealRun, &repo).expect("the scratch repo resolves");
    let redactions = Redactions {
        root: repo.clone(),
        workspace: workspace.id.to_string(),
        project: workspace.project.as_ref().map(|p| p.to_string()),
    };
    Scenario {
        machine,
        repo,
        redactions,
    }
}

fn run_verb(
    scenario: &Scenario,
    verb: impl FnOnce(&mut app::App<NoDocker, FrozenClock, RealFetch>) -> Result<Output, ArmadaError>,
) -> String {
    let _serialised = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let run = NoDocker;
    let workspace = discovery::resolve(&run, &scenario.repo).unwrap();
    let ctx = Ctx {
        workspace: Some(workspace),
        run,
        now: FrozenClock,
        fetch: RealFetch,
    };
    let inherited = BTreeMap::from([(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_default(),
    )]);
    let mut app = app::build(ctx, scenario.machine.home.path(), inherited).unwrap();
    let output = verb(&mut app).expect("the verb answered");
    scenario.redactions.apply(&output.to_json())
}

#[test]
fn init_matches_its_snapshot() {
    let scenario = scenario(CONFIG);
    let json = run_verb(&scenario, |app| verbs::init::run(app, false));
    assert_golden("init", &json);
}

/// **The owned resources are recorded by hand, and they have to be.**
///
/// `results[].owns[]` is populated from the machine store, and the only thing
/// that writes to it today is `init`, which records a declared `owns.release:`
/// and nothing else — containers, volumes and process groups arrive with `up`,
/// which is not built (PLAN.md §6.0). So no reachable invocation produces a
/// non-empty `owns[]`, and a snapshot that waited for one would carry the field
/// only after the milestone that could have renamed it.
///
/// That is exactly the failure this suite exists to catch: **key renames**,
/// which every other test misses because they assert on value objects. Writing
/// the rows directly puts the store in the state `up` will put it in, which is
/// the state the payload has to be right for.
#[test]
fn status_matches_its_snapshot() {
    let scenario = scenario(CONFIG);
    run_verb(&scenario, |app| verbs::init::run(app, false));
    let json = run_verb(&scenario, |app| {
        let id = app.ctx.workspace.as_ref().expect("a workspace").id.clone();
        for (kind, reference) in [
            (OwnedKind::Container, format!("armada-{id}-api")),
            (OwnedKind::Volume, "pgdata".to_string()),
        ] {
            app.db.record_owned(&OwnedRow {
                workspace: id.clone(),
                kind,
                reference,
                boot_id: None,
                pid_started_at: None,
                component: None,
            })?;
        }
        verbs::status::run(
            app,
            armada_helm::args::Common {
                json: true,
                dry_run: false,
                lens: Lens::Workspace,
            },
        )
    });
    assert_golden("status", &json);
}

#[test]
fn clean_matches_its_snapshot() {
    let scenario = scenario(CONFIG);
    run_verb(&scenario, |app| verbs::init::run(app, false));
    let json = run_verb(&scenario, |app| {
        verbs::clean::run(
            app,
            armada_helm::args::Common {
                json: true,
                dry_run: false,
                lens: Lens::Workspace,
            },
            verbs::clean::Filters {
                artifacts: false,
                orphaned: false,
                force: false,
                force_rebuild: false,
            },
        )
    });
    assert_golden("clean", &json);
}

#[test]
fn a_dispatched_command_matches_its_snapshot() {
    let scenario = scenario(CONFIG);
    run_verb(&scenario, |app| verbs::init::run(app, false));
    let json = run_verb(&scenario, |app| {
        verbs::dispatch::run(
            app,
            "echoer",
            &["prune".to_string(), "--dry-run".to_string()],
            true,
        )
    });
    assert_golden("commands", &json);
}

/// `armada manifest check`, which is the verb agents call most and therefore the snapshot
/// most worth having: a rename in this payload surfaces here rather than in
/// somebody else's repository.
#[test]
fn check_matches_its_snapshot() {
    let scenario = scenario(CHECK_CONFIG);
    run_verb(&scenario, |app| verbs::init::run(app, false));
    let json = run_verb(&scenario, |app| {
        verbs::check::run(
            app,
            &armada_helm::args::Check {
                json: true,
                ..Default::default()
            },
            // A snapshot has no terminal watching it, and progress that reached
            // this payload would be the exact bug `Silent` exists to prevent.
            &mut armada_helm::render::progress::Silent,
        )
    });
    assert_golden("check", &json);
}

/// The clock is injected and the path is passed in, so two runs of the same
/// verb on two machines produce the same bytes. If this ever fails, the
/// snapshots are about to start rotting.
/// `armada manifest config scan`, over the one fixture with no `armada.yml`.
///
/// **No scenario and no redaction.** A scan has no workspace, claims no ports
/// and mints no run id, so there is nothing in the payload that differs between
/// two correct machines — which is also why it is the one snapshot here that
/// reads a fixture directory rather than a scratch repo.
#[test]
fn config_scan_matches_its_snapshot() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/next-prisma");
    let output = verbs::config::scan(&root).expect("a readable directory always answers");
    assert_golden("config-scan", &output.to_json());
}

/// `armada manifest config verify`, **both passes**, against a scratch repo.
///
/// The render fixture freezes the failing case, because that is the one the
/// agreed layout draws. This freezes the other one, and it is the half that
/// only a real run can produce: `data.pass_2` exists at all only when pass 1
/// passed, so a snapshot of a failure would never carry the key.
///
/// **Pass 2 runs real commands**, which is why the check it runs is `true` and
/// why it runs inside `Machine`'s own scratch repository and nowhere else.
#[test]
fn config_verify_matches_its_snapshot() {
    let scenario = scenario(VERIFY_CONFIG);
    let json = run_verb(&scenario, |app| {
        verbs::config::verify(app, &mut armada_helm::render::progress::Silent)
    });
    assert_golden("config-verify", &json);
}

/// `armada manifest skills` — the resolved structure, whole.
///
/// **The one snapshot that pins `uses:` being expanded.** The config declares a
/// grant and the command behind it; the payload has to carry both, because the
/// CLI table, a future MCP response and the generated frontmatter are three
/// renderings of this structure and a rename here would silently disagree with
/// two of them.
#[test]
fn skills_matches_its_snapshot() {
    let scenario = scenario(SKILLS_CONFIG);
    let json = run_verb(&scenario, |app| verbs::skills::run(app, None));
    assert_golden("skills", &json);
}

/// A skill with the full shape, and the `commands:` entry its grant names.
const SKILLS_CONFIG: &str = "\
manifest:
  version: 1
  commands:
    tickets:
      cmd: ./exiter.sh 0
      help: Report stale tickets
  components:
    repo:
      match: [\"*.yml\"]
      checks:
        ok:
          cmd: \"true\"
          scope: component
  skills:
    add-endpoint:
      summary: Add an API endpoint, OpenAPI first
      doc: docs/skills/add-endpoint.md
      uses: [tickets]
      verify:
        check: [repo:ok]
      touches:
        - \"openapi.yaml\"
";

/// A config that passes pass 1 and whose one check passes pass 2.
///
/// `match:` names a file the scratch repo really has, because a glob that
/// matches nothing is a pass-1 failure — which is the point of that check and
/// would make this snapshot the failing case twice.
const VERIFY_CONFIG: &str = "\
manifest:
  version: 1
  components:
    repo:
      match: [\"*.yml\"]
      checks:
        ok:
          cmd: \"true\"
          scope: component
";

#[test]
fn the_payloads_are_deterministic_across_runs() {
    let first = run_verb(&scenario(CONFIG), |app| verbs::init::run(app, false));
    let second = run_verb(&scenario(CONFIG), |app| verbs::init::run(app, false));
    assert_eq!(first, second);
}

/// One check that passes, one that fails, one skipped for having no files —
/// the three row shapes `results[]` has.
const CHECK_CONFIG: &str = "\
manifest:
  version: 1
  components:
    api:
      root: services/api
      checks:
        lint: { cmd: \"./exiter.sh 0\" }
        types: { cmd: \"./exiter.sh 0\", scope: component }
        test: { cmd: \"./exiter.sh 2\", scope: component }
";

const CONFIG: &str = "\
manifest:
  version: 1
  components:
    api:
      root: services/api
      setup: [\"true\"]
      run:
        driver: command
        cmd: ./serve
        ports: { api: 3000 }
      owns:
        files: [node_modules]
        release: \"psql -c 'DROP DATABASE app_${workspace.id}'\"
  commands:
    echoer:
      cmd: echo
      help: Echo whatever it is given
";
