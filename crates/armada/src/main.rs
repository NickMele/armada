//! `armada` — one binary, four verbs.
//!
//! `serve` is the daemon; `check` and `run` execute one thing the repository's
//! Manifest declares; `clean` gives its worktrees, branches and Jobs back. Each
//! verb's own module holds what it does and why.
//!
//! # What exits non-zero
//!
//! A genuine refusal — a command line that could not be read, a name the
//! Manifest does not declare, a Fleet that would not start, a clean that could
//! not finish. `check` and `run` carry the command's own exit code out, because
//! a person running a Check by hand wants the answer the Check gave.
//!
//! **`serve` against a live Fleet exits `0`** and names the pid. The state the
//! caller asked for is the state that already holds, and v1's `start` carried a
//! test by that name. `serve`'s module doc holds the whole argument, including
//! the one launchd-shaped rule that is deliberately not implemented yet.

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use armada::cli::{self, Usage, Verb};
use armada::declared::Registry;
use armada::serve::PROVISIONAL_CHECK_BUDGET;
use armada::{clean, declared, say, serve};

#[tokio::main]
async fn main() -> ExitCode {
    let asked = match cli::read(std::env::args().skip(1)) {
        Ok(asked) => asked,
        Err(misread) => {
            eprintln!("{misread}");
            return ExitCode::FAILURE;
        }
    };

    match asked {
        Verb::Help => {
            print!("{Usage}");
            ExitCode::SUCCESS
        }
        Verb::Serve { repository } => match serve::serve(repository).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                chain("Fleet did not start", why.as_ref());
                ExitCode::FAILURE
            }
        },
        Verb::Check { name } => declared_by_the_manifest(Registry::Checks, &name, "check").await,
        Verb::Run { name } => declared_by_the_manifest(Registry::Commands, &name, "run").await,
        Verb::Clean { everything } => clean_this_repository(everything),
    }
}

/// Run one Check or one Command in the repository the caller is standing in.
///
/// **The working directory, never a search upward.** `Setup::at` refuses to
/// adopt an ancestor's Manifest for the daemon, and a Check that quietly ran
/// under a different repository's declaration would be the same failure with
/// less warning.
async fn declared_by_the_manifest(registry: Registry, name: &str, verb: &str) -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(why) => {
            eprintln!("the working directory could not be read: {why}");
            return ExitCode::FAILURE;
        }
    };
    match declared::execute(&root, registry, name, PROVISIONAL_CHECK_BUDGET).await {
        Ok(ran) => {
            say::ran(&ran, verb);
            ExitCode::from(ran.status())
        }
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

fn clean_this_repository(everything: bool) -> ExitCode {
    let (Ok(root), Ok(runtime_file)) = (std::env::current_dir(), fleet::runtime::machine_path())
    else {
        eprintln!("the working directory and HOME are both needed, and one of them is not there");
        return ExitCode::FAILURE;
    };
    let machine: PathBuf = runtime_file
        .parent()
        .expect("the runtime file has a directory")
        .to_path_buf();

    match clean::clean(&root, &machine, everything) {
        Ok(cleaned) => {
            say::cleaned(&cleaned);
            match cleaned.faults.is_empty() {
                true => ExitCode::SUCCESS,
                // Every fault was already named on its own line. This is only
                // the status, so a script wrapping it can tell.
                false => ExitCode::FAILURE,
            }
        }
        Err(why) => {
            // **The reason leads and the outcome trails.** This read
            // `nothing was cleaned: {why}`, and the owner read the first
            // three words as "nothing is running" and stopped there — which
            // is the opposite of what it was saying. What a person can act on
            // goes first; that nothing happened is the least useful part and
            // is what a reader infers anyway from a command that refused.
            eprintln!("{why}\n\nNothing was cleaned.");
            ExitCode::FAILURE
        }
    }
}

/// The whole cause chain, outermost first.
///
/// A failure is carried as a chain rather than a sentence, and printing only
/// the outermost link would throw away the part that says what actually went
/// wrong — which is the entire reason the chain is not flattened until it
/// reaches the wire.
fn chain(what: &str, why: &dyn Error) {
    eprintln!("{what}: {why}");
    let mut cause = why.source();
    while let Some(link) = cause {
        eprintln!("  caused by: {link}");
        cause = link.source();
    }
}
