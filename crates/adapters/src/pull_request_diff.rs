//! The diff of a pull request somebody else opened, for a Code Review Job to be checked
//! against. #903.
//!
//! **Two `gh pr diff` calls**: the paths, then the patch. `--name-only` spares parsing the
//! file list out of the patch, which is `verification`'s to read, not this crate's.

use adapter_traits::PullRequestDiff;

use crate::delivery::run_in;

/// `None` is the forge's silence: no tool, not signed in, or a pull request it will not show.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> Option<PullRequestDiff> {
    let files = asked(in_repo, &["pr", "diff", pull_request, "--name-only"])?;
    let patch = asked(in_repo, &["pr", "diff", pull_request])?;
    Some(PullRequestDiff {
        files: files
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
        patch,
    })
}

fn asked(in_repo: &str, args: &[&str]) -> Option<String> {
    let run = run_in(in_repo, "gh", args).ok()?;
    run.status
        .success()
        .then(|| String::from_utf8_lossy(&run.stdout).into_owned())
}
