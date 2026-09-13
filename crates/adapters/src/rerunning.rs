//! Starting a pull request's failed CI runs again, on a person's press. #905.
//!
//! **The one write to the forge besides a merge**, and taken the same way: only from a
//! press. Nothing is posted: `gh run rerun --failed` starts the runs and writes nothing on
//! the pull request.

use std::collections::BTreeSet;

use adapter_traits::{NotRerun, Rerun};

use crate::delivery::{run_in, said};

/// Ask the forge which checks failed, and start each failed run's failed jobs again.
pub(crate) fn rerun_failed(in_repo: &str, pull_request: &str) -> Result<Rerun, NotRerun> {
    // `gh pr checks` exits non-zero while any check fails or waits, so its output is read
    // whatever the status says.
    let asked = run_in(
        in_repo,
        "gh",
        &[
            "pr",
            "checks",
            pull_request,
            "--json",
            "bucket,link",
            "--jq",
            ".[] | select(.bucket == \"fail\") | .link",
        ],
    )
    .map_err(|why| NotRerun {
        said: why.to_string(),
    })?;
    let runs = run_ids(&String::from_utf8_lossy(&asked.stdout));
    if runs.is_empty() {
        return Err(NotRerun {
            said: match asked.stdout.is_empty() {
                true => said(&asked),
                false => "no failed check names a run the forge can start again".to_string(),
            },
        });
    }
    for id in &runs {
        let rerun =
            run_in(in_repo, "gh", &["run", "rerun", id.as_str(), "--failed"]).map_err(|why| {
                NotRerun {
                    said: why.to_string(),
                }
            })?;
        if !rerun.status.success() {
            return Err(NotRerun { said: said(&rerun) });
        }
    }
    Ok(Rerun { runs: runs.len() })
}

/// The run ids in check links shaped `.../actions/runs/<id>/job/<n>`, each once.
pub(crate) fn run_ids(links: &str) -> BTreeSet<String> {
    links
        .lines()
        .filter_map(|link| {
            let rest = link.split("/actions/runs/").nth(1)?;
            let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
            (!id.is_empty()).then_some(id)
        })
        .collect()
}
