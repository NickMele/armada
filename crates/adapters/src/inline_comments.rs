//! The comments left on individual lines of a pull request's diff.
//!
//! # A second query, on purpose
//!
//! `crate::under_review` answers `reviewDecision`, `statusCheckRollup`,
//! `comments` and `reviews` in one call, because that is what a sweep asked
//! about every open pull request can afford. Inline comments are a different
//! endpoint — the REST review-comments list, not `gh pr view` — and this
//! module is deliberately the only place that calls it, so that budget stays
//! visible as one file rather than as a call site nobody counted.
//!
//! # No parse, same as `under_review`
//!
//! `--jq` does the reading and `--paginate` walks every page, so what reaches
//! this process is one line of `@tsv` per comment; `store` and `ipc` stay the
//! only two crates that deserialise anything.

use adapter_traits::Remark;

use crate::delivery::{run_in, FORGE};
use crate::under_review::as_written;

/// The fields one inline comment is read for, reduced to one `@tsv` line each.
///
/// **A comment with no body is dropped**, for `crate::under_review`'s own
/// reduction's reason: it is not something to offer a person to pick or hand a
/// Drone.
///
/// **`.line`, falling back to `.original_line`.** A comment on a diff that has
/// since moved answers `line: null` and carries where it *was* under
/// `original_line` instead; either is a real position on the forge's own diff
/// and this reduction does not care which.
const REDUCTION: &str = "\
    map(select((.body // \"\") != \"\")) | .[] | \
    [(.id | tostring), (.user.login // \"\"), (.created_at // \"\"), (.body // \"\"), \
     (.html_url // \"\"), (.path // \"\"), ((.line // .original_line // 0) | tostring), \
     (.diff_hunk // \"\")] \
    | @tsv";

/// Ask the forge for every inline comment on one pull request.
///
/// **Empty on any failure**, [`adapter_traits::Delivery::inline_remarks`]'s
/// rule: no tool, not signed in, a pull request whose address does not carry a
/// number, a forge that would not answer — none of them is told apart from
/// "nothing more to add" here, because the caller already has the comments
/// `crate::under_review::read` found and this is strictly additional to them.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> Vec<Remark> {
    let Some(number) = number_of(pull_request) else {
        return Vec::new();
    };
    let path = format!("repos/{{owner}}/{{repo}}/pulls/{number}/comments");
    let Ok(run) = run_in(
        in_repo,
        FORGE,
        &["api", "--paginate", &path, "--jq", REDUCTION],
    ) else {
        return Vec::new();
    };
    if !run.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&run.stdout).into_owned();
    folded(&text)
}

/// The lines `read` printed, folded into `Remark`s. Split out so a test can
/// hand it what `gh api --jq` would print, without a forge, an account or a
/// network — `crate::under_review::folded`'s reason exactly.
pub(crate) fn folded(lines: &str) -> Vec<Remark> {
    lines
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(parsed)
        .collect()
}

/// One `@tsv` line, or `None` where the forge named no comment at all — the
/// same guard `crate::under_review::folded` keeps on the same field, for the
/// same reason: a comment with no id cannot be picked without the press
/// meaning some other one.
fn parsed(line: &str) -> Option<Remark> {
    let mut field = line.split('\t');
    let id = field.next().unwrap_or_default();
    let by = field.next().unwrap_or_default();
    let at = field.next().unwrap_or_default();
    let said = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let path = field.next().unwrap_or_default();
    let line_no: u32 = field.next().unwrap_or_default().parse().unwrap_or(0);
    let hunk = field.next().unwrap_or_default();
    if id.is_empty() {
        return None;
    }
    let mut remark = Remark::written(
        as_written(id),
        as_written(by),
        as_written(at),
        as_written(said),
    );
    if !url.is_empty() {
        remark = remark.with_url(as_written(url));
    }
    if !path.is_empty() {
        remark = remark.with_inline(as_written(path), line_no, as_written(hunk));
    }
    Some(remark)
}

/// The trailing digits of a pull request's address, which is where the
/// forge's own number for it sits — `.../pull/<number>`.
///
/// **`None` rather than a guess.** `gh api`'s path needs the number and
/// nothing else here parses this address at all; a value this cannot read is
/// answered the way every other unreadable forge answer is, by the caller
/// finding nothing more to add.
fn number_of(pull_request: &str) -> Option<&str> {
    let tail = pull_request.rsplit('/').next()?;
    (!tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())).then_some(tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pull_requests_number_is_the_trailing_digits() {
        assert_eq!(
            number_of("https://forge.invalid/armada/armada/pull/42"),
            Some("42")
        );
        assert_eq!(number_of("https://forge.invalid/armada/armada"), None);
        assert_eq!(number_of(""), None);
    }

    #[test]
    fn a_line_with_a_path_and_a_hunk_becomes_an_inline_remark() {
        let line = "PRRC_1\ta-reviewer\t2026-09-08T10:00:00Z\tthis leaks a file handle\thttps://forge.invalid/pull/1#discussion_r1\tsrc/log.rs\t42\t@@ -40,3 +40,3 @@ fn read() {";
        let remarks = folded(line);
        assert_eq!(remarks.len(), 1);
        let remark = &remarks[0];
        assert_eq!(remark.id.as_written(), "PRRC_1");
        assert_eq!(
            remark.url.as_ref().map(|url| url.as_written()),
            Some("https://forge.invalid/pull/1#discussion_r1")
        );
        let inline = remark.inline.as_ref().expect("this comment is inline");
        assert_eq!(inline.path.as_written(), "src/log.rs");
        assert_eq!(inline.line, 42);
        assert_eq!(inline.hunk.as_written(), "@@ -40,3 +40,3 @@ fn read() {");
    }

    #[test]
    fn a_comment_the_forge_named_nothing_is_dropped() {
        let line = "\ta-reviewer\t2026-09-08T10:00:00Z\tsaid\t\t\t0\t";
        assert!(folded(line).is_empty());
    }

    #[test]
    fn a_line_with_no_path_carries_no_inline_context() {
        let line = "PRRC_2\tsomebody\t2026-09-08T10:00:00Z\tsaid\t\t\t0\t";
        let remarks = folded(line);
        assert_eq!(remarks.len(), 1);
        assert!(remarks[0].inline.is_none());
    }
}
