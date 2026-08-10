//! `char` — the binary.
//!
//! **Phase 1 ships no verbs.** The config contract is the phase's whole output,
//! so this entrypoint does the three things that must be right before any verb
//! exists, and nothing else:
//!
//! 1. restores `SIGPIPE`, before a single byte is written — measured, Rust
//!    ignores it and `char status | head` panics with exit 101 otherwise
//!    (`docs/traps.md`);
//! 2. flushes stdout explicitly on every exit path, because `process::exit`
//!    skips a `BufWriter` flush and the failure is *size-dependent*: a 491-byte
//!    payload is lost and a 20 KB one is not, so it passes a test with a large
//!    fixture and silently empties a small real one;
//! 3. exits by error class and by nothing else — `exit = f(error.class)`
//!    (`ARCHITECTURE.md` §1.6).
//!
//! An unknown invocation is therefore `bad_invocation`, exit 2, which is the
//! honest answer for every verb this binary does not have yet. The argument
//! parser, the `--json` envelope and its golden snapshots arrive with the first
//! verb, in phase 2 — a renderer with nothing to render would be inventing the
//! shape of a payload rather than deriving it from a verb.

#![deny(unsafe_code)]

use charkit_core::error::ErrClass;
use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "\
char — one consistent vocabulary for managing a repo's tech stack

  char --version
  char --help

No verbs yet: phase 1 ships the char.yml contract and no runtime.
The schema for char.yml is at crates/core/schema/char.schema.json.
";

fn main() -> ExitCode {
    charkit_adapters::posix::restore_sigpipe();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let requested = args.iter().map(String::as_str).collect::<Vec<_>>();

    match requested.as_slice() {
        ["--version"] | ["-V"] => {
            print_and_flush(&format!("char {}\n", env!("CARGO_PKG_VERSION")));
            ExitCode::SUCCESS
        }
        ["--help"] | ["-h"] => {
            print_and_flush(USAGE);
            ExitCode::SUCCESS
        }
        _ => {
            let class = ErrClass::BadInvocation;
            eprint!("char: no such verb yet\n\n{USAGE}");
            let _ = std::io::stderr().flush();
            let _ = std::io::stdout().flush();
            ExitCode::from(class.exit_code())
        }
    }
}

/// Write and flush. Never through a `BufWriter` that a later `process::exit`
/// could skip — see the module docs and `docs/traps.md`.
fn print_and_flush(text: &str) {
    let mut out = std::io::stdout();
    // A broken pipe here is the ordinary `| head` case: SIGPIPE has been
    // restored, so the process dies silently with 141 rather than reaching
    // this error at all. If it does arrive, there is nowhere left to report it.
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}
