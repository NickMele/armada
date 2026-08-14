//! Path globs, for `match:` (PLAN.md §4.1).
//!
//! **Hand-rolled rather than a crate**, for the reason `adapters::clock`
//! hand-rolls one timestamp format: char needs one operation — does this
//! workspace-relative path match this pattern — and a dependency for it is
//! something to review, audit and upgrade for the life of the project in
//! exchange for sixty lines. It is also the operation `config verify` and the
//! path selector both lean on, so having it be readable matters more than
//! having it be general.
//!
//! **Deliberately not a filesystem walk.** Every path this matches is one git
//! already told char about (PLAN.md §4.1's `${files}`) or one the caller typed.
//! Nothing here opens a directory, which is what keeps it in the pure core.
//!
//! The grammar, and nothing else:
//!
//! | | |
//! |---|---|
//! | `**` | zero or more whole path segments |
//! | `*` | any run of characters inside one segment, `/` excluded |
//! | `?` | one character, `/` excluded |
//! | anything else | itself |
//!
//! There is no brace expansion, no character class and no negation. **`match:`
//! has no negation and that is load-bearing**: PLAN.md §4.1.1 records an attempt
//! to subtract a declared nested workspace from a defaulted glob set, which is
//! not expressible without inventing syntax — and the conclusion was that the
//! premise was wrong rather than that the syntax was missing.

/// Whether a workspace-relative path matches a pattern.
pub fn matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    match_segments(&pattern, &path)
}

/// Whether a path matches any of a set of patterns.
///
/// An empty set matches nothing, which is the answer a component with no
/// `checks:` needs: `match:` exists to scope checks, and a run-only component
/// has nothing to scope, so it gets no globs and no path can select it.
pub fn matches_any(patterns: &[String], path: &str) -> bool {
    patterns.iter().any(|pattern| matches(pattern, path))
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        // The pattern is exhausted: it matches only if the path is too.
        None => path.is_empty(),
        Some((&"**", rest)) => {
            // Zero or more segments. Zero first, so `a/**` matches `a` as well
            // as `a/b/c` — a component rooted at `a` owns the directory itself,
            // and a caller who types `char check a` is not asking a different
            // question from one who types `char check a/b`.
            (0..=path.len()).any(|skip| match_segments(rest, &path[skip..]))
        }
        Some((first, rest)) => match path.split_first() {
            Some((segment, tail)) if match_segment(first, segment) => match_segments(rest, tail),
            Some(_) | None => false,
        },
    }
}

/// One segment against one pattern segment, with `*` and `?`.
fn match_segment(pattern: &str, segment: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let segment: Vec<char> = segment.chars().collect();
    let (mut p, mut s) = (0usize, 0usize);
    // Where to resume if a `*` turns out to have consumed too little. Linear in
    // the common case and quadratic only when a segment is nothing but stars,
    // which is not a shape any config in this project writes.
    let (mut star, mut resume) = (None, 0usize);

    while s < segment.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == segment[s]) {
            p += 1;
            s += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            resume = s;
            p += 1;
        } else if let Some(at) = star {
            p = at + 1;
            resume += 1;
            s = resume;
        } else {
            return false;
        }
    }

    pattern[p..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_path_matches_itself_and_nothing_else() {
        assert!(matches("services/api/views.py", "services/api/views.py"));
        assert!(!matches("services/api/views.py", "services/api/models.py"));
        assert!(!matches("services/api", "services/api/views.py"));
    }

    /// The default `match:` when a component declares `root:` — the one every
    /// fixture with a root relies on.
    #[test]
    fn a_double_star_matches_every_depth_below_a_root() {
        for path in [
            "services/api",
            "services/api/views.py",
            "services/api/a/b/c/d.py",
        ] {
            assert!(matches("services/api/**", path), "{path}");
        }
        assert!(!matches("services/api/**", "services/worker/main.go"));
        assert!(!matches("services/api/**", "services"));
    }

    /// The default `match:` when a component declares no `root:` — the
    /// component *is* the repo.
    #[test]
    fn a_bare_double_star_matches_everything() {
        for path in ["README.md", "a/b.py", "a/b/c/d/e.rs"] {
            assert!(matches("**", path), "{path}");
        }
        // The paired negative, without which this holds over a matcher that
        // says yes to everything.
        assert!(!matches("client/**", "README.md"));
    }

    /// A single star stays inside one segment, which is what stops
    /// `client/*` from claiming `client/a/b.ts`.
    #[test]
    fn a_single_star_does_not_cross_a_separator() {
        assert!(matches("client/*.ts", "client/index.ts"));
        assert!(!matches("client/*.ts", "client/src/index.ts"));
        assert!(matches("*.md", "README.md"));
        assert!(!matches("*.md", "docs/README.md"));
    }

    #[test]
    fn a_question_mark_is_one_character_and_never_a_separator() {
        assert!(matches("a?c", "abc"));
        assert!(!matches("a?c", "ac"));
        assert!(!matches("a?c", "a/c"));
    }

    /// `docs/**/*.md` is the shape a repo writes to mean "markdown anywhere
    /// under docs", including directly under it — which needs `**` to match
    /// zero segments.
    #[test]
    fn a_double_star_in_the_middle_matches_zero_segments_as_well_as_many() {
        assert!(matches("docs/**/*.md", "docs/README.md"));
        assert!(matches("docs/**/*.md", "docs/a/b/README.md"));
        assert!(!matches("docs/**/*.md", "docs/README.txt"));
        assert!(!matches("docs/**/*.md", "src/README.md"));
    }

    #[test]
    fn several_stars_in_one_segment_do_not_run_away() {
        assert!(matches("a*b*c", "aXbYc"));
        assert!(matches("a*b*c", "abc"));
        assert!(!matches("a*b*c", "aXbY"));
        assert!(matches("***", "anything"));
    }

    /// A component with no `checks:` gets no globs, and no path may select it.
    #[test]
    fn an_empty_pattern_set_matches_nothing() {
        assert!(!matches_any(&[], "anything.py"));
        assert!(matches_any(&["**".to_string()], "anything.py"));
    }

    #[test]
    fn matching_any_takes_the_first_pattern_that_fits() {
        let patterns = vec!["client/**".to_string(), "*.md".to_string()];
        assert!(matches_any(&patterns, "client/src/index.ts"));
        assert!(matches_any(&patterns, "README.md"));
        assert!(!matches_any(&patterns, "services/api/views.py"));
    }

    /// A filename may contain almost anything on POSIX, and `${files}` comes
    /// from outside the trust boundary — so the matcher has to be total rather
    /// than merely correct on tidy input.
    #[test]
    fn an_awkward_filename_is_matched_rather_than_panicked_on() {
        for path in [
            "sub/semi;echo INJECTED.py",
            "sub/dollar$(id).py",
            "a file with spaces.py",
            "",
            "trailing/",
            "café/naïve.py",
        ] {
            let _ = matches("**", path);
            let _ = matches("sub/*", path);
            let _ = matches_any(&["**/*.py".to_string()], path);
        }
        assert!(matches("sub/*", "sub/semi;echo INJECTED.py"));
        assert!(matches("**/*.py", "café/naïve.py"));
        assert!(!matches("sub/*", "other/semi;echo INJECTED.py"));
        assert!(!matches("**/*.py", "café/naïve.rs"));
    }
}
