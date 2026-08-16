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
use armada_core::ctx::{Clock, Ctx};
use armada_core::envelope::{Envelope, NoData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::failure::Fault;
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

    // **`$VISUAL` then `$EDITOR`, read once here and passed down as a value** —
    // the same rule that puts `$HOME` and the terminal at the entrypoint
    // (`ARCHITECTURE.md` §1.4). Only `config scan`'s hand-over reads it, for
    // the file it just wrote (`docs/reserved/009-smaller-things-raised-in-use.md`
    // item 2).
    let editor = inherited
        .get("VISUAL")
        .or_else(|| inherited.get("EDITOR"))
        .filter(|value| !value.is_empty())
        .cloned();

    // **The terminal is ambient state, so it is read here and passed down**
    // (`ARCHITECTURE.md` §1.4) — the same rule that puts cwd and `$HOME` on the
    // three lines above. `NO_COLOR` comes out of the environment map this
    // function already built, rather than from a second `std::env` call in the
    // renderer: one read, one answer, and a test can set it.
    let terminal = render::term::Terminal::detect();
    let no_color = inherited.contains_key("NO_COLOR");

    // **Everything the recorder needs, read once at the entrypoint** — the same
    // rule the cwd, `$HOME` and the terminal follow (`ARCHITECTURE.md` §1.4).
    // Nothing below reaches for any of it, and a failure that happens before a
    // verb exists is still recorded because these three are already in hand.
    let ambient = Ambient {
        home: home.as_deref(),
        cwd: &cwd,
        argv: &argv,
    };

    let parsed = match args::parse(&argv) {
        Ok(parsed) => parsed,
        // `--json` is answered even when the failure is the parse itself: the
        // parser carries out the flag it had already seen, so a machine caller
        // probing a verb that does not exist yet still reads an envelope.
        Err(failure) => {
            let style = Style::decide(failure.color, terminal.stdout_is_tty, no_color);
            // **The parser says whose mistake it was**, and it is the only
            // thing that can: a reserved flag refusing by name and a real
            // failure are the same class and the same shape by the time they
            // reach here (`armada_core::failure::Fault`).
            return fail(failure.error, failure.json, style, &ambient, failure.fault);
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
            // **Computed before `dispatch` moves `other`, and read before
            // `dispatch` moves `inherited`.** Both feed the occasional
            // pull-offer (`docs/reserved/009` item 4) after `emit` writes this
            // verb's own envelope: `offerable` excludes the guild's own verbs
            // — asking "pull now?" right after `armada guild pull` just ran
            // would be noise about the thing the caller was already doing —
            // and `in_job` is the one guard `config scan`'s handover did not
            // need, because a Job's Drone has nobody at the other end of
            // stdin at all.
            let offerable = !matches!(other, Invocation::Guild(_));
            let in_job = inherited.contains_key("ARMADA_JOB");
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
                &argv,
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
                Ok(output) => {
                    // **The buffer is written before the answer is, not after.**
                    // `emit` can end the process outright — `posix::die_by_signal`
                    // on an interrupt, and `board --exec` replaces the image
                    // entirely — so a recorder placed after it would silently skip
                    // exactly the runs somebody is most likely to want to report.
                    remember(
                        &argv,
                        &cwd,
                        home.as_deref(),
                        output.exit_code(),
                        &output.to_json(),
                    );
                    emit(
                        output,
                        json,
                        style,
                        terminal,
                        home.as_deref(),
                        &cwd,
                        editor.as_deref(),
                        offerable,
                        in_job,
                    )
                }
                // **A verb that could not answer is Armada's**, always. The
                // refusals a verb authors — `armada failures show <a typo>` —
                // stay recorded on purpose: a prefix that resolved to nothing
                // could equally be Armada mis-resolving one, which is a real
                // bug shape (`docs/reserved/010`, *Known rough edge*).
                Err(error) => fail(error, json, style, &ambient, Fault::Armadas),
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
        | Invocation::Components { json }
        | Invocation::Commands { json } => *json,
        Invocation::MachineInit(init) => init.json,
        Invocation::Doctor { json, .. } => *json,
        Invocation::Settings { json } => *json,
        Invocation::Bridge(bridge) => bridge.json,
        Invocation::Helm(helm) => helm.json,
        Invocation::HelmEnable { json } | Invocation::HelmDisable { json } => *json,
        Invocation::Guild(guild) => guild.json(),
        Invocation::Fleet(fleet) => fleet.json(),
        Invocation::Failures(failures) => failures.json(),
        Invocation::Tasks(tasks) => tasks.json(),
        Invocation::Untried { json, .. } => *json,
        Invocation::Report { json, .. } | Invocation::Task { json, .. } => *json,
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
        Invocation::Settings { .. } => verbs::settings::run(place),
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
            // Whether a person is here is the entrypoint's to know, exactly as
            // it is for `ls`: at a terminal the offered file is a question, and
            // without one it is `--with-skills`.
            args::GuildInvocation::Upgrade { with_skills, .. } => {
                verbs::guild::upgrade(&run, place, ask.as_mut(), terminal.can_ask(), with_skills)
            }
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
            // **Whether a person is here is decided at the entrypoint**, exactly
            // as it is for the interview's widgets, and passed down as a value.
            // A terminal is what turns the listing into a navigable one; there
            // is no flag for that. `--list` only goes the other way, which is
            // what makes "both forms carry the same facts" something a test can
            // check from a terminal.
            args::GuildInvocation::Ls { list, .. } => verbs::guild::ls(
                &run,
                place,
                ask.as_mut(),
                terminal.can_ask() && !list,
                look(style, terminal),
            ),
            args::GuildInvocation::Show { item, .. } => verbs::guild::show(place, &item),
            args::GuildInvocation::Edit { item, from, .. } => verbs::guild::edit(
                &run,
                place,
                ask.as_mut(),
                &item,
                from.as_deref().map(std::path::Path::new),
            ),
            args::GuildInvocation::Delete { item, yes, .. } => {
                verbs::guild::delete(&run, place, ask.as_mut(), &item, yes, terminal.can_ask())
            }
        },
        _ => return None,
    })
}

#[allow(clippy::too_many_arguments)]
fn dispatch(
    invocation: Invocation,
    argv: &[String],
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

    // **Helm is machine-scoped for Fleet's reason and one of its own.** Its
    // whole subject is the fleet across every repository, and routing it through
    // workspace resolution would refuse to start the orchestrator from any
    // directory that is not a workspace — which is most directories, including
    // the one a person opens a terminal in.
    if let Invocation::Helm(options) = &invocation {
        let place = verbs::helm::Where {
            home: home.to_path_buf(),
            armada_home: armada_manifest::machine::armada_home(home),
            claude_home: home.join(".claude"),
            // Read once, at the entrypoint, like `$HOME` and the cwd: the
            // toolbelt's registration names *this* binary, and a bare `armada`
            // there resolves against the session's own `PATH`
            // (`ARCHITECTURE.md` §1.4).
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            boot_id: armada_manifest::machine::boot_id(&run, cwd).ok_or_else(no_boot_id)?,
        };
        return helm(
            &place,
            options,
            &verbs::helm::Options {
                agent: options.agent.clone(),
                new: options.new,
            },
        );
    }

    // **`enable`/`disable` are machine-scoped for the same reason `helm`
    // itself is, and need less of `Where` than it does.** Both write one
    // boolean to `~/.armada/machine.yml` and read nothing about the guild, the
    // persona or the fleet — so unlike the block above, neither needs a boot
    // id, and a machine that cannot answer `sysctl kern.bootsessionuuid`
    // (`no_boot_id`) must still be able to flip this switch.
    if let Invocation::HelmEnable { .. } = &invocation {
        return verbs::helm::enable(&armada_manifest::machine::armada_home(home));
    }
    if let Invocation::HelmDisable { .. } = &invocation {
        return verbs::helm::disable(&armada_manifest::machine::armada_home(home));
    }

    // **Fleet is machine-scoped too, and for its own reason.** A Job's worktree
    // is not the directory the command was typed in, and `armada fleet ls`
    // "does not need the repository the Jobs branched from"
    // (`commands/fleet/ls.md`) — so routing it through workspace resolution
    // would refuse to list the fleet from anywhere but one of its worktrees.
    // **The Bridge is machine-scoped for the same reason and by the same
    // route.** It is a renderer over `fleet ls`, so it needs exactly what `ls`
    // needs and nothing a workspace would add.
    // **`armada failures` joins Fleet and the Bridge here, and for both of their
    // reasons.** The failures it lists happened in whatever directory they
    // happened in — including directories that are not workspaces, which is where
    // the failure that prompted the feature happened — so resolving a workspace
    // first would refuse to show a record of a failure that was *caused* by not
    // having one. And promoting an entry is `fleet spawn`, which needs exactly
    // what `spawn` needs and nothing a workspace would add.
    // **`armada report` runs before every other verb's preconditions, including
    // Fleet's.** It writes into the failures store so it wants Fleet's `Where`,
    // but it must not inherit Fleet's requirements: a person filing a report
    // about a machine that is broken cannot be refused *because the machine is
    // broken*. Two of those refusals are real —
    //
    // - **no workspace here.** Half of what is worth reporting happens outside
    //   one, including the failure that prompted the failure log.
    // - **no boot id.** `armada fleet ls` refuses without one, and *"Armada
    //   will not run on this machine"* is the single most valuable report there
    //   is. So it is optional here and the Jobs diagnostic is what is skipped.
    //
    // Everything else it gathers degrades the same way, in `verbs::report`.
    // **`armada task` runs here too, and for one of `report`'s two reasons.**
    // Capture writes into the failures store, so it wants Fleet's `Where` — and
    // it must not inherit Fleet's requirements, because a thought you had in a
    // directory that is not a workspace is still a thought worth keeping. The
    // boot id is not one of its preconditions either: nothing about writing a
    // sentence down depends on a Drone handle being meaningful.
    let capture = matches!(invocation, Invocation::Task { .. });
    if let Invocation::Report { what, .. } | Invocation::Task { what, .. } = invocation {
        let place = verbs::fleet::Where {
            home: home.to_path_buf(),
            armada_home: armada_manifest::machine::armada_home(home),
            cwd: cwd.to_path_buf(),
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            boot_id: armada_manifest::machine::boot_id(&run, cwd).unwrap_or_default(),
        };
        return match capture {
            true => verbs::tasks::capture(&run, &SystemClock, &place, &what, argv),
            false => verbs::report::run(&run, &SystemClock, &place, &what, argv),
        };
    }

    // **`armada untried` has fewer preconditions than any other verb**, and it
    // has to: it is the verb you reach for when you do not know what works yet.
    // No workspace, no boot id, no guild — it reads one file and diffs it
    // against Armada's own roster.
    if let Invocation::Untried { all, json } = invocation {
        let place = verbs::fleet::Where {
            home: home.to_path_buf(),
            armada_home: armada_manifest::machine::armada_home(home),
            cwd: cwd.to_path_buf(),
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            boot_id: String::new(),
        };
        // A terminal is the flag, decided here rather than in the verb
        // (`ARCHITECTURE.md` §1.4).
        let interactive = terminal.can_ask() && !json;
        let mut asking = at_the_terminal(style, terminal);
        return verbs::untried::ls(
            &SystemClock,
            &place,
            all,
            &mut asking,
            interactive,
            look(style, terminal),
        );
    }

    // **`armada tasks` joins them for `armada failures`' reasons exactly.** It
    // is the same store seen through the other lens, and starting a task is
    // `fleet spawn`, which needs what `spawn` needs and nothing a workspace
    // would add.
    if matches!(
        invocation,
        Invocation::Fleet(_)
            | Invocation::Bridge(_)
            | Invocation::Failures(_)
            | Invocation::Tasks(_)
    ) {
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
        let fleet = match invocation {
            Invocation::Bridge(options) => {
                return bridge(&run, &place, &options, progress, style, terminal)
            }
            Invocation::Failures(failures) => {
                return match *failures {
                    // **A terminal is the flag.** At one, the listing is
                    // navigable; through a pipe, and under `--json` even at a
                    // terminal, it is the same listing printed once. Decided
                    // here rather than in the verb, like every other terminal
                    // question (`ARCHITECTURE.md` §1.4) — which is what lets
                    // the suite drive the navigating against a `TempDir`.
                    args::FailuresInvocation::Ls { all, json } => {
                        let interactive = terminal.can_ask() && !json;
                        let mut asking = at_the_terminal(style, terminal);
                        verbs::failures::ls(
                            &run,
                            &SystemClock,
                            &place,
                            all,
                            &mut asking,
                            interactive,
                            look(style, terminal),
                            progress,
                            verbs::failures::Lens::Failures,
                            None,
                        )
                    }
                    args::FailuresInvocation::Show { id, .. } => {
                        verbs::failures::show(&SystemClock, &place, &id)
                    }
                    args::FailuresInvocation::Clear { id, all, .. } => verbs::failures::clear(
                        &SystemClock,
                        &place,
                        id.as_deref(),
                        all,
                        verbs::failures::Lens::Failures,
                    ),
                    args::FailuresInvocation::Fix { id, dry_run, .. } => {
                        // The same rule `fleet spawn` follows: `None` when
                        // nobody is there to answer. Promotion names the
                        // workflow, so nothing here can reach a question — but
                        // the argument is the verb's, not this call site's.
                        let mut asking = terminal
                            .can_ask()
                            .then(|| at_the_terminal(style, terminal).interactive());
                        verbs::failures::fix(
                            &run,
                            &SystemClock,
                            &place,
                            &id,
                            dry_run,
                            None,
                            asking
                                .as_mut()
                                .map(|ask| ask as &mut dyn armada_helm::ask::Ask),
                            progress,
                        )
                    }
                };
            }
            Invocation::Tasks(tasks) => {
                return match *tasks {
                    // **A terminal is the flag**, exactly as it is for
                    // `armada failures` — decided here and never sniffed in the
                    // verb (`ARCHITECTURE.md` §1.4).
                    args::TasksInvocation::Ls { all, json } => {
                        let interactive = terminal.can_ask() && !json;
                        let mut asking = at_the_terminal(style, terminal);
                        verbs::tasks::ls(
                            &run,
                            &SystemClock,
                            &place,
                            all,
                            &mut asking,
                            interactive,
                            look(style, terminal),
                            progress,
                        )
                    }
                    args::TasksInvocation::Show { id, .. } => {
                        verbs::tasks::show(&SystemClock, &place, &id)
                    }
                    args::TasksInvocation::Clear { id, all, .. } => {
                        verbs::tasks::clear(&SystemClock, &place, id.as_deref(), all)
                    }
                    args::TasksInvocation::Start {
                        id,
                        dry_run,
                        workflow,
                        ..
                    } => {
                        // **`None` when nobody is there to answer**, and here it
                        // can genuinely be reached: a task with no `--workflow`
                        // is classified, and a guess below the threshold asks.
                        // Through a pipe that refuses by naming `--workflow`,
                        // which is the honest answer — a Job started on a coin
                        // flip costs a worktree and a budget to discover.
                        let mut asking = terminal
                            .can_ask()
                            .then(|| at_the_terminal(style, terminal).interactive());
                        verbs::tasks::start(
                            &run,
                            &SystemClock,
                            &place,
                            &id,
                            dry_run,
                            workflow.as_deref(),
                            asking
                                .as_mut()
                                .map(|ask| ask as &mut dyn armada_helm::ask::Ask),
                            progress,
                        )
                    }
                };
            }
            Invocation::Fleet(fleet) => fleet,
            _ => unreachable!("every arm is matched above"),
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
            args::FleetInvocation::Show { job, .. } => {
                verbs::fleet::show(&run, &SystemClock, &place, &job)
            }
            args::FleetInvocation::Board { job, exec, .. } => {
                let output = verbs::fleet::board(&place, &job)?;
                if exec {
                    // **The process is replaced**, so the exit code becomes
                    // `claude`'s and Armada is no longer in the picture — which
                    // is the whole of "Armada does not own a terminal".
                    board_exec(&place, &output)?;
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
            args::FleetInvocation::Pause { job, .. } => {
                verbs::fleet::pause(&run, &SystemClock, &place, &job)
            }
            args::FleetInvocation::Resume { job, .. } => {
                verbs::fleet::resume(&run, &SystemClock, &place, &job)
            }
            args::FleetInvocation::Tick { job, watch, .. } => {
                verbs::fleet::tick(&run, &SystemClock, &place, job.as_deref(), watch)
            }
            args::FleetInvocation::Reap {
                jobs, dry_run, yes, ..
            } => reap(&run, &place, &jobs, dry_run, yes, style, terminal),
            args::FleetInvocation::Inbox { job, all, .. } => {
                verbs::fleet::inbox(&SystemClock, &place, job.as_deref(), all)
            }
        };
    }

    // **This machine's own verbs run before anything of a workspace's is
    // opened.** See `machine_scoped` for why both halves of that matter.
    if matches!(
        invocation,
        Invocation::MachineInit(_)
            | Invocation::Doctor { .. }
            | Invocation::Settings { .. }
            | Invocation::Guild(_)
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
    app.handoff = detach_handoff(&app.inherited, app.ctx.workspace.as_ref());

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
        Invocation::Commands { .. } => verbs::commands::run(&mut app),
        Invocation::Version | Invocation::Help(_) => unreachable!("handled before dispatch"),
        Invocation::MachineInit(_)
        | Invocation::Doctor { .. }
        | Invocation::Settings { .. }
        | Invocation::Guild(_)
        | Invocation::Fleet(_)
        | Invocation::Mcp { .. } => unreachable!("machine-scoped, and handled above"),
        Invocation::Bridge(_)
        | Invocation::Helm(_)
        | Invocation::HelmEnable { .. }
        | Invocation::HelmDisable { .. }
        | Invocation::Failures(_)
        | Invocation::Tasks(_)
        | Invocation::Untried { .. }
        | Invocation::Report { .. }
        | Invocation::Task { .. } => {
            unreachable!("machine-scoped, and handled above")
        }
    }
}

/// `armada helm` — assemble the launch, and enter it only if this machine has
/// said yes.
///
/// **Whether `--exec` runs at all is decided before the verb runs, not
/// after.** A refusal that had already written the configuration files would
/// have changed the machine in order to say no — the same rule `armada doctor
/// --fix` follows, and the rule the no-guild refusal inside the verb follows
/// one level down. So [`verbs::helm::entering_allowed`] is read first, and
/// [`verbs::helm::entering_is_off`] is returned before [`verbs::helm::run`]
/// ever touches disk.
///
/// **Off it stops there; on, it becomes [`helm_exec`].** The switch decides
/// whether the process is ever replaced — it does not change what
/// `armada helm` writes or reports either way, which is what keeps `--json`
/// honest about a launch that was merely assembled.
fn helm(
    place: &verbs::helm::Where,
    options: &args::Helm,
    wanted: &verbs::helm::Options,
) -> Result<Output, ArmadaError> {
    if options.exec && !verbs::helm::entering_allowed(&place.armada_home) {
        return Err(verbs::helm::entering_is_off());
    }
    let output = verbs::helm::run(&SystemClock, place, wanted)?;
    if options.exec {
        // **The process is replaced.** This returns only if the exec itself
        // failed — a successful one never comes back.
        helm_exec(place, &output)?;
    }
    Ok(output)
}

/// `armada helm --exec`, once the switch says yes: record that the
/// conversation has started, then become `claude`.
///
/// **`mark_started` runs before the exec, never after — there is no after.**
/// A process that has just been replaced cannot come back to write a file, so
/// the record has to already be true the moment it might become permanent.
///
/// It returns only if the exec **failed**, exactly as `armada fleet board
/// --exec`'s [`board_exec`] does not return on success — the same shape,
/// arriving at the other process this binary can become.
fn helm_exec(place: &verbs::helm::Where, output: &Output) -> Result<(), ArmadaError> {
    use std::os::unix::process::CommandExt;

    let Output::Helm(envelope) = output else {
        return Ok(());
    };
    let data = &envelope.data;
    verbs::helm::mark_started(
        place,
        &armada_core::helm::Session {
            uuid: data.uuid.clone(),
            agent: data.agent.clone(),
            // Overwritten to `true` by `mark_started` regardless of what is
            // passed here; kept `false` because that is what a launch this
            // verb just assembled actually knows.
            started: false,
        },
    )?;

    let error = std::process::Command::new(&data.argv[0])
        .args(&data.argv[1..])
        .exec();

    // **The printed line, not the argv.** The argv carries the reader's own
    // `voice.md` inline; an error that pasted it would answer a failed exec with
    // several kilobytes of their prose, and the one thing they need — the
    // command to run themselves — would be somewhere in the middle of it.
    Err(ArmadaError {
        class: ErrClass::Environment,
        r#where: data.command.clone(),
        message: format!("could not enter Helm: {error}"),
        next_action: Some(format!("run it yourself: {}", data.command)),
    })
}

/// `armada fleet reap` — the preview, the answer to it, and then the reap.
///
/// **The preview is mandatory and it is the feature** (`commands/fleet/reap.md`).
/// A bulk delete that only listed names would be asking a person to approve a
/// decision on less information than the machine already has; what makes the
/// answer possible is what each Job is *holding* — a port block, a worktree —
/// because that is the cost of not reaping, and it is invisible everywhere else.
///
/// **Three surfaces and one rule: nothing is reaped without an answer.**
///
/// | Given | What happens |
/// |---|---|
/// | `--dry-run` | the plan, and nothing else, at every surface |
/// | `--yes` | the plan is not shown and the reap happens — what a pipe passes |
/// | a terminal | the plan is printed and the question is put |
/// | neither, no terminal | the plan, and `bad_invocation` naming `--yes` |
///
/// The last row is the one worth stating: a destructive bulk action with nobody
/// there to confirm must refuse rather than proceed, because "nobody said no" is
/// not consent. `--json` changes only who reads the answer, exactly as it does
/// everywhere else — a `--json` reap without `--yes` emits the plan.
fn reap(
    run: &RealRun,
    place: &verbs::fleet::Where,
    jobs: &[String],
    dry_run: bool,
    yes: bool,
    style: Style,
    terminal: render::term::Terminal,
) -> Result<Output, ArmadaError> {
    let plan = verbs::fleet::reap_plan(run, &SystemClock, place)?;
    let Output::ReapPlan(envelope) = &plan else {
        unreachable!("reap_plan answers with a plan");
    };

    if dry_run {
        return Ok(plan);
    }

    // **Named beats inferred.** `--job` is what the Bridge's preview dispatches
    // once a person has ticked the rows they meant, and the plan's own default
    // set is what a bare `armada fleet reap` takes.
    let targets: Vec<String> = match jobs.is_empty() {
        false => jobs.to_vec(),
        true => envelope
            .data
            .results
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.uuid.clone())
            .collect(),
    };
    if targets.is_empty() {
        // Nothing to do is the plan, which already says so — and `SKIPPED` is
        // what "there was nothing to do" is called.
        return Ok(plan);
    }

    if !yes {
        if !terminal.can_ask() {
            write_err(&render::human(&plan, style, terminal));
            return Err(ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "fleet reap".to_string(),
                message: format!(
                    "{} Jobs would be reaped and there is nobody here to confirm it",
                    targets.len()
                ),
                next_action: Some(
                    "`armada fleet reap --dry-run` shows the plan; `--yes` reaps it".to_string(),
                ),
            });
        }
        write_err(&render::human(&plan, style, terminal));
        use armada_helm::ask::{Ask, Choice};
        let taken = at_the_terminal(style, terminal).interactive().choose(
            &format!("Reap {} Jobs?", targets.len()),
            &[
                Choice::new("keep them", "nothing is deleted"),
                Choice::new("reap", "ends them and releases what they hold"),
            ],
            // **The default is the one that changes nothing.** A confirmation
            // whose default is destructive is a confirmation answered by a
            // stray `enter`.
            1,
        );
        if taken != 2 {
            return Ok(plan);
        }
    }

    verbs::fleet::reap(run, &SystemClock, place, &targets)
}

/// `armada bridge` — one frame, or the screen.
///
/// **Every key that leaves calls a verb this file already dispatches**, with the
/// identical arguments a shell would give it. That is what keeps the Bridge a
/// rendering choice rather than an architectural one: there is no code path
/// below here that a person at a prompt could not reach by typing the verb
/// (`commands/helm/bridge.md`).
///
/// **Spawning comes back, which is why this is a loop.** Two of the four
/// departures end the session — `q` and a successful board — and one of them
/// never did anything worth ending it for: having created a Job, the screen you
/// want is the one that watches Jobs, with the new one on it. So `n` gives the
/// terminal back for exactly as long as `armada fleet spawn` needs it — a live
/// progress table and, when the guess is not confident, the workflow question,
/// both of which want the terminal Armada draws every other prompt in
/// (PLAN.md §3.1.1) — and then takes the screen again.
fn bridge(
    run: &RealRun,
    place: &verbs::fleet::Where,
    options: &args::Bridge,
    progress: &mut dyn render::progress::Progress,
    style: Style,
    terminal: render::term::Terminal,
) -> Result<Output, ArmadaError> {
    // **Parsed before the screen is taken.** An unparseable `--filter` is exit 2
    // and must not first blank somebody's terminal to say so.
    let filter = armada_core::fleet::bridge::parse_filter(options.filter.as_deref().unwrap_or(""))?;

    if options.once {
        return verbs::bridge::once(run, &SystemClock, place, filter.as_ref());
    }
    // **Both streams, the same rule every widget follows.** stdout decides
    // whether the screen was seen and stdin decides whether a key can arrive; a
    // Bridge drawn into a pipe is a screen nobody reads that never ends.
    if !terminal.can_ask() {
        return Err(verbs::bridge::no_screen());
    }

    let watching = verbs::bridge::Options {
        filter: options.filter.clone(),
        interval_s: options.interval_s,
        once: options.once,
        json: options.json,
    };
    // **Held across every re-entry**, so a spawn does not cost the cursor
    // position and the filter the reader set before pressing `n`. A screen that
    // came back to row one under no filter is a screen that came back as a
    // different screen.
    let mut screen = armada_core::fleet::bridge::Screen {
        filter: filter.clone(),
        ..Default::default()
    };

    use armada_core::fleet::bridge::Departure;
    loop {
        let (frame, departure) = armada_helm::bridge::watch(
            run,
            &SystemClock,
            place,
            &watching,
            &mut screen,
            style,
            terminal,
            &mut |action| on_screen(run, place, action),
        )?;

        match departure {
            // **Quitting leaves the last frame in the scrollback.** The screen
            // is gone and what it was showing is not, which is the difference
            // between closing a view and losing what you were looking at.
            Departure::Quit => return Ok(verbs::bridge::envelope(frame)),

            // `armada fleet board <job> --exec`, exactly.
            //
            // **Boarding is the one action that has to end the screen**,
            // because it *replaces this process*: from the `exec` on, the tty,
            // the signals and the exit code all belong to `claude`. Failing to
            // board does not end it — the failure comes back as a notice, and
            // the reader is still looking at the fleet.
            Departure::Board(target) => {
                let output = verbs::fleet::board(place, &target.uuid)?;
                board_exec(place, &output)?;
                return Ok(output);
            }

            // `armada fleet spawn "<task>"` — the verb, with the task the
            // compose box already holds, and then back to the Bridge.
            //
            // **Not silent, which was the third defect.** Classification is one
            // call to a model and it is the whole of the wait — 8.8 seconds in
            // the run that reported this — and the Bridge used to spend it
            // showing nothing at all, because the reporter it passed was
            // `Silent`. The terminal is its own again by the time this runs, so
            // it gets the same live table `armada fleet spawn` draws in a shell:
            // one table, on stderr, for every audience (PLAN.md §3.1.1).
            Departure::Spawn(task) => {
                screen.notice = Some(spawned(
                    verbs::fleet::spawn(
                        run,
                        &SystemClock,
                        place,
                        &args::Spawn {
                            task,
                            ..args::Spawn::default()
                        },
                        Some(&mut at_the_terminal(style, terminal).interactive()),
                        progress,
                    ),
                    style,
                    terminal,
                ));
                // The table and the question are done with the terminal; give
                // the progress viewport back before the alt screen takes it.
                progress.finish();
            }

            // `armada fleet answer <job> "<answer>"`.
            Departure::Answer(target) => {
                let Some(said) = ask_text(
                    style,
                    terminal,
                    &format!("Your answer to `{}`:", target.job),
                ) else {
                    return Ok(verbs::bridge::envelope(frame));
                };
                return verbs::fleet::answer(run, &SystemClock, place, &target.uuid, &said);
            }
        }
    }
}

/// What a spawn started from the Bridge leaves behind: a report in the
/// scrollback and one line to carry back onto the screen.
///
/// **The report goes to stderr and the notice goes to the frame.** stdout is
/// still the Bridge's — it carries the last frame, once, when the reader
/// leaves — so a second envelope on it would be two payloads for one
/// invocation. Everything a spawn has to say is already the interactive half of
/// the conversation, which is the stream the progress table and every prompt
/// already use.
///
/// **A spawn that failed is a notice, not the end of the screen** — the rule
/// every on-screen action already follows.
fn spawned(
    outcome: Result<Output, ArmadaError>,
    style: Style,
    terminal: render::term::Terminal,
) -> String {
    match outcome {
        Ok(output) => {
            let said = match &output {
                Output::Spawn(envelope) => format!(
                    "`{}` spawned — {} · {}",
                    envelope.data.name,
                    envelope.data.workflow,
                    envelope.data.state.word()
                ),
                _ => "spawned".to_string(),
            };
            write_err(&render::human(&output, style, terminal));
            said
        }
        Err(error) => {
            let said = match &error.next_action {
                Some(next) => format!("{} — {next}", error.message),
                None => error.message.clone(),
            };
            write_err(&render::error_lines(&error, style));
            said
        }
    }
}

/// Carry out one on-screen action, and say what to put under the table.
///
/// **Every failure here is a line and not the end of the screen.** That is the
/// class fix (`commands/helm/bridge.md`): an action that tore the Bridge down to
/// report a missing worktree took away the view of four other Jobs to say
/// something about a fifth, and said it into a shell nobody was looking at any
/// more. Only a terminal that has actually gone ends the Bridge.
///
/// **Each of these is still the verb a person could type**, with the arguments a
/// shell would give it — which is the rule that keeps the Bridge a rendering
/// choice rather than an architectural one. What changed is where the answer is
/// printed.
fn on_screen(
    run: &RealRun,
    place: &verbs::fleet::Where,
    action: &armada_core::fleet::bridge::Action,
) -> armada_core::fleet::bridge::Done {
    use armada_core::fleet::bridge::{Action, Done, Mode, Reap, ReapRow};

    /// One failure, as the line the screen shows. **The class and the locator
    /// are dropped and the sentence is not**: a notice is one line wide, and the
    /// sentence is the half that says what went wrong.
    fn said(error: &ArmadaError) -> String {
        match &error.next_action {
            Some(next) => format!("{} — {next}", error.message),
            None => error.message.clone(),
        }
    }

    match action {
        Action::Abort(target) => {
            match verbs::fleet::kill(run, &SystemClock, place, Some(&target.uuid), false, false) {
                Ok(output) => Done::said(match killed_cleanly(&output) {
                    Some(error) => format!("`{}` ended — {}", target.job, said(&error)),
                    None => format!("`{}` aborted", target.job),
                }),
                Err(error) => Done::said(format!("`{}` — {}", target.job, said(&error))),
            }
        }

        Action::Pause(target) => {
            match verbs::fleet::pause(run, &SystemClock, place, &target.uuid) {
                Ok(_) => Done::said(format!("`{}` paused — p resumes it", target.job)),
                Err(error) => Done::said(format!("`{}` — {}", target.job, said(&error))),
            }
        }

        Action::Resume(target) => {
            match verbs::fleet::resume(run, &SystemClock, place, &target.uuid) {
                Ok(_) => Done::said(format!("`{}` resumed", target.job)),
                Err(error) => Done::said(format!("`{}` — {}", target.job, said(&error))),
            }
        }

        // **`r` reads the plan and opens the preview.** It reaps nothing, which
        // is what makes it safe to press out of curiosity — and being safe to
        // press is what makes it get read.
        Action::Preview => match verbs::fleet::reap_plan(run, &SystemClock, place) {
            Err(error) => Done::said(said(&error)),
            Ok(output) => {
                let verbs::Output::ReapPlan(envelope) = output else {
                    unreachable!("reap_plan answers with a plan");
                };
                if envelope.data.results.is_empty() {
                    return Done::said("nothing to reap — every Job is working");
                }
                let rows: Vec<ReapRow> = envelope
                    .data
                    .results
                    .iter()
                    .map(|row| ReapRow {
                        target: armada_core::fleet::bridge::Target {
                            job: row.job.clone(),
                            uuid: row.uuid.clone(),
                        },
                        state: row.state,
                        selected: row.selected,
                        holding: holding(row),
                    })
                    .collect();
                Done {
                    notice: format!(
                        "{} of {} ticked — space toggles, enter reaps, esc cancels",
                        envelope.data.selected,
                        rows.len()
                    ),
                    mode: Some(Mode::Reaping(Reap {
                        rows,
                        cursor: Default::default(),
                    })),
                }
            }
        },

        Action::Reap(targets) => {
            let jobs: Vec<String> = targets.iter().map(|t| t.uuid.clone()).collect();
            match verbs::fleet::reap(run, &SystemClock, place, &jobs) {
                Ok(output) => Done::said(match killed_cleanly(&output) {
                    // **One Job that would not clean does not hide the rest.**
                    Some(error) => format!("reaped {} — {}", jobs.len(), said(&error)),
                    None => format!("reaped {}", jobs.len()),
                }),
                Err(error) => Done::said(said(&error)),
            }
        }

        // **`c` reports the actual switch rather than a fixed sentence, and it
        // says so on the screen rather than by ending it.** The Bridge does
        // not exec on this key — dropping into a session from inside another
        // full-screen view is its own piece of work and not this one's — so
        // `c` names the verb to run instead, and names it correctly whichever
        // way `armada helm enable`/`disable` last left it.
        Action::Chat => Done::said(match verbs::helm::entering_allowed(&place.armada_home) {
            true => {
                "`armada helm --exec` is on for this machine — run it yourself to enter".to_string()
            }
            false => format!(
                "`armada helm --exec` is off — `{}` turns it on; enter boards the selected Job",
                verbs::helm::ENABLE
            ),
        }),
    }
}

/// What one reap candidate is holding, for the preview's own column.
fn holding(row: &armada_core::envelope::ReapCandidate) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(block) = row.port_block {
        parts.push(format!("ports {}-{}", block.from, block.to));
    }
    if row.worktree_exists {
        parts.push("worktree".to_string());
    }
    parts.push(format!("branch {}", row.branch));
    parts.join(", ")
}

/// The first failure a `kill` carried, if it carried one.
///
/// **Carried rather than raised is the contract** — the Job is ended either way
/// (`commands/fleet/kill.md`) — so the caller has to look at the envelope to
/// find out whether anything was left behind.
fn killed_cleanly(output: &Output) -> Option<ArmadaError> {
    match output {
        Output::Kill(envelope) => envelope
            .data
            .results
            .iter()
            .find_map(|killed| killed.error.clone()),
        _ => None,
    }
}

/// Ask for a paragraph, in the box the interview already uses.
///
/// **Off the alternate screen, never on it.** The Bridge has given the terminal
/// back by the time this runs, so the question and the answer land in the
/// scrollback where the reader can see what they typed.
///
/// **With the keys named, which is what it was missing.** The question used to
/// be printed on its own and the box opened under it advertising nothing, so the
/// only way out of it was a chord you had to already know — and the first person
/// to meet it guessed. [`render::editing`] is the header `armada guild edit`
/// already puts above the same widget, with the same three keys, so there is one
/// convention rather than two.
fn ask_text(style: Style, terminal: render::term::Terminal, question: &str) -> Option<String> {
    write_err(&render::editing(question, style, terminal.usable_width()));
    match armada_helm::ask::editor::read(style, "") {
        armada_helm::ask::editor::Answer::Given(text) if !text.trim().is_empty() => Some(text),
        // Nothing typed, `esc`, or a terminal that would not open a box: the
        // Bridge simply reports the frame it was showing. A verb started on an
        // empty prompt is worse than one not started.
        _ => None,
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
///
/// **The worktree is expanded before it is `chdir`-ed into, and that is the
/// whole of one bug.** A Job record keeps its worktree as `~/…`
/// ([`verbs::fleet::Where::expand`] says why), and `chdir` does no tilde
/// expansion — that is the shell's job, and there is no shell here. Handing the
/// stored string straight to `current_dir` made `enter` on the Bridge, and
/// `armada fleet board --exec` from a prompt, fail on every Job on the machine
/// with `No such file or directory` about a directory that was there all along.
fn board_exec(place: &verbs::fleet::Where, output: &Output) -> Result<(), ArmadaError> {
    use std::os::unix::process::CommandExt;

    let Output::Board(envelope) = output else {
        return Ok(());
    };
    let data = &envelope.data;
    let cwd = place.expand(&data.worktree);
    // Said before the exec rather than as an `ENOENT` afterwards, because
    // `exec`'s errno cannot tell a missing directory from a missing `claude`.
    if !cwd.is_dir() {
        return Err(ArmadaError {
            class: ErrClass::Environment,
            r#where: data.worktree.clone(),
            message: format!(
                "`{}` has no worktree left to board: `{}` is gone",
                data.job, data.worktree
            ),
            next_action: Some(format!(
                "`armada fleet kill {}` ends it and releases what it holds",
                data.job
            )),
        });
    }

    let mut argv = data.command.split(' ');
    let program = argv.next().unwrap_or("claude");
    let error = std::process::Command::new(program)
        .args(argv)
        .current_dir(&cwd)
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

/// How `armada guild ls` draws what it shows mid-session.
///
/// **Gathered here rather than read down there**, which is `ARCHITECTURE.md`
/// §1.4: at a terminal it prints a table and a file's content on stderr while it
/// is running, and nothing below the entrypoint asks the terminal anything.
fn look(style: Style, terminal: render::term::Terminal) -> verbs::guild::Look {
    verbs::guild::Look { style, terminal }
}

/// The secret handoff a detaching parent left on stdin, if this is that child.
///
/// **Read here and nowhere else**, because this is the entrypoint and stdin is
/// ambient (`ARCHITECTURE.md` §1.4). The verb underneath gets a `String` on
/// [`app::App::handoff`] exactly as it gets the environment on `inherited`.
///
/// Three guards, and all of them are about never blocking:
///
/// - **`ARMADA_DETACH_RUN` must hold a real run id.** Nothing else in Armada
///   reads stdin at startup, and an unconditional read would hang every ordinary
///   invocation whose stdin is the terminal it was typed at. The id is *parsed*
///   rather than merely present, so that this agrees with
///   `verbs::check::adopted_run` — which validates for its own reason, the id
///   becoming a path — and a malformed value takes the ordinary resolving path
///   in both places instead of one each.
/// - **The run must still be offering itself.** The variable is inherited
///   wholesale by everything the run spawns (PLAN.md §4.5), so a real id at any
///   depth below the detached child is the ordinary case rather than a strange
///   one — measured, an `armada manifest check` run from inside `cargo test`
///   had it. Such a process is not the child the parent wrote a payload for,
///   and reading stdin on its behalf means either swallowing input meant for
///   something else or blocking on a pipe with a writer that is not going to
///   close it. `runs::adoption_offered` is a read and never takes the offer:
///   the taking belongs to the run that carries it out, and `--status` must
///   not write.
/// - **stdin must not be a terminal.** Armada sets that variable and Armada
///   reads it back, so a person who exported it by hand is not a detached child
///   — and the honest answer for them is an ordinary run, not a wait on input
///   that will never come. A detaching parent gives its child a pipe, writes the
///   payload and closes it, so the child's read returns at once.
///
/// Why the payload travels this way at all — rather than in a file, in Armada's
/// own environment, or in argv — is [`armada_helm::secrets`], which is also the
/// only thing that can read it back.
fn detach_handoff(
    inherited: &BTreeMap<String, String>,
    workspace: Option<&armada_core::workspace::Workspace>,
) -> Option<String> {
    use std::io::{IsTerminal, Read};
    let run = inherited
        .get(armada_helm::verbs::check::DETACH_RUN_VAR)
        .and_then(|value| armada_core::run::RunId::parse(value).ok())?;
    if !armada_manifest::runs::adoption_offered(&workspace?.root, &run) {
        return None;
    }
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return None;
    }
    let mut payload = String::new();
    // A read that fails is a run with no secrets rather than a run that cannot
    // start: the grant it cannot satisfy will fail by name a moment later, which
    // says far more than "could not read stdin" would.
    stdin.read_to_string(&mut payload).ok()?;
    Some(payload)
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
/// **The hand-over execs rather than spawning.** Armada has produced the
/// evidence and has nothing further to do, so it gets out of the way entirely —
/// the same shape `armada fleet board --exec` takes handing over a session.
///
/// **Writing is the other answer, and it is two questions and not one.** The
/// first asks what should happen; only then are the proposals put up to be
/// ticked. A reader who wants an agent should not have to walk a tick list to
/// say so, and a reader who wants the proposals is owed the chance to reject
/// each one — *"we can even make it interactive before they even jump into
/// anything on their own where they can check which ones it got correct."*
fn hand_over(
    output: &Output,
    style: Style,
    terminal: render::term::Terminal,
    home: Option<&std::path::Path>,
    cwd: &std::path::Path,
    editor: Option<&str>,
) {
    let Output::Scan(envelope) = output else {
        return;
    };
    if envelope.data.handover != armada_core::scan::Handover::Ask {
        return;
    }

    // **Nothing is offered to be written over a config that is already
    // here.** `scan` is allowed to run in a configured repository and to report
    // what it found; whether the file still agrees with the repository is drift,
    // and drift is not built (`docs/reserved/007`). `config_present` — not
    // `proposals.is_empty()` — is what keeps the write option off the list:
    // the two cases look the same by count and mean opposite things.
    let config_present = envelope.data.evidence.config_present;
    let proposals: &[armada_core::propose::Proposal] = match config_present {
        true => &[],
        false => &envelope.data.proposals,
    };

    let mut ask = at_the_terminal(style, terminal);
    let chosen = armada_helm::ask::Ask::choose(
        &mut ask,
        verbs::config::ONBOARD_QUESTION,
        &verbs::config::onboarding_choices(proposals.len(), config_present),
        verbs::config::stop(proposals.len(), config_present),
    );
    match verbs::config::next(chosen, config_present) {
        verbs::config::Next::Stop => return,
        verbs::config::Next::Write => {
            let wrote = verbs::config::confirm(&mut ask, proposals, cwd);
            write_err(&match &wrote {
                verbs::config::Wrote::Config(lines) => render::wrote_config(*lines, style),
                verbs::config::Wrote::Nothing => render::wrote_nothing(style),
                verbs::config::Wrote::Failed(error) => render::error_lines(error, style),
            });
            // **Only once the write actually happened.** `Nothing` and
            // `Failed` both mean there is no `armada.yml` to open, and opening
            // an editor on nothing is not the middle option anyone asked for.
            if matches!(wrote, verbs::config::Wrote::Config(_)) {
                if let Err(error) =
                    verbs::config::open_in_editor(&armada_manifest::process::RealRun, editor, cwd)
                {
                    write_err(&render::error_lines(&error, style));
                }
            }
            return;
        }
        verbs::config::Next::HandOver => {}
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

#[allow(clippy::too_many_arguments)]
fn emit(
    output: Output,
    json: bool,
    style: Style,
    terminal: render::term::Terminal,
    home: Option<&std::path::Path>,
    cwd: &std::path::Path,
    editor: Option<&str>,
    offerable: bool,
    in_job: bool,
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
        hand_over(&output, style, terminal, home, cwd, editor);
        // **The occasional pull-offer, last of all** — after the verb's own
        // report and after its hand-over, so it never competes with either for
        // what the reader reads first (`docs/reserved/009` item 4).
        // `offerable` already excludes the guild's own verbs; `armada_guild::
        // offer::eligible` (inside `maybe_offer`) excludes `--json`, no
        // terminal and `ARMADA_JOB`, and its own elapsed-time gate is what
        // keeps this off the hot path on every other invocation.
        if offerable {
            if let Some(home) = home {
                let place = verbs::guild::Where {
                    armada_home: armada_manifest::machine::armada_home(home),
                    cwd: cwd.to_path_buf(),
                    claude_home: home.join(".claude"),
                };
                let mut ask = at_the_terminal(style, terminal);
                verbs::offer::maybe_offer(
                    &RealRun,
                    &SystemClock,
                    &place,
                    &mut ask,
                    style,
                    terminal,
                    json,
                    in_job,
                );
            }
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

/// What the failure recorder needs, read once at the entrypoint.
///
/// **`home` is optional and the two other fields are not**, because `$HOME` is
/// itself one of the things that can be missing — and a machine with no `$HOME`
/// has nowhere to keep a record of the failure that says so.
struct Ambient<'a> {
    home: Option<&'a std::path::Path>,
    cwd: &'a std::path::Path,
    argv: &'a [String],
}

/// Write this failure into `~/.armada/failures.jsonl`
/// ([`armada_core::failure`]).
///
/// **Nothing about the failure changes because of this call.** The error still
/// renders, the exit code is still `f(error.class)`, and every path in here that
/// cannot do its job simply does not do it — a logger that turned a bug into a
/// different bug would be worse than no logger at all.
///
/// **Every failure, and still no filtering by class.** The reasoning is in
/// [`armada_core::failure`]: a wrong class is itself a symptom, so a filter that
/// trusted the class would discard exactly the reports worth keeping. The site
/// is the filter — this is the path taken when Armada could not answer, and a
/// check whose tests failed *did* answer and leaves through [`emit`].
///
/// **The one thing the site could not filter is `fault`.** A reserved verb or
/// flag refusing by name reaches this function exactly as a panic does, and it
/// is not a failure — it is the answer that was asked for. Whose mistake it was
/// is carried here from the line that refused
/// ([`armada_core::failure::records`]), because nothing downstream of that line
/// can still tell.
/// Write this run into the ring buffer, so a later `armada report` can attach
/// it.
///
/// **The set this exists for is the runs that succeeded.** `record` below keeps
/// the failures; this keeps everything, which is the whole point — the run that
/// prompted `armada report` exited `0` and printed `CREATED worktree` for work
/// it had correctly not done, and no failure recorder could ever have held it
/// (`docs/reserved/014`).
///
/// **Silent, and never able to change what the run answered.** Same rule as
/// `record`: a recorder that turns a working command into a failing one is
/// worse than no recorder. The write returns `bool` and it is ignored here.
///
/// **Every string is redacted on the way in** ([`armada_helm::redact`]), so a
/// token typed on a command line is not at rest in `~/.armada/` even for the
/// runs nobody ever reports.
fn remember(
    argv: &[String],
    cwd: &std::path::Path,
    home: Option<&std::path::Path>,
    exit: u8,
    envelope: &str,
) {
    let Some(home) = home else {
        return;
    };
    // A throwaway worktree's runs are not the machine's to keep, for the reason
    // `armada_core::failure::scratch` gives about failures: the row names a
    // directory that will not exist by the time anybody reads it. Here it also
    // keeps a fleet of agents from evicting the ten runs the person at the
    // terminal actually did.
    if armada_core::failure::scratch(cwd) {
        return;
    }
    let now = SystemClock;
    let latest = armada_core::recent::note(
        argv,
        home,
        cwd,
        exit,
        Some(envelope),
        &now.wall_rfc3339(),
        now.wall_ms(),
        &|text| armada_helm::redact::scrub(text),
    );
    let armada_home = armada_manifest::machine::armada_home(home);
    let _ = armada_manifest::recent::record(&armada_manifest::recent::path(&armada_home), latest);

    // **The same run, counted against Armada's own roster** — `armada untried`,
    // whose whole subject is the verbs this machine has *never* run. It lives
    // here rather than in a recorder of its own because the two answer the same
    // question at different lengths: the buffer keeps ten runs and can say what
    // you just did, and this keeps one line per verb forever and can say what
    // you never have.
    //
    // **Nothing a person typed is written.** `matched` only ever hands back a
    // name that was already on the roster, so a repository's declared command
    // and a typo alike are counted as nothing at all — which is why this needs
    // no redaction of its own.
    if let Some(verb) = armada_core::untried::matched(
        &armada_core::recent::verb_of(argv),
        &armada_helm::args::every_verb(),
    ) {
        let _ = armada_manifest::untried::record(
            &armada_manifest::untried::path(&armada_home),
            &verb,
            now.wall_ms(),
            exit == 0,
        );
    }
}

fn record(error: &ArmadaError, ambient: &Ambient, fault: Fault) {
    if !armada_core::failure::records(fault, ambient.cwd) {
        return;
    }
    let Some(home) = ambient.home else {
        return;
    };
    let now = SystemClock;
    let (_, line) = armada_core::failure::failed(
        error,
        home,
        ambient.cwd,
        ambient.argv,
        &now.wall_rfc3339(),
        now.wall_ms(),
    );
    let _ = armada_manifest::failures::append(
        &armada_manifest::failures::path(&armada_manifest::machine::armada_home(home)),
        &line,
    );
}

fn fail(error: ArmadaError, json: bool, style: Style, ambient: &Ambient, fault: Fault) -> ExitCode {
    record(&error, ambient, fault);
    let code = error.class.exit_code();
    // **The ring buffer keeps a failing run too, and it keeps the ones the
    // failure log deliberately does not.** A refusal Armada meant never reaches
    // `record` (`armada_core::failure::Fault`) — but *"`--detach` is not built
    // yet, so I ran it without and it did the wrong thing"* is a real report,
    // and the run before it is the evidence. The two files answer different
    // questions: one is *what should be fixed*, this one is *what was run*.
    remember(
        ambient.argv,
        ambient.cwd,
        ambient.home,
        code,
        &Envelope {
            schema_version: armada_core::envelope::SCHEMA_VERSION,
            verb: "armada".to_string(),
            workspace: None,
            status: Status::Failed,
            error: Some(error.clone()),
            data: NoData {},
        }
        .to_json(),
    );
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
