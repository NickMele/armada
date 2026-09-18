//! `armada land preflight` — ready a branch to join the line, and stamp the
//! tree it was measured on.

use std::path::Path;

use super::armada_cli::covers;
use super::dir::StateDir;
use super::env::Env;
use super::git::checked;
use super::repo::{changed_paths, current_branch, is_ancestor, merge_base, remote_head, rev_parse};
use super::shell::gh_view;
use super::stamp::{write_stamp, PreflightStamp};
use super::stop::Refused;

/// What a clean preflight leaves behind, for the CLI to print.
pub struct Preflighted {
    pub branch: String,
    pub pull_request: u64,
    pub tree: String,
    pub checks: Vec<String>,
}

/// The branch, head and tree a stamp or a queue entry may be built from —
/// refused unless every byte is committed and pushed.
pub fn ready_tree(cwd: &Path, env: &Env) -> Result<(String, String, String), Refused> {
    let found = current_branch(cwd);
    let branch = match &found {
        Some(name) if name != &env.base => found.clone().unwrap(),
        _ => {
            return Err(Refused(format!(
                "check out the branch to land; this is {}",
                found.as_deref().unwrap_or("a detached HEAD")
            )));
        }
    };

    let dirty = checked(cwd, &["status", "--porcelain", "--untracked-files=all"])?;
    let dirty = String::from_utf8_lossy(&dirty.stdout).trim().to_string();
    if !dirty.is_empty() {
        return Err(Refused(format!(
            "the tree has uncommitted or untracked files, and a stamp would not \
             describe what lands — commit or remove them:\n{dirty}"
        )));
    }

    let head = rev_parse(cwd, "HEAD")?;
    let pushed = remote_head(cwd, &env.remote, &branch)?;
    if pushed.as_deref() != Some(head.as_str()) {
        let ahead = pushed
            .as_deref()
            .is_some_and(|pushed| is_ancestor(cwd, &head, pushed));
        if ahead {
            return Err(Refused(format!(
                "{remote}/{branch} is ahead of this worktree — a turn merged {base} in and pushed it. \
                 Take it: `git fetch {remote} && git reset --hard {remote}/{branch}`. Never force-push over it.",
                remote = env.remote,
                base = env.base,
            )));
        }
        return Err(Refused(format!(
            "{remote}/{branch} is not this commit — `git push -u {remote} {branch}` first",
            remote = env.remote,
        )));
    }

    let tree = rev_parse(cwd, "HEAD^{tree}")?;
    Ok((branch, head, tree))
}

pub fn preflight(cwd: &Path, env: &Env) -> Result<Preflighted, Refused> {
    let state = StateDir::resolve(cwd).map_err(|why| Refused(why.to_string()))?;
    if let Some((name, value)) = env.relative_binaries() {
        return Err(Refused(format!(
            "{name}={value} is relative, and the gate runs in another directory — give an absolute path"
        )));
    }
    let (branch, head, tree) = ready_tree(cwd, env)?;

    let no_pull_request = || {
        Refused(format!(
            "no pull request for {branch} — `{} pr create` first",
            env.gh
        ))
    };
    let pr = gh_view(&env.gh, cwd, &branch, "number,state,headRefOid,baseRefName")
        .ok_or_else(no_pull_request)?;
    let number = pr.number.ok_or_else(no_pull_request)?;
    let state_word = pr.state.as_deref().unwrap_or("");
    let base_ref = pr.base_ref_name.as_deref().unwrap_or("");
    if state_word != "OPEN" || base_ref != env.base {
        return Err(Refused(format!(
            "pull request #{number} is {state_word} against {base_ref}, not open against {}",
            env.base
        )));
    }
    let their_head = pr.head_ref_oid.as_deref().unwrap_or("");
    if their_head != head {
        return Err(Refused(format!(
            "the forge still shows #{number} at {}; run preflight again in a moment",
            their_head.get(..10).unwrap_or(their_head)
        )));
    }

    checked(cwd, &["fetch", "--quiet", &env.remote, &env.base])?;
    let base = merge_base(cwd, "HEAD", &format!("{}/{}", env.remote, env.base))?;
    let changed = changed_paths(cwd, &base, &head)?;
    let hits = covers(&env.armada, cwd, &changed).map_err(|stopped| Refused(stopped.detail))?;

    write_stamp(
        &state,
        &PreflightStamp {
            branch: branch.clone(),
            head,
            tree: tree.clone(),
            base,
            pr: number,
            checks: hits.clone(),
        },
    )
    .map_err(|why| Refused(why.to_string()))?;

    Ok(Preflighted {
        branch,
        pull_request: number,
        tree,
        checks: hits,
    })
}
