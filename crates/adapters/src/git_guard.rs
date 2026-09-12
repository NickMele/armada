//! Denies the git verbs that write a commit, a ref, the index or history —
//! once at spawn, and a second time when a declared command names one.
//!
//! Three surprises, each measured against the real CLI rather than assumed,
//! shape the rules below: deny beats an identical allow (so an ambient
//! setting granting `git commit` still loses); `git -C <path> commit` does
//! not match a rule keyed on `commit`; and a deny on bare `branch` also
//! blocks the narrower `branch --list` a Drone is granted. Each rule's own
//! comment below says which surprise it answers.

/// Verbs that write a commit, a ref, the index or history, and render no
/// read-only rule anywhere — safe to deny outright, one rule each. `push` is
/// its own rule below, kept apart so [`would_push`]'s refusal can still name
/// it specifically.
const MUTATING_GIT_VERBS: &[&str] = &[
    "commit",
    "add",
    "rm",
    "mv",
    "reset",
    "rebase",
    "merge",
    "cherry-pick",
    "revert",
    "am",
    "apply",
    "tag",
    "checkout",
    "switch",
    "restore",
    "stash",
    "pull",
    "worktree",
];

/// Global flags that point git at a different repository or working tree
/// before the subcommand runs, so `commit` is never the second word.
/// Measured: an identical `Bash(git commit:*)` deny did not match `git -C
/// <path> commit`, and the commit landed. No read-only grant renders any of
/// these, so denying them wholesale costs nothing granted.
const REDIRECT_FLAGS: &[&str] = &["-C", "--git-dir", "--work-tree"];

/// `branch`'s mutating flags, named individually because the bare verb is
/// not deniable without also denying the granted `--list` form — measured:
/// a `Bash(git branch:*)` deny also denied `Bash(git branch --list:*)`.
const BRANCH_MUTATING_FLAGS: &[&str] = &[
    "-d", "-D", "-m", "-M", "-c", "-C", "--delete", "--move", "--copy",
];

/// The `--disallowedTools` entries a Drone's launch always carries, whatever
/// it was granted — not conditioned on
/// [`adapter_traits::Grant::ReadTheRepository`], so the floor holds for a
/// Drone with no git access too.
pub(crate) fn disallowed_git_rules() -> Vec<String> {
    let mut rules: Vec<String> = MUTATING_GIT_VERBS
        .iter()
        .map(|verb| format!("Bash(git {verb}:*)"))
        .collect();
    rules.push(String::from("Bash(git push:*)"));
    for flag in REDIRECT_FLAGS {
        rules.push(format!("Bash(git {flag}:*)"));
    }
    for flag in BRANCH_MUTATING_FLAGS {
        rules.push(format!("Bash(git branch {flag}:*)"));
    }
    rules
}

/// Whether a declared command would push, checked on its own git subcommand
/// rather than the word `push` — `git stash push` names `stash`, and a
/// refusal that fired on the word would deny it the same way it would deny
/// `cargo test --features push`.
pub(crate) fn would_push(run: &str) -> bool {
    segments(run).any(|segment| git_subcommand(segment) == Some("push"))
}

/// Whether a declared command names one of [`MUTATING_GIT_VERBS`] or `branch`
/// with one of [`BRANCH_MUTATING_FLAGS`] — checked the same way as
/// [`would_push`], naming what it found so the refusal can say so.
pub(crate) fn denied_mutating_git(run: &str) -> Option<String> {
    segments(run).find_map(|segment| {
        let subcommand = git_subcommand(segment)?;
        if MUTATING_GIT_VERBS.contains(&subcommand) {
            return Some(format!("`{subcommand}`"));
        }
        if subcommand == "branch" {
            let flag = segment
                .split_whitespace()
                .find(|word| BRANCH_MUTATING_FLAGS.contains(word))?;
            return Some(format!("`branch {flag}`"));
        }
        None
    })
}

/// One shell segment's git subcommand, if it runs `git` at all: the first
/// word after the program that is not one of git's own flags, skipping the
/// value of a flag that takes one so the value is never mistaken for the
/// subcommand (`git -C <path> commit` names `commit`, not the path).
fn git_subcommand(segment: &str) -> Option<&str> {
    let mut words = segment.split_whitespace();
    let program = words.next()?;
    if program.rsplit('/').next().unwrap_or(program) != "git" {
        return None;
    }
    while let Some(word) = words.next() {
        if !word.starts_with('-') {
            return Some(word);
        }
        if matches!(word, "-C" | "-c" | "--git-dir" | "--work-tree") {
            words.next();
        }
    }
    None
}

/// A declared command's shell segments, split the way a shell chains
/// commands, so a denied verb after `&&` on a segment whose own program is
/// not `git` is still found.
fn segments(run: &str) -> impl Iterator<Item = &str> {
    run.split(|c| matches!(c, '&' | '|' | ';'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dash_c_argument_is_never_mistaken_for_the_subcommand() {
        assert_eq!(
            git_subcommand("git -C /repos/armada commit -m x"),
            Some("commit")
        );
        assert_eq!(
            git_subcommand("git -C /repos/armada status"),
            Some("status")
        );
    }

    #[test]
    fn a_bare_branch_listing_names_no_mutating_flag() {
        assert_eq!(denied_mutating_git("git branch --list"), None);
        assert_eq!(denied_mutating_git("git branch"), None);
    }

    #[test]
    fn a_branch_delete_is_named() {
        assert!(denied_mutating_git("git branch -D old").is_some());
    }
}
