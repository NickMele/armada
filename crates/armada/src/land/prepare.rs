//! What a turn keeps its two worktrees prepared with, before
//! `verify-foundations` and before a Check — `scripts/land`'s own
//! `nothing_left`, `seed` and `setup`.

use std::path::Path;

use super::armada_cli::run_command;
use super::env::Env;
use super::git::checked;
use super::shell::run;
use super::stop::Stopped;
use super::worktree::main_tree;

/// The gate worktree's tracked files must be exactly what the candidate
/// commit holds — checked after each phase that might have left something
/// behind. **Tracked only**: the seed and `setup.requires` write untracked
/// build output into the same tree, which is what they are for.
pub fn nothing_left(worktree: &Path, after: &str) -> Result<(), Stopped> {
    let output = checked(worktree, &["status", "--porcelain", "--untracked-files=no"])?;
    let left = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if left.is_empty() {
        return Ok(());
    }
    Err(Stopped::stopped(format!(
        "{after} left files the commit does not carry, so the Checks would \
         measure a tree nothing can merge:\n{left}"
    )))
}

/// Clone [`Env::seed`]'s paths, copy-on-write, from the main checkout into
/// the gate worktree, only where the source exists and the target does
/// not. **A clone that fails stops the turn** — never a silent fallback to
/// a cold build, so the two sides of a comparison are prepared the same way
/// by construction.
pub fn seed(repo: &Path, worktree: &Path, env: &Env, logs: &Path) -> Result<(), Stopped> {
    let main = main_tree(repo).map_err(|why| Stopped::stopped(why.to_string()))?;
    let log = logs.join("setup.log");
    for path in &env.seed {
        let source = main.join(path);
        let target = worktree.join(path);
        if !source.exists() || target.exists() {
            continue;
        }
        let source = source.to_string_lossy().to_string();
        let target = target.to_string_lossy().to_string();
        let ran = run(&["cp", "-cR", &source, &target], worktree, None, Some(&log))?;
        if !ran.success() {
            return Err(Stopped::stopped(format!(
                "`{path}` could not be cloned into {}, so this tree would be prepared \
                 differently from the one it is compared against; see setup.log",
                worktree.display()
            )));
        }
    }
    Ok(())
}

/// Run every [`Env::setup`] name, only where a Check is about to — the
/// Manifest's `setup.requires`, a second copy of the Manifest's own answer.
pub fn setup(worktree: &Path, env: &Env, logs: &Path) -> Result<(), Stopped> {
    let log = logs.join("setup.log");
    for name in &env.setup {
        run_command(&env.armada, worktree, name, &log)?;
    }
    Ok(())
}
