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
    CheckData, CleanData, DispatchData, Envelope, GrantedCommand, InitData, PortReport, Released,
    ResolvedSkillView, ResultRow, ScanData, SkillsData, StatusData, Unreclaimed, VerifyData,
};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::id::WorkspaceId;
use armada_core::ports::{PortBlock, PortState};
use armada_core::reap::ReapPlan;
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
            port_block: block(),
            claimed_at: "2026-08-09T14:02:11Z".to_string(),
            ports: BTreeMap::from([("api".to_string(), 5460), ("web".to_string(), 5461)]),
            reaped: ReapPlan::default(),
            results: vec![api],
        },
    )));
    assert_render("init", &output);
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
#[test]
fn config_scan_matches_its_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/next-prisma");
    let files = armada_manifest::scan::read(&root);
    let evidence = armada_core::scan::scan(&files);
    let output = Output::Scan(Box::new(Envelope::ok(
        "config scan",
        None,
        Status::Ok,
        ScanData {
            results: armada_core::scan::findings(&evidence),
            evidence,
        },
    )));
    assert_render("config-scan", &output);
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

/// `armada --help`, which is the page the milestone was opened for.
#[test]
fn the_help_pages_match_their_fixtures() {
    let mut failures = Vec::new();
    for (case, topic) in [
        ("help", Topic::Root),
        ("help-manifest", Topic::Manifest),
        ("help-check", Topic::Verb("check")),
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
        .replace('→', "->")
        .replace(" · ", ", ")
}
