//! Whether a citation quotes the material it was shown, or invents it.
//!
//! # The defect
//!
//! One refusal read *as scope notes "Implementation, tests, and the IPC/UI
//! wiring itself are not done…"*, and that sentence was in no note, no
//! submission, no evidence row and no file. A refusal persuades because it
//! cites, so an invented citation is not a strict verdict — it is one nobody
//! outside the call can check.
//!
//! # A quoted span is the one part of an answer a machine can check
//!
//! The rest is the Judge's own reading, which nothing here can grade.
//! Quotation marks are different: they claim these words are *in the material
//! above*, and [`Brief::question`](crate::Brief::question) is the whole of the
//! material above. So the check is containment — no second call, and no
//! opinion about whether the reading was right.
//!
//! # It cannot be narrowed to `produced`
//!
//! The obvious cut — only `produced` claims the material contains something, so
//! check only that — reopens #171, where the invented sentence was in
//! `expected`. `expected` is where a call reaches for the yardstick it was shown
//! and so where it fabricates one. What is honestly quoted and what is
//! fabricated are not sorted by field.

/// The shortest run of words a quotation mark makes a claim about. Under four
/// it is a term or an emphasis — `"met"`, `"the queue"` — and not a claim about
/// how something is worded; the invented sentence was fourteen.
///
/// A judgement rather than a measurement: there is no calibration record to set
/// it from (#154). Deliberately the loose end, because a false positive demotes
/// an honest refusal to a call that could not be answered, and both stop the
/// step for a person.
const A_CITATION: usize = 4;

/// The first quoted span in `cited` that appears nowhere in `shown`.
///
/// `None` means every quotation long enough to be one is in the material — not
/// that the answer is right, which is nothing this can know.
///
/// **Each side of an elision is checked on its own.** A model quoting across a
/// cut writes `"the note says X … and then Y"`, and requiring the ellipsis
/// itself to appear would fail every truncated quotation, which is most of them.
pub(crate) fn invented(cited: &str, shown: &str) -> Option<String> {
    let material = words(shown);
    quotations(cited).into_iter().find(|span| {
        span.split('\u{2026}')
            .flat_map(|part| part.split("..."))
            .any(|part| {
                let quoted = words(part);
                quoted.split_whitespace().count() >= A_CITATION && !material.contains(&quoted)
            })
    })
}

/// Every span between a pair of quotation marks, straight or typographic.
///
/// Marks are paired in the order they appear rather than by kind, so a curly
/// open closed by a straight mark still reads as one span — a model mixing them
/// mid-line is not an escape from the check. An unclosed final mark opens
/// nothing.
fn quotations(cited: &str) -> Vec<String> {
    let marks: Vec<usize> = cited
        .char_indices()
        .filter(|(_, c)| matches!(c, '"' | '\u{201c}' | '\u{201d}'))
        .map(|(at, _)| at)
        .collect();
    marks
        .chunks_exact(2)
        .map(|pair| {
            let opened = pair[0] + cited[pair[0]..].chars().next().map_or(1, char::len_utf8);
            cited[opened..pair[1]].trim().to_string()
        })
        .filter(|span| !span.is_empty())
        .collect()
}

/// The text as a sequence of lowercase words, space-delimited at both ends.
///
/// **Punctuation, case and line breaks are dropped**, because a brief is
/// wrapped and a diff is indented: an exact byte match would fail on where the
/// source happened to break a line. What is caught is an invented sentence, not
/// a mistyped dash.
///
/// Padding is what makes `contains` a word-boundary test: without it `"the ipc"`
/// would be found inside `the ipcs`.
fn words(text: &str) -> String {
    let mut said = String::with_capacity(text.len() + 2);
    said.push(' ');
    for character in text.chars() {
        match character.is_alphanumeric() {
            true => said.extend(character.to_lowercase()),
            false if !said.ends_with(' ') => said.push(' '),
            false => {}
        }
    }
    if !said.ends_with(' ') {
        said.push(' ');
    }
    said
}
