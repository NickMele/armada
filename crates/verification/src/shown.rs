//! What the call was shown, in labelled parts, and where in it a quotation is.
//!
//! # [`located`] answers this question about a different document
//!
//! [`located::in_the_patch`](mod@crate::located) places a gaming citation in
//! the diff, because a gaming flag is about the change. A Judge's citation is
//! about **what it was shown**, which is the brief — the file
//! `Judgment::brief_path` names, written byte for byte from
//! [`Brief::question`](crate::Brief::question). So a line here is a line of a
//! file somebody can open, and the two placements do not compete.
//!
//! # The labels come from assembly, never from a scan
//!
//! A brief's headings are prose, and a reader that recognised them would be a
//! second authority on the brief's shape — one that goes wrong silently the day
//! a heading is reworded. [`Laid`] records what it put where as it puts it, so
//! the label on a citation is the label the assembler used.
//!
//! # The same normalisation the invention check uses
//!
//! [`quoted::words`](mod@crate::quoted) is what decides whether a quotation is
//! in the material, and it is what decides where — [`located`]'s reason, one
//! document over. A span that passes containment and then places nowhere would
//! be two readings of one brief disagreeing, with no way to tell which was
//! wrong.

use core_model::Citation;

use crate::quoted;

/// A labelled run of lines in a brief, counted from one and inclusive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Region {
    label: String,
    from: u32,
    to: u32,
}

/// The brief, assembled in labelled parts.
///
/// **Every part is pushed through here, including the ones with no label.** A
/// preamble and an answer format are lines of the file too, and a counter that
/// only advanced over the labelled parts would put every citation on the wrong
/// line.
#[derive(Debug, Default)]
pub(crate) struct Laid {
    text: String,
    regions: Vec<Region>,
    /// The line the next character appended will be on, counted from one.
    line: u32,
}

impl Laid {
    pub(crate) fn new() -> Laid {
        Laid {
            text: String::new(),
            regions: Vec::new(),
            line: 1,
        }
    }

    /// Text that belongs to no named part — the preamble, the question, the
    /// answer format. It moves the counter and nothing else.
    pub(crate) fn loose(&mut self, text: &str) {
        self.push(text);
    }

    /// Text under a label. Empty text writes nothing and claims no lines: a
    /// step naming no reference has no reference region, rather than one
    /// spanning zero lines that a citation could still land in.
    pub(crate) fn part(&mut self, label: &str, text: &str) {
        if text.is_empty() {
            return;
        }
        let from = self.line;
        self.push(text);
        // A part that ends in a newline ends on the line before the one the
        // counter now names, which is the blank the next part starts on.
        let to = match text.ends_with('\n') {
            true => self.line.saturating_sub(1),
            false => self.line,
        };
        self.regions.push(Region {
            label: String::from(label),
            from,
            to: to.max(from),
        });
    }

    fn push(&mut self, text: &str) {
        self.text.push_str(text);
        self.line += text.matches('\n').count() as u32;
    }

    /// The assembled text and the map of what is where.
    pub(crate) fn done(self) -> (String, Vec<Region>) {
        (self.text, self.regions)
    }
}

/// Where in `shown` each of `cited`'s quotations is, for the ones `shown`
/// holds.
///
/// **Only what is quoted, and only what is found.** A refusal that describes
/// the material in its own words has cited in a way nothing can place, which
/// [`Brief::read`](crate::Brief::read) accepts and this answers for with an
/// empty list — never with a guess. A quotation shorter than
/// [`quoted::A_CITATION`] is a term rather than a claim about wording, and is
/// skipped here for the reason it is skipped there.
///
/// **Quotations are placed in the order they were written**, and a quotation
/// found twice is placed at its first occurrence. A citation naming two places
/// has to pick one, and the one it led with is the one it was arguing from —
/// which is [`located::in_the_patch`](mod@crate::located)'s rule, kept
/// deliberately identical.
pub(crate) fn placed(shown: &str, regions: &[Region], cited: &str) -> Vec<Citation> {
    let flat = Flat::of(shown);
    let mut found = Vec::new();
    for span in quoted::spans(cited) {
        for part in quoted::elisions(&span) {
            let words = quoted::words(&part);
            if words.split_whitespace().count() < quoted::A_CITATION {
                continue;
            }
            if let Some((from_line, to_line)) = flat.holding(&words) {
                found.push(Citation {
                    region: labelling(regions, from_line),
                    from_line,
                    to_line,
                });
            }
        }
    }
    found
}

/// Which part of the brief holds a line.
///
/// **A line inside no part reads as the brief itself.** The preamble, the
/// question and the answer format are unlabelled and a quotation can land in
/// any of them — a criterion quoted back at itself does — so there is a word
/// for that rather than a citation with no region.
fn labelling(regions: &[Region], line: u32) -> String {
    regions
        .iter()
        .find(|region| line >= region.from && line <= region.to)
        .map_or_else(|| String::from(THE_BRIEF), |region| region.label.clone())
}

/// What a citation landing outside every labelled part says it landed in.
const THE_BRIEF: &str = "brief";

/// How many lines one quotation may span before this stops looking for it.
///
/// A quotation crossing a line break is ordinary — a brief is wrapped and a
/// diff is indented — and one crossing eight is not a quotation, it is a model
/// pasting a region back. **A judgement rather than a measurement**, like
/// [`quoted::A_CITATION`], and the loose end is the same way round: a span this
/// misses is simply not placed, which costs a row on a list rather than a
/// verdict.
const A_QUOTATION: usize = 8;

/// The brief as one run of words, with where each line's words begin in it.
///
/// **Built once and searched with `find`**, rather than re-normalising a window
/// per line. The brief carries the whole branch diff, so a scan that normalised
/// a sliding window would be quadratic in the thing that is already the largest
/// string in the system.
struct Flat {
    words: String,
    /// One entry per line that carries any word: where its words begin in
    /// `words`, and which line of the brief it is. Strictly increasing, so a
    /// byte offset resolves by search.
    marks: Vec<(usize, u32)>,
}

impl Flat {
    fn of(shown: &str) -> Flat {
        // The pad is `quoted::words`' own, and it is what makes `contains` a
        // word-boundary test rather than a substring one.
        let mut words = String::from(" ");
        let mut marks = Vec::new();
        for (at, line) in shown.lines().enumerate() {
            let said = quoted::words(line);
            let said = said.trim();
            if said.is_empty() {
                continue;
            }
            marks.push((words.len(), at as u32 + 1));
            words.push_str(said);
            words.push(' ');
        }
        Flat { words, marks }
    }

    /// The first and last line holding `span`, where the brief holds it at all.
    fn holding(&self, span: &str) -> Option<(u32, u32)> {
        let at = self.words.find(span)?;
        // `span` is padded at both ends, so the first word of the match starts
        // one byte in. Without that a span beginning a line resolves to the
        // line before it, whose trailing pad is the byte `find` answered with.
        let from = self.line_at(at + 1)?;
        let to = self.line_at(at + span.len().saturating_sub(1))?;
        (to.saturating_sub(from) < A_QUOTATION as u32).then_some((from, to))
    }

    /// The line whose words hold a byte offset.
    fn line_at(&self, at: usize) -> Option<u32> {
        let after = self.marks.partition_point(|(begins, _)| *begins <= at);
        self.marks.get(after.checked_sub(1)?).map(|(_, line)| *line)
    }
}

/// A digest over the exact text one call was sent.
///
/// **A comparison, never a signature.** What it answers is whether two members
/// of one panel were handed the same object — rule 5's whole guarantee, which
/// until now was asserted by the shape of a loop and observable nowhere. Both
/// values are written by one build in one pass, and nothing authenticates
/// anything with it.
///
/// **FNV-1a, written out rather than pulled in.** A hash crate under
/// `verification` would be a dependency carried by every crate that decides a
/// verdict, bought for eight lines that will never change — and a digest that
/// is only ever compared with another digest from the same run needs neither
/// collision resistance nor stability across releases. What it does need is to
/// be the same function on both sides of a comparison, which is what writing it
/// down once here buys.
pub fn digest(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
