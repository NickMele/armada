//! Denies the git verbs that write a commit, a ref, the index or history —
//! once at spawn, and a second time when a declared command names one.
//!
//! Three surprises, each measured against the real CLI rather than assumed,
//! shape the rules below: deny beats an identical allow; a global flag before
//! the subcommand (`-C`, `-c`, `--namespace`, ...) moves it out of a
//! verb-keyed rule's reach; and a deny on bare `branch` also blocks the
//! narrower `branch --list` a Drone would otherwise be granted, with no rule
//! able to tell that grant apart from `git branch <name>`, which creates one.
//! Each rule's own comment below says which surprise it answers and what was
//! measured.

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
    // Not deniable as a read-only verb at all: no rule tells `branch --list`
    // apart from a bare `git branch <name>`, which creates one. See the
    // module doc and `READ_ONLY_GIT_VERBS` in `harness.rs`.
    "branch",
];

/// Denies every git invocation that carries a global flag before its
/// subcommand, whatever that subcommand is.
///
/// Measured against the real CLI: `Bash(git -*)` — a wildcard glued to the
/// flag's own token — denied `git -c x=y commit`, `git --no-pager commit`
/// and `git --namespace n commit` alike, while `git status`, `git log` and
/// every other bare verb in `READ_ONLY_GIT_VERBS` still ran. `Bash(git -:*)`,
/// shaped like the verb rules above, did not: the CLI matches whole tokens,
/// not a token split at a bare trailing `-`, so it never matched `-c` or
/// `--no-pager`. No read-only grant renders a global flag, so this costs
/// nothing granted.
const GLOBAL_FLAG_DENY: &str = "Bash(git -*)";

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
    rules.push(String::from(GLOBAL_FLAG_DENY));
    rules
}

/// Whether a declared command would push, checked on its own git subcommand
/// rather than the word `push` — `git stash push` names `stash`, and a
/// refusal that fired on the word would deny it the same way it would deny
/// `cargo test --features push`.
pub(crate) fn would_push(run: &str) -> bool {
    segments(run).any(|segment| git_subcommand(segment) == Some("push"))
}

/// Whether a declared command names one of [`MUTATING_GIT_VERBS`] — checked
/// the same way as [`would_push`], naming what it found so the refusal can
/// say so.
pub(crate) fn denied_mutating_git(run: &str) -> Option<String> {
    segments(run).find_map(|segment| {
        let subcommand = git_subcommand(segment)?;
        MUTATING_GIT_VERBS
            .contains(&subcommand)
            .then(|| format!("`{subcommand}`"))
    })
}

/// Whether a declared command carries a git global flag before its
/// subcommand — [`GLOBAL_FLAG_DENY`] denies every such form wholesale, so a
/// command shaped this way is denied at runtime whatever subcommand follows,
/// and is refused here rather than rendered as an allow that deny would
/// silently outrank. Named on the flag found, since the subcommand behind it
/// is not reliably parseable — that is exactly the property being denied.
pub(crate) fn git_global_flag(run: &str) -> Option<String> {
    segments(run).find_map(|segment| {
        let mut words = segment.split_whitespace();
        let program = words.next()?;
        if program.rsplit('/').next().unwrap_or(program) != "git" {
            return None;
        }
        let next = words.next()?;
        next.starts_with('-').then(|| next.to_string())
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

    /// The gap this closed: a bare `git branch <name>` creates a branch and
    /// no rule can tell it from `git branch --list`, so neither is granted
    /// read-only and both are denied the same way as every other mutating
    /// verb.
    #[test]
    fn a_bare_branch_is_denied_whether_or_not_it_carries_a_flag() {
        assert!(denied_mutating_git("git branch --list").is_some());
        assert!(denied_mutating_git("git branch").is_some());
        assert!(denied_mutating_git("git branch new-feature").is_some());
    }

    #[test]
    fn a_branch_delete_is_named() {
        assert!(denied_mutating_git("git branch -D old").is_some());
    }

    /// The other gap: a global flag before the subcommand is denied wholesale
    /// by [`GLOBAL_FLAG_DENY`] regardless of what follows, so it is named on
    /// the flag rather than the (unparseable) subcommand behind it.
    #[test]
    fn a_global_flag_before_the_subcommand_is_named() {
        assert_eq!(
            git_global_flag("git -c user.name=x commit -m x"),
            Some(String::from("-c"))
        );
        assert_eq!(
            git_global_flag("git --namespace n commit"),
            Some(String::from("--namespace"))
        );
        assert_eq!(
            git_global_flag("git --no-pager log"),
            Some(String::from("--no-pager"))
        );
    }

    #[test]
    fn a_bare_verb_carries_no_global_flag() {
        assert_eq!(git_global_flag("git commit -m x"), None);
        assert_eq!(git_global_flag("git status"), None);
        assert_eq!(git_global_flag("cargo build --features push"), None);
    }
}
