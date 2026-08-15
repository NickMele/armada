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
use std::os::unix::process::CommandExt;
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
            let answered = dispatch(
                other,
                &cwd,
                home.as_deref(),
                inherited,
                progress.as_mut(),
                style,
                terminal,
            );
            // **The entrypoint gives the lines back, on both paths.** A verb
            // that returned early — a worktree that would not be created, a
            // lease that never came — would otherwise leave its viewport
            // holding the bottom of the terminal while the error printed into
            // it. Doing it here rather than at each `?` is the same rule that
            // puts `$HOME` and the cwd here: the terminal is the entrypoint's,
            // and a verb should not have to remember it owns one
            // (`ARCHITECTURE.md` §1.4). `finish` is idempotent, so a verb that
            // ends its own table early still may.
            progress.finish();
            match answered {
                Ok(output) => emit(output, json, style, terminal, home.as_deref()),
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
/// The table is drawn on **stderr and never stdout** (PLAN.md §3.1.1): stdout
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
    Box::new(render::live::Watcher::new(style, terminal))
}

fn json_wanted(invocation: &Invocation) -> bool {
    match invocation {
        Invocation::Init(common) | Invocation::Status(common) => common.json,
        Invocation::Services { json, .. } => *json,
        Invocation::Clean { common, .. } => common.json,
        Invocation::Check(check) => check.json,
        Invocation::Dispatch { json, .. } => *json,
        Invocation::Config { json, .. }
        | Invocation::Skills { json, .. }
        | Invocation::Components { json } => *json,
        Invocation::MachineInit(init) => init.json,
        Invocation::Doctor { json, .. } => *json,
        Invocation::Guild(guild) => guild.json(),
        Invocation::Fleet(fleet) => fleet.json(),
        Invocation::Mcp { json } => *json,
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
        Box::new(at_the_terminal(style, terminal))
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
            args::GuildInvocation::Project { remove, .. } => verbs::guild::project(place, remove),
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

    // **The MCP server is machine-scoped, and for both of Fleet's reasons at
    // once.** Its Fleet tools act on Jobs in worktrees that are not this
    // directory, and its Manifest tools resolve a workspace per call — so
    // resolving one here, before a single request has arrived, would refuse to
    // start the server anywhere but inside a repository. It is also started by
    // whatever registered it, from wherever that happened to be.
    if let Invocation::Mcp { .. } = invocation {
        return armada_helm::mcp::serve(armada_helm::mcp::world::World {
            cwd: cwd.to_path_buf(),
            home: home.to_path_buf(),
            inherited,
            // Read once, at the entrypoint, like `$HOME` and the cwd: the Fleet
            // tools run `armada manifest init` in a worktree and have to know
            // which binary they are (`ARCHITECTURE.md` §1.4).
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            boot_id: armada_manifest::machine::boot_id(&run, cwd).ok_or_else(no_boot_id)?,
        });
    }

    // **Fleet is machine-scoped too, and for its own reason.** A Job's worktree
    // is not the directory the command was typed in, and `armada fleet ls`
    // "does not need the repository the Jobs branched from"
    // (`commands/fleet/ls.md`) — so routing it through workspace resolution
    // would refuse to list the fleet from anywhere but one of its worktrees.
    if let Invocation::Fleet(fleet) = invocation {
        let place = verbs::fleet::Where {
            home: home.to_path_buf(),
            armada_home: armada_manifest::machine::armada_home(home),
            cwd: cwd.to_path_buf(),
            // **Read once, at the entrypoint**, like `$HOME` and the cwd: Fleet
            // runs `armada manifest init` in a worktree and has to know which
            // binary it is (`ARCHITECTURE.md` §1.4).
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            // Required rather than optional, for the reason `app::build`
            // requires it: without one, every Drone handle looks stale across a
            // reboot, so Armada would either refuse to stop its own Drones
            // forever or signal a recycled pid.
            boot_id: armada_manifest::machine::boot_id(&run, cwd).ok_or_else(no_boot_id)?,
        };
        return match *fleet {
            args::FleetInvocation::Spawn(spawn) => {
                // **`None` when nobody is there to answer**, which is the same
                // rule `config scan`'s hand-over applies: both streams, decided
                // at the entrypoint and passed down rather than sniffed inside a
                // verb (`ARCHITECTURE.md` §1.4). A low-confidence spawn then
                // refuses instead of waiting on an answer that cannot arrive.
                let mut asking = terminal
                    .can_ask()
                    .then(|| at_the_terminal(style, terminal).interactive());
                verbs::fleet::spawn(
                    &run,
                    &SystemClock,
                    &place,
                    &spawn,
                    asking
                        .as_mut()
                        .map(|ask| ask as &mut dyn armada_helm::ask::Ask),
                    progress,
                )
            }
            args::FleetInvocation::Ls {
                all,
                needs_attention,
                ..
            } => verbs::fleet::ls(&run, &SystemClock, &place, all, needs_attention),
            args::FleetInvocation::Board { job, exec, .. } => {
                let output = verbs::fleet::board(&place, &job)?;
                if exec {
                    // **The process is replaced**, so the exit code becomes
                    // `claude`'s and Armada is no longer in the picture — which
                    // is the whole of "Armada does not own a terminal".
                    board_exec(&output)?;
                }
                Ok(output)
            }
            args::FleetInvocation::Kill {
                job,
                keep_branch,
                keep_worktree,
                ..
            } => verbs::fleet::kill(
                &run,
                &SystemClock,
                &place,
                job.as_deref(),
                keep_branch,
                keep_worktree,
            ),
            args::FleetInvocation::Answer { job, answer, .. } => {
                verbs::fleet::answer(&run, &SystemClock, &place, &job, &answer)
            }
            args::FleetInvocation::Inbox { job, all, .. } => {
                verbs::fleet::inbox(&SystemClock, &place, job.as_deref(), all)
            }
        };
    }

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
        json,
    } = &invocation
    {
        return verbs::config::scan(&run, cwd, Some(home), terminal, *json);
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
        Invocation::Components { .. } => verbs::components::run(&mut app),
        Invocation::Version | Invocation::Help(_) => unreachable!("handled before dispatch"),
        Invocation::MachineInit(_)
        | Invocation::Doctor { .. }
        | Invocation::Guild(_)
        | Invocation::Fleet(_)
        | Invocation::Mcp { .. } => unreachable!("machine-scoped, and handled above"),
    }
}

/// The refusal when this machine cannot say which boot it is on.
///
/// **Required rather than degraded**, for the reason `app::build` states: a boot
/// id is what tells a live process group from a recycled pid, so without one
/// Armada would either refuse to stop its own Drones forever or signal a
/// stranger's process. Refusing to run is better than either.
fn no_boot_id() -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: "boot_id".to_string(),
        message: "this machine reports no boot id".to_string(),
        next_action: Some(
            "Armada needs `sysctl kern.bootsessionuuid` on darwin or \
             /proc/sys/kernel/random/boot_id on Linux"
                .to_string(),
        ),
    }
}

/// `armada fleet board --exec`: change directory and become `claude`.
///
/// **Replacing the process rather than spawning one is the point.** Armada owns
/// no terminal (`commands/fleet/board.md`); with `--exec` it hands its own
/// process over, so the exit code, the signals and the tty all belong to the
/// session from that moment and Armada is not sitting in the middle of a
/// conversation it has no part in.
///
/// It returns only if the exec **failed** — a successful one never comes back.
fn board_exec(output: &Output) -> Result<(), ArmadaError> {
    use std::os::unix::process::CommandExt;

    let Output::Board(envelope) = output else {
        return Ok(());
    };
    let data = &envelope.data;
    let mut argv = data.command.split(' ');
    let program = argv.next().unwrap_or("claude");
    let error = std::process::Command::new(program)
        .args(argv)
        .current_dir(&data.worktree)
        .exec();

    Err(ArmadaError {
        class: ErrClass::Environment,
        r#where: data.command.clone(),
        message: format!("could not board: {error}"),
        next_action: Some(format!("cd {} && {}", data.worktree, data.command)),
    })
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

/// The one thing in the process that asks a person a question.
///
/// **Widgets only when both stdin and stdout are a terminal**, which is the same
/// rule `armada_core::scan::handover` applies and is here for the same reason:
/// stdin decides whether an answer can arrive and stdout decides whether the
/// question was seen. Computed at the entrypoint and passed down as a value, so
/// no widget sniffs the ambient world for itself (`ARCHITECTURE.md` §1.4).
fn at_the_terminal(
    style: Style,
    terminal: render::term::Terminal,
) -> armada_helm::ask::AtTheTerminal<std::io::Stderr, std::io::BufReader<std::io::Stdin>> {
    let at = armada_helm::ask::AtTheTerminal::new(
        std::io::stderr(),
        std::io::BufReader::new(std::io::stdin()),
        style,
        terminal.width,
    );
    if terminal.can_ask() {
        at.interactive()
    } else {
        at
    }
}

/// Put `config scan`'s choice, and act on it.
///
/// **Only ever reached for [`Handover::Ask`]**, which the core decides and
/// which is only ever produced when both stdin and stdout are a terminal. That
/// is the guarantee that matters: an agent running `config scan` inside a Job
/// must never block on stdin that will never arrive, and the way it never does
/// is that no code path here is reachable without a person on the other end.
///
/// **The same selector the interview uses**, drawn below the report rather than
/// printed into it. That is the whole of what changed: this used to be a list of
/// numbers in the middle of the output followed by a silent `read_line`, and the
/// person it was put to could not tell it was waiting for him.
///
/// **Option 1 execs rather than spawning.** Armada has produced the evidence
/// and has nothing further to do, so it gets out of the way entirely — the same
/// shape `armada fleet board --exec` takes handing over a session.
fn hand_over(
    output: &Output,
    style: Style,
    terminal: render::term::Terminal,
    home: Option<&std::path::Path>,
) {
    let Output::Scan(envelope) = output else {
        return;
    };
    if envelope.data.handover != armada_core::scan::Handover::Ask {
        return;
    }

    let chosen = armada_helm::ask::Ask::choose(
        &mut at_the_terminal(style, terminal),
        verbs::config::ONBOARD_QUESTION,
        &verbs::config::onboarding_choices(),
        verbs::config::STOP,
    );
    if chosen != verbs::config::HAND_OVER {
        return;
    }

    // **The skill's prose, not its name.** `Handover::Ask` is only produced when
    // the guild has the file, so a failure to read it here is a race or a
    // permission problem rather than the ordinary absence — reported either way,
    // because handing over without the instructions is the failure this exists
    // to remove.
    let skill = home.map(|home| {
        armada_guild::layout::Guild::at(&armada_manifest::machine::armada_home(home))
            .skill(armada_guild::layout::ONBOARD_REPO)
    });
    let body = match skill.as_ref().map(std::fs::read_to_string) {
        Some(Ok(body)) => body,
        other => {
            let why = match other {
                Some(Err(error)) => error.to_string(),
                _ => "$HOME is not set, so Armada cannot find the guild".to_string(),
            };
            write_err(&render::error_lines(
                &ArmadaError {
                    class: ErrClass::Environment,
                    r#where: skill
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| armada_guild::layout::ONBOARD_REPO.to_string()),
                    message: format!("cannot read the onboarding skill: {why}"),
                    next_action: Some("`armada guild init` writes it".to_string()),
                },
                Style::plain(),
            ));
            return;
        }
    };

    let argv = armada_guild::layout::skill_argv(&body);
    let error = std::process::Command::new(&argv[0]).args(&argv[1..]).exec();
    // `exec` returns only on failure. Reported rather than swallowed: the
    // reader asked for a session and is owed the reason there is not one.
    write_err(&render::error_lines(
        &ArmadaError {
            class: ErrClass::Environment,
            // The argv carries the whole skill, so the *command* is what is
            // named here — an error report is not the place to repeat a
            // kilobyte of prose the reader just asked to be given.
            r#where: armada_guild::layout::skill_command_line(armada_guild::layout::ONBOARD_REPO),
            message: format!("could not start the onboarding session: {error}"),
            next_action: Some("install claude, or put it on PATH".to_string()),
        },
        Style::plain(),
    ));
}

fn emit(
    output: Output,
    json: bool,
    style: Style,
    terminal: render::term::Terminal,
    home: Option<&std::path::Path>,
) -> ExitCode {
    // **`mcp serve` reports on stderr, and it is the one verb that must.**
    // stdout *is* the transport: it carried JSON-RPC frames until the moment
    // this line runs, and a summary written into it is a frame the client
    // cannot parse. Measured against a real client — the line arrived after the
    // last response, as one unparseable trailing document.
    //
    // This is the same rule the spinner and the interview prompt already
    // follow, arriving at the one verb where stdout does not belong to a person
    // at all (PLAN.md §3.1.1).
    if matches!(output, Output::Mcp(_)) {
        let text = match json {
            true => output.to_json(),
            false => render::human(&output, style, terminal),
        };
        write_err(&text);
        return ExitCode::from(output.exit_code());
    }
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
        // **After the evidence is written and flushed, never before.** The
        // question is about what the reader has just seen, and a prompt that
        // arrives first is a prompt answered blind.
        hand_over(&output, style, terminal, home);
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
