//! Proving what merged, and waiting for the forge to see a pushed candidate —
//! `scripts/land`'s own `prove` and `wait_for_head`.

use std::path::Path;
use std::time::{Duration, Instant};

use super::dir::StateDir;
use super::env::Env;
use super::git::{best_effort, checked};
use super::outcome::{OutcomePatch, OutcomeState};
use super::queue::QueueEntry;
use super::repo::rev_parse;
use super::shell::gh_view;
use super::stop::Stopped;

/// Poll `gh pr view` until the pushed `candidate` shows up as the pull
/// request's head, up to [`Env::head_wait`].
pub fn wait_for_head(
    repo: &Path,
    env: &Env,
    pull_request: u64,
    candidate: &str,
) -> Result<(), Stopped> {
    let deadline = Instant::now() + env.head_wait;
    let pr = pull_request.to_string();
    loop {
        let seen = gh_view(&env.gh, repo, &pr, "headRefOid").and_then(|view| view.head_ref_oid);
        if seen.as_deref() == Some(candidate) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Stopped::stopped(format!(
                "the forge did not show #{pull_request} at {} within {} s; land again",
                short(candidate),
                env.head_wait.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Confirm the merge, and answer whether the base it landed on is the one
/// it was gated against — the turn's very last act, whatever it finds.
pub fn prove(repo: &Path, state: &StateDir, env: &Env, entry: &QueueEntry, base: &str) -> Stopped {
    match try_prove(repo, env, entry, base) {
        Ok(done) | Err(done) => {
            let _ = std::fs::remove_file(state.stamp_path(&entry.branch));
            done
        }
    }
}

fn try_prove(repo: &Path, env: &Env, entry: &QueueEntry, base: &str) -> Result<Stopped, Stopped> {
    let pr = entry.pr.to_string();
    let view = gh_view(&env.gh, repo, &pr, "state,mergeCommit");
    let commit = view
        .as_ref()
        .and_then(|v| v.merge_commit.as_ref())
        .map(|m| m.oid.clone());
    let merged = view.as_ref().and_then(|v| v.state.as_deref()) == Some("MERGED");
    let Some(commit) = commit.filter(|_| merged) else {
        return Ok(Stopped::stopped(format!(
            "#{} did not read as merged afterwards: {}",
            entry.pr,
            view.and_then(|v| v.state)
                .unwrap_or_else(|| "no answer".to_string())
        )));
    };

    checked(repo, &["fetch", "--quiet", &env.remote, &env.base])?;
    let first_parent = rev_parse(repo, &format!("{commit}^1"))?;

    let cleanup = if entry.worktree.is_empty() {
        Vec::new()
    } else {
        vec![
            "once `git status --porcelain` there prints nothing (agent-worktrees):".to_string(),
            format!(
                "git worktree remove --force {}",
                shell_quote(&entry.worktree)
            ),
            format!("git branch -D {}", shell_quote(&entry.branch)),
        ]
    };

    if first_parent != base {
        return Ok(Stopped::of(
            OutcomeState::Ungated,
            format!(
                "#{} merged as {}, but its first parent is {}, not the {} it was gated against: \
                 an ungated combination landed on {}. Say so to the owner.",
                entry.pr,
                short(&commit),
                short(&first_parent),
                short(base),
                env.base
            ),
            OutcomePatch {
                merge_commit: Some(commit),
                cleanup: Some(cleanup),
                ..OutcomePatch::default()
            },
        ));
    }

    let _ = best_effort(
        repo,
        &["push", "--quiet", &env.remote, "--delete", &entry.branch],
    );
    Ok(Stopped::of(
        OutcomeState::Landed,
        format!(
            "#{} merged as {} onto {}, the base it was gated against",
            entry.pr,
            short(&commit),
            short(base)
        ),
        OutcomePatch {
            merge_commit: Some(commit),
            cleanup: Some(cleanup),
            ..OutcomePatch::default()
        },
    ))
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}

/// `shlex.quote`, near enough: a token made only of the characters a shell
/// never treats specially is printed bare, and anything else is
/// single-quoted with its own single quotes escaped.
fn shell_quote(value: &str) -> String {
    let plain = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c));
    if plain {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
