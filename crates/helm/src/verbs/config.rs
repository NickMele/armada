//! `armada manifest config` — layers 1 and 3 of the bootstrap sandwich
//! (PLAN.md §5).
//!
//! ```text
//! 1. scan     Armada    an evidence report, never a config
//! 2. author   an agent  the armada.yml, from evidence + schema + an example
//! 3. verify   Armada    pass/fail, with the key path and the fix
//! ```
//!
//! Layer 2 is still not here, and its absence is still the design: deciding
//! which of four test scripts is canonical is judgement, and a scanner that
//! supplies judgement produces a file nobody can trust and everybody has to
//! re-read.
//!
//! **What `scan` now does between 1 and 2 is propose what it can prove**
//! (`docs/reserved/007`) — a `pnpm-lock.yaml` is proof of pnpm and a script
//! named exactly `test` is proof of a test check, while `test:changed` is proof
//! of nothing. A reader ticks what is right and Armada writes only that, so the
//! judgement stays his and the transcription stops being his.
//!
//! **`scan` is the one verb that runs with no `armada.yml` at all** (PLAN.md
//! §2.1), which is why it takes a directory rather than an [`App`]: there is no
//! workspace to resolve, no lease to hold and nothing to reap.
//!
//! [`App`]: crate::app::App

use armada_core::config::ResolvedConfig;
use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{aggregate, Envelope, ResultRow, ScanData, VerifyData};
use armada_core::error::{ArmadaError, Status};
use armada_core::glob;
use armada_core::verify::{self, Answers};
use armada_core::{ports, scan};
use armada_manifest::{config_file, git};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::app::App;
use crate::args::BUILTIN_VERBS;
use crate::ask::Choice;
use crate::render::progress::Progress;
use crate::render::term::Terminal;
use crate::verbs::Output;

/// `armada manifest config scan`, over one directory.
///
/// **Exit 0 whenever the directory is readable**
/// (`docs/commands/manifest/config.md`): it reports rather than judges, so
/// there is no outcome it could fail on. A file that does not parse contributes
/// nothing and the other twelve pieces of evidence still print.
/// `home` is `~/`, for finding the guild that holds the onboarding skill;
/// `terminal` says which audience is reading, which is what decides whether
/// there is anything to ask. Both are the entrypoint's to know
/// (`ARCHITECTURE.md` §1.4) and neither is looked up here.
pub fn scan(
    run: &impl Run,
    root: &Path,
    home: Option<&Path>,
    terminal: Terminal,
    json: bool,
) -> Result<Output, ArmadaError> {
    let files = armada_manifest::scan::read(run, root);
    let evidence = scan::scan(&files);

    // **Manifest hands over; it does not depend on what it hands over to.** The
    // skill belongs to the guild and this is `helm`, the one crate permitted to
    // name both (`ARCHITECTURE.md` §1.9). Nothing under `crates/manifest` learns
    // that an agent exists.
    let skill = home.is_some_and(|home| {
        armada_guild::layout::Guild::at(&armada_manifest::machine::armada_home(home))
            .has_skill(armada_guild::layout::ONBOARD_REPO)
    });
    let command = armada_guild::layout::skill_command_line(armada_guild::layout::ONBOARD_REPO);

    Ok(Output::Scan(Box::new(Envelope::ok(
        "config scan",
        // **No workspace id, deliberately.** A repository being scanned has not
        // been claimed and has no identity yet; inventing one here would be the
        // first thing `scan` decided.
        None,
        Status::Ok,
        ScanData {
            results: scan::findings(&evidence),
            // **Computed even for a repository that already has an
            // `armada.yml`.** They are not offered to be written there — that is
            // drift, and drift is not built — but a consumer asking `--json`
            // what this repository proves gets the same answer whichever state
            // it is in, and an agent comparing the two lists is exactly what
            // drift will be built out of.
            proposals: armada_core::propose::propose(&evidence),
            evidence,
            handover: scan::handover(
                json,
                terminal.stdin_is_tty,
                terminal.stdout_is_tty,
                skill,
                &command,
            ),
        },
    ))))
}

/// The question `config scan` ends on, and its answers.
///
/// **Here rather than in the renderer**, which is where the options used to be
/// spelled: they are now put by a selector on stderr, printed as a list when
/// there is no selector to draw, and echoed afterwards — three call sites, so
/// the words live once, next to the verb that means them.
pub const ONBOARD_QUESTION: &str = "Write armada.yml now?";

/// The question the tick list puts, once the reader has said he wants the
/// proposals written.
pub const CONFIRM_QUESTION: &str = "Which of these did it get right?";

/// What the reader chose to happen next.
///
/// **An enum rather than three numbered constants**, because the numbers move:
/// a repository whose evidence proves nothing is offered two options and one
/// whose evidence proves something is offered three, and a caller comparing
/// against a `1` would silently mean *write* in one repository and *hand over*
/// in the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Put the proposals up to be ticked, and write what survives.
    Write,
    /// Hand the repository to an agent.
    HandOver,
    /// Print the evidence and do nothing.
    Stop,
}

/// The answers, in the order they are offered.
///
/// **The proposals come first when there are any**, because they are the cheap
/// answer: a proposal a reader corrects costs no tokens and the same file
/// authored by an agent costs a session. The hand-over stays underneath it, for
/// the repositories where the evidence genuinely does not settle it — which this
/// reduces the number of and does not replace.
pub fn onboarding_choices(proposals: usize) -> Vec<Choice> {
    let mut out = Vec::new();
    if proposals > 0 {
        out.push(Choice::new(
            "write what the scan can prove",
            &format!("{proposals} proposed lines, yours to tick"),
        ));
    }
    out.push(Choice::new(
        "let an agent write it with me",
        "opens claude here",
    ));
    out.push(Choice::new(
        "print the evidence and stop",
        "I will write armada.yml myself",
    ));
    out
}

/// What a one-based answer to [`onboarding_choices`] means.
pub fn next(chosen: usize, proposals: usize) -> Next {
    match (chosen, proposals > 0) {
        (1, true) => Next::Write,
        (1, false) | (2, true) => Next::HandOver,
        _ => Next::Stop,
    }
}

/// The answer an unanswered question takes: print the evidence and stop.
///
/// **Doing nothing is the default**, because the two alternatives are launching
/// a session nobody asked for and writing a file nobody confirmed.
pub fn stop(proposals: usize) -> usize {
    onboarding_choices(proposals).len()
}

/// One tickable line per proposal: what it would write, and the file that proves
/// it.
///
/// **The key path is the label and the value is the aside.** A reader deciding
/// whether `components.web.checks.test` is right is deciding about `pnpm run
/// test`, and a list that showed him only one of the two would be a list he had
/// to cross-reference against the report above it.
pub fn proposal_choices(proposals: &[armada_core::propose::Proposal]) -> Vec<Choice> {
    proposals
        .iter()
        .map(|proposal| Choice::new(&proposal.at, &proposal.writes))
        .collect()
}

/// What the tick list ended in.
#[derive(Debug)]
pub enum Wrote {
    /// The file, and how many lines went into it.
    Config(usize),
    /// **Nothing, and not as a failure.** Esc at the tick list is one way to get
    /// here and unticking every row is the other; both are the reader answering
    /// the question he was asked, and the evidence he was answering about is
    /// still on the screen above him.
    Nothing,
    /// The file could not be written, or was already there.
    Failed(Box<ArmadaError>),
}

/// Put the proposals up to be ticked, and write what survives.
///
/// **The whole exchange, in one testable place.** The entrypoint owns the
/// terminal and nothing else here: it hands over an `Ask` and a directory and
/// renders what comes back, which is what keeps this path covered by a scripted
/// conversation rather than only by somebody pressing keys.
pub fn confirm(
    ask: &mut impl crate::ask::Ask,
    proposals: &[armada_core::propose::Proposal],
    root: &Path,
) -> Wrote {
    // Everything offered here is provable, so the list opens with all of it
    // ticked: the reader's work is taking off what his repository disagrees
    // with, which is the shorter half of the job.
    let all = vec![true; proposals.len()];
    let Some(ticked) = ask.pick(CONFIRM_QUESTION, &proposal_choices(proposals), &all) else {
        return Wrote::Nothing;
    };

    let accepted = armada_core::propose::accepted(proposals, &ticked);
    if accepted.is_empty() {
        return Wrote::Nothing;
    }
    match write(root, &accepted) {
        Ok(()) => Wrote::Config(accepted.len()),
        Err(error) => Wrote::Failed(Box::new(error)),
    }
}

/// Write the confirmed proposals as this repository's `armada.yml`.
///
/// **The one thing `config scan` writes, and only after a person has ticked
/// it.** `scan` is the verb that runs in a repository with no config and it
/// decides nothing on its own; this is reached from one place, behind a selector
/// that is only ever drawn when both streams are a terminal.
///
/// **It refuses rather than overwrites.** A repository that gained an
/// `armada.yml` between the scan and the answer — another agent, another
/// terminal, a `git pull` — is one where the file in front of Armada is not the
/// one it was proposing against. Whether the two agree is drift, and drift is
/// not built.
pub fn write(root: &Path, accepted: &[armada_core::propose::Proposal]) -> Result<(), ArmadaError> {
    let path = root.join("armada.yml");
    if path.exists() {
        return Err(ArmadaError {
            class: armada_core::error::ErrClass::BadConfig,
            r#where: "armada.yml".to_string(),
            message: "armada.yml is already here, so nothing was written".to_string(),
            next_action: Some(
                "read it, or move it aside and run `armada manifest config scan` again".to_string(),
            ),
        });
    }
    std::fs::write(&path, armada_core::propose::write(accepted)).map_err(|error| ArmadaError {
        class: armada_core::error::ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("could not write armada.yml: {error}"),
        next_action: None,
    })
}

/// `armada manifest config verify`, in the two passes PLAN.md §5 specifies.
///
/// ```text
/// pass 1  STATIC     schema, references, argv[0]; seconds; failures short-circuit
/// pass 2  FOR REAL   the check suite, run exactly as `armada manifest check` runs it
/// ```
///
/// **Pass 2 is a real run and not a simulation.** An earlier draft had verify
/// dry-invoke every `cmd:` with `--help` / `--version` / `--dry-run`, which was
/// the worst of both worlds: Armada cannot know which of those three a given
/// tool accepts, so against the fixture set it would either run the Playwright
/// suite, create a Kubernetes cluster, or fail a correct config. Guessing a flag
/// is not verification. If you want to know a config works, run it.
///
/// **Consequence, stated plainly:** this is not a seconds-long verb. Pass 1 is,
/// and it catches the hallucinated script name that motivated layer 3 in the
/// first place; pass 2 takes as long as the repo's checks take. An authoring
/// loop iterating on pass-1 failures stays fast, and a full verify is a build.
pub fn verify<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    let workspace = app.ctx.workspace()?.clone();
    let text = config_file::read(&workspace.config_path())?;
    let label = workspace.config_label.clone();

    let started = app.ctx.now.mono();
    let (schema, parsed) = read_and_validate(&text, &label);
    let schema: Vec<&verify::Finding> = schema.iter().collect();
    let schema_row = row("schema", &schema, None, app.ctx.now.mono() - started);

    // **Short-circuit, and say so by omission.** A config the schema rejects has
    // no resolved form, so `references` and `argv[0]` did not run — and a row
    // claiming they did, in any status, would be a claim about work nobody did.
    let Some(config) = parsed else {
        return Ok(answer(&workspace, vec![schema_row], 0, None));
    };

    let at = app.ctx.now.mono();
    let answers = look(app, &workspace, &config)?;
    let block = app.machine.port_block_size;
    // **A representative block from this machine's own floor.** `verify` asks
    // whether every `${port.NAME}` resolves inside a block of this width, which
    // is a question about the count and not about the numbers — but reporting
    // it against a range the machine will never claim would make a reader chase
    // a discrepancy that is only this line's.
    let base = app.machine.port_base;
    let report = verify::pass_one(
        &config,
        &answers,
        ports::PortBlock {
            from: base,
            to: base + block.saturating_sub(1),
        },
        &BUILTIN_VERBS,
        &label,
    );
    let elapsed = app.ctx.now.mono() - at;

    let mut results = vec![schema_row];
    results.push(row(
        "references",
        &report.under("references"),
        Some(verify::REFERENCES_DETAIL),
        elapsed,
    ));
    results.push(row("argv[0]", &report.under("argv[0]"), None, elapsed));

    if !report.passed() {
        return Ok(answer(&workspace, results, report.unchecked, None));
    }

    // Pass 2 inherits `check`'s semantics wholesale, including that **verify
    // does not stop what it started**: a check declaring `needs:` starts its
    // services and leaves them up, same rule and same reason as `check`.
    let Output::Check(run) = super::check::run(app, &Default::default(), progress)? else {
        return Err(ArmadaError {
            class: armada_core::error::ErrClass::ArmadaBug,
            r#where: "config verify".to_string(),
            message: "pass 2 did not answer with a run".to_string(),
            next_action: None,
        });
    };
    Ok(answer(
        &workspace,
        results,
        report.unchecked,
        Some(Box::new(run.data)),
    ))
}

/// The document as a value for the schema, and its resolved form when the
/// contract accepts it.
///
/// **Two entry points to one contract, and both are consulted** (PLAN.md
/// §4.1.1): the schema owns every rule readable from a single value, and the
/// structs own everything needed to produce a typed value at all. A document
/// either of them rejects has failed the `schema` row, because from the
/// reader's side they are the same statement.
fn read_and_validate(text: &str, file: &str) -> (Vec<verify::Finding>, Option<ResolvedConfig>) {
    let mut findings = verify::schema_findings(text, file);

    let resolved = armada_core::config::parse(text, file)
        .and_then(|config| {
            armada_core::config::resolve(config, &armada_core::config::Defaults::built_in(), file)
        })
        .map_err(|error| {
            findings.push(verify::Finding {
                check: "schema",
                error,
            })
        })
        .ok();
    (findings, resolved)
}

/// Answer everything [`verify::probe`] asked about, from the filesystem.
fn look<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    workspace: &armada_core::workspace::Workspace,
    config: &ResolvedConfig,
) -> Result<Answers, ArmadaError> {
    let probe = verify::probe(config);
    let root = &workspace.root;

    // **The tracked set, not a walk.** `match:` decides which changed files
    // select a check, and the file set it is scoped against is the one `check
    // --all-files` uses — so a glob that "matches nothing" here means the same
    // thing it will mean at run time.
    let tracked = git::tracked_files(&app.ctx.run, root)?;
    let matched: BTreeSet<String> = probe
        .globs
        .iter()
        .filter(|pattern| tracked.iter().any(|path| glob::matches(pattern, path)))
        .cloned()
        .collect();

    // A path counts only if it is there **and** still inside the workspace once
    // symlinks are resolved. The schema cannot see the second half, and `clean
    // --artifacts` deletes what these name.
    let inside: BTreeSet<String> = probe
        .paths
        .iter()
        .filter(|relative| {
            let target = root.join(relative);
            match (
                armada_manifest::fs::canonical(&target),
                armada_manifest::fs::canonical(root),
            ) {
                (Ok(target), Ok(root)) => target.starts_with(&root),
                _ => false,
            }
        })
        .cloned()
        .collect();

    let resolvable: BTreeMap<String, bool> = probe
        .programs
        .iter()
        .map(|program| {
            let under = program
                .root
                .as_ref()
                .map_or_else(|| root.clone(), |sub| root.join(sub));
            (
                program.at.clone(),
                resolves(&app.inherited, &program.argv0, &under, root),
            )
        })
        .collect();

    Ok(Answers {
        resolvable,
        inside,
        matched,
        package_manager: package_manager(&app.ctx.run, root),
    })
}

/// Whether `argv[0]` names something this machine can execute.
///
/// Two ways, and the second is the one a monorepo needs: on `PATH`, or an
/// executable file under the component root — `./scripts/kind-up.sh` is a
/// perfectly good `cmd:` and is on nobody's `PATH`.
fn resolves(inherited: &BTreeMap<String, String>, argv0: &str, under: &Path, root: &Path) -> bool {
    if argv0.contains('/') {
        return [under, root]
            .iter()
            .any(|base| is_executable(&base.join(argv0)));
    }
    inherited
        .get("PATH")
        .map(|path| std::env::split_paths(path).any(|dir| is_executable(&dir.join(argv0))))
        .unwrap_or(false)
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// The package manager this repository's lockfile names, for the suggestion.
///
/// **A fact, and the only use made of it is to phrase a question.** `verify`
/// says "did you mean `pnpm exec vitest run`?" because `pnpm-lock.yaml` is
/// there and pnpm is what writes it — it does not decide that the repository
/// runs anything.
fn package_manager(run: &impl Run, root: &Path) -> Option<String> {
    let evidence = scan::scan(&armada_manifest::scan::read(run, root));
    evidence
        .lockfiles
        .iter()
        // Only the managers whose `exec` subcommand exists; suggesting `cargo
        // exec` would be a guess wearing a fact's clothes.
        .find_map(|lockfile| {
            let manager = lockfile.manager.split('@').next().unwrap_or_default();
            ["pnpm", "npm", "yarn", "bun"]
                .contains(&manager)
                .then(|| manager.to_string())
        })
}

/// One pass-1 row: passing with its label, or failing with the first finding.
///
/// **The first finding and not all of them**, because a row is one line: the
/// rest reach the reader through `--json`, and every one of them contributes a
/// fix line under the summary.
fn row(id: &str, findings: &[&verify::Finding], label: Option<&str>, ms: u64) -> ResultRow {
    let mut row = ResultRow::new(id, Status::Pass);
    row.duration_ms = Some(ms);
    match findings.first() {
        Some(finding) => {
            row.status = Status::Failed;
            row.error = Some(finding.error.clone());
        }
        None => row.reason = label.map(str::to_string),
    }
    row
}

/// The envelope, with the error aggregated the way every other verb aggregates.
fn answer(
    workspace: &armada_core::workspace::Workspace,
    results: Vec<ResultRow>,
    unchecked: usize,
    pass_2: Option<Box<armada_core::envelope::CheckData>>,
) -> Output {
    // Pass 2's own verdict wins when there is one: pass 1 passed, so the thing
    // the caller is being told about is the run.
    let (status, error) = match &pass_2 {
        Some(run) => (
            if run.results.iter().all(|r| r.status != Status::Failed) {
                Status::Pass
            } else {
                Status::Failed
            },
            aggregate(&run.results, "checks"),
        ),
        None => match aggregate(&results, "checks") {
            Some(error) => (Status::Failed, Some(error)),
            None => (Status::Pass, None),
        },
    };

    Output::Verify(Box::new(Envelope {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        verb: "config verify".to_string(),
        workspace: Some(workspace.id.clone()),
        status,
        error,
        data: VerifyData {
            results,
            unchecked,
            pass_2,
        },
    }))
}
