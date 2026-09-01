//! Where in the patch a citation points, for the citations that point.
//!
//! # It is a lookup, never a construction
//!
//! [`in_the_patch`] answers only with a file the patch names and a line the
//! patch holds. There is no path here from a citation to a location the patch
//! does not contain, which is the point: an uncited flag is unactionable and a
//! **wrongly** cited one is worse, because it sends a person to a file
//! believing the finding is in it.
//!
//! # The same normalisation the invention check uses
//!
//! [`quoted::words`](mod@crate::quoted) is what decides whether a quotation is
//! in the material, and it is what decides where. Sharing it is what makes the
//! two agree by construction — a span that passes containment and then
//! resolves to nowhere would be a disagreement between two readings of one
//! diff, and there would be no way to tell which was wrong.
//!
//! # A removed line has a file and no line
//!
//! [`CitedAt`]'s numbering is post-image, so the coordinate this can establish
//! for a `-` line is the file alone. That is the common case rather than the
//! edge: a gaming citation usually quotes what the change took away. See
//! `CitedAt` in `core-model` for why answering with the pre-image number
//! instead was rejected.

use core_model::{CitedAt, RepoPath};

use adapter_traits::Patch;

use crate::quoted;

/// Where the patch holds the first of a citation's quotations that it holds at
/// all. `None` where the citation quotes nothing, quotes too little to be a
/// claim, or quotes something the patch does not have.
///
/// **Quotations are read in the order they were written**, and the first one
/// the patch holds wins. A citation naming two places has to pick one, and the
/// one it led with is the one it was arguing from.
pub(crate) fn in_the_patch(patch: &Patch, cited: &str) -> Option<CitedAt> {
    let spans: Vec<String> = quoted::spans(cited)
        .iter()
        .flat_map(|span| elisions(span))
        .map(|part| quoted::words(&part))
        .filter(|words| words.split_whitespace().count() >= quoted::A_CITATION)
        .collect();
    spans.iter().find_map(|span| holding(patch, span))
}

/// Each side of an elision on its own, for [`quoted::invented`]'s reason: a
/// model quoting across a cut writes `"drops X … and Y"`, and looking for the
/// whole of that finds nothing anywhere.
fn elisions(span: &str) -> Vec<String> {
    span.split('\u{2026}')
        .flat_map(|part| part.split("..."))
        .map(str::to_string)
        .collect()
}

/// The first line of the patch holding `span`, as a location.
///
/// `span` is already normalised by [`quoted::words`], which pads at both ends
/// — so `contains` here is a word-boundary test and not a substring one.
fn holding(patch: &Patch, span: &str) -> Option<CitedAt> {
    let mut file: Option<String> = None;
    // The post-image line the next added or context line will be. Meaningless
    // until a hunk header sets it, and no line is offered before one does.
    let mut post: Option<u32> = None;
    for line in patch.as_str().lines() {
        if let Some(named) = header_path(line) {
            file = Some(named);
            post = None;
            continue;
        }
        if let Some(start) = hunk_start(line) {
            post = Some(start);
            continue;
        }
        let Some(marked) = marked(line) else {
            continue;
        };
        let found = quoted::words(marked.text).contains(span);
        let at = match marked.in_post_image {
            // A removed line. It is a place in the patch and not a place in
            // the file, so the file is the whole of what can be answered.
            false => None,
            true => post,
        };
        if marked.in_post_image {
            post = post.map(|n| n + 1);
        }
        if found {
            let path = RepoPath::new(file?);
            return Some(match at {
                Some(line) => CitedAt::at_line(path, line),
                None => CitedAt::in_file(path),
            });
        }
    }
    None
}

/// One line of a hunk: its text without the marker, and whether the file still
/// has it once the change lands.
struct Marked<'a> {
    text: &'a str,
    in_post_image: bool,
}

/// A line of a hunk, or `None` for anything else the patch carries — the
/// `index` line, a mode line, the `+++`/`---` pair, `\ No newline at end of
/// file`. Reading one of those as content would both mis-count the post image
/// and offer a header as a citation.
fn marked(line: &str) -> Option<Marked<'_>> {
    if line.starts_with("+++") || line.starts_with("---") {
        return None;
    }
    match line.chars().next() {
        Some('+') => Some(Marked {
            text: &line[1..],
            in_post_image: true,
        }),
        Some('-') => Some(Marked {
            text: &line[1..],
            in_post_image: false,
        }),
        // Context: unchanged, in both images, and it advances the count.
        Some(' ') => Some(Marked {
            text: &line[1..],
            in_post_image: true,
        }),
        _ => None,
    }
}

/// The path a `diff --git a/x b/y` header names, taking the post-image side.
pub(crate) fn header_path(line: &str) -> Option<String> {
    let rest = line.strip_prefix("diff --git ")?;
    let (_, after) = rest.rsplit_once(" b/")?;
    Some(after.to_string())
}

/// The post-image line a `@@ -a,b +c,d @@` header opens at.
///
/// The count after the comma is deliberately not read: what is wanted is where
/// counting starts, and a hunk of one line is written `+c` with no comma at
/// all.
pub(crate) fn hunk_start(line: &str) -> Option<u32> {
    let rest = line.strip_prefix("@@ ")?;
    let plus = rest
        .split_whitespace()
        .find_map(|part| part.strip_prefix('+'))?;
    plus.split(',').next()?.parse().ok()
}
