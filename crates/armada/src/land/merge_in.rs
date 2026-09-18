//! Bringing the base into the candidate worktree, on top of
//! [`Delivery::bring_up_to_date`] — the clean and conflicted cases are that
//! call's; regenerating a conflicted generated file is this line's own, on
//! top of it. `#1315`.
//!
//! **Built on `bring_up_to_date` rather than a call of its own.** That call
//! already merges the base in with no hooks, as this crate's own commit
//! identity, and leaves conflict markers for a caller to clear — exactly
//! what a turn needs before it can tell a real conflict from one only in a
//! generated file. [`adapter_traits::Worktree::at`] is the seam: it needs no
//! Job, no [`adapter_traits::WorktreeSpec`], and no branch actually checked
//! out under the name it carries — `crate::merging_in`'s own git calls (in
//! `adapters`) read the path from it, and the branch only names a stash tag
//! that call never needs here, since [`super::worktree::reused`] always
//! hands back a clean tree.

use std::path::Path;

use adapter_traits::{Base, BroughtUpToDate, Delivery};
use adapters::GitVcs;

use super::git::best_effort;
use super::shell::run;

/// Why merging the base in did not leave a clean, committed candidate.
pub enum MergeInFailed {
    /// At least one conflict is not a generated file's — the merge was
    /// aborted, and these are the files a person resolves by hand.
    Conflict { files: Vec<String> },
    /// The merge itself, or a regeneration, did not run — the merge was
    /// aborted (where one was in progress) either way.
    Stopped(String),
}

/// Merge `base` into the branch checked out at `worktree_path`, and clear
/// any conflict that is only a generated file's — checking out the base's
/// own copy and rebuilding it with the command its header names.
pub fn merge_in(
    worktree_path: &Path,
    branch: &str,
    base: &str,
    logs: &Path,
) -> Result<(), MergeInFailed> {
    let vcs = GitVcs::new();
    let worktree = adapter_traits::Worktree::at(
        worktree_path.to_string_lossy().to_string(),
        branch.to_string(),
    );
    let merged = vcs.bring_up_to_date(&worktree, &Base::Inferred(base.to_string()));
    let files = match merged {
        Ok(BroughtUpToDate::Clean { .. }) => return Ok(()),
        Ok(BroughtUpToDate::Conflicted { files, .. }) => files,
        Ok(BroughtUpToDate::PutBack { .. }) => {
            let _ = best_effort(worktree_path, &["merge", "--abort"]);
            return Err(MergeInFailed::Stopped(format!(
                "{base} does not merge into {branch}: the merge failed without leaving a conflict to resolve"
            )));
        }
        Err(why) => return Err(MergeInFailed::Stopped(why.said())),
    };

    let mut regenerate: Vec<(String, String)> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    for path in &files {
        let theirs = best_effort(worktree_path, &["show", &format!(":3:{path}")])
            .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
            .unwrap_or_default();
        match generated_command(&theirs) {
            Some(command) => regenerate.push((path.clone(), command)),
            None => others.push(path.clone()),
        }
    }
    if !others.is_empty() {
        let _ = best_effort(worktree_path, &["merge", "--abort"]);
        return Err(MergeInFailed::Conflict { files: others });
    }

    // Generated files are rebuilt from the merged sources, never
    // hand-resolved — the base's own copy first, so the regeneration
    // command reads sources that already hold the merge.
    let paths: Vec<&str> = regenerate.iter().map(|(path, _)| path.as_str()).collect();
    let mut checkout = vec!["checkout", "--theirs", "--"];
    checkout.extend(paths.iter().copied());
    if !ok(best_effort(worktree_path, &checkout)) {
        let _ = best_effort(worktree_path, &["merge", "--abort"]);
        return Err(MergeInFailed::Stopped(
            "`git checkout --theirs` failed while regenerating a conflicted file".to_string(),
        ));
    }

    let mut distinct_commands: Vec<String> = Vec::new();
    for (_, command) in &regenerate {
        if !distinct_commands.contains(command) {
            distinct_commands.push(command.clone());
        }
    }
    let log = logs.join("regenerate.log");
    for command in &distinct_commands {
        let argv: Vec<&str> = command.split_whitespace().collect();
        let ran = run(&argv, worktree_path, None, Some(&log));
        if !matches!(ran, Ok(ran) if ran.success()) {
            let _ = best_effort(worktree_path, &["merge", "--abort"]);
            return Err(MergeInFailed::Stopped(format!(
                "`{command}` failed while regenerating a conflicted file; see {}",
                log.display()
            )));
        }
    }

    // `-A`, not the conflicted paths: one generator writes several files,
    // and a side effect left unstaged is checked in the worktree and not in
    // the commit.
    if !ok(best_effort(worktree_path, &["add", "-A"])) {
        let _ = best_effort(worktree_path, &["merge", "--abort"]);
        return Err(MergeInFailed::Stopped(
            "`git add -A` failed while finishing the merge".to_string(),
        ));
    }
    if !ok(best_effort(
        worktree_path,
        &["commit", "--quiet", "--no-edit"],
    )) {
        let _ = best_effort(worktree_path, &["merge", "--abort"]);
        return Err(MergeInFailed::Stopped(
            "`git commit` failed while finishing the merge".to_string(),
        ));
    }
    Ok(())
}

fn ok(run: std::io::Result<std::process::Output>) -> bool {
    run.map(|output| output.status.success()).unwrap_or(false)
}

/// The command a "theirs" conflicted file's own header names, from its
/// first five lines — `` GENERATED by `<command>` `` — or `None` for a file
/// that says nothing about how it was made.
fn generated_command(text: &str) -> Option<String> {
    let head: String = text.lines().take(5).collect::<Vec<_>>().join("\n");
    let marker = "GENERATED by `";
    let start = head.find(marker)? + marker.len();
    let rest = &head[start..];
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}
