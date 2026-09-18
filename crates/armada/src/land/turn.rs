//! One branch's turn: read its pull request, gate it, merge it, prove it —
//! retried up to [`ROUNDS`] times against a base that keeps moving.
//! `scripts/land`'s own `take_turn`.

use std::path::Path;

use super::dir::StateDir;
use super::env::{Env, ROUNDS};
use super::gating::gate;
use super::git::checked;
use super::outcome::OutcomePatch;
use super::outcome::OutcomeState;
use super::queue::QueueEntry;
use super::repo::{is_ancestor, remote_head};
use super::say::say;
use super::shell::{gh_view, run};
use super::stop::Stopped;

pub fn take_turn(repo: &Path, state: &StateDir, env: &Env, entry: &QueueEntry) -> Stopped {
    let branch = entry.branch.as_str();
    let logs = state.path().join("logs").join(super::dir::key(branch));
    let _ = std::fs::remove_dir_all(&logs);
    if let Err(why) = std::fs::create_dir_all(&logs) {
        return Stopped::stopped(format!("{} could not be created: {why}", logs.display()));
    }

    if let Err(stopped) = say(
        state,
        branch,
        OutcomeState::Gating,
        "reading the pull request",
        OutcomePatch {
            logs: Some(Vec::new()),
            failed: Some(Vec::new()),
            already: Some(Vec::new()),
            new_lines: Some(Vec::new()),
            conflicts: Some(Vec::new()),
            ..OutcomePatch::default()
        },
    ) {
        return stopped;
    }
    let mut gated_base = super::outcome::read_outcome(state, branch)
        .ok()
        .flatten()
        .and_then(|outcome| outcome.gated_base);
    let mut pushed = super::outcome::read_outcome(state, branch)
        .ok()
        .flatten()
        .and_then(|outcome| outcome.pushed);

    for _ in 0..ROUNDS {
        let pr = entry.pr.to_string();
        let view = gh_view(
            &env.gh,
            repo,
            &pr,
            "state,baseRefName,headRefOid,mergeCommit",
        );
        let Some(view) = view else {
            return Stopped::stopped(format!(
                "`{} pr view {}` answered nothing",
                env.gh, entry.pr
            ));
        };
        if view.state.as_deref() == Some("MERGED") {
            if let Some(base) = gated_base {
                return super::prove::prove(repo, state, env, entry, &base);
            }
        }
        if view.state.as_deref() != Some("OPEN") {
            return Stopped::stopped(format!(
                "pull request #{} is {}",
                entry.pr,
                view.state.unwrap_or_default()
            ));
        }
        let their_base = view.base_ref_name.unwrap_or_default();
        if their_base != env.base {
            return Stopped::stopped(format!(
                "#{} is against {their_base} now, not {} — nothing here gates that base",
                entry.pr, env.base
            ));
        }

        let head = match remote_head(repo, &env.remote, branch) {
            Ok(head) => head,
            Err(why) => return Stopped::stopped(why.to_string()),
        };
        if head.as_deref() != Some(entry.head.as_str()) && head != pushed {
            return Stopped::stopped(format!(
                "{branch} moved on {} since it was queued — preflight and land again",
                env.remote
            ));
        }
        if let Err(why) = checked(
            repo,
            &[
                "fetch",
                "--quiet",
                &env.remote,
                &env.base,
                &format!("refs/heads/{branch}"),
            ],
        ) {
            return why.into();
        }
        let base = match remote_head(repo, &env.remote, &env.base) {
            Ok(Some(base)) => base,
            Ok(None) => {
                return Stopped::stopped(format!("{}/{} does not exist", env.remote, env.base))
            }
            Err(why) => return Stopped::stopped(why.to_string()),
        };
        let head = match head {
            Some(head) => head,
            None => return Stopped::stopped(format!("{branch} does not exist on {}", env.remote)),
        };

        let moved = !is_ancestor(repo, &base, &head);
        let candidate = match gate(repo, state, env, entry, &head, &base, &logs, moved) {
            Ok(candidate) => candidate,
            Err(stopped) => return stopped,
        };
        gated_base = Some(base.clone());
        pushed = Some(candidate.clone());

        if let Err(stopped) = say(
            state,
            branch,
            OutcomeState::Merging,
            format!(
                "merging #{} at {} onto {}",
                entry.pr,
                short(&candidate),
                short(&base)
            ),
            OutcomePatch {
                gated_base: Some(base.clone()),
                candidate: Some(candidate.clone()),
                ..OutcomePatch::default()
            },
        ) {
            return stopped;
        }

        match remote_head(repo, &env.remote, &env.base) {
            Ok(Some(now)) if now == base => {}
            _ => continue,
        }

        let log = logs.join("merge.log");
        let merged = run(
            &[
                env.gh.as_str(),
                "pr",
                "merge",
                &pr,
                "--merge",
                "--match-head-commit",
                &candidate,
            ],
            repo,
            None,
            Some(&log),
        );
        match merged {
            Ok(merged) if !merged.success() => {
                let after = gh_view(&env.gh, repo, &pr, "state");
                if after.and_then(|v| v.state).as_deref() != Some("MERGED") {
                    let said = if !merged.stderr().trim().is_empty() {
                        merged.stderr()
                    } else {
                        merged.stdout()
                    };
                    return Stopped::stopped(format!(
                        "the forge refused the merge: {}",
                        said.trim()
                    ));
                }
            }
            Err(stopped) => return stopped,
            _ => {}
        }
        return super::prove::prove(repo, state, env, entry, &base);
    }
    Stopped::stopped(format!(
        "{} moved during each of {ROUNDS} gates; land again when it is quieter",
        env.base
    ))
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}
