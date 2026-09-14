//! Filing an issue on the forge from a review finding, on a person's confirm. #906.
//!
//! **Written as the person**, so it runs only from their confirm of a draft they could edit.
//! Nothing is posted on the pull request.

use adapter_traits::{FiledIssue, NotFiled};

use crate::delivery::{run_in, said};

/// File the issue in the repository the Job serves, and name where it landed.
pub(crate) fn file_issue(in_repo: &str, title: &str, body: &str) -> Result<FiledIssue, NotFiled> {
    let filed = run_in(
        in_repo,
        "gh",
        &["issue", "create", "--title", title, "--body", body],
    )
    .map_err(|why| NotFiled {
        said: why.to_string(),
    })?;
    if !filed.status.success() {
        return Err(NotFiled { said: said(&filed) });
    }
    issue_url(&String::from_utf8_lossy(&filed.stdout))
        .map(|url| FiledIssue { url })
        .ok_or_else(|| NotFiled {
            said: "the forge filed the issue but printed no address for it".to_string(),
        })
}

/// The issue's address, which `gh issue create` prints as its last line.
pub(crate) fn issue_url(printed: &str) -> Option<String> {
    printed
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with("https://") && line.contains("/issues/"))
        .map(str::to_string)
}
