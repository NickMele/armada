//! Armada's dev tooling. Run as `cargo xtask <command>`.
//!
//! Rust rather than a script because this repo has no interpreter and should
//! not acquire one: `README.md` claims "no interpreter, no toolchain, nothing
//! to provision", and a lint that needs Python and a venv contradicts that on
//! two CI platforms. The predecessor of `doclint` was three Python files, and
//! porting them removed the whole provisioning question.

mod blocks;
mod boundaries;
mod docs;
mod keys;
mod privacy;
mod xref;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
cargo xtask <command>

  doclint [-v]    every documentation check below, in order
  xref            resolve every §N.N against the heading index
  blocks          parse every fenced yaml / json / sh block
  keys            config keys in examples vs prose (-v lists one-sided keys)
  privacy         the ARCHITECTURE.md §2.4 leak rules, over every tracked file
  history         the same rules over every ref and commit — a report, not a gate
  boundaries      the ARCHITECTURE.md §1.5 and §1.9 layers contract, over the
                  crate graph: dependencies point inward, nothing points upward

Exit: 0 clean, 1 findings, 2 could not run.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let command = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str);

    let root = match repo_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("xtask: {e}");
            return ExitCode::from(2);
        }
    };

    let corpus = match docs::load(&root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("xtask: could not read the corpus: {e}");
            return ExitCode::from(2);
        }
    };

    let mut findings = Vec::new();
    let mut ran = Vec::new();

    match command {
        Some("xref") => {
            findings.extend(xref::check(&corpus));
            ran.push("xref");
        }
        Some("blocks") => {
            findings.extend(blocks::check(&corpus));
            ran.push("blocks");
        }
        Some("keys") => {
            findings.extend(keys::check(&corpus, verbose));
            ran.push("keys");
        }
        Some("boundaries") => match boundaries::check(&root) {
            Ok(f) => {
                findings.extend(f);
                ran.push("boundaries");
            }
            Err(e) => {
                eprintln!("xtask: boundaries: {e}");
                return ExitCode::from(2);
            }
        },
        Some("privacy") => match privacy::check(&root) {
            Ok(f) => {
                findings.extend(f);
                ran.push(privacy_label(&root));
            }
            Err(e) => {
                eprintln!("xtask: privacy: {e}");
                return ExitCode::from(2);
            }
        },
        Some("history") => match privacy::history(&root) {
            Ok(f) => {
                findings.extend(f);
                ran.push(history_label(&root));
            }
            Err(e) => {
                eprintln!("xtask: history: {e}");
                return ExitCode::from(2);
            }
        },
        Some("doclint") | None => {
            findings.extend(xref::check(&corpus));
            findings.extend(blocks::check(&corpus));
            findings.extend(keys::check(&corpus, verbose));
            ran.extend(["xref", "blocks", "keys"]);
            match privacy::check(&root) {
                Ok(f) => {
                    findings.extend(f);
                    ran.push(privacy_label(&root));
                }
                Err(e) => {
                    eprintln!("xtask: privacy: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        Some(other) => {
            eprintln!("xtask: unknown command `{other}`\n\n{USAGE}");
            return ExitCode::from(2);
        }
    }

    if findings.is_empty() {
        println!(
            "clean — {} (corpus: {} files)",
            ran.join(", "),
            corpus.len()
        );
        return ExitCode::SUCCESS;
    }

    for f in &findings {
        println!("{f}");
    }
    println!("\n{} finding(s) — {}", findings.len(), ran.join(", "));
    ExitCode::from(1)
}

/// How the privacy check is named in the summary line.
///
/// Both of its rules run either way, but rule 1 has nothing to match until a
/// private name is configured — locally in `.claude/contamination.local`, on CI
/// in the repository secret (`ARCHITECTURE.md` §2.4). Reporting that run as a
/// plain `privacy` claims a pass the check never made, which is the same
/// silently-green failure §2.4 has already been bitten by twice.
fn privacy_label(root: &Path) -> &'static str {
    if privacy::name_rule_armed(root) {
        "privacy"
    } else {
        "privacy (name rule unconfigured)"
    }
}

/// The same distinction for `history`, which needs it more: its `$HOME` rule
/// finds nothing in a repository whose commits were all written elsewhere, so
/// an unconfigured run is *expected* to be silent and reads as an all-clear on
/// exactly the surface the operator asked about.
fn history_label(root: &Path) -> &'static str {
    if privacy::name_rule_armed(root) {
        "history"
    } else {
        "history (name rule unconfigured)"
    }
}

/// The repo root, from `CARGO_MANIFEST_DIR` rather than the cwd.
///
/// `cargo xtask` can be invoked from any subdirectory, and a lint that reports
/// different results depending on where it was run is worse than no lint.
fn repo_root() -> Result<PathBuf, String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR unset — run via `cargo xtask`".to_string())?;
    PathBuf::from(&manifest)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("no parent for {manifest}"))
}
