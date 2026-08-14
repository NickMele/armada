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
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use verbs::Output;

const USAGE: &str = "\
armada — one consistent vocabulary for managing a repo's tech stack

  armada manifest init    [--json] [--dry-run]   claim this workspace: ports, .armada/, setup
  armada manifest clean   [--json] [--dry-run] [--project|--all]
                          [--orphaned] [--artifacts] [--force]
                                                 release what this workspace owns
  armada manifest clean --orphaned --force-rebuild
                                                 rebuild an unreadable ~/.armada/manifest.db
  armada manifest status  [--json] [--project|--all]
                                                 what is running, mine, and stale
  armada manifest check   [--json] [--dry-run] [--all-files] [--fix] [--wait]
                          [--files <path>…] [--component <name>] [--jobs <n>] [<selector>]
                                                 lint / format / test
  armada manifest <name> …                       a commands: entry from this repo's armada.yml

  armada --version
  armada --help

Global flags come before the module. Everything after a commands: name is the
child's, including flags Armada itself defines.

Manifest is the module that is built. Not built yet: the `armada init`,
`doctor`, `guild`, `fleet`, `helm` and `bridge` surfaces (PHASES.md §8), and
Manifest's own up, down, config, agents-md, explain, and check's
--detach/--status.
";

fn main() -> ExitCode {
    posix::restore_sigpipe();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let inherited: BTreeMap<String, String> = std::env::vars().collect();

    // **The terminal is ambient state, so it is read here and passed down**
    // (`ARCHITECTURE.md` §1.4). `NO_COLOR` comes out of the environment map this
    // function already built, rather than from a second `std::env` call in the
    // renderer — one read, one answer, and a test can set it.
    let no_color = inherited.contains_key("NO_COLOR");
    let stdout_is_tty = std::io::stdout().is_terminal();

    let parsed = match args::parse(&argv) {
        Ok(parsed) => parsed,
        // `--json` is answered even when the failure is the parse itself: the
        // parser carries out the flag it had already seen, so a machine caller
        // probing a verb that does not exist yet still reads an envelope.
        Err(failure) => {
            let style = Style::decide(failure.color, stdout_is_tty, no_color);
            return fail(failure.error, failure.json, style);
        }
    };
    let style = Style::decide(parsed.color, stdout_is_tty, no_color);

    match parsed.invocation {
        Invocation::Version => {
            write_out(&format!("armada {}\n", env!("CARGO_PKG_VERSION")));
            ExitCode::SUCCESS
        }
        Invocation::Help => {
            write_out(USAGE);
            ExitCode::SUCCESS
        }
        other => {
            let json = json_wanted(&other);
            match dispatch(other, &cwd, home.as_deref(), inherited) {
                Ok(output) => emit(output, json, style),
                Err(error) => fail(error, json, style),
            }
        }
    }
}

fn json_wanted(invocation: &Invocation) -> bool {
    match invocation {
        Invocation::Init(common) | Invocation::Status(common) => common.json,
        Invocation::Clean { common, .. } => common.json,
        Invocation::Check(check) => check.json,
        Invocation::Dispatch { json, .. } => *json,
        Invocation::Version | Invocation::Help => false,
    }
}

fn dispatch(
    invocation: Invocation,
    cwd: &std::path::Path,
    home: Option<&std::path::Path>,
    inherited: BTreeMap<String, String>,
) -> Result<Output, ArmadaError> {
    let home = home.ok_or_else(|| ArmadaError {
        class: ErrClass::Environment,
        r#where: "HOME".to_string(),
        message: "$HOME is not set, so Armada cannot find ~/.armada/".to_string(),
        next_action: Some("set HOME, then retry unchanged".to_string()),
    })?;

    let run = RealRun;

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
        Invocation::Check(check) => verbs::check::run(&mut app, &check),
        Invocation::Dispatch { name, argv, json } => {
            verbs::dispatch::run(&mut app, &name, &argv, json)
        }
        Invocation::Version | Invocation::Help => unreachable!("handled before dispatch"),
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

fn emit(output: Output, json: bool, style: Style) -> ExitCode {
    if json {
        write_out(&output.to_json());
    } else {
        let text = render::human(&output, style);
        if output.exit_code() == 0 {
            write_out(&text);
        } else {
            write_err(&text);
        }
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
