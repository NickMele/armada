//! Which paths a Check covers, and the one glob dialect that says so.
//!
//! `packages/**` and `packages/*` meaning the same thing quietly is a Check
//! that stops running and nobody notices. So the dialect is small enough to
//! state in full, and `docs/contracts/configuration.md` states it:
//!
//! | Written | Matches |
//! |---|---|
//! | literal text | itself, exactly |
//! | `*` | any run of characters inside one segment, **never** a `/` |
//! | `**` | zero or more whole segments |
//!
//! Segments split on `/`, patterns are repository-relative, a path is matched
//! whole, and `**` is legal only as an entire segment.
//!
//! **Every other glob metacharacter is refused by name**, because a `?` read
//! as the letter `?` is a pattern that matches nothing and says nothing about
//! why. [`BadPattern`] names which character it was.
//!
//! **Absent means always, and it is not spelled here.** A Check declaring no
//! `when` carries no [`Covers`] at all, and [`Covers::of`] refuses an empty
//! list — so "always" and "never" cannot collide in one value.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Why a pattern is not one this dialect can read.
///
/// **Named per character**, so the message can say which dialect the author
/// was probably writing in rather than reporting that the pattern did not
/// match anything at run time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BadPattern {
    /// Nothing, or only whitespace.
    Empty,
    /// A leading `/`. Patterns are repository-relative and there is no second
    /// spelling of the root.
    Absolute,
    /// A trailing `/`. A pattern names paths, and a directory is named by what
    /// is under it — `packages/**`.
    TrailingSlash,
    /// An empty segment, from `a//b`.
    EmptySegment,
    /// `**` sharing a segment with anything else, as in `packages/**x`. It
    /// stands for whole segments, so it is a whole segment or it is a typo.
    StarStarNotAlone { segment: String },
    /// Three or more stars in one run.
    TooManyStars { segment: String },
    /// A metacharacter of some other dialect. Refused rather than matched
    /// literally: a `?` read as the letter `?` is a pattern that matches
    /// nothing and says nothing about why.
    Unsupported { found: char },
}

impl core::fmt::Display for BadPattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BadPattern::Empty => f.write_str("it is empty"),
            BadPattern::Absolute => {
                f.write_str("it starts with `/`, and a pattern is repository-relative")
            }
            BadPattern::TrailingSlash => f.write_str(
                "it ends with `/`; name what is under a directory instead, as in `packages/**`",
            ),
            BadPattern::EmptySegment => f.write_str("it holds an empty path segment"),
            BadPattern::StarStarNotAlone { segment } => write!(
                f,
                "`{segment}` mixes `**` with other text, and `**` stands for whole segments"
            ),
            BadPattern::TooManyStars { segment } => {
                write!(f, "`{segment}` holds more than two stars in a row")
            }
            BadPattern::Unsupported { found } => write!(
                f,
                "`{found}` is not in this dialect, which is literal text, `*` and `**`"
            ),
        }
    }
}

/// One segment of a parsed pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment {
    /// `**` — zero or more whole segments.
    AnyDepth,
    /// Literal text, `*`, or a mix of the two. Held as the written text and
    /// matched by [`one_segment`], which is where `*` stops at a `/` by never
    /// being shown one.
    Named(String),
}

/// A path pattern in Armada's dialect, already checked.
///
/// **Holding one is proof it parsed.** There is no constructor taking a bare
/// string and no field to set afterwards, so nothing downstream can match
/// against a pattern nobody read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathPattern {
    written: String,
    segments: Vec<Segment>,
}

impl PathPattern {
    /// Read a pattern, or say what is wrong with it.
    pub fn parse(written: &str) -> Result<PathPattern, BadPattern> {
        let trimmed = written.trim();
        if trimmed.is_empty() {
            return Err(BadPattern::Empty);
        }
        if trimmed.starts_with('/') {
            return Err(BadPattern::Absolute);
        }
        if trimmed.ends_with('/') {
            return Err(BadPattern::TrailingSlash);
        }
        let mut segments = Vec::new();
        for segment in trimmed.split('/') {
            segments.push(read_segment(segment)?);
        }
        Ok(PathPattern {
            written: trimmed.to_string(),
            segments,
        })
    }

    /// The pattern as the author wrote it, trimmed. What a message quotes.
    pub fn as_str(&self) -> &str {
        &self.written
    }

    /// Whether this pattern covers one repository-relative path.
    ///
    /// **The path is matched whole**, and what happened to the file is not a
    /// parameter: a deletion, a rename and an edit are all a change to the
    /// path they name. See [`Covers::matches_any`].
    pub fn matches(&self, path: &str) -> bool {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        walk(&self.segments, &parts)
    }
}

/// Which paths a Check covers. **Never empty.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Covers {
    patterns: Vec<PathPattern>,
}

impl Covers {
    /// Every pattern a Check declared. `None` where the list is empty, which
    /// is a Check that could never run.
    pub fn of(patterns: Vec<PathPattern>) -> Option<Covers> {
        match patterns.is_empty() {
            true => None,
            false => Some(Covers { patterns }),
        }
    }

    pub fn patterns(&self) -> &[PathPattern] {
        &self.patterns
    }

    /// Whether any changed path is covered by any pattern.
    ///
    /// **The kind of change is not read, and that is the decision.** A file
    /// removed from `packages/` is a change to `packages/`, and a rename
    /// arrives from `WorkProduct::changed_files` as two paths — the old one
    /// deleted and the new one added, because rename detection is off in the
    /// git adapter — so either side of a rename covers the Check on its own.
    pub fn matches_any(&self, changed: &[String]) -> bool {
        changed
            .iter()
            .any(|path| self.patterns.iter().any(|pattern| pattern.matches(path)))
    }

    /// The patterns, comma-separated, for the sentence a skipped Check
    /// records.
    pub fn written(&self) -> String {
        let mut out = String::new();
        for (n, pattern) in self.patterns.iter().enumerate() {
            if n > 0 {
                out.push_str(", ");
            }
            out.push_str(pattern.as_str());
        }
        out
    }
}

/// One segment of the written pattern, checked against the dialect.
fn read_segment(segment: &str) -> Result<Segment, BadPattern> {
    if segment.is_empty() {
        return Err(BadPattern::EmptySegment);
    }
    for found in segment.chars() {
        if matches!(found, '?' | '[' | ']' | '{' | '}' | '!' | '\\') {
            return Err(BadPattern::Unsupported { found });
        }
    }
    if segment == "**" {
        return Ok(Segment::AnyDepth);
    }
    if segment.contains("***") {
        return Err(BadPattern::TooManyStars {
            segment: segment.to_string(),
        });
    }
    if segment.contains("**") {
        return Err(BadPattern::StarStarNotAlone {
            segment: segment.to_string(),
        });
    }
    Ok(Segment::Named(segment.to_string()))
}

/// Match a segment list against a path's segments.
///
/// Recursive on `**` alone, which is the only branch: every other segment
/// consumes exactly one path segment, so the walk is linear except where a
/// pattern says it does not know how deep to go.
fn walk(segments: &[Segment], parts: &[&str]) -> bool {
    match segments.split_first() {
        // A pattern that is used up matches a path that is used up. `**` is
        // what makes the second half reachable with parts left over, and it
        // consumes them in its own arm.
        None => parts.is_empty(),
        Some((Segment::AnyDepth, rest)) => {
            // Zero segments first, then one, then two: `packages/**` matches
            // `packages` as well as `packages/a/b.ts`, which is what "zero or
            // more" says and is why the dialect states it in those words.
            (0..=parts.len()).any(|taken| walk(rest, &parts[taken..]))
        }
        Some((Segment::Named(named), rest)) => match parts.split_first() {
            None => false,
            Some((part, tail)) => one_segment(named, part) && walk(rest, tail),
        },
    }
}

/// One segment's text against one path segment. `*` matches any run of
/// characters here and cannot cross a `/`, because it is never handed one.
fn one_segment(named: &str, part: &str) -> bool {
    match named.split_once('*') {
        None => named == part,
        Some((before, after)) => {
            let Some(rest) = part.strip_prefix(before) else {
                return false;
            };
            // Every split of what is left, so a second `*` in the same segment
            // gets its own chance. `*` may match nothing.
            (0..=rest.len())
                .any(|taken| rest.is_char_boundary(taken) && one_segment(after, &rest[taken..]))
        }
    }
}
