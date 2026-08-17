//! Golden snapshots of the **human** render, in `tests/golden/render/`.
//!
//! `tests/golden/*.json` freezes the machine contract. These freeze the other
//! one. Until now the agreed layout lived in a document and in this renderer's
//! head, which meant any future change to `render.rs` was indistinguishable from
//! an improvement — and the layout was agreed precisely because a CLI whose
//! shape drifts is a CLI an agent has to re-learn.
//!
//! **The fixture is the specification and the code follows it.** A mismatch is a
//! defect until someone decides otherwise, in that order.
//!
//! Two files per case, and the pair is the point:
//!
//! | File | Audience |
//! |---|---|
//! | `<case>.tty` | a person at a terminal — real ANSI escapes, typographic dashes |
//! | `<case>.plain` | an agent reading stdout — no escapes, ASCII |
//!
//! **Both are rendered at the same width**, so the only differences between them
//! are styling. [`the_two_audiences_differ_only_in_styling`] proves that
//! mechanically, which is the guarantee of PLAN.md §3.1.1 stated as a test
//! rather than as an intention.
//!
//! **There is no update flag, on purpose** — the same reasoning as the JSON
//! goldens. On a mismatch the render is written next to the fixture as
//! `<case>.<audience>.actual` and the failure tells you it is there: escape
//! sequences cannot be retyped by hand, so the bytes have to be recoverable, but
//! recovering them is a deliberate `mv` after you have looked at the diff rather
//! than a flag you reach for without reading.

use armada_core::envelope::{
    Asked, BridgeData, CheckData, CleanData, CommandView, CommandsData, ComponentView,
    ComponentsData, DispatchData, DoctorData, Envelope, Evidence, FailureData, FailuresData,
    Finding, FleetLsData, GateRow, GrantedCommand, GuildChange, GuildChangeData, GuildChoice,
    GuildItemData, GuildItemRow, GuildListData, GuildSyncData, Headline, InboxData, InboxRow,
    InitData, InitDryRun, JobRow, Locality, MachineInitData, MenuData, MenuRow, NoteRow,
    PortReport, Problem, Projection, Released, ResolvedSkillView, ResultRow, RunView, ScanData,
    ServicesData, SettingRow, SettingsData, Settled, ShowData, SkillsData, SpawnData, StatusData,
    StepRow, Sync, SyncItem, TickData, TickRow, TransitionRow, Unreclaimed, UpDryRun, VerifyData,
    Window,
};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::fleet::job::Remaining;
use armada_core::fleet::workflow::{Budget, OnExhausted};
use armada_core::fleet::JobState;
use armada_core::id::WorkspaceId;
use armada_core::ports::{PortBlock, PortState};
use armada_core::reap::ReapPlan;
use armada_core::scan::{Handover, TellWhy};
use armada_helm::render::help::Topic;
use armada_helm::render::style::Style;
use armada_helm::render::term::Terminal;
use armada_helm::render::{self, palette};
use armada_helm::verbs::Output;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// **Both audiences are rendered at eighty columns.** A `.tty` captured at the
/// developer's terminal width would differ from a `.plain` captured at the
/// non-TTY default for a reason that has nothing to do with styling, and the
/// comparison between them is the most valuable thing these files do.
const WIDTH: usize = 80;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/render")
}

/// The two ways one render is read.
fn audiences() -> [(&'static str, Style, Terminal); 2] {
    [
        ("tty", Style::painted(), Terminal::at(WIDTH)),
        ("plain", Style::plain(), Terminal::piped()),
    ]
}

/// Compare one render against its fixture, returning what to say if it differs.
///
/// **Every audience is written before any of them fails.** Panicking on the
/// first mismatch would leave the second `.actual` unwritten, so a reader who
/// changed the renderer would recover half the evidence and have to run the
/// suite again to get the rest.
#[must_use]
fn check_golden(case: &str, audience: &str, actual: &str) -> Option<String> {
    let path = golden_dir().join(format!("{case}.{audience}"));
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if actual == expected {
        return None;
    }
    let beside = path.with_extension(format!("{audience}.actual"));
    std::fs::create_dir_all(golden_dir()).ok();
    std::fs::write(&beside, actual).expect("the actual render is recoverable");
    Some(format!(
        "\n{} is out of date.\n\n\
         There is no update flag. The render was written to\n  {}\n\
         Read it — `cat -v` shows the escapes — decide whether the change is\n\
         intended, and `mv` it over the fixture yourself. The fixture is the\n\
         agreed layout (docs/reference-output/command-output.html); a change\n\
         here is a change to that agreement.\n\n\
         --- expected ---\n{}\n--- actual ---\n{}",
        path.display(),
        beside.display(),
        show(&expected),
        show(actual)
    ))
}

fn report(failures: Vec<String>) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Escapes made visible, so a failure in a terminal is readable.
fn show(text: &str) -> String {
    text.replace('\x1b', "\\e")
}

/// Render one output for both audiences and compare each.
fn assert_render(case: &str, output: &Output) {
    report(
        audiences()
            .into_iter()
            .filter_map(|(audience, style, terminal)| {
                check_golden(case, audience, &render::human(output, style, terminal))
            })
            .collect(),
    );
}

/// The same comparison **at a width [`WIDTH`] cannot reach**.
///
/// **Only the command centre needs this, and it needs it because it has two
/// shapes.** `033` draws all five boxes side by side at [`render::WIDE`] and
/// drops whole panels below it, so a suite fixed at eighty columns can only ever
/// see the narrow one — which is half of why the wide layout shipped untested.
fn assert_render_at(case: &str, width: usize, output: &Output) {
    report(
        [
            ("tty", Style::painted(), Terminal::at(width)),
            ("plain", Style::plain(), Terminal::at(width)),
        ]
        .into_iter()
        .filter_map(|(audience, style, terminal)| {
            check_golden(case, audience, &render::human(output, style, terminal))
        })
        .collect(),
    );
}

// ----------------------------------------------------------------- the world
// the fixtures describe: one workspace, deterministic in every field

/// **The four panels beside JOBS, small but real** — `033`'s command centre.
///
/// One row per box is enough to prove the layout: what these fixtures measure is
/// whether the boxes are drawn, at what widths, and whether any line overruns
/// the terminal, not the contents of tables the `fleet-inbox`, `guild-ls` and
/// `doctor` fixtures already cover in full.
fn panels() -> Box<armada_core::envelope::Panels> {
    let findings = vec![
        Finding::settled("drones", Settled::Ok, "2 alive, both with a Job"),
        Finding::needs(
            "disk",
            Problem::Partial,
            "~/.armada is 2.1 GiB",
            "`armada fleet reap` frees most of it",
        ),
    ];
    Box::new(armada_core::envelope::Panels {
        inbox: InboxData {
            open: 1,
            results: vec![InboxRow {
                uuid: "9f14c2ab".to_string(),
                job_uuid: Some("2b9f4d81-3c7a-4e15-9a08-6f2d1e4b7c53".to_string()),
                job: "nightly-flake".to_string(),
                kind: "NEEDS_HUMAN".to_string(),
                raised_at: "2026-08-09T14:02:11Z".to_string(),
                waiting_s: 8 * 60,
                body: "the CI timeout is 30s and the flake needs 90s. Raise it?".to_string(),
                answered: None,
                closed: None,
            }],
        },
        guild: GuildListData {
            at: "~/.armada/guild".to_string(),
            facts: vec!["1 skill".to_string(), "4 workflows".to_string()],
            template: None,
            items: vec![GuildItemRow {
                kind: "skill".to_string(),
                name: "review-diff".to_string(),
                path: "skills/review-diff/SKILL.md".to_string(),
                opens: "~/.armada/guild/skills/review-diff/SKILL.md".to_string(),
                detail: "read a diff and write REVIEW.md".to_string(),
                bytes: 2_048,
            }],
        },
        system: DoctorData {
            tally: DoctorData::tally(&findings),
            headline: DoctorData::headline(&findings),
            results: findings,
        },
    })
}

fn workspace() -> WorkspaceId {
    WorkspaceId::from_stored("3d9cc7ba")
}

fn block() -> PortBlock {
    PortBlock {
        from: 5460,
        to: 5469,
    }
}

fn tool_failed(message: &str, r#where: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::ToolFailed,
        r#where: r#where.to_string(),
        message: message.to_string(),
        next_action: None,
    }
}

fn check_row(id: &str, status: Status, ms: u64, detail: Option<&str>) -> ResultRow {
    let mut row = ResultRow::new(id, status);
    row.duration_ms = Some(ms);
    row.log = Some(format!(
        ".armada/run/01M00WRY00CYTZ44/logs/{}.log",
        id.replace(':', ".")
    ));
    if let Some(detail) = detail {
        row.error = Some(tool_failed(detail, id));
    }
    row
}

fn check_envelope(status: Status, error: Option<ArmadaError>, rows: Vec<ResultRow>) -> Output {
    Output::Check(Box::new(Envelope {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        verb: "check".to_string(),
        workspace: Some(workspace()),
        status,
        error,
        data: CheckData {
            run_id: "01M00WRY00CYTZ44".to_string(),
            results: rows,
            reaped_runs: Vec::new(),
            // The fixtures freeze an attached run's layout, which is the one a
            // reader meets at a terminal. A detached run adds one row and is
            // asserted where the flag is, in `detach.rs`.
            detached: None,
        },
    }))
}

// ----------------------------------------------------------------- the cases

/// `armada manifest init` on a workspace with ports and a setup step.
#[test]
fn init_matches_its_fixture() {
    let mut api = ResultRow::new("api", Status::Ready);
    api.duration_ms = Some(1_200);
    let output = Output::Init(Box::new(Envelope::ok(
        "init",
        Some(workspace()),
        Status::Ready,
        InitData {
            port_block: Some(block()),
            claimed_at: "2026-08-09T14:02:11Z".to_string(),
            ports: BTreeMap::from([("api".to_string(), 5460), ("web".to_string(), 5461)]),
            reaped: ReapPlan::default(),
            results: vec![api],
        },
    )));
    assert_render("init", &output);
}

/// **A preview's reap rows say `WOULD` too, and the same table proves it.**
///
/// `init --dry-run` renders `data.would_reap` through the very function that
/// renders a real `init`'s `data.reaped`, so it reported `REAPED workspace
/// <id>, directory gone` for a workspace still on disk — under a summary
/// reading `dry run, nothing was changed`. That is the `fleet spawn` defect
/// again, three lines further down the same file: a preview and a receipt drawn
/// by one function with one vocabulary.
///
/// The rows a reap *declines* to touch already read conditionally — `KEPT`,
/// `UNSWEPT` — so those are asserted unchanged, which is what makes this a fix
/// to the reclaiming rows rather than a rewording of the table.
#[test]
fn an_init_preview_would_reap_rather_than_having_reaped() {
    let plan = ReapPlan {
        workspaces: vec![WorkspaceId::from_stored("3d9cc7ba")],
        leases: vec!["run/3d9cc7ba".to_string()],
        skipped: vec!["docker unreachable".to_string()],
        ..ReapPlan::default()
    };

    let render = |output: &Output| render::human(output, Style::plain(), Terminal::piped());

    let previewed = render(&Output::InitDryRun(Box::new(Envelope::ok(
        "init",
        Some(workspace()),
        Status::Ready,
        InitDryRun {
            would_claim: Some(block()),
            would_run: vec!["pnpm install".to_string()],
            would_reap: plan.clone(),
        },
    ))));
    let done = render(&Output::Init(Box::new(Envelope::ok(
        "init",
        Some(workspace()),
        Status::Ready,
        InitData {
            port_block: Some(block()),
            claimed_at: "2026-08-09T14:02:11Z".to_string(),
            ports: BTreeMap::from([("api".to_string(), 5460)]),
            reaped: plan,
            results: vec![ResultRow::new("api", Status::Ready)],
        },
    ))));

    // **The status column, not the whole render.** `REAPED` is also the name of
    // the second column, so a bare `contains` matches the header and passes on
    // the broken output — which it did, first time.
    let status_words = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.starts_with("  "))
            .filter_map(|line| line.split_whitespace().next().map(str::to_string))
            .collect()
    };

    assert!(
        status_words(&done).contains(&"REAPED".to_string()),
        "a real reap stopped saying so:\n{done}"
    );
    assert!(
        !status_words(&previewed).contains(&"REAPED".to_string()),
        "a preview claimed a reap it did not perform:\n{previewed}"
    );
    assert!(
        status_words(&previewed).contains(&"WOULD".to_string()),
        "the preview's reap rows say nothing conditional:\n{previewed}"
    );
    // A row a reap declines to touch is already conditional in both, and stays
    // spelled the way it was.
    for kept in ["UNSWEPT", "docker unreachable"] {
        assert!(previewed.contains(kept), "{previewed}");
        assert!(done.contains(kept), "{done}");
    }
    assert!(previewed.contains("nothing was changed"));
}

/// `armada manifest up`: one service ready, one that never answered.
///
/// **The case is deliberately mixed**, because `PARTIAL` is the state the two
/// verbs added and the one a single-row fixture could not pin: "three of five
/// worked" and "nothing worked" demand different actions and would otherwise
/// both read `FAILED` (PLAN.md §3.1). It also freezes the two things a bare
/// status word cannot say — the ready-check that was waited on, and the log
/// path, which appears under the row that failed and under no other.
#[test]
fn up_matches_its_fixture() {
    let mut postgres = ResultRow::new("postgres", Status::Up);
    postgres.duration_ms = Some(1_900);
    postgres.reason = Some("tcp pg (5460)".to_string());
    postgres.owns = vec!["container:armada-3d9cc7ba-postgres-1".to_string()];
    postgres.ports = BTreeMap::from([(
        "pg".to_string(),
        PortReport {
            port: 5460,
            state: PortState::Listening,
        },
    )]);

    let mut web = ResultRow::new("web", Status::Timeout);
    web.duration_ms = Some(60_000);
    web.log = Some(".armada/logs/web.log".to_string());
    web.reason = Some("http http://127.0.0.1:5461/healthz".to_string());
    web.owns = vec!["pgid:4212".to_string()];
    web.ports = BTreeMap::from([(
        "web".to_string(),
        PortReport {
            port: 5461,
            state: PortState::Reserved,
        },
    )]);
    web.error = Some(ArmadaError {
        class: ErrClass::Timeout,
        r#where: "web".to_string(),
        message: "the ready-check did not pass within 60s".to_string(),
        next_action: Some("raise `ready.timeout:`, or read the service's log".to_string()),
    });

    let mut envelope = Envelope::failed(
        "up",
        Some(workspace()),
        ArmadaError {
            class: ErrClass::Timeout,
            r#where: "web".to_string(),
            message: "1 of 2 services did not succeed".to_string(),
            next_action: Some("raise `ready.timeout:`, or read the service's log".to_string()),
        },
        ServicesData {
            port_block: Some(block()),
            results: vec![postgres, web],
        },
    );
    envelope.status = Status::Partial;
    assert_render("up", &Output::Up(Box::new(envelope)));
}

/// `armada manifest down`: stopped, and the port block **kept**.
///
/// That last row is the whole distinction from `clean`, and a reader who
/// cannot see it has to run `status` to find out whether the next `up` gets the
/// same ports.
#[test]
fn down_matches_its_fixture() {
    let mut web = ResultRow::new("web", Status::Down);
    web.duration_ms = Some(320);
    let mut postgres = ResultRow::new("postgres", Status::Down);
    postgres.duration_ms = Some(1_100);
    postgres.ports = BTreeMap::from([(
        "pg".to_string(),
        PortReport {
            port: 5460,
            state: PortState::Reserved,
        },
    )]);

    let output = Output::Down(Box::new(Envelope::ok(
        "down",
        Some(workspace()),
        Status::Down,
        ServicesData {
            port_block: Some(block()),
            results: vec![web, postgres],
        },
    )));
    assert_render("down", &output);
}

/// `armada manifest up --dry-run`: the argv, and the wait beside it.
#[test]
fn up_dry_run_matches_its_fixture() {
    let output = Output::UpDryRun(Box::new(Envelope::ok(
        "up",
        Some(workspace()),
        Status::Up,
        UpDryRun {
            would_run: vec![
                "postgres: docker compose -f - -p armada-3d9cc7ba --project-directory \
                 /scratch/repo up -d"
                    .to_string(),
                "web: pnpm dev --port 5461".to_string(),
            ],
            would_wait: vec![
                "postgres: tcp pg (5460) (90s)".to_string(),
                "web: http http://127.0.0.1:5461/healthz (60s)".to_string(),
            ],
        },
    )));
    assert_render("up-dry-run", &output);
}

/// `armada manifest status`: what is running, what is mine, what is stale.
#[test]
fn status_matches_its_fixture() {
    let mut row = ResultRow::new("3d9cc7ba", Status::Ok);
    row.path = Some("/scratch/repo".to_string());
    row.port_block = Some(block());
    row.ports = BTreeMap::from([
        (
            "api".to_string(),
            PortReport {
                port: 5460,
                state: PortState::Listening,
            },
        ),
        (
            "web".to_string(),
            PortReport {
                port: 5461,
                state: PortState::Reserved,
            },
        ),
        (
            "postgres".to_string(),
            PortReport {
                port: 5462,
                state: PortState::Conflict,
            },
        ),
    ]);
    row.leases = vec!["run:3d9cc7ba".to_string()];
    // **Five, so the fixture pins the truncation and not just the list.** The
    // human render names three and counts the rest; `--json` carries all five.
    row.owns = vec![
        "container:armada-3d9cc7ba-api".to_string(),
        "container:armada-3d9cc7ba-db".to_string(),
        "network:armada-3d9cc7ba".to_string(),
        "pgid:4212".to_string(),
        "volume:pgdata".to_string(),
    ];
    // **The one stale row is fourth in `owns`, so the fixture pins the reorder
    // and not merely the word.** The human render names three and counts the
    // rest; left in sorted order the leaked group would fall into the `+2` and a
    // reader would never see the only row they can act on.
    row.stale = vec!["pgid:4212".to_string()];
    // A run in flight and one that stopped without deciding, because those are
    // the only two words this table ever prints and a fixture with one of them
    // pins half a rule.
    row.runs = vec![
        RunView {
            run_id: "01M048YQMSD6YP48".to_string(),
            status: Status::Running,
            pgid: 4212,
            log: ".armada/run/01M048YQMSD6YP48/detach.log".to_string(),
            started_at: "2026-08-09T14:02:11Z".to_string(),
        },
        RunView {
            run_id: "01M048KKTG19V63A".to_string(),
            status: Status::Dead,
            pgid: 4098,
            log: ".armada/run/01M048KKTG19V63A/detach.log".to_string(),
            started_at: "2026-08-09T13:40:02Z".to_string(),
        },
    ];
    let output = Output::Status(Box::new(Envelope::ok(
        "status",
        Some(workspace()),
        Status::Ok,
        StatusData {
            scope: "workspace".to_string(),
            results: vec![row],
            unreclaimed: vec![Unreclaimed {
                workspace: workspace(),
                command: "psql -c 'DROP DATABASE app_3d9cc7ba'".to_string(),
                workspace_exists: true,
            }],
        },
    )));
    assert_render("status", &output);
}

/// **A passing run**: no log paths anywhere, because there is nothing to read.
#[test]
fn a_passing_check_matches_its_fixture() {
    let output = check_envelope(
        Status::Pass,
        None,
        vec![
            check_row("armada:boundaries", Status::Pass, 328, None),
            check_row("armada:docs", Status::Pass, 458, None),
            check_row("armada:fmt", Status::Pass, 282, None),
            check_row("armada:lint", Status::Pass, 2_600, None),
        ],
    );
    assert_render("check-pass", &output);
}

/// **A failing run**: one log path, under the one row that has something in it.
#[test]
fn a_failing_check_matches_its_fixture() {
    let mut skipped = ResultRow::new("armada:e2e", Status::Skipped);
    skipped.reason = Some("no matching files".to_string());
    let output = check_envelope(
        Status::Failed,
        Some(tool_failed("1 of 4 checks did not pass", "armada:test")),
        vec![
            check_row("armada:boundaries", Status::Pass, 328, None),
            check_row("armada:docs", Status::Pass, 458, None),
            skipped,
            check_row(
                "armada:test",
                Status::Failed,
                26_754,
                Some("3 failed, 214 passed"),
            ),
        ],
    );
    assert_render("check-fail", &output);
}

/// **An interrupted run**, which is the case the summary line got wrong.
///
/// The report was `ABORTED  3 passed · 2 failed` over a run holding one failure
/// and one check the operator had just stopped. An aborted check reached no
/// verdict — the core says so in `implied_class` — so counting it as a failure
/// told the reader a check had gone wrong when what had gone wrong was that they
/// pressed ctrl-c. `1 failed · 1 aborted` is two facts, and both are true.
///
/// It also freezes the `ABORTED` row itself, which no fixture held: the reaped
/// row and the interrupted row are both from the run that was reported.
#[test]
fn an_aborted_check_counts_the_abort_separately() {
    let mut stopped = ResultRow::new("armada:docs", Status::Aborted);
    stopped.duration_ms = Some(423_000);
    stopped.reason = Some("the run was stopped".to_string());
    let mut output = check_envelope(
        Status::Aborted,
        Some(ArmadaError {
            class: ErrClass::Aborted,
            r#where: "armada:docs".to_string(),
            message: "the run was stopped".to_string(),
            next_action: None,
        }),
        vec![
            check_row("armada:boundaries", Status::Pass, 1_300, None),
            stopped,
            check_row("armada:test", Status::Failed, 23_000, Some("exited 101")),
        ],
    );
    if let Output::Check(envelope) = &mut output {
        envelope.data.reaped_runs = vec!["01M014JQW2NDSDPP".to_string()];
    }
    assert_render("check-aborted", &output);
}

/// `armada manifest clean` on a workspace that owned things.
#[test]
fn clean_matches_its_fixture() {
    let mut row = ResultRow::new("3d9cc7ba", Status::Clean);
    row.released = Some(Released {
        processes: 1,
        containers: 2,
        networks: 0,
        volumes: 1,
        images: 0,
        port_block: true,
        files: 0,
    });
    let output = Output::Clean(Box::new(Envelope::ok(
        "clean",
        Some(workspace()),
        Status::Clean,
        CleanData {
            reaped: ReapPlan::default(),
            results: vec![row],
            unreclaimed: vec![Unreclaimed {
                workspace: workspace(),
                command: "psql -c 'DROP DATABASE app_3d9cc7ba'".to_string(),
                workspace_exists: true,
            }],
            skipped: vec!["b7c20d11".to_string()],
        },
    )));
    assert_render("clean", &output);
}

/// **A clean that owned nothing, which must not look like a failure.**
///
/// The requirement the agreed layout is most explicit about: the table is drawn,
/// the row is present, and the detail is a placeholder rather than an absence.
/// A verb that prints nothing at all when it had nothing to do is a verb whose
/// silence a reader has to interpret.
#[test]
fn a_clean_that_owned_nothing_matches_its_fixture() {
    let output = Output::Clean(Box::new(Envelope::ok(
        "clean",
        Some(workspace()),
        Status::Clean,
        CleanData {
            reaped: ReapPlan::default(),
            results: vec![ResultRow::new("3d9cc7ba", Status::Clean)],
            unreclaimed: Vec::new(),
            skipped: Vec::new(),
        },
    )));
    assert_render("clean-empty", &output);
}

/// A `commands:` entry Armada refused to run. The success case prints nothing at
/// all and is asserted in `render.rs`; there is no fixture for an empty file.
#[test]
fn a_refused_dispatch_matches_its_fixture() {
    let output = Output::Dispatch(Box::new(Envelope::failed(
        "commands",
        Some(workspace()),
        ArmadaError {
            class: ErrClass::BadConfig,
            r#where: "armada.yml:commands.worktrees.cmd".to_string(),
            message: "`git-worktree-helper` is not on PATH".to_string(),
            next_action: Some("install it, or point cmd: at a path in the repo".to_string()),
        },
        DispatchData {
            command: "worktrees".to_string(),
            dispatched: false,
            child_exit: None,
            argv: Vec::new(),
        },
    )));
    assert_render("dispatch-refused", &output);
}

/// `armada manifest config scan`, over the one fixture with no `armada.yml`.
///
/// **Rendered from the real directory, not from a hand-built envelope.** Every
/// other case here constructs its payload, because the world it describes —
/// leases, pids, port claims — has no on-disk form. A scan's world is exactly a
/// directory, so building the envelope by hand would test the renderer against
/// a scan nobody performed, and the two parsers between the files and the table
/// are where this verb's mistakes live.
///
/// **`Ask` ends the report and draws nothing.** It used to print a menu of two
/// numbers here, which is what a real reader could not tell was a prompt; the
/// question is a selector on stderr now (`ask::select`), below the evidence
/// rather than inside it. A widget is not byte-comparable, so this fixture
/// freezes the half that is — everything stdout carries — and the widget's
/// behaviour is covered by unit tests over its key handling.
#[test]
fn config_scan_matches_its_fixture() {
    assert_render("config-scan", &scan_of("next-prisma", Handover::Ask));
}

/// `armada manifest config scan` over a **monorepo**, which is the shape the
/// first version was blind to.
///
/// **The regression, frozen.** Every lockfile and manifest here sits one level
/// down and was reported `absent`; the root holds only the compose file and the
/// workflow, which were the only two things that version found. Nothing but a
/// fixture of this shape would have caught it, and nothing but a byte
/// comparison keeps it caught.
#[test]
fn config_scan_over_a_monorepo_matches_its_fixture() {
    assert_render(
        "config-scan-monorepo",
        &scan_of("polyglot-monorepo", Handover::Ask),
    );
}

/// **The form an agent sees**, and the reason there are two fixtures.
///
/// A menu drawn for a reader with no stdin is a question it cannot satisfy, in
/// the place an instruction belongs — so the same evidence ends with the
/// command instead. This is the render that matters most in practice: agents
/// call this CLI constantly and almost none of them are at a terminal.
#[test]
fn config_scan_for_a_reader_that_cannot_answer_matches_its_fixture() {
    assert_render(
        "config-scan-piped",
        &scan_of("next-prisma", tell(TellWhy::NotATerminal)),
    );
}

/// **A skill that is not there is said out loud.** Offering to launch one
/// produces a failure at the moment the reader was expecting help, so the
/// absence is reported and the command is printed anyway — it is what
/// `armada guild init` will make work.
#[test]
fn config_scan_without_the_onboarding_skill_matches_its_fixture() {
    assert_render(
        "config-scan-no-skill",
        &scan_of("next-prisma", tell(TellWhy::NoSkill)),
    );
}

/// The hand-over as the verb builds it: the pasteable line, and why it is being
/// printed rather than put as a question.
///
/// **The command is the guild's, not this test's.** A fixture that spelled it
/// out would keep passing after the invocation changed, which is exactly what
/// happened: `claude /onboard-repo` was frozen in a golden and the session it
/// opened answered `unknown command /onboard-repo`.
fn tell(why: TellWhy) -> Handover {
    Handover::Tell {
        why,
        command: armada_guild::layout::skill_command_line(armada_guild::layout::ONBOARD_REPO),
    }
}

/// The evidence a real directory yields, as the verb would answer it.
///
/// `handover` is a parameter rather than a detection, because it is the one
/// thing about this payload that does not come from the directory — and it is
/// what makes the interactive and the piped forms two fixtures rather than two
/// renders of one.
fn scan_of(fixture: &str, handover: Handover) -> Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(fixture);
    let files = armada_manifest::scan::read(&armada_manifest::process::RealRun, &root);
    let evidence = armada_core::scan::scan(&files);
    Output::Scan(Box::new(Envelope::ok(
        "config scan",
        None,
        Status::Ok,
        ScanData {
            results: armada_core::scan::findings(&evidence),
            proposals: armada_core::propose::propose(&evidence),
            evidence,
            handover,
        },
    )))
}

/// `armada manifest config verify`, with pass 1 failing.
///
/// **Hand-built, unlike `config scan` beside it.** A verify payload carries
/// durations, and a duration comes from the injected clock rather than from the
/// filesystem — so a fixture rendered from a real run would either need a clock
/// stubbed to hundredths or would carry whatever this machine took. The checks
/// themselves are covered where they are decided, in `armada_core::verify`.
///
/// The failing case is the one the agreed layout draws, and it is the one worth
/// freezing: it is the only one that shows `pass 2 not attempted`, the
/// `unchecked` row and a fix line all at once.
#[test]
fn config_verify_matches_its_fixture() {
    let mut schema = ResultRow::new("schema", Status::Pass);
    schema.duration_ms = Some(100);

    let mut references = ResultRow::new("references", Status::Pass);
    references.duration_ms = Some(100);
    references.reason = Some(armada_core::verify::REFERENCES_DETAIL.to_string());

    let mut argv0 = ResultRow::new("argv[0]", Status::Failed);
    argv0.duration_ms = Some(200);
    argv0.error = Some(ArmadaError {
        class: ErrClass::BadConfig,
        r#where: "armada.yml:manifest.components.web.checks.test.cmd".to_string(),
        message: "`vitest` not on PATH or in root".to_string(),
        next_action: Some(
            "web:test declares `vitest run`; did you mean `pnpm exec vitest run`?".to_string(),
        ),
    });

    let results = vec![schema, references, argv0];
    let output = Output::Verify(Box::new(Envelope {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        verb: "config verify".to_string(),
        workspace: Some(workspace()),
        status: Status::Failed,
        error: armada_core::envelope::aggregate(&results, "checks"),
        data: VerifyData {
            results,
            // **Three, so the fixture pins the count and not just the row.**
            // The number is the honest cost of `shell: true`, and a row that
            // said only `unchecked` would be the footnote it is not.
            unchecked: 3,
            pass_2: None,
        },
    }));
    assert_render("config-verify", &output);
}

/// The two skills the `polyglot-web` fixture declares, which is the fixture
/// that owns the `skills:` axis: one with the full shape and one minimal.
fn declared_skills() -> Vec<ResolvedSkillView> {
    vec![
        ResolvedSkillView {
            name: "add-endpoint".to_string(),
            summary: "Add an API endpoint, OpenAPI first, then the generated client".to_string(),
            doc: "docs/skills/add-endpoint.md".to_string(),
            uses: vec![GrantedCommand {
                name: "tickets".to_string(),
                cmd: "uv run scripts/tickets.py".to_string(),
            }],
            verify: vec!["api:types".to_string(), "web:lint".to_string()],
            touches: vec![
                "backend/openapi.yaml".to_string(),
                "frontend/src/generated/**".to_string(),
            ],
        },
        // **A skill that grants nothing and verifies nothing is still a real
        // thing** — prose the repository wants read, with a name. The listing
        // has to draw it exactly as it draws the other.
        ResolvedSkillView {
            name: "triage-flake".to_string(),
            summary: "Work out whether a failing test is flaky or genuinely broken".to_string(),
            doc: "docs/skills/triage-flake.md".to_string(),
            uses: Vec::new(),
            verify: Vec::new(),
            touches: Vec::new(),
        },
    ]
}

fn skills_envelope(skills: Vec<ResolvedSkillView>) -> Output {
    let results = skills
        .iter()
        .map(|skill| {
            let mut row = ResultRow::new(skill.name.clone(), Status::Ok);
            row.reason = Some(skill.summary.clone());
            row
        })
        .collect();
    Output::Skills(Box::new(Envelope::ok(
        "skills",
        Some(workspace()),
        Status::Ok,
        SkillsData { results, skills },
    )))
}

/// `armada manifest skills` — the listing.
#[test]
fn skills_matches_its_fixture() {
    assert_render("skills", &skills_envelope(declared_skills()));
}

/// `armada manifest skills show <name>` — one skill, grants expanded.
///
/// **The grant table is what `show` adds**, and it is the same shape `status`
/// draws its holdings with. It is drawn only here because at eighty columns a
/// listing cannot carry four columns of it, which is the whole reason the two
/// views differ at all.
#[test]
fn skills_show_matches_its_fixture() {
    let one = declared_skills().into_iter().take(1).collect();
    assert_render("skills-show", &skills_envelope(one));
}

/// `armada manifest components` — the listing that says what `--component`
/// takes.
///
/// **The report it answers**: a reader about to narrow a run had no way to learn
/// the names except by opening `armada.yml`. The fixture holds all three shapes
/// a component comes in — one with a service and checks, one with checks alone,
/// and one with neither — because the `STATUS` word is the only thing that
/// distinguishes them and a fixture with one row would freeze none of that.
#[test]
fn components_matches_its_fixture() {
    fn view(name: &str, root: Option<&str>, runs: bool, checks: &[&str]) -> ComponentView {
        ComponentView {
            name: name.to_string(),
            root: root.map(str::to_string),
            runs,
            checks: checks.iter().map(|c| (*c).to_string()).collect(),
        }
    }
    let components = vec![
        view("api", Some("services/api"), true, &["lint", "test"]),
        view("docs", Some("docs"), false, &[]),
        view("web", Some("apps/web"), false, &["e2e", "lint", "types"]),
    ];
    let results = components
        .iter()
        .map(|component| {
            let mut row = ResultRow::new(component.name.clone(), Status::Ok);
            row.reason = Some(component.checks.join(", "));
            row
        })
        .collect();
    let output = Output::Components(Box::new(Envelope::ok(
        "components",
        Some(workspace()),
        Status::Ok,
        ComponentsData {
            results,
            components,
        },
    )));
    assert_render("components", &output);
}

/// `armada manifest commands` — the listing that says what a repository's own
/// verbs are.
///
/// **The report it answers**: "I have no idea what `<name>` means", said of the
/// root help page's `<name>` row. The names live in `armada.yml` and nothing
/// printed them, so the only way to learn a repository's verbs was to open the
/// file — the same gap `skills` and `components` had, and the last of the three.
///
/// The fixture holds all three shapes an entry comes in: one with a `help:` and
/// a secrets grant, one with a `help:` and none, and one with neither — because
/// the detail column falls back to the command string and a fixture without that
/// row would freeze the sentence case only.
fn command_view(name: &str, cmd: &str, help: Option<&str>, secrets: &[&str]) -> CommandView {
    CommandView {
        name: name.to_string(),
        cmd: cmd.to_string(),
        help: help.map(str::to_string),
        stdio: match secrets.is_empty() {
            true => "inherit".to_string(),
            false => "pipe".to_string(),
        },
        secrets: secrets.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn commands_envelope(commands: Vec<CommandView>) -> Output {
    let results = commands
        .iter()
        .map(|command| {
            let mut row = ResultRow::new(command.name.clone(), Status::Ok);
            row.reason = Some(command.help.clone().unwrap_or_else(|| command.cmd.clone()));
            row
        })
        .collect();
    Output::Commands(Box::new(Envelope::ok(
        "commands",
        Some(workspace()),
        Status::Ok,
        CommandsData { results, commands },
    )))
}

#[test]
fn commands_matches_its_fixture() {
    assert_render(
        "commands",
        &commands_envelope(vec![
            command_view(
                "deploy",
                "./scripts/deploy.sh",
                Some("Deploy this branch to staging"),
                &["GITHUB_TOKEN"],
            ),
            command_view("seed-db", "pnpm prisma db seed", None, &[]),
            command_view(
                "worktrees",
                "uv run scripts/worktrees.py",
                Some("Create and tear down git worktrees"),
                &[],
            ),
        ]),
    );
}

/// **A repository that declares none is the common case, not an edge one**, and
/// it is the one where a reader most needs to be told something. An empty table
/// with the ordinary trailer would answer "run one of these" against a list of
/// nothing.
#[test]
fn commands_with_nothing_declared_matches_its_fixture() {
    assert_render("commands-none", &commands_envelope(Vec::new()));
}

// ------------------------------------------------------------------- M2: the
// three layouts that came up out of `pending/`

fn settled(check: &str, status: Settled, detail: &str) -> Finding {
    Finding::settled(check, status, detail)
}

fn needs(check: &str, status: Problem, detail: &str, remedy: &str) -> Finding {
    Finding::needs(check, status, detail, remedy)
}

/// **`armada init` on a machine that has never seen Armada** — the transcript,
/// including the one question and the first interview prompt.
///
/// Transcribed from `docs/reference-output/command-output.html` into
/// `pending/init-machine.plain` before any of this existed, and moved up here
/// unchanged. The drawing shows question 1 and then the verdict rather than all
/// five, which is what it froze and therefore what this renders.
///
/// **The wordmark is not in this fixture**, though `armada init` is one of its
/// two sites. It is drawn in `main`: the pair of files is rendered at one width
/// for both audiences, and a decoration that appears in only one of them is not
/// a *styling* difference the pair property can express.
#[test]
fn armada_init_matches_its_fixture() {
    let output = Output::MachineInit(Box::new(Envelope::ok(
        "init",
        None,
        Status::Ready,
        MachineInitData {
            results: vec![
                settled("git", Settled::Found, "2.51.0"),
                settled("claude", Settled::Found, "2.0.14"),
                needs(
                    "docker",
                    Problem::Missing,
                    "not required by every repo",
                    "not required by every repo",
                ),
                settled("~/.armada", Settled::Created, "guild/, jobs/, workspaces/"),
            ],
            guild: Some(GuildChoice {
                question: "Do you already have a guild?".to_string(),
                options: vec![
                    "pull from a remote".to_string(),
                    "import a bundle".to_string(),
                    "build one now".to_string(),
                ],
                chosen: 3,
            }),
            imported: vec![
                "imported from ~/.claude/".to_string(),
                "19 skills".to_string(),
                "12 hooks".to_string(),
                "4 plugins".to_string(),
                "CLAUDE.md".to_string(),
            ],
            // **The line that says the guild is in effect.** Whichever of the
            // three answers brought it here, `armada init` ends with it where
            // Claude Code reads it — a machine that has never seen Armada gets a
            // working setup, not a directory.
            projected: Some(Projection {
                at: "~/.claude/".to_string(),
                results: Vec::new(),
                facts: vec!["20 placed".to_string()],
                kept: 0,
                headline: None,
            }),
            asked: vec![
                Asked {
                    number: 1,
                    of: armada_guild::interview::COUNT,
                    prompt: armada_guild::interview::QUESTIONS[0].prompt.to_string(),
                    purpose: armada_guild::interview::QUESTIONS[0].purpose.to_string(),
                    writes: armada_guild::interview::QUESTIONS[0].writes.to_string(),
                    keeps: armada_guild::interview::QUESTIONS[0].keeps.to_string(),
                    prose: true,
                    // **What import wrote, as the question shows it.** A prompt
                    // that says *enter keeps what import found* over nothing is
                    // a default the reader cannot accept with confidence, which
                    // is what a real first run said about it.
                    standing: Some(
                        "Lead with the answer. Tables for anything comparative.".to_string(),
                    ),
                },
                // **One of the three limit questions is in the fixture**, so the
                // transcript proves what a question asking for a number actually
                // looks like: one number, its default on the `now` line, the same
                // number after `enter keeps`, and a prompt that says what happens
                // when it is reached. The old single question asked for
                // `20, 600k, 90m` and was reported as confusing twice.
                Asked {
                    number: 5,
                    of: armada_guild::interview::COUNT,
                    prompt: armada_guild::interview::QUESTIONS[4].prompt.to_string(),
                    purpose: armada_guild::interview::QUESTIONS[4].purpose.to_string(),
                    writes: armada_guild::interview::QUESTIONS[4].writes.to_string(),
                    keeps: armada_guild::interview::QUESTIONS[4].keeps.to_string(),
                    prose: false,
                    standing: Some(armada_guild::interview::QUESTIONS[4].keeps.to_string()),
                },
            ],
            questions: armada_guild::interview::COUNT,
            answered: 0,
            guild_path: "~/.armada/guild".to_string(),
        },
    )));
    assert_render("init-machine", &output);
}

/// The rows a prune draws, and **the fact the whole table exists to carry**:
/// whose each volume is.
///
/// **Built by hand rather than by running the verb**, for the reason the suite's
/// own rule gives — no test may remove a real docker resource on this machine.
/// The numbers are the measured ones: a machine holding 171 volumes and 12.0 GB
/// of which almost none is Armada's. Three rows stand for the three outcomes,
/// because a fixture with only the easy one freezes a layout nobody will see.
fn prune_row(
    status: Status,
    reference: &str,
    owner: armada_core::disk::Ownership,
    bytes: Option<u64>,
    detail: Option<&str>,
) -> armada_core::envelope::PruneRow {
    armada_core::envelope::PruneRow {
        status,
        reference: reference.to_string(),
        kind: armada_core::disk::DiskKind::Volumes,
        owner,
        bytes,
        detail: detail.map(str::to_string),
    }
}

fn prune_fixture() -> armada_core::envelope::PruneData {
    use armada_core::disk::Ownership;
    armada_core::envelope::PruneData {
        results: vec![
            // Armada's, idle, and taken — the only kind that opens ticked.
            prune_row(
                Status::Clean,
                "armada-a3f91c02_pgdata",
                Ownership::Armada,
                Some(79_020_000),
                None,
            ),
            // Armada's, and somebody is working in that worktree right now.
            prune_row(
                Status::Skipped,
                "armada-b7c14e90_pgdata",
                Ownership::Armada,
                Some(48_210_000),
                Some("armada's, in use"),
            ),
            // **The row the verb exists to get right.** Twelve gigabytes that
            // Armada did not create and cannot identify.
            prune_row(
                Status::Skipped,
                "someone-elses_pgdata",
                Ownership::Unlabelled,
                Some(12_010_000_000),
                Some("not armada's"),
            ),
        ],
        freed: Some(79_020_000),
        withheld: Vec::new(),
        skipped: Vec::new(),
    }
}

#[test]
fn prune_matches_its_fixture() {
    let output = Output::Prune(Box::new(Envelope::ok(
        "prune",
        None,
        Status::Partial,
        prune_fixture(),
    )));
    assert_render("prune", &output);
}

/// **A run with nobody to ask removes nothing, and says which rule stopped it.**
/// This is the shape an agent gets, and the fixture exists so that the sentence
/// naming the flag cannot quietly go missing.
#[test]
fn prune_with_nobody_to_ask_matches_its_fixture() {
    let mut data = prune_fixture();
    data.freed = Some(0);
    data.results[0].status = Status::Skipped;
    data.results[0].detail = Some("would go".to_string());
    data.withheld = vec![
        "no terminal to ask at; `--yes` removes armada's own".to_string(),
        "1 of these is not armada's; only a person can remove it".to_string(),
    ];
    let output = Output::Prune(Box::new(Envelope::ok("prune", None, Status::Skipped, data)));
    assert_render("prune-preview", &output);
}

/// **`armada doctor`, and the `→` lines that are the point of it.**
///
/// Two corrections to the transcribed fixture, both made by hand and recorded
/// here rather than absorbed.
///
/// The first: the drawing's summary reads `4 ok · 1 missing · 2 warnings` over a
/// table with **three** `ok` rows. Six rows, seven counted. The tally is derived
/// from the rows by [`armada_helm::verbs`], so shipping the drawing's arithmetic
/// would have meant either freezing a summary that miscounts its own table or
/// hand-writing a tally the code cannot produce. One digit is not the layout.
///
/// The second is the layout, and it came from running the verb: the drawing is a
/// flat table with its `→` lines collected underneath, and a real report of it
/// had three `guild` rows scattered among unrelated ones with nothing to say
/// which belonged together. This fixture freezes the grouped form — one table
/// per check, the check's name as its heading, and each `→` line under the row
/// it answers.
///
/// **The `partial guild` row now carries a fix and previously did not.** The old
/// note said a fragment's fix was prose rather than a command and therefore no
/// remedy; the reader it reached could not tell what he was being asked to do.
/// A sentence naming the file and what to do with it is a fix.
#[test]
fn doctor_matches_its_fixture() {
    let results = vec![
        settled("git", Settled::Ok, "2.51.0"),
        settled("claude", Settled::Ok, "2.0.14"),
        needs(
            "docker",
            Problem::Missing,
            "compose driver unavailable",
            "install docker, or accept that compose repos will not start",
        ),
        needs(
            "~/.armada",
            Problem::Missing,
            "jobs/ and workspaces/ are missing; Jobs and worktrees go there",
            "armada init --force",
        ),
        needs(
            "guild",
            Problem::Stale,
            "3 commits behind origin",
            "armada guild pull",
        ),
        needs(
            "guild",
            Problem::Partial,
            "voice.md still as imported",
            "write ~/.armada/guild/voice.md in your own words",
        ),
        // **The projection group**, which `doctor.md` reserved and could not
        // report until there was a projector to compare against. It sits after
        // the guild it is a projection *of* and before `manifest.db`, so the
        // report reads outwards: your guild, then where it landed, then the
        // machine's own store.
        needs(
            "~/.claude",
            Problem::Partial,
            "1 file yours rather than the guild's: hooks/stop-notify.sh",
            "delete yours and run armada guild project, or keep it",
        ),
        needs(
            "~/.claude",
            Problem::Stale,
            "2 files not what the guild says",
            "armada guild project",
        ),
        settled("manifest.db", Settled::Ok, "2 workspaces, 0 orphans"),
        // **The two disk rows, and the reason there are two of them.** They sit
        // beside `manifest.db` because they answer its question — what has
        // quietly accumulated — and they are separate rows because the remedies
        // differ: the machine's is the reader's own `docker volume prune`, and
        // Armada's is `armada manifest clean --all`. The numbers here are the
        // measured ones from the machine the check was written for, where every
        // one of the 171 volumes was somebody else's compose work.
        settled(
            "docker disk",
            Settled::Ok,
            "12.0 GB reclaimable in 1 image, 171 volumes",
        ),
        settled(
            "docker disk",
            Settled::Ok,
            "none of it is armada's; `docker volume prune` is yours",
        ),
    ];
    let output = Output::Doctor(Box::new(Envelope::ok(
        "doctor",
        None,
        Status::Partial,
        DoctorData {
            tally: DoctorData::tally(&results),
            headline: DoctorData::headline(&results),
            results,
        },
    )));
    assert_render("doctor", &output);
}

/// **`armada settings`** — one machine-local row per Manifest and Helm key,
/// and one synced row for a guild's `settings.json`, which is what a machine
/// with a customised `machine.yml` and an ordinary guild actually shows: two
/// colours, and the STATUS column is what tells them apart rather than the
/// path in DETAIL.
#[test]
fn settings_matches_its_fixture() {
    fn row(locality: Locality, name: &str, value: &str, at: &str) -> SettingRow {
        SettingRow {
            locality,
            name: name.to_string(),
            value: value.to_string(),
            at: at.to_string(),
        }
    }
    let output = Output::Settings(Box::new(Envelope::ok(
        "settings",
        None,
        Status::Ok,
        SettingsData {
            settings: vec![
                row(
                    Locality::Machine,
                    "manifest.cpu_slots",
                    "6",
                    "~/.armada/machine.yml",
                ),
                row(
                    Locality::Machine,
                    "manifest.port_block_size",
                    "10",
                    "~/.armada/machine.yml",
                ),
                row(
                    Locality::Machine,
                    "helm.enter",
                    "on",
                    "~/.armada/machine.yml",
                ),
                // **The one Helm setting no verb writes**, which is why the
                // listing is where a reader finds out it exists at all.
                row(
                    Locality::Machine,
                    "helm.mode",
                    "auto",
                    "~/.armada/machine.yml",
                ),
                row(
                    Locality::Synced,
                    "guild.settings.json",
                    "3 settings",
                    "~/.armada/guild/settings.json",
                ),
            ],
        },
    )));
    assert_render("settings", &output);
}

/// **`armada guild pull` that found a conflict**, which is the case worth
/// freezing: nothing was applied, and the rows are what is waiting.
#[test]
fn guild_pull_matches_its_fixture() {
    let item = |status: Sync, item: &str, detail: &str| SyncItem {
        status,
        item: item.to_string(),
        detail: detail.to_string(),
    };
    let output = Output::GuildSync(Box::new(Envelope::ok(
        "guild pull",
        None,
        Status::Failed,
        GuildSyncData {
            remote: Some("git@example.com:me/guild.git".to_string()),
            ahead: 2,
            behind: 3,
            results: vec![
                item(Sync::Added, "skills", "add-migration, triage-flake"),
                item(Sync::Changed, "hooks", "stop-notify.sh"),
                item(Sync::Conflict, "voice.md", "edited here and on origin"),
                item(Sync::Unchanged, "workflows", "4"),
            ],
            applied: false,
            headline: Some(Headline::NeedsAttention),
            // A divergence applied nothing, so there was nothing new to
            // project — `guild-project` is the fixture that freezes that half.
            projected: None,
        },
    )));
    assert_render("guild-pull", &output);
}

/// **`armada guild ls` — the listing, which is the verb.**
///
/// A terminal draws these rows as a selector and this is what everything else
/// gets, so freezing it freezes the half of PLAN.md §3.1.1 that is easy to lose:
/// an interactive verb whose non-interactive form drifted would still look right
/// to the person who built it.
///
/// **Every kind is in the fixture on purpose.** The STATUS column is the kind,
/// so a fixture with only skills in it would not pin the column width — and the
/// `memory` row that says a fragment is still Armada's example is the row a
/// reader most often came for.
#[test]
fn guild_ls_matches_its_fixture() {
    let item =
        |kind: &str, name: &str, path: &str, opens: &str, detail: &str, bytes: u64| GuildItemRow {
            kind: kind.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            opens: opens.to_string(),
            detail: detail.to_string(),
            bytes,
        };
    let output = Output::GuildList(Box::new(Envelope::ok(
        "guild ls",
        None,
        Status::Ready,
        GuildListData {
            at: "~/.armada/guild".to_string(),
            items: vec![
                item(
                    "memory",
                    "voice.md",
                    "voice.md",
                    "voice.md",
                    "150 words maximum. Lead with the answer.",
                    412,
                ),
                item(
                    "memory",
                    "expectations.md",
                    "expectations.md",
                    "expectations.md",
                    "still Armada's example text",
                    904,
                ),
                item(
                    "skill",
                    "onboard-repo",
                    "skills/onboard-repo",
                    "skills/onboard-repo/SKILL.md",
                    "Write a repository's armada.yml with them.",
                    6210,
                ),
                item(
                    "subagent",
                    "helm.md",
                    "subagents/helm.md",
                    "subagents/helm.md",
                    "The one agent you talk to.",
                    3180,
                ),
                item(
                    "workflow",
                    "bug.yml",
                    "workflows/bug.yml",
                    "workflows/bug.yml",
                    "4 steps, reproduce, fix, review, land",
                    2044,
                ),
                item(
                    "hook",
                    "stop-notify.sh",
                    "hooks/stop-notify.sh",
                    "hooks/stop-notify.sh",
                    "sh, 12 lines",
                    284,
                ),
                item(
                    "permissions",
                    "permissions.yml",
                    "permissions.yml",
                    "permissions.yml",
                    "dontAsk, 8 allowed, 16 denied",
                    2688,
                ),
                item(
                    "schema",
                    "workflow.schema.json",
                    "workflows/workflow.schema.json",
                    "workflows/workflow.schema.json",
                    "what every workflow is checked against",
                    5312,
                ),
            ],
            facts: vec![
                "1 skill".to_string(),
                "1 hook".to_string(),
                "1 subagent".to_string(),
                "1 workflow".to_string(),
            ],
            // **The provenance is on the line a reader is already reading.**
            // `armada guild upgrade` merges against this, and a version nobody
            // can look at is one nobody can trust (`docs/reserved/006`).
            template: Some("0.1.0 8f3a1c0d9e21".to_string()),
        },
    )));
    assert_render("guild-ls", &output);
}

/// **`armada guild upgrade` — what Armada has learned since, merged in.**
///
/// The fixture holds all four outcomes at once, because the reassurance is the
/// thing a reader came for: the schema and the persona took the update, the
/// skill was offered and not taken, and the three files that are **you** say in
/// words that no release will ever touch them. A layout that only showed what
/// changed would leave the one question this verb raises unanswered.
#[test]
fn guild_upgrade_matches_its_fixture() {
    let row = |status: Sync, item: &str, detail: &str| SyncItem {
        status,
        item: item.to_string(),
        detail: detail.to_string(),
    };
    let output = Output::GuildUpgrade(Box::new(Envelope::ok(
        "guild upgrade",
        None,
        Status::Ready,
        armada_core::envelope::GuildUpgradeData {
            at: "~/.armada/guild".to_string(),
            from: None,
            to: "0.1.0 8f3a1c0d9e21".to_string(),
            adopted: Some("c401123abcde".to_string()),
            results: vec![
                row(Sync::Added, "templates.yml", "Armada's"),
                row(Sync::Changed, "subagents/helm.md", "operating knowledge"),
                row(
                    Sync::Unchanged,
                    "workflows/workflow.schema.json",
                    "already what Armada ships",
                ),
                row(
                    Sync::Unchanged,
                    "skills/onboard-repo/SKILL.md",
                    "offered; --with-skills takes it",
                ),
                row(
                    Sync::Unchanged,
                    "voice.md",
                    "yours — no release ever updates it",
                ),
                row(
                    Sync::Unchanged,
                    "expectations.md",
                    "yours — no release ever updates it",
                ),
                row(
                    Sync::Unchanged,
                    "how-i-work.md",
                    "yours — no release ever updates it",
                ),
            ],
            applied: true,
            headline: None,
            projected: None,
        },
    )));
    assert_render("guild-upgrade", &output);
}

/// **`armada guild show` — one item, and the layout the terminal's *view*
/// draws too.**
///
/// Freezing it freezes the thing the split was for: `show` and the *view*
/// action build the same envelope and go through the same renderer, so a fixture
/// here is a fixture for both. The body is deliberately front-matter plus prose,
/// because that is the shape a `SKILL.md` has and the shape that would tempt a
/// renderer into reflowing it.
#[test]
fn guild_show_matches_its_fixture() {
    let output = Output::GuildItem(Box::new(Envelope::ok(
        "guild show",
        None,
        Status::Ready,
        GuildItemData {
            at: "~/.armada/guild".to_string(),
            item: GuildItemRow {
                kind: "skill".to_string(),
                name: "onboard-repo".to_string(),
                path: "skills/onboard-repo".to_string(),
                opens: "skills/onboard-repo/SKILL.md".to_string(),
                detail: "Write a repository's armada.yml with them.".to_string(),
                bytes: 6210,
            },
            body: "---\nname: onboard-repo\ndescription: Write a repository's armada.yml \
                   with them.\n---\n\n# Onboard a repository\n\nOne question at a time, and \
                   nothing written before they confirm.\n"
                .to_string(),
        },
    )));
    assert_render("guild-show", &output);
}

/// **`armada guild edit` that refused to commit**, which is the case worth
/// freezing.
///
/// The file is on disk, the history did not move, and `guild push` will carry
/// nothing — three facts a reader has to be able to tell apart from "the command
/// broke". `REFUSED` is orange rather than red for exactly that reason.
#[test]
fn guild_edit_refused_matches_its_fixture() {
    let output = Output::GuildChange(Box::new(Envelope::failed(
        "guild edit",
        None,
        ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: "workflows/bug.yml".to_string(),
            message: "workflows/bug.yml does not validate: `steps` is a required property"
                .to_string(),
            next_action: Some(
                "fix it and run `armada guild edit workflows/bug.yml` again, or \
                 `git -C ~/.armada/guild checkout workflows/bug.yml` to put it back"
                    .to_string(),
            ),
        },
        GuildChangeData {
            at: "~/.armada/guild".to_string(),
            item: GuildItemRow {
                kind: "workflow".to_string(),
                name: "bug.yml".to_string(),
                path: "workflows/bug.yml".to_string(),
                opens: "workflows/bug.yml".to_string(),
                detail: "does not parse".to_string(),
                bytes: 1980,
            },
            outcome: GuildChange::Refused,
            reading: "`steps` is a required property".to_string(),
            committed: false,
            referenced_by: Vec::new(),
        },
    )));
    assert_render("guild-edit-refused", &output);
}

/// **`armada guild delete`, with something still naming it.**
///
/// The guild syncs, so the removal is committed — and the row saying a workflow
/// still names the skill that has just gone is the only place that connection is
/// ever drawn.
#[test]
fn guild_delete_matches_its_fixture() {
    let output = Output::GuildChange(Box::new(Envelope::ok(
        "guild delete",
        None,
        Status::Ready,
        GuildChangeData {
            at: "~/.armada/guild".to_string(),
            item: GuildItemRow {
                kind: "skill".to_string(),
                name: "add-migration".to_string(),
                path: "skills/add-migration".to_string(),
                opens: "skills/add-migration/SKILL.md".to_string(),
                detail: "Write a migration and its rollback.".to_string(),
                bytes: 1402,
            },
            outcome: GuildChange::Deleted,
            reading: "Write a migration and its rollback.".to_string(),
            committed: true,
            referenced_by: vec!["workflows/feature.yml".to_string()],
        },
    )));
    assert_render("guild-delete", &output);
}

/// `armada guild project` — the verb that puts the guild on Claude Code's load
/// path, and the one row a reader has to act on.
///
/// **`CONFLICT` is the load-bearing row.** A file left exactly as it was because
/// somebody edited it by hand is the whole reason `PLAN.md` §13.2 specifies a
/// hash rather than a copy, and a layout that buried it among the files that did
/// move would be a layout that lost it.
#[test]
fn guild_project_matches_its_fixture() {
    let item = |status: Sync, item: &str, detail: &str| SyncItem {
        status,
        item: item.to_string(),
        detail: detail.to_string(),
    };
    let output = Output::GuildProject(Box::new(Envelope::ok(
        "guild project",
        None,
        Status::Partial,
        Projection {
            at: "~/.claude/".to_string(),
            results: vec![
                item(Sync::Added, "skills", "add-migration, onboard-repo"),
                item(Sync::Changed, "agents", "helm.md"),
                item(
                    Sync::Conflict,
                    "hooks",
                    "edited here; delete it to take the guild's",
                ),
                item(Sync::Unchanged, "skills", "triage-flake"),
            ],
            facts: vec![
                "2 placed".to_string(),
                "1 updated".to_string(),
                "1 left as yours".to_string(),
            ],
            kept: 1,
            headline: Some(Headline::NeedsAttention),
        },
    )));
    assert_render("guild-project", &output);
}

/// `armada --help`, which is the page the milestone was opened for.
///
/// **A representative sample rather than all thirty.** Every module page is
/// here, because those are the pages a reader arrives at; one verb page per
/// module, plus a machine verb, is enough to freeze the *shape* a verb page has
/// — and `render::help`'s own tests are what hold the other twenty-six to it.
/// Thirty more fixture pairs would be thirty more files to re-approve for a
/// one-word change to a shared heading, which is how a golden suite stops being
/// read.
#[test]
fn the_help_pages_match_their_fixtures() {
    let mut failures = Vec::new();
    for (case, topic) in [
        ("help", Topic::Root),
        ("help-manifest", Topic::Manifest),
        ("help-guild", Topic::Guild),
        ("help-fleet", Topic::Fleet),
        ("help-check", Topic::Verb("manifest check")),
        ("help-fleet-spawn", Topic::Verb("fleet spawn")),
        ("help-guild-init", Topic::Verb("guild init")),
        ("help-doctor", Topic::Verb("doctor")),
        ("help-settings", Topic::Verb("settings")),
    ] {
        for (audience, style, terminal) in audiences() {
            failures.extend(check_golden(
                case,
                audience,
                &render::help::render(topic, style, terminal),
            ));
        }
    }
    report(failures);
}

// --------------------------------------------------------------------- M3: the
// two layouts that were agreed before anything existed to draw them

/// `armada fleet ls`, against the layout `render_pending.rs` held for M3.
///
/// **Nothing about this was renegotiated when the verb shipped**, which is the
/// entire purpose of having written the fixture first: the columns, their order,
/// the `-` in a row that has not run, and the summary line were settled in
/// `docs/reference-output/command-output.html` and this render follows them.
///
/// **One column was added afterwards, deliberately and for a reason on the
/// record**: `ID`, the Job's uuid cut to eight characters
/// (`docs/reserved/005-inbox-label-not-identity.md`). A listing of names alone
/// cannot tell two Jobs of one name apart, and the ambiguity refusal that
/// results prints exactly these eight characters as what to type instead — so
/// the table shows them rather than making a reader run a second command to
/// learn them.
///
/// **The rows carry uuid-shaped uuids now.** They were `<name>-uuid`, which
/// was invisible while nothing drew them and would render as `rate-lim` in a
/// column that is meant to be recognisably an id.
///
/// **The cost is ten columns off `DETAIL`, and at eighty two of these rows
/// truncate.** That is the trade taken knowingly: `DETAIL` is the flexible
/// column, so a fixed one is paid for out of it, and the alternative on offer
/// was a table from which the disambiguating fact is simply absent. The
/// truncated half is recoverable — `armada fleet show <id>` prints the entry
/// body whole — while a Job you cannot name is not recoverable from anywhere.
#[test]
fn fleet_ls_matches_its_fixture() {
    // Eight, and every one is a column of the row being built. A struct here
    // would be `JobRow` with different field names.
    #[allow(clippy::too_many_arguments)]
    fn row(
        name: &str,
        uuid: &str,
        workflow: &str,
        state: JobState,
        detail: &str,
        cost_usd: f64,
        runtime_s: u64,
        needs_attention: bool,
    ) -> JobRow {
        JobRow {
            uuid: uuid.to_string(),
            name: name.to_string(),
            workflow: workflow.to_string(),
            state,
            detail: detail.to_string(),
            step: detail.to_string(),
            on_step_s: None,
            // Carried by the listing and drawn by the Bridge, never by `ls` —
            // `DETAIL` answers what a Job is doing now, the task answers what it
            // was asked to do.
            task: format!("the {name} task"),
            runtime_s,
            cost_usd,
            tokens: 120_000,
            turns: 4,
            budget_remaining: Remaining {
                attempts: 2,
                cost_usd: 8.75,
                wall_clock_ms: 1_860_000,
            },
            needs_attention,
            acting: None,
            acting_for_s: None,
        }
    }

    let results = vec![
        row(
            "rate-limit",
            "c19d0a34-3069-4115-ad92-e81f486ce8b9",
            "feature",
            JobState::Running,
            "implement, check green",
            2.10,
            14 * 60,
            false,
        ),
        row(
            "carina-schema",
            "94b1fd2e-6288-46f8-83f0-0d7d857e64cd",
            "feature",
            JobState::Running,
            "plan, awaiting you",
            0.45,
            3 * 60,
            true,
        ),
        row(
            "xlsx-report",
            "3d9cc7ba-1f40-4a6e-9c21-5b8e0d2a7f13",
            "bug",
            JobState::Stalled,
            "no output for 6m",
            4.60,
            22 * 60,
            false,
        ),
        row(
            "release-merge",
            "7f2ab618-58d3-4c07-b9e4-1a6c39fd80ae",
            "feature",
            JobState::Blocked,
            "wants CI timeout raised",
            1.25,
            65 * 60,
            true,
        ),
        // **The row that has not run yet**, and the one the layout was drawn to
        // pin: a Job with no spend and no run time gets a placeholder in both
        // columns rather than `$0.00` and `0s`, because a zero reads as a
        // measurement and nothing has been measured.
        //
        // **Its `ID` is not a placeholder.** A uuid is minted before anything
        // runs (PLAN.md §14.1), so a queued Job has one — that is the point of
        // minting it first.
        row(
            "nightly-flake",
            "e52eaad5-d231-4067-ac5b-6a083c3897d6",
            "bug",
            JobState::Queued,
            "",
            0.0,
            0,
            false,
        ),
    ];

    let output = Output::FleetLs(Box::new(Envelope::ok(
        "fleet ls",
        None,
        // A progress state, which is what a read verb is allowed (PLAN.md §3.1).
        Status::Running,
        FleetLsData {
            needs_you: results.iter().filter(|row| row.needs_attention).count(),
            spent_usd: results.iter().map(|row| row.cost_usd).sum(),
            results,
            // **Carried and not drawn here.** `ls` is a listing of Jobs and the
            // window is a fact about the account; the surface that leads with it
            // is the Bridge (`020` §4), which reads this listing rather than a
            // second source. A `--json` consumer gets it either way.
            windows: Vec::new(),
        },
    )));
    assert_render("fleet-ls", &output);
}

/// `armada fleet inbox` — **the table `001` is about, with the id it lacked.**
///
/// This listing had no fixture at all until now, which is why two things could
/// be wrong in it without anybody noticing:
///
/// - **No `ID` column.** Every entry has carried its own `uuid` in `--json`
///   since it was written and the table never drew it, so the only way to refer
///   to a row was *"the second one"* — which is
///   `docs/reserved/001-raised-items-need-identity.md`'s complaint, printed.
///   `armada fleet ls` was given the same column for the same reason
///   (`docs/reserved/005-inbox-label-not-identity.md`); this table was missed.
/// - **`BLOCKED` never painted red.** The colour was chosen by matching
///   `row.kind` against `"blocked"`, and the field holds `Kind::word()`, which
///   is `BLOCKED`. The arm was dead, so the one kind that cannot proceed
///   without you read in the same orange as one that merely asked a question.
///
/// **The three rows are the three states a reader has to tell apart**: a
/// question waiting, a Job that cannot move, and one already answered. The
/// third greys — `PLAN.md` §15.4's rule that a signal which never stands down
/// stops being one — and it keeps its id, because `armada fleet inbox --all`
/// is how you find what a Job asked before it ended.
#[test]
fn fleet_inbox_matches_its_fixture() {
    fn row(uuid: &str, job: &str, kind: &str, body: &str, waiting_s: u64) -> InboxRow {
        InboxRow {
            uuid: uuid.to_string(),
            job_uuid: Some("c19d0a34-3069-4f6a-9d1e-2b7c8a5f0e11".to_string()),
            job: job.to_string(),
            kind: kind.to_string(),
            raised_at: "2026-08-09T14:02:11Z".to_string(),
            waiting_s,
            body: body.to_string(),
            answered: None,
            closed: None,
        }
    }

    let mut answered = row(
        "7d1c5e02-8a44-4b90-9f31-0c6d2e8a1b47",
        "rate-limit",
        "NEEDS_HUMAN",
        "which of the two ports should the proxy take?",
        3 * 60 * 60,
    );
    answered.answered = Some("the higher one".to_string());

    let results = vec![
        row(
            "4f2a91c8-6b03-4d17-8e5a-91c30b6f2d84",
            "nightly-flake",
            "NEEDS_HUMAN",
            "raise the CI timeout to 90s, or drop the flaky test?",
            65 * 60,
        ),
        row(
            "b83e7a15-2c9d-4e60-b1f7-58a04d3c9e26",
            "rate-limit",
            "BLOCKED",
            "the staging database is not reachable from the worktree",
            22 * 60,
        ),
        answered,
    ];

    let output = Output::Inbox(Box::new(Envelope::ok(
        "fleet inbox",
        None,
        // **An empty or a full inbox is `OK` either way.** Nothing here failed;
        // the question is what is waiting, and the count answers it.
        Status::Ok,
        InboxData {
            open: results.iter().filter(|row| row.is_open()).count(),
            results,
        },
    )));
    assert_render("fleet-inbox", &output);
}

/// One pass of the workflow loop over a fleet — **every word `did` can take,
/// once each**, because the colour is chosen from that word and a fixture that
/// showed three of the eight would freeze a third of the layout.
///
/// **The `DETAIL` column is `<step> · <predicate> — <why>`**, and the two rows
/// with no predicate are the ones where the loop never reached a gate: a Drone
/// mid-exchange, and a Job stopped before anything was gathered. They are in
/// here so the fallback spelling is pinned too, rather than being whatever the
/// format string happens to do the day somebody edits it.
#[test]
fn fleet_tick_matches_its_fixture() {
    fn row(
        job: &str,
        step: &str,
        did: &str,
        state: JobState,
        verdict: Option<armada_core::fleet::Verdict>,
        predicate: Option<&str>,
        why: &str,
    ) -> TickRow {
        TickRow {
            job: job.to_string(),
            step: step.to_string(),
            did: did.to_string(),
            state,
            verdict,
            predicate: predicate.map(str::to_string),
            evidence: Vec::new(),
            why: why.to_string(),
            // Absent on every pass but the one that ends a Job, so absent here:
            // this fixture freezes the ordinary rows.
            released: None,
        }
    }

    let results = vec![
        row(
            "rate-limit",
            "fix",
            "advanced",
            JobState::Running,
            Some(armada_core::fleet::Verdict::Pass),
            Some("check_passes"),
            "`fix` passed; it is on `land`",
        ),
        row(
            "bad-parse",
            "reproduce",
            "waiting",
            JobState::Running,
            None,
            Some("failing_test_exists"),
            "check run 01M00WRY00CYTZ44 is still RUNNING",
        ),
        row(
            "xlsx-report",
            "fix",
            "retried",
            JobState::Running,
            Some(armada_core::fleet::Verdict::Failed),
            Some("check_passes"),
            "`fix` did not pass (the suite is red); attempt 3",
        ),
        // No gate was reached: its Drone is mid-exchange.
        row(
            "carina-schema",
            "plan",
            "working",
            JobState::Running,
            None,
            None,
            "its Drone is still working",
        ),
        row(
            "release-merge",
            "approval",
            "asked",
            JobState::Paused,
            Some(armada_core::fleet::Verdict::NeedsHuman),
            Some("human_approves"),
            "does this look right to you?",
        ),
        // The honest edge of M4 (docs/reserved/016): nothing can decide it.
        row(
            "flaky-suite",
            "review",
            "halted",
            JobState::Paused,
            Some(armada_core::fleet::Verdict::NeedsHuman),
            Some("review_clean"),
            "`review` cannot be gated: `review_clean` is settled by a reviewer Job",
        ),
        row(
            "nightly-flake",
            "land",
            "finished",
            JobState::Done,
            Some(armada_core::fleet::Verdict::Pass),
            Some("branch_exists"),
            "`land` was its last step",
        ),
        // Also no gate: a ceiling stops the Job before anything is gathered.
        row(
            "port-sweep",
            "implement",
            "idle",
            JobState::Paused,
            None,
            None,
            "it is waiting on you",
        ),
    ];

    let output = Output::Tick(Box::new(Envelope::ok(
        "fleet tick",
        None,
        Status::Ok,
        TickData { moved: 4, results },
    )));
    assert_render("fleet-tick", &output);
}

/// One frame of `armada bridge`, which is what `--once` and `--json` emit and
/// what the live screen redraws.
///
/// **The Bridge's columns in Armada's shape.** `commands/helm/bridge.md` settles
/// which columns a frame has — the Job, its state, the task, run time, spend and
/// whether it needs you — and every one of them is here. What this fixture
/// freezes is the *shape* they are drawn in, which is this repository's and not
/// the page's drawing: status first and always a word, no symbol anywhere, and
/// one render for both audiences. A `●` in a needs-you column would fail
/// [`no_fixture_carries_a_status_symbol`] two tests below, which is the rule
/// stating itself.
///
/// **There is no progress column**, deliberately: nothing emits
/// percent-complete, and a bar computed from a turn count is a guess drawn as a
/// measurement.
#[test]
fn bridge_matches_its_fixture() {
    assert_render("bridge", &a_whole_frame());
}

/// **The same frame at [`render::WIDE`]** — `033`'s wide mock-up, all five boxes
/// side by side.
///
/// **A second fixture rather than a second layout.** The screen chooses between
/// two shapes by width and this is the one an eighty-column suite cannot see, so
/// while `033`'s panels lived in the live `paint()` path alone there was no
/// fixture over either shape — `--once` was still drawing the pre-`033` table.
#[test]
fn the_wide_bridge_matches_its_fixture() {
    assert_render_at("bridge-wide", armada_helm::render::WIDE, &a_whole_frame());
}

/// One frame of the command centre: the fleet, and the four panels beside it.
fn a_whole_frame() -> Output {
    /// `on_step` is the step and how long it has been on it, together, because
    /// the second means nothing without the first.
    // Eight fields of a nine-field row, written out rather than bundled: a
    // struct literal here would be the `JobRow` this already builds, and a
    // builder would be a second description of one fixture.
    #[allow(clippy::too_many_arguments)]
    fn row(
        name: &str,
        uuid: &str,
        state: JobState,
        on_step: (&str, Option<u64>),
        task: &str,
        cost_usd: f64,
        runtime_s: u64,
        needs_attention: bool,
    ) -> JobRow {
        let (step, on_step_s) = on_step;
        JobRow {
            // **The same five uuids `fleet-ls` uses**, because it is the same
            // fleet drawn by a second surface — and because the point of the
            // `ID` column is that the id is opaque where the handle is not, so a
            // fixture whose ids were built out of the handles would prove
            // nothing.
            uuid: uuid.to_string(),
            name: name.to_string(),
            workflow: "feature".to_string(),
            state,
            detail: step.to_string(),
            step: step.to_string(),
            on_step_s,
            task: task.to_string(),
            runtime_s,
            cost_usd,
            tokens: 120_000,
            turns: 4,
            budget_remaining: Remaining {
                attempts: 2,
                cost_usd: 8.75,
                wall_clock_ms: 1_860_000,
            },
            needs_attention,
            acting: None,
            acting_for_s: None,
        }
    }

    let mut results = vec![
        row(
            "rate-limit",
            "c19d0a34-3069-4115-ad92-e81f486ce8b9",
            JobState::Running,
            ("implement", Some(12 * 60)),
            "add gateway limiter",
            2.10,
            14 * 60,
            false,
        ),
        row(
            "carina-schema",
            "94b1fd2e-6288-46f8-83f0-0d7d857e64cd",
            JobState::Running,
            ("plan", Some(3 * 60)),
            "migrate schema",
            0.45,
            3 * 60,
            false,
        ),
        // **A step with no duration beside it**, which is the ordinary case for
        // a Drone that never reported crossing a boundary: the step is what the
        // record says and nothing measured how long it has been there, so
        // nothing is drawn rather than a `0s` that reads as a measurement.
        row(
            "xlsx-report",
            "3d9cc7ba-1f40-4a6e-9c21-5b8e0d2a7f13",
            JobState::Stalled,
            ("reproduce", None),
            "generate report",
            4.60,
            22 * 60,
            false,
        ),
        row(
            "release-merge",
            "7f2ab618-58d3-4c07-b9e4-1a6c39fd80ae",
            JobState::Blocked,
            ("implement", Some(18 * 60)),
            "merge release",
            1.25,
            65 * 60,
            true,
        ),
    ];
    // **`NEEDS YOU` carries the question, not `YES`** (`020` §"Also decided"),
    // so most answers need no second screen. `armada fleet ls` folds an open
    // inbox entry's body into `detail`, and this is the one row that has one —
    // which is also why the other three keep `detail == step` and draw nothing
    // in that column at all.
    results[3].detail = "the CI timeout is 30s and the flake needs 90s. Raise it?".to_string();

    let output = Output::Bridge(Box::new(Envelope::ok(
        "bridge",
        None,
        Status::Running,
        BridgeData {
            needs_you: results.iter().filter(|row| row.needs_attention).count(),
            spent_usd: results.iter().map(|row| row.cost_usd).sum(),
            running: results
                .iter()
                .filter(|row| row.state == JobState::Running)
                .count(),
            filter: None,
            hidden: 0,
            // **The whole command centre, because that is what `--once` emits**
            // (`033`). This fixture is the only way the panels are seen without
            // a live terminal, and while `033`'s boxes lived in `paint()` alone
            // it was measuring the pre-`033` single table.
            cwd: "~/code/api".to_string(),
            panels: Some(panels()),
            // **The windows lead the summary line and spend follows them**
            // (`020` §4). Both halves of each are here because both were
            // measured: the percentage is Claude Code's `utilization` floored,
            // and the reset is its `resetsAt` turned into a countdown.
            //
            // **Two of them, soonest reset first**, which is the order they
            // matter in. The line used to carry one, chosen by furthest reset —
            // an arithmetic the weekly window won every time it existed, hiding
            // the five-hour one that is about to stop the fleet.
            windows: vec![
                Window {
                    kind: "five_hour".to_string(),
                    used_percent: Some(71),
                    resets_in_s: Some(2 * 3_600 + 14 * 60),
                },
                Window {
                    kind: "seven_day".to_string(),
                    used_percent: Some(24),
                    resets_in_s: Some(4 * 86_400),
                },
            ],
            results,
        },
    )));
    output
}

/// A filtered frame, and the two columns that disappear when nothing fills
/// them.
///
/// **The counts are over what is shown and the rest is accounted for.** A
/// filtered screen reporting the whole fleet's totals would be answering a
/// question nobody asked, so the filter and what it hid are on the summary line.
///
/// `NEEDS YOU` is gone here because no row filled it — a header is a claim that
/// somebody is waiting (`docs/commands/render.md`).
#[test]
fn bridge_filtered_matches_its_fixture() {
    let results = vec![JobRow {
        uuid: "94b1fd2e-6288-46f8-83f0-0d7d857e64cd".to_string(),
        name: "carina-schema".to_string(),
        workflow: "feature".to_string(),
        state: JobState::Running,
        detail: "implement".to_string(),
        step: "implement".to_string(),
        on_step_s: Some(3 * 60),
        task: "migrate schema".to_string(),
        runtime_s: 3 * 60,
        cost_usd: 0.45,
        tokens: 12_000,
        turns: 1,
        budget_remaining: Remaining {
            attempts: 2,
            cost_usd: 8.75,
            wall_clock_ms: 2_520_000,
        },
        acting: None,
        acting_for_s: None,
        needs_attention: false,
    }];

    let output = Output::Bridge(Box::new(Envelope::ok(
        "bridge",
        None,
        Status::Running,
        BridgeData {
            needs_you: 0,
            spent_usd: 0.45,
            running: 1,
            filter: Some("state=RUNNING".to_string()),
            hidden: 3,
            // **Fleet-only, which is the layout's other branch.** A frame read
            // for the table alone says so with `None`, and the command centre
            // draws the boxes it has rather than claiming panels nobody
            // gathered. What this fixture is about is which columns disappear,
            // and three tables of unrelated content around them would bury it.
            cwd: "~/code/api".to_string(),
            panels: None,
            // **A window with no percentage, which is the ordinary case.** The
            // `utilization` field only rides along once the service has crossed
            // a threshold, so most frames know when the window resets and not
            // how much of it is gone — and the line says what it has rather than
            // computing the rest.
            //
            // **It survives the filter**, unlike every other number here: the
            // rows are what `state=RUNNING` selected, and the window is the
            // account's.
            windows: vec![Window {
                kind: "five_hour".to_string(),
                used_percent: None,
                resets_in_s: Some(43 * 60),
            }],
            results,
        },
    )));
    assert_render("bridge-filtered", &output);
}

/// `armada helm` — what was wired, and the command that would enter it.
///
/// **The fixture's job here is the last line.** This verb starts nothing, and
/// the one way a reader learns that is by being told: a render that reported
/// four `WRITTEN` rows and a launch command, without saying no session was
/// opened, reads exactly like a Helm that is now running. The layout is frozen
/// so that sentence cannot quietly leave.
///
/// **The command is never elided**, which is why it sits on its own line rather
/// than in a cell. A truncated launch command is not a shorter answer; it is an
/// argv that starts an unconfigured session.
#[test]
fn helm_matches_its_fixture() {
    use armada_core::envelope::{Conversation, HelmData, Wired, Wiring};

    let wired = |what: &str, at: &str, detail: &str| Wired {
        what: what.to_string(),
        at: at.to_string(),
        state: Wiring::Written,
        detail: detail.to_string(),
    };

    let output = Output::Helm(Box::new(Envelope::ok(
        "helm",
        None,
        Status::Ok,
        HelmData {
            agent: "helm".to_string(),
            uuid: "15bfa340-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
            conversation: Conversation::New,
            argv: vec![
                "claude".to_string(),
                "--agent".to_string(),
                "helm".to_string(),
                // The reader's own words, inline, exactly as `--exec` would
                // hand them to `claude` — and exactly what the printed line
                // below must *not* paste.
                "--append-system-prompt".to_string(),
                "# Your user's own standing instructions\n\n…".to_string(),
                "--mcp-config".to_string(),
                "~/.armada/helm/mcp.json".to_string(),
                "--plugin-dir".to_string(),
                "~/.armada/helm/plugin".to_string(),
                "--settings".to_string(),
                "~/.armada/helm/settings.json".to_string(),
                // The mode the session enters under. A Drone gets `dontAsk`
                // because nobody is there to answer; this reader is sitting in
                // front of it, so `auto` asks them the questions worth asking.
                "--permission-mode".to_string(),
                "auto".to_string(),
                "--session-id".to_string(),
                "15bfa340-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
            ],
            // **The same launch, and the reason it is a separate field.** The
            // argv above carries the reader's prose as bytes; this is the line
            // a person pastes to reproduce it, which is why the prose appears
            // here as `"$(cat …)"` and the fixture freezes that it never
            // appears any other way.
            command: "claude --agent helm --append-system-prompt \
                      \"$(cat ~/.armada/helm/system-prompt.md)\" \
                      --mcp-config ~/.armada/helm/mcp.json \
                      --plugin-dir ~/.armada/helm/plugin \
                      --settings ~/.armada/helm/settings.json \
                      --permission-mode auto \
                      --session-id 15bfa340-33b1-4f81-bd7f-688f0f01dbb0"
                .to_string(),
            results: vec![
                wired(
                    "toolbelt",
                    "~/.armada/helm/mcp.json",
                    "armada over stdio: fleet.* and manifest.*",
                ),
                wired(
                    "monitor",
                    "~/.armada/helm/plugin",
                    "live push: every inbox line arrives mid-turn",
                ),
                wired(
                    "backstop",
                    "~/.armada/helm/stop-inbox.sh",
                    "Stop hook: a turn does not end while the inbox is unread",
                ),
                wired(
                    "voice",
                    "~/.armada/helm/system-prompt.md",
                    "voice.md, how-i-work.md: your words, and they outrank the persona",
                ),
                Wired {
                    state: Wiring::Unchanged,
                    ..wired(
                        "conversation",
                        "~/.armada/helm/session.json",
                        "not started yet: the next launch mints it",
                    )
                },
            ],
            launched: false,
            entering: false,
        },
    )));
    assert_render("helm", &output);
}

/// `armada fleet show` — **the view the Bridge's `NEEDS YOU: YES` did not have.**
///
/// The complaint this freezes the answer to: a paused Job, a truncated task and
/// a column saying somebody is needed, with no way at all to find out why. Every
/// half of that is here — the entry that raised the flag in its own words, the
/// task whole, the step, the ceilings, and what the Job is still holding.
///
/// **Two things are prose and everything else is a column, deliberately.** The
/// question and the task are the two values a column would have to truncate, and
/// a truncated answer to *why does this need me* is not a shorter answer, it is
/// the wrong one. The rest are facts that compare down a column.
///
/// **There is no progress column here either.** A detail view is the place one
/// would look most like a measurement, and nothing emits percent-complete
/// (PHASES.md §9.1 F2) — so turns, tokens and wall clock are drawn against their
/// ceilings as the numbers they are.
#[test]
fn fleet_show_matches_its_fixture() {
    let output = Output::Show(Box::new(Envelope::ok(
        "fleet show",
        None,
        Status::Ok,
        show_data(),
    )));
    assert_render("fleet-show", &output);
}

/// The case that has no answer anywhere else: a Job the record calls `RUNNING`
/// whose Drone is **gone**, still holding a port block nothing is listening on.
///
/// **Three facts, drawn as three rows, because they disagree.** What the record
/// says, whether the process group is still Armada's and what is still claimed
/// are separate questions; every other view Armada draws folds them into one
/// state word, and this Job reads as healthy in all of them.
///
/// It also has nothing in its inbox, which is the second half of the case: a Job
/// can be badly wrong and have raised nothing, because the thing that would have
/// raised it is the thing that died.
#[test]
fn fleet_show_with_a_dead_drone_matches_its_fixture() {
    let mut data = show_data();
    data.job = "xlsx-report".to_string();
    data.workflow = "bug".to_string();
    // **`PAUSED`, which is where `needs_a_person` is true with nothing in the
    // inbox** — the exact row the complaint was about. The reason it needs you
    // is the state itself, and the pane says so by having no `ASKED` block at
    // all rather than by leaving a `YES` unexplained.
    data.state = JobState::Paused;
    data.recorded_state = JobState::Running;
    data.drone_alive = false;
    data.needs_attention = true;
    data.asked = Vec::new();
    data.repo = "reports".to_string();
    data.progress = vec![NoteRow {
        at: "2026-08-09T14:31:11Z".to_string(),
        ago_s: 36 * 60,
        step: "reproduce".to_string(),
        body: "eleven runs, no empty sheet yet; widening the fixture".to_string(),
    }];
    data.step = "reproduce".to_string();
    data.attempt = 1;
    data.on_step_s = Some(36 * 60);
    // **The sharpest predicate, with the test it names.** `failing_test_exists`
    // is what stops a Drone "fixing" a bug it never reproduced and closing
    // green, and a pane that showed the word without the test would be hiding
    // the half that makes it a gate.
    data.gate = Some(GateRow {
        must: "failing_test_exists".to_string(),
        test: Some("reports::xlsx::empty_sheet".to_string()),
        artifact: None,
        answered_by_a_person: false,
    });
    data.transitions = vec![TransitionRow {
        at: "2026-08-09T14:31:11Z".to_string(),
        ago_s: 36 * 60,
        step: "reproduce".to_string(),
        event: "entered".to_string(),
        attempt: 1,
        must: None,
        evidence: Vec::new(),
    }];
    data.task = "the nightly xlsx export writes an empty sheet about one run in \
                 five; reproduce it, then fix it"
        .to_string();
    data.branch = "armada/xlsx-report".to_string();
    data.worktree = "~/.armada/workspaces/reports/xlsx-report".to_string();
    let output = Output::Show(Box::new(Envelope::ok("fleet show", None, Status::Ok, data)));
    assert_render("fleet-show-gone", &output);
}

/// One Job with something of everything: an answered question, an open one, a
/// budget part spent and two notes.
fn show_data() -> ShowData {
    ShowData {
        job: "release-merge".to_string(),
        uuid: "8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
        workflow: "feature".to_string(),
        state: JobState::Blocked,
        recorded_state: JobState::Running,
        acting: None,
        acting_for_s: None,
        drone_pgid: Some(48122),
        drone_alive: true,
        step: "implement".to_string(),
        attempt: 2,
        on_step_s: Some(18 * 60),
        // The `feature` starter's own predicate for this step. It is what says
        // *why it is still here*, and it is the fact the pane had no way to draw
        // before: a step advances when its predicate holds.
        gate: Some(GateRow {
            must: "check_passes".to_string(),
            test: None,
            artifact: None,
            answered_by_a_person: false,
        }),
        // **A step entered, a step completed, a step restarted** — and the two
        // words that must not collapse into one: `ATTEMPTED` is the Drone
        // saying it believes it is finished, `FAILED` is the gate disagreeing
        // with it four minutes later, carrying the exit code that decided it.
        //
        // Newest first, the same way `progress` is.
        transitions: vec![
            TransitionRow {
                at: "2026-08-09T14:49:11Z".to_string(),
                ago_s: 18 * 60,
                step: "implement".to_string(),
                event: "restarted".to_string(),
                attempt: 2,
                must: None,
                evidence: Vec::new(),
            },
            TransitionRow {
                at: "2026-08-09T14:43:11Z".to_string(),
                ago_s: 24 * 60,
                step: "implement".to_string(),
                event: "failed".to_string(),
                attempt: 1,
                must: Some("check_passes".to_string()),
                evidence: vec![Evidence {
                    kind: "check".to_string(),
                    scope: "orders:test".to_string(),
                    exit: 1,
                }],
            },
            TransitionRow {
                at: "2026-08-09T14:41:11Z".to_string(),
                ago_s: 26 * 60,
                step: "implement".to_string(),
                event: "attempted".to_string(),
                attempt: 1,
                must: None,
                evidence: Vec::new(),
            },
            TransitionRow {
                at: "2026-08-09T14:26:11Z".to_string(),
                ago_s: 41 * 60,
                step: "implement".to_string(),
                event: "entered".to_string(),
                attempt: 1,
                must: None,
                evidence: Vec::new(),
            },
            TransitionRow {
                at: "2026-08-09T14:25:11Z".to_string(),
                ago_s: 42 * 60,
                step: "plan".to_string(),
                event: "completed".to_string(),
                attempt: 1,
                must: Some("subjob_passed".to_string()),
                evidence: vec![Evidence {
                    kind: "subjob".to_string(),
                    scope: "release-plan".to_string(),
                    exit: 0,
                }],
            },
            TransitionRow {
                at: "2026-08-09T14:02:11Z".to_string(),
                ago_s: 65 * 60,
                step: "plan".to_string(),
                event: "entered".to_string(),
                attempt: 1,
                must: None,
                evidence: Vec::new(),
            },
        ],
        task: "merge the release branch and cut 4.2, resolving the migration \
               conflict in orders before the tag goes out"
            .to_string(),
        runtime_s: 65 * 60,
        created_at: "2026-08-09T14:02:11Z".to_string(),
        cost_usd: 1.25,
        tokens: 119_900,
        turns: 4,
        budget: Budget {
            attempts: 2,
            cost_usd: 10.00,
            wall_clock_ms: 90 * 60 * 1_000,
            on_exhausted: OnExhausted::NeedsHuman,
        },
        budget_remaining: Remaining {
            attempts: 2,
            cost_usd: 8.75,
            wall_clock_ms: 25 * 60 * 1_000,
        },
        repo: "orders".to_string(),
        branch: "armada/release-merge".to_string(),
        worktree: "~/.armada/workspaces/orders/release-merge".to_string(),
        port_block: Some(PortBlock {
            from: 5470,
            to: 5479,
        }),
        needs_attention: true,
        asked: vec![
            InboxRow {
                uuid: "e30b91aa".to_string(),
                job_uuid: Some("7f2ab618-58d3-4c07-b9e4-1a6c39fd80ae".to_string()),
                job: "release-merge".to_string(),
                kind: "NEEDS_HUMAN".to_string(),
                raised_at: "2026-08-09T14:20:11Z".to_string(),
                waiting_s: 47 * 60,
                body: "should the 4.2 tag be signed with the release key?".to_string(),
                answered: Some("yes, and push the tag once check is green".to_string()),
                closed: None,
            },
            InboxRow {
                uuid: "e4f1a2c9".to_string(),
                job_uuid: Some("7f2ab618-58d3-4c07-b9e4-1a6c39fd80ae".to_string()),
                job: "release-merge".to_string(),
                kind: "BLOCKED".to_string(),
                raised_at: "2026-08-09T14:58:11Z".to_string(),
                waiting_s: 9 * 60,
                body: "the release branch carries two migrations that both rename \
                       the orders index. Squash them into one, or revert 0042 and \
                       re-cut it? Both are safe; the second loses the rename."
                    .to_string(),
                answered: None,
                closed: None,
            },
        ],
        progress: vec![
            NoteRow {
                at: "2026-08-09T15:01:11Z".to_string(),
                ago_s: 6 * 60,
                step: "implement".to_string(),
                body: "rebased onto main, two migration conflicts left".to_string(),
            },
            NoteRow {
                at: "2026-08-09T14:26:11Z".to_string(),
                ago_s: 41 * 60,
                step: "plan".to_string(),
                body: "approach agreed: squash the two migrations".to_string(),
            },
        ],
        steps: vec![
            StepRow {
                id: "plan".to_string(),
                status: "PASS".to_string(),
                must: "subjob_passed".to_string(),
            },
            StepRow {
                id: "implement".to_string(),
                status: "BLOCKED".to_string(),
                must: "check_passes".to_string(),
            },
            StepRow {
                id: "review".to_string(),
                status: "QUEUED".to_string(),
                must: "review_clean".to_string(),
            },
            StepRow {
                id: "land".to_string(),
                status: "QUEUED".to_string(),
                must: "branch_exists".to_string(),
            },
        ],
    }
}

// ------------------------------------------------- Armada's own failures

/// A recorded failure, authored rather than captured.
///
/// **No real content of anybody's**, here least of all: the fixture for a
/// feature whose subject is local paths is the one place a home directory would
/// walk into a public repository. Every path below is invented, and the log's
/// own rule — nothing absolute under `$HOME` is ever written — is what makes
/// `~/code/orders` the shape a real one takes too.
fn recorded(
    id: &str,
    state: armada_core::failure::State,
    class: ErrClass,
    r#where: &str,
    message: &str,
    count: usize,
    age_s: u64,
) -> armada_core::failure::Entry {
    armada_core::failure::Entry {
        id: id.to_string(),
        state,
        origin: armada_core::failure::Origin::Observed,
        class: Some(class),
        r#where: r#where.to_string(),
        message: message.to_string(),
        next: Some("reinstall armada, then retry unchanged".to_string()),
        argv: "armada bridge".to_string(),
        cwd: "~/code/orders".to_string(),
        workspace: None,
        count,
        first_at: "2026-08-09T13:02:11Z".to_string(),
        last_at: "2026-08-09T14:58:11Z".to_string(),
        last_ms: 1_754_748_000_000,
        age_s,
        job: None,
        diagnostics: None,
    }
}

/// A **filed** report, authored the same way and for the same reason.
///
/// **Every attachment is invented and none is realistic-looking on purpose.**
/// This is the fixture for the feature whose whole subject is *attaching your
/// machine to a record*, so it is the one place a real path, a real Job name or
/// a real token would walk into a public repository. What it does have to be is
/// the *shape* a real one takes — a version, a system, a doctor finding, and
/// runs that lead up to the complaint.
fn filed(id: &str, what: &str, age_s: u64) -> armada_core::failure::Entry {
    let ran = |verb: &str, argv: &str, exit: u8, at_ms: u64| armada_core::recent::Ran {
        at: "2026-08-09T14:58:11Z".to_string(),
        at_ms,
        verb: verb.to_string(),
        argv: argv.to_string(),
        cwd: "~/code/orders".to_string(),
        exit,
        envelope: None,
    };
    armada_core::failure::Entry {
        id: id.to_string(),
        state: armada_core::failure::State::Open,
        origin: armada_core::failure::Origin::Reported,
        // **No class, and the row still reads.** Armada did not notice, so it
        // attributed nothing; the DETAIL cell leads with the origin instead.
        class: None,
        r#where: String::new(),
        message: what.to_string(),
        next: None,
        argv: format!("armada report '{what}'"),
        cwd: "~/code/orders".to_string(),
        workspace: None,
        count: 1,
        first_at: "2026-08-09T14:58:11Z".to_string(),
        last_at: "2026-08-09T14:58:11Z".to_string(),
        last_ms: 1_754_748_000_000,
        age_s,
        job: None,
        diagnostics: Some(Box::new(armada_core::failure::Diagnostics {
            armada: "0.1.0".to_string(),
            claude: Some("2.0.14".to_string()),
            system: "linux x86_64".to_string(),
            cwd: "~/code/orders".to_string(),
            workspace: Some("~/code/orders".to_string()),
            manifest: true,
            doctor: vec!["STALE guild: 3 commits behind origin".to_string()],
            recent: vec![
                ran(
                    "fleet spawn",
                    "armada fleet spawn 'add the report verb' --dry-run",
                    0,
                    1_754_748_000_000,
                ),
                ran("fleet ls", "armada fleet ls", 0, 1_754_747_940_000),
            ],
            failures: vec!["a1b2c3d4 could not be found to run".to_string()],
            jobs: vec!["orders-fix-a1b2 RUNNING bug".to_string()],
        })),
    }
}

/// `armada failures` — one row per distinct failure, with a count and an age.
///
/// **The count is what makes this observability rather than a log.** Four
/// occurrences of one bug and four different bugs are different facts, and the
/// row that cannot tell them apart is the row nobody reads twice.
#[test]
fn failures_matches_its_fixture() {
    let mut fixing = recorded(
        "7c1e40aa",
        armada_core::failure::State::Fixing,
        ErrClass::ArmadaBug,
        "spawn",
        "the worktree was already there and was not Armada's",
        1,
        26 * 60,
    );
    fixing.job = Some("fix-7c1e40aa".to_string());

    let output = Output::Failures(Box::new(Envelope::ok(
        "failures",
        None,
        Status::Ok,
        FailuresData {
            results: vec![
                recorded(
                    "a91f0c37",
                    armada_core::failure::State::Open,
                    ErrClass::Environment,
                    "~/.cargo/bin/armada",
                    "`armada manifest clean` could not be found to run",
                    4,
                    9 * 60,
                ),
                fixing,
                recorded(
                    "3bb27d15",
                    armada_core::failure::State::Open,
                    ErrClass::BadInvocation,
                    "brdige",
                    "unknown command `brdige`",
                    1,
                    3 * 60 * 60,
                ),
            ],
            open: 2,
        },
    )));
    assert_render("failures", &output);
}

/// `armada failures` on a machine nothing has gone wrong on.
///
/// **Said in words rather than drawn as an empty table**, the same rule the
/// inbox follows: "nothing recorded" and "nobody looked" read identically
/// otherwise, and only one of them is good news.
#[test]
fn failures_empty_matches_its_fixture() {
    let output = Output::Failures(Box::new(Envelope::ok(
        "failures",
        None,
        Status::Ok,
        FailuresData::default(),
    )));
    assert_render("failures-empty", &output);
}

/// `armada failures show <id>` — the failure as the terminal printed it, and
/// then what a Job would be told about it.
#[test]
fn failure_show_matches_its_fixture() {
    let entry = recorded(
        "a91f0c37",
        armada_core::failure::State::Open,
        ErrClass::Environment,
        "~/.cargo/bin/armada",
        "`armada manifest clean` could not be found to run",
        4,
        9 * 60,
    );
    let task = armada_core::failure::task(&entry);
    let output = Output::Failure(Box::new(Envelope::ok(
        "failures show",
        None,
        Status::Ok,
        FailureData {
            results: vec![entry],
            task,
        },
    )));
    assert_render("failure-show", &output);
}

/// `armada failures show <an inbox entry's id>` — **the fourth origin on the
/// screen the other three already share.**
///
/// `docs/reserved/001-raised-items-need-identity.md` asks that every item Helm
/// surfaces have an id, and three of the four already did. This is what the
/// fourth looks like once it is in the same id space, and the fixture pins the
/// two things that are different about it:
///
/// - **No hand-over block.** Every other origin ends with *the task a Job would
///   be given*; a raised item's Job already exists and is stopped in front of
///   the question, so there is nothing to hand over and the heading would be
///   describing a Job that is already running.
/// - **`armada fleet answer`, twice, where `fix` and `clear` would go.** Both
///   of those refuse a raised item, and a row whose only offered actions cannot
///   work is exactly the defect
///   `docs/reserved/005-inbox-label-not-identity.md` records.
///
/// **`FIXING`, not `OPEN`.** A raised item is not a row nobody has started; it
/// is a row with a Drone on it, which is what that word already means here.
#[test]
fn failure_show_of_a_raised_item_matches_its_fixture() {
    let entry = armada_fleet::inbox::Entry {
        uuid: "4f2a91c8-6b03-4d17-8e5a-91c30b6f2d84".to_string(),
        job_uuid: Some("c19d0a34-3069-4f6a-9d1e-2b7c8a5f0e11".to_string()),
        job: "nightly-flake".to_string(),
        kind: armada_fleet::inbox::Kind::NeedsHuman,
        raised_at: "2026-08-09T14:02:11Z".to_string(),
        raised_ms: 0,
        body: "raise the CI timeout to 90s, or drop the flaky test?".to_string(),
        answered: None,
        closed: None,
    }
    .as_entry();
    let task = armada_core::failure::task(&entry);
    assert_eq!(task, "", "a raised item has no task to hand over");

    let output = Output::Failure(Box::new(Envelope::ok(
        "failures show",
        None,
        Status::Ok,
        FailureData {
            results: vec![entry],
            task,
        },
    )));
    assert_render("failure-show-raised", &output);
}

/// `armada report` — a filing, with everything Armada gathered so that nobody
/// had to paste it.
///
/// **The layout this freezes is the answer to the ask.** The description is
/// quoted rather than dressed up as an error Armada reported; the machine facts
/// join the same facts table the entry already draws; and the runs get their own
/// `STATUS · NAME · DETAIL · TIME` table, because whether each one said it
/// worked is the column that makes them worth attaching at all.
#[test]
fn report_matches_its_fixture() {
    let entry = filed(
        "6d40b1e9",
        "the dry-run said CREATED worktree and made nothing",
        0,
    );
    let task = armada_core::failure::task(&entry);
    let output = Output::Failure(Box::new(Envelope::ok(
        "report",
        None,
        Status::Ok,
        FailureData {
            results: vec![entry],
            task,
        },
    )));
    assert_render("report", &output);
}

/// `armada failures` with **both origins in one listing** — the whole argument
/// for one store rather than two.
///
/// A reader triaging on a Monday morning does not care which half of the machine
/// noticed, so the two kinds are one table and the DETAIL cell's first word is
/// what tells them apart: a class when Armada assigned one, `reported` when it
/// did not.
#[test]
fn failures_with_a_filed_report_matches_its_fixture() {
    let output = Output::Failures(Box::new(Envelope::ok(
        "failures",
        None,
        Status::Ok,
        FailuresData {
            open: 2,
            results: vec![
                filed(
                    "6d40b1e9",
                    "the dry-run said CREATED worktree and made nothing",
                    4 * 60,
                ),
                recorded(
                    "a91f0c37",
                    armada_core::failure::State::Open,
                    ErrClass::Environment,
                    "~/.cargo/bin/armada",
                    "`armada manifest clean` could not be found to run",
                    4,
                    9 * 60,
                ),
            ],
        },
    )));
    assert_render("failures-reported", &output);
}

/// A **written task**, authored the same way as [`recorded`] and [`filed`].
///
/// `workspace` is the field this fixture exists for: `None` is what a task
/// written outside any `armada.yml` carries, and `Some` is a monorepo's finer
/// unit than `cwd`'s repository — `cwd` is `~/code/storefront` for both of a
/// monorepo's tasks, and only this field tells `storefront/web` from
/// `storefront/backend`.
fn written(
    id: &str,
    what: &str,
    workspace: Option<&str>,
    age_s: u64,
) -> armada_core::failure::Entry {
    armada_core::failure::Entry {
        id: id.to_string(),
        state: armada_core::failure::State::Open,
        origin: armada_core::failure::Origin::Written,
        class: None,
        r#where: String::new(),
        message: what.to_string(),
        next: None,
        argv: format!("armada task '{what}'"),
        cwd: "~/code/storefront".to_string(),
        workspace: workspace.map(str::to_string),
        count: 1,
        first_at: "2026-08-09T14:58:11Z".to_string(),
        last_at: "2026-08-09T14:58:11Z".to_string(),
        last_ms: 1_754_748_000_000,
        age_s,
        job: None,
        diagnostics: None,
    }
}

/// `armada tasks` — the `WORKSPACE` column, present because at least one row
/// filled it, and blank rather than `-` on the row that did not.
///
/// **The scenario is `docs/reserved/002-tasks.md`'s own**: a monorepo, two
/// tasks written in two of its workspaces, told apart though both share one
/// repository. The third row is a task written where capture found no
/// `armada.yml` at all — the case the column must not guess at.
#[test]
fn tasks_matches_its_fixture() {
    let output = Output::Failures(Box::new(Envelope::ok(
        "tasks",
        None,
        Status::Ok,
        FailuresData {
            open: 3,
            results: vec![
                written(
                    "a1b2c3d4",
                    "wire the new port allocator",
                    Some("~/code/storefront/web"),
                    9 * 60,
                ),
                written(
                    "5e6f7081",
                    "rename the retry helper",
                    Some("~/code/storefront/backend"),
                    26 * 60,
                ),
                written("ff001122", "look into the flaky golden", None, 3 * 60 * 60),
            ],
        },
    )));
    assert_render("tasks", &output);
}

/// `armada fleet spawn`, against the layout `render_pending.rs` held for M3.
///
/// **The confidence is on the screen**, which is the one thing PLAN.md §14.2
/// asks of this verb beyond doing the work: a guess has to be visible as a
/// guess, or nobody knows to override it.
#[test]
fn fleet_spawn_matches_its_fixture() {
    let output = Output::Spawn(Box::new(Envelope::ok(
        "fleet spawn",
        None,
        Status::Ready,
        SpawnData {
            uuid: "8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
            name: "rate-limit".to_string(),
            workflow: "feature".to_string(),
            confidence: Some(0.91),
            // **The drawing's own path, kept verbatim.** The shipped policy puts
            // a Job under `~/.armada/workspaces/<repo>/<name>`
            // (`commands/fleet/spawn.md`), and this fixture froze the *layout*
            // rather than the path — the cell holds whatever the envelope
            // carries, so changing the string here would test nothing and
            // changing the fixture would renegotiate a settled drawing.
            worktree: "~/.armada/workspaces/rate-limit".to_string(),
            branch: "armada/rate-limit".to_string(),
            port_block: Some(PortBlock {
                from: 5470,
                to: 5479,
            }),
            budget: Budget {
                attempts: 2,
                cost_usd: 10.00,
                wall_clock_ms: 90 * 60 * 1_000,
                on_exhausted: OnExhausted::NeedsHuman,
            },
            step: "plan".to_string(),
            state: JobState::Running,
            classify_ms: Some(800),
            prepare_ms: 300,
            // **A spawn reports the handle, not a spend.** It returns while the
            // Drone is still working, so there is nothing to have spent yet —
            // and the row that used to carry a duration for the turn now
            // carries the placeholder, which the fixture already drew.
            pgid: Some(4212),
        },
    )));
    assert_render("fleet-spawn", &output);
}

/// `armada fleet spawn --dry-run`, which must not read as a spawn that happened.
///
/// **The fixture beside it is the whole assertion.** `fleet-spawn.plain` and
/// `fleet-spawn-dry-run.plain` are the same four rows in the same four columns,
/// and the only thing separating them is the vocabulary — so a reader comparing
/// the two files sees exactly what a reader comparing two terminals would.
/// Before this, the two renders were byte-identical apart from the elapsed
/// times: a preview said `CREATED worktree <path>` for a directory that does not
/// exist and closed on `QUEUED … armada fleet board rate-limit`, an action that
/// refuses because there is no Job to board.
///
/// **`WOULD` rather than a word of its own**, because `manifest init`, `manifest
/// up` and `manifest clean` already say it and a second conditional vocabulary
/// would be worse than either. The classify row keeps `CLASSIFIED` because it is
/// the one step a preview really takes.
#[test]
fn fleet_spawn_dry_run_matches_its_fixture() {
    let output = Output::Spawn(Box::new(Envelope::ok(
        "fleet spawn",
        None,
        // The status the dry-run arm of `verbs::fleet::spawn` returns, and what
        // the renderer reads to know this is a preview.
        Status::Skipped,
        SpawnData {
            uuid: "8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
            name: "rate-limit".to_string(),
            workflow: "feature".to_string(),
            confidence: Some(0.91),
            worktree: "~/.armada/workspaces/rate-limit".to_string(),
            branch: "armada/rate-limit".to_string(),
            // **No block, because none was claimed.** A preview does not run
            // `armada manifest init`, so it does not know which span it would
            // get — and inventing one would be the same defect in a new column.
            port_block: None,
            budget: Budget {
                attempts: 2,
                cost_usd: 10.00,
                wall_clock_ms: 90 * 60 * 1_000,
                on_exhausted: OnExhausted::NeedsHuman,
            },
            step: "plan".to_string(),
            // Still `Queued`, because the record was built and never saved —
            // which is why the footer leads with the envelope's `SKIPPED`
            // instead. A `JobState` here would name the state of a Job that has
            // no record.
            state: JobState::Queued,
            // Real, and the only real measurement on the table: the classifying
            // call is the one step a dry run makes.
            classify_ms: Some(800),
            prepare_ms: 0,
            pgid: None,
        },
    )));
    assert_render("fleet-spawn-dry-run", &output);
}

/// **A preview and a spawn are told apart by the envelope's status, and by
/// nothing else.**
///
/// `render::spawn` reads `Status::Skipped` to mean "this did not happen", which
/// is safe for exactly as long as the dry-run arm is the only thing that returns
/// it. Inverted — with both arms returning the same status — the two renders go
/// back to being indistinguishable, which is the defect. So the pairing is
/// asserted here rather than left as a comment: the real spawn's four rows say
/// what was done, the preview's three say what would be, and neither text
/// appears in the other.
#[test]
fn a_preview_and_a_spawn_are_told_apart_by_status() {
    let spawn = |status: Status, state: JobState, block: Option<PortBlock>| {
        render::human(
            &Output::Spawn(Box::new(Envelope::ok(
                "fleet spawn",
                None,
                status,
                SpawnData {
                    uuid: "8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
                    name: "rate-limit".to_string(),
                    workflow: "feature".to_string(),
                    confidence: Some(0.91),
                    worktree: "~/.armada/workspaces/rate-limit".to_string(),
                    branch: "armada/rate-limit".to_string(),
                    port_block: block,
                    budget: Budget {
                        attempts: 2,
                        cost_usd: 10.00,
                        wall_clock_ms: 90 * 60 * 1_000,
                        on_exhausted: OnExhausted::NeedsHuman,
                    },
                    step: "plan".to_string(),
                    state,
                    classify_ms: Some(800),
                    prepare_ms: 300,
                    pgid: None,
                },
            ))),
            Style::plain(),
            Terminal::piped(),
        )
    };

    let real = spawn(
        Status::Ready,
        JobState::Running,
        Some(PortBlock {
            from: 5470,
            to: 5479,
        }),
    );
    let preview = spawn(Status::Skipped, JobState::Queued, None);

    // What a spawn says and a preview must never say. Every one of these was in
    // the reported output of a run that created nothing.
    for done in ["CREATED", "CLAIMED", "STARTED", "fleet board"] {
        assert!(real.contains(done), "a real spawn stopped saying `{done}`");
        assert!(
            !preview.contains(done),
            "a preview says `{done}`, which a reader cannot tell from a spawn:\n{preview}"
        );
    }
    // **`QUEUED` gets its own line because it is a `JobState`, not a status.**
    // A real spawn that has not started its Drone yet legitimately leads with
    // it, so it is not in the list above — but a preview has no Job at all, and
    // leading with the state of one is how the reported output claimed a Job
    // existed. This is the assertion that would have caught that.
    assert!(
        !preview.contains("QUEUED"),
        "a preview named a Job state for a Job with no record:\n{preview}"
    );
    // And what a preview says instead.
    for would in ["WOULD", "SKIPPED", "dry run", "nothing was spawned"] {
        assert!(
            preview.contains(would),
            "a preview stopped saying `{would}`"
        );
        assert!(
            !real.contains(would),
            "a real spawn says `{would}`, which reads as a preview:\n{real}"
        );
    }
    // The one row a preview really does perform keeps its past tense, with the
    // interval it really took.
    assert!(preview.contains("CLASSIFIED"));
    assert!(preview.contains("0.8s"));
    // And the one it does not: no interval is reported for work not done.
    assert!(
        !preview.contains("0.0s"),
        "a preview timed work that never happened:\n{preview}"
    );
}

/// **A guess is said in words, not left as a decimal in a column.**
///
/// A real spawn classified a task as `design` at `0.10` and proceeded silently.
/// A tenth is a coin flip, and `0.10` printed as one column among five tells a
/// reader nothing unless they already know the threshold — which is the opposite
/// of PLAN.md §14.2's *"the confidence surfaced so a guess is visible as a
/// guess"*.
///
/// **No fixture, deliberately.** The agreed layout covers the confident case and
/// is unchanged by this; inventing a second drawing for the warning would be
/// renegotiating a settled one. What is pinned here is that the warning appears,
/// that it names the flag that replaces it, and that a confident spawn is left
/// exactly as it was.
#[test]
fn a_low_confidence_spawn_says_so_in_words_and_names_the_override() {
    let render = |confidence: Option<f64>| {
        let output = Output::Spawn(Box::new(Envelope::ok(
            "fleet spawn",
            None,
            Status::Ready,
            SpawnData {
                uuid: "8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
                name: "this-test".to_string(),
                workflow: "design".to_string(),
                confidence,
                worktree: "~/.armada/workspaces/api/this-test".to_string(),
                branch: "armada/this-test".to_string(),
                port_block: None,
                budget: Budget {
                    attempts: 2,
                    cost_usd: 10.00,
                    wall_clock_ms: 90 * 60 * 1_000,
                    on_exhausted: OnExhausted::NeedsHuman,
                },
                step: "explore".to_string(),
                state: JobState::Running,
                classify_ms: Some(20_600),
                prepare_ms: 300,
                pgid: Some(4212),
            },
        )));
        render::human(&output, Style::plain(), Terminal::piped())
    };

    let guessed = render(Some(0.10));
    assert!(guessed.contains("a guess"), "{guessed}");
    assert!(guessed.contains("low confidence"), "{guessed}");
    assert!(
        guessed.contains("--workflow design|plan|feature|bug"),
        "the warning does not say how to fix it:\n{guessed}"
    );

    // **The confident case is untouched**, which is what keeps the agreed layout
    // agreed.
    let confident = render(Some(0.94));
    assert!(!confident.contains("a guess"), "{confident}");
    assert!(!confident.contains("low confidence"), "{confident}");

    // An override is neither: you named it, so there is nothing to warn about.
    let named = render(None);
    assert!(named.contains("you named it"), "{named}");
    assert!(!named.contains("low confidence"), "{named}");
}

// ---------------------------------------------------------------- the property
// the pair of files exists to prove

/// **The two audiences differ in styling and in nothing else** (PLAN.md §3.1.1).
///
/// Strip every escape from the `.tty` file, fold the typographic characters back
/// to their ASCII forms, and what remains must be the `.plain` file. This is the
/// guarantee the whole render layer is for, checked against files rather than
/// against a render — so it also catches a fixture that was updated on one side
/// only.
///
/// **Both sides are folded, not just the terminal one.** An em dash has two
/// sources: [`Style::nothing`], which is typographic for a person and ASCII for
/// an agent, and ordinary prose in a help page, which is an em dash for
/// everybody. Folding one side would report the second kind as a difference
/// between the audiences, which it is not. Every replacement is one column for
/// one column, so folding both sides cannot hide a column that moved — which is
/// the thing this test exists to catch.
#[test]
fn the_two_audiences_differ_only_in_styling() {
    let mut checked = 0;
    for entry in std::fs::read_dir(golden_dir()).expect("tests/golden/render exists") {
        let path = entry.expect("readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("tty") {
            continue;
        }
        let plain_path = path.with_extension("plain");
        let tty = std::fs::read_to_string(&path).expect("a .tty fixture");
        let plain = std::fs::read_to_string(&plain_path)
            .unwrap_or_else(|_| panic!("{} has no .plain twin", path.display()));

        assert!(
            tty.contains('\x1b'),
            "{} carries no escapes — is it really the terminal render?",
            path.display()
        );
        assert!(
            !plain.contains('\x1b'),
            "{} carries an escape, which is the one thing it must not",
            plain_path.display()
        );
        assert_eq!(
            without_wordmark(&fold(&strip_ansi(&tty))),
            without_wordmark(&fold(&plain)),
            "{} and its .plain twin are two different renders, not one render \
             twice. Same columns, same order, same words — styling is the only \
             thing that may differ (PLAN.md §3.1.1).",
            path.display()
        );
        checked += 1;
    }
    assert!(checked >= 8, "only {checked} pairs were checked");
}

/// **No status is signalled by colour alone.** Every fixture's status column is
/// a word, so a monochrome terminal, a pipe and a screen reader lose emphasis
/// and no information.
#[test]
fn no_fixture_carries_a_status_symbol() {
    for entry in std::fs::read_dir(golden_dir()).expect("tests/golden/render exists") {
        let path = entry.expect("readable").path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for symbol in ['✓', '✗', '✔', '✘', '×', '●', '○', '⚠'] {
            assert!(
                !text.contains(symbol),
                "{} contains {symbol}. A symbol that only appears at a terminal \
                 gives the two audiences different shapes, and one shape is the point.",
                path.display()
            );
        }
    }
}

/// Every escape in a `.tty` fixture is one the palette can account for. A
/// hand-edited fixture carrying an invented colour would otherwise pass.
#[test]
fn every_colour_in_a_fixture_is_from_the_palette() {
    let known: Vec<&str> = EVERY_ROLE
        .iter()
        .map(|role| role.fg())
        .chain([palette::RESET, palette::BOLD])
        .collect();

    for entry in std::fs::read_dir(golden_dir()).expect("tests/golden/render exists") {
        let path = entry.expect("readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("tty") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a .tty fixture");
        for sequence in escapes(&text) {
            assert!(
                known.contains(&sequence.as_str()),
                "{} uses {}, which is not in docs/commands/render.md's palette",
                path.display(),
                show(&sequence)
            );
        }
    }
}

const EVERY_ROLE: [palette::Role; 12] = [
    palette::Role::Void,
    palette::Role::Foreground,
    palette::Role::SignalAmber,
    palette::Role::NavalBlue,
    palette::Role::BeaconGreen,
    palette::Role::DistressRed,
    palette::Role::FlareOrange,
    palette::Role::RadarCyan,
    palette::Role::StasisPurple,
    palette::Role::AbortPink,
    palette::Role::SteelGrey,
    palette::Role::DeepSlate,
];

/// Every CSI sequence in the text, whole.
fn escapes(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            continue;
        }
        let mut sequence = String::from('\x1b');
        for c in chars.by_ref() {
            sequence.push(c);
            if c == 'm' {
                break;
            }
        }
        found.push(sequence);
    }
    found
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        for c in chars.by_ref() {
            if c == 'm' {
                break;
            }
        }
    }
    out
}

/// The typographic characters, folded to what the agent audience receives.
///
/// The dashes are one column for one column, so folding them can never move a
/// column — which is what makes the comparison above a test of styling rather
/// than of layout.
///
/// **Two replacements are not**, and both are confined to prose for exactly
/// that reason. `·` and `, ` differ by a column, and `→` and `->` differ by a
/// column; a summary line and a fix line are sentences rather than rows, so
/// neither can shear a table. That confinement is the rule those two glyphs
/// carry — [`Style::between`] says a summary line is prose and not a column,
/// and [`Style::arrow`] says a fix line may only ever open a line. A cell
/// holding either of them would make this test pass over two renders whose
/// columns genuinely disagree, which is the one thing it exists to catch.
///
/// [`Style::between`]: armada_helm::render::style::Style::between
/// [`Style::arrow`]: armada_helm::render::style::Style::arrow
fn fold(text: &str) -> String {
    text.replace(['—', '–'], "-")
        .replace('›', ">")
        .replace('→', "->")
        .replace(" · ", ", ")
}

/// The wordmark removed, so the comparison above is about the report.
///
/// **The banner is styling, and it is the largest piece of styling Armada has.**
/// `render/banner.rs` suppresses it on non-TTY stdout for the same reason
/// [`Style::nothing`] folds an em dash to a hyphen: anything decorative is for
/// the person and not for the parser. It carries no column, no word and no
/// number — six lines of block characters spelling the tool's own name — so a
/// `.tty` that opens with it and a `.plain` that does not are one render twice.
///
/// **Narrow on purpose: exactly the six known lines, matched literally.** It
/// cannot hide a difference in the report, because no line of any report is one
/// of these. The alternative considered and rejected was *"drop leading lines
/// until the two align"*, which would silently absorb a real row.
///
/// [`Style::nothing`]: armada_helm::render::style::Style::nothing
fn without_wordmark(text: &str) -> String {
    const ARMADA: [&str; 6] = [
        " █████╗ ██████╗ ███╗   ███╗ █████╗ ██████╗  █████╗",
        "██╔══██╗██╔══██╗████╗ ████║██╔══██╗██╔══██╗██╔══██╗",
        "███████║██████╔╝██╔████╔██║███████║██║  ██║███████║",
        "██╔══██║██╔══██╗██║╚██╔╝██║██╔══██║██║  ██║██╔══██║",
        "██║  ██║██║  ██║██║ ╚═╝ ██║██║  ██║██████╔╝██║  ██║",
        "╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝  ╚═╝",
    ];
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.iter().take(ARMADA.len()).eq(ARMADA.iter()) {
        // The wordmark, and the blank line `banner` writes after it.
        lines.drain(..ARMADA.len());
        if lines.first() == Some(&"") {
            lines.remove(0);
        }
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// the front door

/// Bare `armada` — five modules, a word and a fact each
/// (`docs/reserved/020-the-tui-decided.md`'s menu decision).
///
/// **Frozen because it is the first screen anybody sees.** Everything else in
/// this file is a report somebody went looking for; this is the one a person
/// meets before they know what Armada is, and its layout drifting is a
/// first impression drifting.
///
/// **What the fixture is asserting, beyond the bytes**: Helm leads, every row
/// opens with a status word from the fixed vocabulary, `DETAIL` carries counts
/// rather than a second status word, and there is **no summary line** — the
/// absence is the point. A headline over the five would be the one field
/// computed from two modules at once, which is what `ARCHITECTURE.md` §1.9 is
/// about on a screen that touches everything.
#[test]
fn menu_matches_its_fixture() {
    let results = vec![
        MenuRow {
            module: "helm".to_string(),
            status: Status::Ready,
            fact: "resumes your conversation".to_string(),
            verb: "armada helm".to_string(),
        },
        MenuRow {
            module: "fleet".to_string(),
            status: Status::Waiting,
            // Counts, never an aggregate word — `020`'s sixth decision, on a
            // second surface.
            fact: "4 jobs · 2 need you · 1 stalled".to_string(),
            verb: "armada fleet ls".to_string(),
        },
        MenuRow {
            module: "inbox".to_string(),
            status: Status::Waiting,
            fact: "2 questions waiting on you".to_string(),
            verb: "armada fleet inbox".to_string(),
        },
        MenuRow {
            module: "manifest".to_string(),
            status: Status::Ready,
            fact: "armada.yml — this workspace".to_string(),
            verb: "armada manifest status".to_string(),
        },
        MenuRow {
            module: "guild".to_string(),
            status: Status::Ready,
            fact: "19 skills · 2 hooks · 4 workflows".to_string(),
            verb: "armada guild ls".to_string(),
        },
    ];

    let output = Output::Menu(Box::new(Envelope::ok(
        "menu",
        None,
        Status::Ok,
        MenuData { results },
    )));
    assert_render("menu", &output);
}

/// A machine with nothing on it still gets five rows, and each says what to
/// type.
///
/// **The empty case is the one a new reader meets**, so it is frozen too. A
/// front door that went blank on a fresh install would be the screen failing at
/// exactly the moment it is most needed.
#[test]
fn menu_on_a_fresh_machine_matches_its_fixture() {
    let results = vec![
        MenuRow {
            module: "helm".to_string(),
            status: Status::Down,
            fact: "off on this machine".to_string(),
            // **The switch, not the verb it gates.** A row that said `armada
            // helm` beside `DOWN` would advertise the command that refuses.
            verb: "armada helm enable".to_string(),
        },
        MenuRow {
            module: "fleet".to_string(),
            status: Status::Ok,
            fact: "no jobs".to_string(),
            verb: "armada fleet ls".to_string(),
        },
        MenuRow {
            module: "inbox".to_string(),
            status: Status::Ok,
            fact: "nothing open".to_string(),
            verb: "armada fleet inbox".to_string(),
        },
        MenuRow {
            module: "manifest".to_string(),
            status: Status::Down,
            fact: "no armada.yml here".to_string(),
            verb: "armada manifest init".to_string(),
        },
        MenuRow {
            module: "guild".to_string(),
            status: Status::Down,
            fact: "no guild yet".to_string(),
            verb: "armada init".to_string(),
        },
    ];

    let output = Output::Menu(Box::new(Envelope::ok(
        "menu",
        None,
        Status::Ok,
        MenuData { results },
    )));
    assert_render("menu-fresh", &output);
}

// ---------------------------------------------------------------------------
// `020` §5 — an action with a duration gets a state word

/// A Job being aborted says `ABORTING`, and names the slow part — on **both**
/// listings (`docs/reserved/020-the-tui-decided.md` §5).
///
/// **This is the bug the whole of `020` was written around.** The owner aborted
/// a Job, pressed `y`, and the screen said nothing for several seconds while
/// `armada manifest clean` talked to docker. The abort was working. A working
/// abort and a hung one were the same screen, because both rows said `RUNNING`.
///
/// **Both surfaces in one test, deliberately.** They are two renderers over one
/// listing, and the failure `020` §5 names is one of them remembering to prefer
/// the action while the other forgets — a test per surface would pass while they
/// disagreed. This one cannot.
///
/// **No golden pair, and that is why it is a separate test.** A fixture freezes
/// a layout; an action is a transient, and a frame caught mid-abort is not a
/// layout anybody should have to regenerate by hand to change an unrelated
/// column. What is asserted here is the substitution, which is the behaviour.
#[test]
fn a_job_being_acted_on_says_the_action_and_names_the_slow_part() {
    let untouched = aborting_row();
    let mut acting = untouched.clone();
    acting.acting = Some(armada_core::fleet::job::Doing {
        acting: armada_core::fleet::Acting::Aborting,
        slow: Some("docker".to_string()),
        since_ms: 1_000,
    });
    acting.acting_for_s = Some(12);

    for (surface, before, after) in [
        (
            "fleet ls",
            fleet_ls_text(&untouched),
            fleet_ls_text(&acting),
        ),
        ("bridge", bridge_text(&untouched), bridge_text(&acting)),
    ] {
        // **The row, not the render.** The summary line under both tables
        // carries the *envelope's* status, and a fleet with a Job in it is
        // `RUNNING` whatever anybody is doing to one of its rows — that word
        // describes the listing and this test is about the row.
        let (before, after) = (job_line(surface, &before), job_line(surface, &after));
        assert!(
            before.contains("RUNNING") && !before.contains("ABORTING"),
            "{surface} says the state while nobody is acting:\n{before}"
        );
        assert!(
            after.contains("ABORTING"),
            "{surface} drew the state over a running abort, which is 020 §5's silence:\n{after}"
        );
        assert!(
            !after.contains("RUNNING"),
            "{surface} drew both words for one row:\n{after}"
        );
        // **The stage and its clock, which is the half that makes the word
        // useful.** `ABORTING` says an abort is happening; `docker 12s` says
        // which part of it is the one taking the time.
        assert!(
            after.contains("docker 12s"),
            "{surface} did not name the slow part:\n{after}"
        );
        // **No bar and no spinner** (PHASES.md §9.1 F2, `020` §5). The status
        // word carries the fact that something is running, which is exactly why
        // a bar computed from a turn count is refused: it would be a guess drawn
        // as a measurement.
        for banned in ['█', '▓', '▒', '░', '%'] {
            assert!(
                !after.contains(banned),
                "{surface} drew a progress indicator `{banned}`:\n{after}"
            );
        }
    }
}

/// The other two words reach both listings the same way, and none of the three
/// is a Job state.
#[test]
fn reaping_and_pausing_reach_the_row_the_same_way() {
    for acting in armada_core::fleet::Acting::ALL {
        let mut row = aborting_row();
        row.acting = Some(armada_core::fleet::job::Doing {
            acting,
            slow: Some("worktree".to_string()),
            since_ms: 0,
        });
        row.acting_for_s = Some(3);
        for (surface, text) in [
            ("fleet ls", fleet_ls_text(&row)),
            ("bridge", bridge_text(&row)),
        ] {
            assert!(
                text.contains(acting.word()),
                "{surface} lost `{}`:\n{text}",
                acting.word()
            );
            assert!(
                text.contains("worktree 3s"),
                "{surface} lost the slow part under `{}`:\n{text}",
                acting.word()
            );
        }
    }
}

/// An action that has not reached anything slow gets the word and no stage.
///
/// **A stage that has not started is not a stage to name**, which is the refusal
/// that already keeps `implement 0s` off the `STEP` column: a zero reads as a
/// measurement, and nothing has been measured.
#[test]
fn an_action_with_nothing_slow_yet_names_no_stage() {
    let mut row = aborting_row();
    row.acting = Some(armada_core::fleet::job::Doing::started(
        armada_core::fleet::Acting::Aborting,
        0,
    ));
    row.acting_for_s = None;
    for (surface, text) in [
        ("fleet ls", fleet_ls_text(&row)),
        ("bridge", bridge_text(&row)),
    ] {
        assert!(text.contains("ABORTING"), "{surface}:\n{text}");
        assert!(
            !text.contains("0s"),
            "{surface} drew a clock nobody started:\n{text}"
        );
    }
}

/// The one table row about `rate-limit`, out of a whole render.
///
/// **Every assertion above is about a row**, and both listings print a summary
/// line under the table carrying the envelope's own status word. Matching on the
/// whole render would let that line answer for the row — which is how a test
/// that reads `RUNNING` somewhere on the screen passes over a row that says
/// `ABORTING`, and over one that does not.
fn job_line<'a>(surface: &str, text: &'a str) -> &'a str {
    text.lines()
        .find(|line| line.contains("rate-limit"))
        .unwrap_or_else(|| panic!("{surface} drew no row for `rate-limit`:\n{text}"))
}

/// One Job, `RUNNING` and untouched, as both listings receive it.
fn aborting_row() -> JobRow {
    JobRow {
        uuid: "c19d0a34-3069-4115-ad92-e81f486ce8b9".to_string(),
        name: "rate-limit".to_string(),
        workflow: "feature".to_string(),
        // **Still `RUNNING` on disk**, which is the whole reason the two words
        // are laid over one another rather than folded into one enum: a crash
        // mid-abort must not leave a Job claiming a state no verb reached.
        state: JobState::Running,
        detail: "implement".to_string(),
        step: "implement".to_string(),
        on_step_s: Some(840),
        task: "hold the rate limit".to_string(),
        runtime_s: 840,
        cost_usd: 2.10,
        tokens: 120_000,
        turns: 4,
        budget_remaining: Remaining {
            attempts: 2,
            cost_usd: 8.75,
            wall_clock_ms: 1_860_000,
        },
        needs_attention: false,
        acting: None,
        acting_for_s: None,
    }
}

fn fleet_ls_text(row: &JobRow) -> String {
    let output = Output::FleetLs(Box::new(Envelope::ok(
        "fleet ls",
        None,
        Status::Running,
        FleetLsData {
            results: vec![row.clone()],
            needs_you: 0,
            spent_usd: row.cost_usd,
            windows: Vec::new(),
        },
    )));
    render::human(&output, Style::plain(), Terminal::piped())
}

/// **Fleet-only, deliberately.** These helpers measure one column against one
/// row, and the four panels beside JOBS would put three tables of unrelated
/// content around the thing being measured.
fn bridge_text(row: &JobRow) -> String {
    let output = Output::Bridge(Box::new(Envelope::ok(
        "bridge",
        None,
        Status::Running,
        BridgeData {
            needs_you: 0,
            spent_usd: row.cost_usd,
            running: 1,
            filter: None,
            hidden: 0,
            cwd: String::new(),
            panels: None,
            windows: Vec::new(),
            results: vec![row.clone()],
        },
    )));
    render::human(&output, Style::plain(), Terminal::piped())
}
