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
    Asked, BridgeData, CheckData, CleanData, ComponentView, ComponentsData, DispatchData,
    DoctorData, Envelope, Finding, FleetLsData, GrantedCommand, GuildChange, GuildChangeData,
    GuildChoice, GuildItemData, GuildItemRow, GuildListData, GuildSyncData, Headline, InitData,
    JobRow, MachineInitData, PortReport, Problem, Projection, Released, ResolvedSkillView,
    ResultRow, ScanData, ServicesData, Settled, SkillsData, SpawnData, StatusData, Sync, SyncItem,
    Unreclaimed, UpDryRun, VerifyData,
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

// ----------------------------------------------------------------- the world
// the fixtures describe: one workspace, deterministic in every field

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
            asked: vec![Asked {
                number: 1,
                of: 5,
                prompt: armada_guild::interview::QUESTIONS[0].prompt.to_string(),
                purpose: armada_guild::interview::QUESTIONS[0].purpose.to_string(),
                writes: armada_guild::interview::QUESTIONS[0].writes.to_string(),
                keeps: armada_guild::interview::QUESTIONS[0].keeps.to_string(),
                prose: true,
                // **What import wrote, as the question shows it.** A prompt that
                // says *enter keeps what import found* over nothing is a default
                // the reader cannot accept with confidence, which is what a real
                // first run said about it.
                standing: Some(
                    "Lead with the answer. Tables for anything comparative.".to_string(),
                ),
            }],
            questions: 5,
            answered: 0,
            guild_path: "~/.armada/guild".to_string(),
        },
    )));
    assert_render("init-machine", &output);
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
        },
    )));
    assert_render("guild-ls", &output);
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
#[test]
fn fleet_ls_matches_its_fixture() {
    fn row(
        name: &str,
        workflow: &str,
        state: JobState,
        detail: &str,
        cost_usd: f64,
        runtime_s: u64,
        needs_attention: bool,
    ) -> JobRow {
        JobRow {
            uuid: format!("{name}-uuid"),
            name: name.to_string(),
            workflow: workflow.to_string(),
            state,
            detail: detail.to_string(),
            // Carried by the listing and drawn by the Bridge, never by `ls` —
            // `DETAIL` answers what a Job is doing now, the task answers what it
            // was asked to do.
            task: format!("the {name} task"),
            runtime_s,
            cost_usd,
            tokens: 120_000,
            turns: 4,
            budget_remaining: Remaining {
                iterations: 8,
                tokens: 280_000,
                wall_clock_ms: 1_860_000,
            },
            needs_attention,
        }
    }

    let results = vec![
        row(
            "rate-limit",
            "feature",
            JobState::Running,
            "implement, check green",
            2.10,
            14 * 60,
            false,
        ),
        row(
            "carina-schema",
            "feature",
            JobState::Running,
            "plan, awaiting you",
            0.45,
            3 * 60,
            true,
        ),
        row(
            "xlsx-report",
            "bug",
            JobState::Stalled,
            "no output for 6m",
            4.60,
            22 * 60,
            false,
        ),
        row(
            "release-merge",
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
        row("nightly-flake", "bug", JobState::Queued, "", 0.0, 0, false),
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
        },
    )));
    assert_render("fleet-ls", &output);
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
    fn row(
        name: &str,
        state: JobState,
        task: &str,
        cost_usd: f64,
        runtime_s: u64,
        needs_attention: bool,
    ) -> JobRow {
        JobRow {
            uuid: format!("{name}-uuid"),
            name: name.to_string(),
            workflow: "feature".to_string(),
            state,
            detail: "implement".to_string(),
            task: task.to_string(),
            runtime_s,
            cost_usd,
            tokens: 120_000,
            turns: 4,
            budget_remaining: Remaining {
                iterations: 8,
                tokens: 280_000,
                wall_clock_ms: 1_860_000,
            },
            needs_attention,
        }
    }

    let results = vec![
        row(
            "rate-limit",
            JobState::Running,
            "add gateway limiter",
            2.10,
            14 * 60,
            false,
        ),
        row(
            "carina-schema",
            JobState::Running,
            "migrate schema",
            0.45,
            3 * 60,
            false,
        ),
        row(
            "xlsx-report",
            JobState::Stalled,
            "generate report",
            4.60,
            22 * 60,
            false,
        ),
        row(
            "release-merge",
            JobState::Blocked,
            "merge release",
            1.25,
            65 * 60,
            true,
        ),
    ];

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
            results,
        },
    )));
    assert_render("bridge", &output);
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
        uuid: "carina-schema-uuid".to_string(),
        name: "carina-schema".to_string(),
        workflow: "feature".to_string(),
        state: JobState::Running,
        detail: "implement".to_string(),
        task: "migrate schema".to_string(),
        runtime_s: 3 * 60,
        cost_usd: 0.45,
        tokens: 12_000,
        turns: 1,
        budget_remaining: Remaining {
            iterations: 11,
            tokens: 388_000,
            wall_clock_ms: 2_520_000,
        },
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
            results,
        },
    )));
    assert_render("bridge-filtered", &output);
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
                iterations: 20,
                tokens: 600_000,
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
                    iterations: 15,
                    tokens: 500_000,
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
            fold(&strip_ansi(&tty)),
            fold(&plain),
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
