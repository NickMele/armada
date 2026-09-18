//! `armada land --status [branch]` — where the line and one branch stand,
//! answered from disk.

use std::path::Path;

use super::dir::StateDir;
use super::outcome::read_outcome;
use super::queue::queued;
use super::repo::{common_git_dir, current_branch};
use super::stop::Refused;

/// The exit `--status` gives once a branch has no outcome at all —
/// `scripts/land`'s `EXIT["unknown"]`, the one code no
/// [`super::outcome::OutcomeState`] carries.
pub const UNKNOWN: u8 = 8;

/// Print the line and one branch's own outcome, and answer with the exit
/// code `docs/practices/running-locally.md` documents for it.
pub fn status(cwd: &Path, branch: Option<&str>) -> Result<u8, Refused> {
    let state = StateDir::resolve(cwd).map_err(|why| Refused(why.to_string()))?;
    let line = queued(&state).map_err(|why| Refused(why.to_string()))?;
    if !line.is_empty() {
        // Takes up a turn a killed runner left: a dead runner's entry stays
        // queued, and the next `--status` starts one to pick it back up.
        let exe = std::env::current_exe()
            .map_err(|why| Refused(format!("this binary's own path could not be read: {why}")))?;
        let common = common_git_dir(cwd)?;
        super::runner::ensure_runner(&exe, &state, &common)
            .map_err(|why| Refused(why.to_string()))?;
    }

    if line.is_empty() {
        println!("merge line: empty");
    } else {
        println!("merge line: {} in line", line.len());
    }
    for (n, entry) in line.iter().enumerate() {
        let held = read_outcome(&state, &entry.branch).map_err(|why| Refused(why.to_string()))?;
        let (word, detail) = held
            .map(|o| (o.state.word(), o.detail))
            .unwrap_or(("waiting", String::new()));
        println!(
            "  {}. {}  #{}  {word}: {detail}",
            n + 1,
            entry.branch,
            entry.pr
        );
    }

    let branch = match branch.map(str::to_string).or_else(|| current_branch(cwd)) {
        Some(branch) => branch,
        None => return Ok(0),
    };
    let held = read_outcome(&state, &branch).map_err(|why| Refused(why.to_string()))?;
    let Some(held) = held else {
        println!("{branch}: nothing known");
        return Ok(UNKNOWN);
    };
    println!("{branch}: {} — {}", held.state.word(), held.detail);
    let lists: [&[String]; 6] = [
        &held.failed,
        &held.already,
        &held.new_lines,
        &held.conflicts,
        &held.logs,
        &held.cleanup,
    ];
    for items in lists {
        for item in items {
            println!("  {item}");
        }
    }
    Ok(held.state.exit_code() as u8)
}
