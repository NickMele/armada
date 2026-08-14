//! `armada` — the binary.
//!
//! The entrypoint does the four things that must be right before any verb
//! exists, and then gets out of the way:
//!
//! 1. **Restores `SIGPIPE`**, before a single byte is written — measured, Rust
//!    ignores it and `armada manifest status | head` panics with exit 101 otherwise
//!    (`docs/traps.md`).
//! 2. **Reads the ambient world exactly once** — cwd, `$HOME`, the environment
//!    — and passes it down. Nothing below this file may reach for any of them
//!    (`ARCHITECTURE.md` §1.4), because `--project` and `--all` operate on
//!    workspaces that are not the current directory.
//! 3. **Flushes stdout explicitly on every exit path**, because `process::exit`
//!    skips a `BufWriter` flush and the failure is *size-dependent*: a 491-byte
//!    payload is lost and a 20 KB one is not, so it passes a test with a large
//!    fixture and silently empties a small real one.
//! 4. **Exits by error class and by nothing else** — `exit = f(error.class)`,
//!    with two carve-outs the rule has no room for: signal-derived codes, and a
//!    dispatched child's code, which passes through verbatim.

#![deny(unsafe_code)]

use armada_helm::{app, args, render, verbs};

use args::Invocation;
use armada_core::ctx::Ctx;
use armada_core::envelope::{Envelope, NoData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_manifest::clock::SystemClock;
use armada_manifest::net::RealFetch;
use armada_manifest::process::RealRun;
use armada_manifest::{discovery, posix};
use render::style::Style;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use verbs::Output;

fn main() -> ExitCode {
    posix::restore_sigpipe();
    // **Without this, Ctrl-C leaves the children running.** They are `setsid`'d
    // into their own sessions, so a signal delivered to Armada never reaches
    // them (`PHASES.md` §9.3). Trapping it lets the run loop end the run
    // properly — kill each group, mark the rest ABORTED — instead of the
    // process dying and orphaning a `cargo test`.
    posix::catch_interrupts();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let inherited: BTreeMap<String, String> = std::env::vars().collect();

    // **The terminal is ambient state, so it is read here and passed down**
    // (`ARCHITECTURE.md` §1.4) — the same rule that puts cwd and `$HOME` on the
    // three lines above. `NO_COLOR` comes out of the environment map this
    // function already built, rather than from a second `std::env` call in the
    // renderer: one read, one answer, and a test can set it.
    let terminal = render::term::Terminal::detect();
    let no_color = inherited.contains_key("NO_COLOR");

    let parsed = match args::parse(&argv) {
        Ok(parsed) => parsed,
        // `--json` is answered even when the failure is the parse itself: the
        // parser carries out the flag it had already seen, so a machine caller
        // probing a verb that does not exist yet still reads an envelope.
        Err(failure) => {
            let style = Style::decide(failure.color, terminal.stdout_is_tty, no_color);
            return fail(failure.error, failure.json, style);
        }
    };
    let style = Style::decide(parsed.color, terminal.stdout_is_tty, no_color);

    match parsed.invocation {
        Invocation::Version => {
            write_out(&format!("armada {}\n", env!("CARGO_PKG_VERSION")));
            ExitCode::SUCCESS
        }
        Invocation::Help(topic) => {
            write_out(&render::help::render(topic, style, terminal));
            ExitCode::SUCCESS
        }
        other => {
            let json = json_wanted(&other);
            // **The colour decision, taken a second time for the second
            // stream.** Not a second decision: the same function, asked about
            // stderr. `armada manifest check | jq` is a piped stdout and a
            // terminal stderr, and it wants an unstyled payload *and* a live
            // spinner — one answer could not express that.
            let mut progress = reporter(
                json,
                Style::decide(parsed.color, terminal.stderr_is_tty, no_color),
                terminal,
            );
            match dispatch(
                other,
                &cwd,
                home.as_deref(),
                inherited,
                progress.as_mut(),
                style,
                terminal,
            ) {
                Ok(output) => emit(output, json, style, terminal),
                Err(error) => fail(error, json, style),
            }
        }
    }
}

/// Who, if anyone, is watching this run go by.
///
/// **Three conditions, and all three must hold**: stderr is a terminal, so there
/// is a person rather than a log file; colour is on, since a caller who turned
/// it off did not ask for an animation either; and the caller did not ask for
/// `--json`, which is a parser waiting for one payload and nothing else.
///
/// A spinner is drawn on **stderr and never stdout** (PLAN.md §3.1.1): stdout
/// carries the result, and a frame of animation in it is what breaks the one
/// consumer the envelope exists for.
fn reporter(
    json: bool,
    style: Style,
    terminal: render::term::Terminal,
) -> Box<dyn render::progress::Progress> {
    if json || !terminal.stderr_is_tty || !style.enabled() {
        return Box::new(render::progress::Silent);
    }
    Box::new(render::progress::Spinner::new(
        std::io::stderr(),
        style,
        terminal,
    ))
}

fn json_wanted(invocation: &Invocation) -> bool {
    match invocation {
        Invocation::Init(common) | Invocation::Status(common) => common.json,
        Invocation::Services { json, .. } => *json,
        Invocation::Clean { common, .. } => common.json,
        Invocation::Check(check) => check.json,
        Invocation::Dispatch { json, .. } => *json,
        Invocation::Config { json, .. } | Invocation::Skills { json, .. } => *json,
        Invocation::MachineInit(init) => init.json,
        Invocation::Doctor { json, .. } => *json,
        Invocation::Guild(guild) => guild.json(),
        Invocation::Version | Invocation::Help(_) => false,
    }
}

/// The verbs that describe **this machine** rather than a workspace.
///
/// They run before workspace discovery and before `app::build`, and both halves
/// of that matter. `armada init` is the first thing anyone types, on a machine
/// with no `~/.armada/` and usually from a directory with no `armada.yml`;
/// routing it through the ordinary path would make it fail on the absence of
/// exactly the things it exists to create. `armada doctor` is run *because*
/// something is wrong, including an unreadable `manifest.db` — which
/// `app::build` opens. The same reasoning `clean --force-rebuild` already
/// carries above.
fn machine_scoped(
    invocation: Invocation,
    place: &verbs::guild::Where,
    style: Style,
    terminal: render::term::Terminal,
) -> Option<Result<Output, ArmadaError>> {
    let run = RealRun;
    // **The prompt is written to stderr, exactly as the spinner is.** stdout
    // carries the finished transcript once, at the end, which is what keeps
    // `armada init --json` working without a special case.
    let mut ask: Box<dyn armada_helm::ask::Ask> = if terminal.stderr_is_tty {
        Box::new(armada_helm::ask::AtTheTerminal::new(
            std::io::stderr(),
            std::io::BufReader::new(std::io::stdin()),
            style,
        ))
    } else {
        // No terminal, no interview. Every question has a default and taking
        // all of them leaves a working guild (PLAN.md §13.4) — a prompt written
        // into a log file that nobody can answer would hang a CI run instead.
        Box::new(armada_helm::ask::Defaults)
    };

    Some(match invocation {
        Invocation::MachineInit(init) => {
            if init.defaults {
                ask = Box::new(armada_helm::ask::Defaults);
            }
            verbs::machine::run(
                &run,
                place,
                ask.as_mut(),
                &verbs::machine::Options {
                    guild: init.guild.clone(),
                    bundle: init.bundle.clone(),
                    defaults: init.defaults,
                    force: init.force,
                },
            )
        }
        Invocation::Doctor { fix, .. } => {
            if fix {
                return Some(Err(ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: "--fix".to_string(),
                    message: "`armada doctor --fix` is not built yet".to_string(),
                    next_action: Some(
                        "every finding names the command that fixes it; run that".to_string(),
                    ),
                }));
            }
            verbs::doctor::run(&run, place)
        }
        Invocation::Guild(guild) => match *guild {
            args::GuildInvocation::Init {
                from,
                no_import,
                remote,
                defaults,
                force,
                ..
            } => {
                if defaults {
                    ask = Box::new(armada_helm::ask::Defaults);
                }
                verbs::guild::init(
                    &run,
                    place,
                    ask.as_mut(),
                    &verbs::guild::InitOptions {
                        from: from.map(PathBuf::from),
                        no_import,
                        remote,
                        force,
                    },
                )
            }
            args::GuildInvocation::Pull { .. } => verbs::guild::pull(&run, place),
            args::GuildInvocation::Push { force, .. } => verbs::guild::push(&run, place, force),
            args::GuildInvocation::Export {
                out,
                include_secrets,
                ..
            } => verbs::guild::export(
                &run,
                place,
                out.as_deref().map(std::path::Path::new),
                include_secrets,
            ),
            args::GuildInvocation::Import {
                path, merge, force, ..
            } => verbs::guild::import_bundle(&run, place, &place.cwd.join(&path), merge, force),
        },
        _ => return None,
    })
}

#[allow(clippy::too_many_arguments)]
fn dispatch(
    invocation: Invocation,
    cwd: &std::path::Path,
    home: Option<&std::path::Path>,
    inherited: BTreeMap<String, String>,
    progress: &mut dyn render::progress::Progress,
    style: Style,
    terminal: render::term::Terminal,
) -> Result<Output, ArmadaError> {
    let home = home.ok_or_else(|| ArmadaError {
        class: ErrClass::Environment,
        r#where: "HOME".to_string(),
        message: "$HOME is not set, so Armada cannot find ~/.armada/".to_string(),
        next_action: Some("set HOME, then retry unchanged".to_string()),
    })?;

    let run = RealRun;

    // **This machine's own verbs run before anything of a workspace's is
    // opened.** See `machine_scoped` for why both halves of that matter.
    if matches!(
        invocation,
        Invocation::MachineInit(_) | Invocation::Doctor { .. } | Invocation::Guild(_)
    ) {
        let place = verbs::guild::Where {
            armada_home: armada_manifest::machine::armada_home(home),
            cwd: cwd.to_path_buf(),
            claude_home: home.join(".claude"),
        };
        if let Some(outcome) = machine_scoped(invocation, &place, style, terminal) {
            return outcome;
        }
        unreachable!("every machine-scoped invocation is handled");
    }

    // **The recovery path runs before anything is opened.** `armada manifest clean
    // --orphaned --force-rebuild` exists for a `manifest.db` Armada cannot read, and
    // `app::build` opens that database — so routing it through the ordinary
    // path would fail on exactly the thing it exists to repair. It also needs
    // no workspace: it is most useful from a shell that happens to be anywhere.
    if let Invocation::Clean {
        common,
        artifacts,
        orphaned,
        force,
        force_rebuild: true,
    } = &invocation
    {
        if let Some(refusal) = rebuild_refusal(*artifacts, *orphaned, *force) {
            return Err(refusal);
        }
        return verbs::clean::rebuild(&run, &SystemClock, home, common.dry_run);
    }

    // **`config scan` is the one verb that runs in a repo with no `armada.yml`
    // at all** (PLAN.md §2.1), and it is answered here for that reason: routing
    // it through workspace resolution would fail on exactly the situation it
    // exists for. It reads a directory, takes no lease and opens no database,
    // so it needs none of what `app::build` assembles.
    if let Invocation::Config {
        sub: args::ConfigSub::Scan,
        ..
    } = &invocation
    {
        return verbs::config::scan(cwd);
    }

    // Two invocations legitimately run outside any workspace: asking about
    // *this workspace* requires a `armada.yml`, asking about *the machine* does
    // not. `clean --orphaned` is most needed from a shell that happens to be
    // anywhere, and nothing else on the machine reaps orphaned ports and
    // containers — a rule that made it resolve a local workspace first would
    // fail before it could do the one job only it does.
    let machine_scoped = matches!(
        &invocation,
        Invocation::Status(common) if common.lens == armada_core::scope::Lens::All
    ) || matches!(
        &invocation,
        Invocation::Clean { common, .. } if common.lens == armada_core::scope::Lens::All
    );

    let workspace = match discovery::resolve(&run, cwd) {
        Ok(workspace) => Some(workspace),
        Err(error) if machine_scoped => {
            let _ = error;
            None
        }
        Err(error) => return Err(error),
    };

    let ctx = Ctx {
        workspace,
        run,
        now: SystemClock,
        fetch: RealFetch,
    };
    let mut app = app::build(ctx, home, inherited)?;

    match invocation {
        Invocation::Init(common) => verbs::init::run(&mut app, common.dry_run),
        Invocation::Services {
            direction,
            selector,
            dry_run,
            ..
        } => verbs::services::run(&mut app, direction, selector.as_deref(), dry_run),
        Invocation::Status(common) => verbs::status::run(&mut app, common),
        Invocation::Clean {
            common,
            artifacts,
            orphaned,
            force,
            force_rebuild,
        } => verbs::clean::run(
            &mut app,
            common,
            verbs::clean::Filters {
                artifacts,
                orphaned,
                force,
                force_rebuild,
            },
        ),
        Invocation::Check(check) => verbs::check::run(&mut app, &check, progress),
        Invocation::Dispatch { name, argv, json } => {
            verbs::dispatch::run(&mut app, &name, &argv, json)
        }
        Invocation::Config { sub, .. } => match sub {
            args::ConfigSub::Scan => unreachable!("answered before the workspace is resolved"),
            args::ConfigSub::Verify => verbs::config::verify(&mut app, progress),
        },
        Invocation::Skills { show, .. } => verbs::skills::run(&mut app, show.as_deref()),
        Invocation::Version | Invocation::Help(_) => unreachable!("handled before dispatch"),
        Invocation::MachineInit(_) | Invocation::Doctor { .. } | Invocation::Guild(_) => {
            unreachable!("machine-scoped, and handled above")
        }
    }
}

/// The invocation `--force-rebuild` insists on, or `None` if this is it.
///
/// **`armada manifest clean --orphaned --force-rebuild` is the invocation `PLAN.md` §4.3
/// spells, so it is the one that has to work.** `--all` is accepted too and
/// changes nothing: the pass is machine-scoped either way, because it
/// enumerates every labelled resource on the daemon and removes on `ENOENT`
/// across namespaces. That is worth telling a caller who typed the
/// narrower-looking form — but it is told in the *output*, in
/// [`crate::verbs::clean::rebuild`]'s namespace note and in its `--dry-run`
/// preview, rather than by refusing what the corpus documents. Whether the flag
/// should be required is a question for phase 2.5, and is recorded there
/// (`docs/PHASES.md`).
///
/// `--artifacts` and `--force` are refused rather than ignored: neither has a
/// meaning here. The rebuild reads no `armada.yml`, so there are no declared
/// `owns.files` to delete, and it takes no lease, so there is no liveness guard
/// to override.
fn rebuild_refusal(artifacts: bool, orphaned: bool, force: bool) -> Option<ArmadaError> {
    let refusal = |r#where: &str, message: String| {
        Some(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: r#where.to_string(),
            message,
            next_action: Some("`armada manifest clean --orphaned --force-rebuild`".to_string()),
        })
    };

    if !orphaned {
        return refusal(
            "--force-rebuild",
            concat!(
                "--force-rebuild rebuilds manifest.db from labels alone, and only ",
                "--orphaned bounds that to workspaces whose directory is gone",
            )
            .to_string(),
        );
    }
    for (flag, given) in [("--artifacts", artifacts), ("--force", force)] {
        if given {
            return refusal(
                flag,
                format!(
                    "{flag} has no meaning alongside --force-rebuild: that path reads no \
                     armada.yml and takes no lease"
                ),
            );
        }
    }
    None
}

fn emit(output: Output, json: bool, style: Style, terminal: render::term::Terminal) -> ExitCode {
    if json {
        write_out(&output.to_json());
    } else {
        // **The wordmark's second site** (`docs/commands/render.md`), and it is
        // drawn here rather than inside the renderer: the golden pair for
        // `armada init` is rendered at one width for both audiences, and a
        // decoration that appears in only one of them is not a *styling*
        // difference the pair property can express. Every suppression rule
        // still lives in `render::banner`, so this call site cannot draw it
        // under conditions the other one refuses.
        if matches!(output, Output::MachineInit(_)) {
            write_out(&render::banner::banner(style, terminal));
        }
        let text = render::human(&output, style, terminal);
        if output.exit_code() == 0 {
            write_out(&text);
        } else {
            write_err(&text);
        }
    }
    // **A signal has no error class, so it does not get the class's code**
    // (`ARCHITECTURE.md` §1.6). The envelope above still says `aborted`,
    // because that describes the run; the exit code describes the signal, and
    // every shell reads 130 for Ctrl-C. Written first, then exited.
    if posix::interrupted() {
        posix::die_by_signal();
    }
    ExitCode::from(output.exit_code())
}

fn fail(error: ArmadaError, json: bool, style: Style) -> ExitCode {
    let code = error.class.exit_code();
    if json {
        // The envelope shape never varies. `workspace` is `null` when
        // resolution is what failed, and a consumer must tolerate that — it
        // cannot be "always the invoking workspace" when there isn't one.
        let envelope: Envelope<NoData> = Envelope {
            schema_version: armada_core::envelope::SCHEMA_VERSION,
            verb: "armada".to_string(),
            workspace: None,
            status: Status::Failed,
            error: Some(error),
            data: NoData {},
        };
        write_out(&envelope.to_json());
    } else {
        write_err(&render::error_lines(&error, style));
    }
    ExitCode::from(code)
}

/// Write and flush. Never through a `BufWriter` that a later exit could skip.
fn write_out(text: &str) {
    let mut out = std::io::stdout();
    // A broken pipe here is the ordinary `| head` case: SIGPIPE has been
    // restored, so the process dies silently with 141 rather than reaching this
    // error at all. If it does arrive, there is nowhere left to report it.
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}

fn write_err(text: &str) {
    let mut err = std::io::stderr();
    let _ = err.write_all(text.as_bytes());
    let _ = err.flush();
}
