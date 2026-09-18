//! `armada land` — join the merge line, and return at once.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use fleet::clock::{Clock, SystemClock};

use super::codec;
use super::dir::StateDir;
use super::env::Env;
use super::outcome::{read_outcome, Outcome, OutcomeState, Place};
use super::preflight::ready_tree;
use super::queue::{nonce, queued, read_queue_entry, write_queue_entry, QueueEntry};
use super::repo::{common_git_dir, rev_parse};
use super::stamp::read_stamp;
use super::stop::Refused;

pub struct Queued {
    pub branch: String,
    pub pull_request: u64,
    pub ahead: usize,
}

pub fn land(cwd: &Path, env: &Env) -> Result<Queued, Refused> {
    let state = StateDir::resolve(cwd).map_err(|why| Refused(why.to_string()))?;
    if let Some((name, value)) = env.relative_binaries() {
        return Err(Refused(format!(
            "{name}={value} is relative, and the gate runs in another directory — give an absolute path"
        )));
    }
    let (branch, head, tree) = ready_tree(cwd, env)?;

    let stamp = read_stamp(&state, &branch).map_err(|why| Refused(why.to_string()))?;
    let stamp = match stamp {
        Some(stamp) if stamp.tree == tree && stamp.head == head => stamp,
        _ => {
            return Err(Refused(
                "this tree has no preflight stamp — `armada land preflight` first".to_string(),
            ))
        }
    };

    let held = read_queue_entry(&state, &branch).map_err(|why| Refused(why.to_string()))?;
    let before = read_outcome(&state, &branch).map_err(|why| Refused(why.to_string()))?;
    let place: Place = match (&held, &before) {
        (Some(held), _) => held.place,
        (None, Some(before))
            if matches!(before.state, OutcomeState::Conflict | OutcomeState::Red)
                && before.place.is_some() =>
        {
            before.place.expect("checked above")
        }
        _ => fresh_place(),
    };

    let top = rev_parse(cwd, "--show-toplevel")?;
    write_queue_entry(
        &state,
        &QueueEntry {
            branch: branch.clone(),
            pr: stamp.pr,
            head,
            tree,
            place,
            worktree: top,
            nonce: nonce(),
        },
    )
    .map_err(|why| Refused(why.to_string()))?;

    // A fresh record, never a merge onto whatever the branch's last turn
    // left — `scripts/land`'s own `land()` writes this with `write_json`,
    // not `say()`, so a resubmit after a conflict starts this turn's outcome
    // clean apart from the place it kept.
    codec::write(
        &state.outcome_path(&branch),
        &Outcome {
            branch: branch.clone(),
            state: OutcomeState::Waiting,
            detail: "in line".to_string(),
            updated: SystemClock::new().now().as_str().to_string(),
            runner: std::process::id(),
            pr: Some(stamp.pr),
            place: Some(place),
            logs: Vec::new(),
            failed: Vec::new(),
            already: Vec::new(),
            new_lines: Vec::new(),
            conflicts: Vec::new(),
            pushed: None,
            gated_base: None,
            candidate: None,
            merge_commit: None,
            cleanup: Vec::new(),
        },
    )
    .map_err(|why| Refused(why.to_string()))?;

    let exe = std::env::current_exe()
        .map_err(|why| Refused(format!("this binary's own path could not be read: {why}")))?;
    let common = common_git_dir(cwd)?;
    super::runner::ensure_runner(&exe, &state, &common).map_err(|why| Refused(why.to_string()))?;

    let ahead = queued(&state)
        .map_err(|why| Refused(why.to_string()))?
        .into_iter()
        .filter(|entry| entry.place < place)
        .count();

    Ok(Queued {
        branch,
        pull_request: stamp.pr,
        ahead,
    })
}

fn fresh_place() -> Place {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos() as Place)
        .unwrap_or(0)
}
