//! Resolving a bare issue link, for the one forge this workspace already
//! assumes elsewhere.
//!
//! # Why this exists
//!
//! A Drone's sandbox has no network egress, so a request that is nothing but
//! a link leaves it nothing to work from — it cannot fetch the link itself.
//! Fleet's own process is unsandboxed and already has a network; it simply
//! never used it before a request reached a Drone. [`IssueLookup`] is
//! that use: it recognises the one link shape the Job Board's `BRIEF` field
//! shows today and renders the CLI call that would read it, so `fleet` can
//! run that call before the request becomes a Job's `facts`.
//!
//! # `--jq` does the parsing, not this crate
//!
//! The rendered call asks `gh` for `title,body` and reduces them to plain text
//! with its own `--jq` filter. Nothing here parses a byte of the answer — it
//! only reads whatever comes back on stdout, which is text already, not JSON.
//! That keeps this crate off `serde_json`'s allowlist by never needing it.

use adapter_traits::{LinkLookup, LookupCall};

/// One shape only: `github.com/<owner>/<repo>/issues/<number>`, with or
/// without a scheme in front and whatever query or fragment trails the
/// number. Nothing else — a pull request link, a comment anchor, or a second
/// link in the same request — is this milestone's to chase; see the issues
/// tracking a general resolver.
pub struct IssueLookup;

impl LinkLookup for IssueLookup {
    fn resolve(&self, request: &str) -> Option<LookupCall> {
        let (owner, repo, number) = issue_reference(request)?;
        Some(LookupCall::rendered(
            "gh",
            vec![
                "issue".into(),
                "view".into(),
                number,
                "--repo".into(),
                format!("{owner}/{repo}"),
                "--json".into(),
                "title,body".into(),
                "--jq".into(),
                r#".title + "\n\n" + .body"#.into(),
            ],
        ))
    }
}

/// The owner, the repository and the issue number, if `request` names one.
///
/// Scans for the host literally rather than parsing a URL: the shape this
/// looks for is narrow enough that a `/`-split answers it, and a full URL
/// parser would be a dependency bought for one path segment.
fn issue_reference(request: &str) -> Option<(String, String, String)> {
    const HOST: &str = "github.com/";
    let after_host = &request[request.find(HOST)? + HOST.len()..];
    let mut segments = after_host.split(['/', '?', '#', ' ', '\n']);
    let owner = segments.next()?;
    let repo = segments.next()?;
    let marker = segments.next()?;
    let number = segments.next()?;
    if marker != "issues" || owner.is_empty() || repo.is_empty() {
        return None;
    }
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((owner.to_string(), repo.to_string(), number.to_string()))
}
