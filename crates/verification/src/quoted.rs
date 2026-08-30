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
//!
//! # Only a quotation attributed to a source claims anything
//!
//! Two Jobs stopped here on honest refusals. One wrote its own standard inside
//! quotation marks — *"A plan addressing all five variants as stated: …"* — and
//! the other its own paraphrase of the facts it had been shown. Neither names a
//! source, and neither is a claim that those words are anywhere: they are the
//! Judge's wording, in quotation marks. Containment has nothing to check in
//! them and answers the only way it can, which is wrongly.
//!
//! So [`invented`] reads a quotation only where the words in front of it put it
//! **in** something — *as scope notes*, *the diff shows*, *per serving.rs*.
//! That is what makes it a claim, and it is the claim that is checked.
//!
//! Raising [`A_CITATION`] would not have done this. The fabrication in #171 is
//! ten words and the two honest spans are nineteen and twenty-nine, so every
//! threshold that lets them through lets the fabrication through first.
//! Narrowing by field would not have done it either: #171 and the first of the
//! two are both in `expected`.

/// The shortest run of words a quotation mark makes a claim about. Under four
/// it is a term or an emphasis — `"met"`, `"the queue"` — and not a claim about
/// how something is worded; the invented sentence was fourteen.
///
/// A judgement rather than a measurement: there is no calibration record to set
/// it from (#154). Deliberately the loose end, because a false positive demotes
/// an honest refusal to a call that could not be answered, and both stop the
/// step for a person.
const A_CITATION: usize = 4;

/// The first quoted span in `cited` that is attributed to a source and appears
/// nowhere in `shown`.
///
/// `None` means every quotation that claimed to come from the material is in
/// it — not that the answer is right, which is nothing this can know.
///
/// **Each side of an elision is checked on its own.** A model quoting across a
/// cut writes `"the note says X … and then Y"`, and requiring the ellipsis
/// itself to appear would fail every truncated quotation, which is most of them.
pub(crate) fn invented(cited: &str, shown: &str) -> Option<String> {
    let material = words(shown);
    quotations(cited)
        .into_iter()
        .filter(|quotation| attributed(&quotation.runup))
        .map(|quotation| quotation.span)
        .find(|span| {
            span.split('\u{2026}')
                .flat_map(|part| part.split("..."))
                .any(|part| {
                    let quoted = words(part);
                    quoted.split_whitespace().count() >= A_CITATION && !material.contains(&quoted)
                })
        })
}

/// One quotation, and what was written in front of it.
struct Quotation {
    /// The words between the marks.
    span: String,
    /// What runs from the end of the previous quotation — or the start of the
    /// citation — up to this one's opening mark. **Not the whole citation**:
    /// see [`attributed`] for why the window stops where it does.
    runup: String,
}

/// Whether the run-up puts the quotation *in* something.
///
/// **This is a phrase list, and a phrase list is a heuristic.** There is no
/// structural signal underneath it: the two false positives and the one true
/// positive are the same shape — a `field:` line with a quoted run of words in
/// it — and what separates them is English. Every alternative considered read
/// worse. Extracting the material's own headings and matching them in the
/// run-up sounds structural, but the brief carries the whole diff and the whole
/// note, so almost any noun matches something in it. Treating a quotation that
/// begins its field as unattributed fits all three cases we have, but it is a
/// test for a bare quotation rather than for attribution, and *the plan should
/// establish "…"* would walk straight past it.
///
/// **An unrecognised construction reads as unattributed and is not checked.**
/// The two costs are not symmetric. A missed fabrication reaches a person as a
/// refusal, with the words to argue with; a false positive demotes an honest
/// refusal to [`Unreadable`](crate::Unreadable) and the reason is in a log file
/// nobody is looking at. Two Jobs in a row stopped that way and neither reached
/// a Check.
///
/// **Attribution has to be in the run-up, not somewhere in the field.** It is
/// cut at the previous quotation and at the last sentence boundary, so a model
/// that attributes one quotation does not make the next one checkable, and a
/// sentence that names a source does not vouch for the sentence after it.
///
/// What it misses, written down rather than discovered: attribution after the
/// quotation (*"…", the note says*), attribution by a bare colon (*the scope
/// note: "…"*), and any verb of saying not in the list. Each of those is a
/// fabrication that goes unchecked.
fn attributed(runup: &str) -> bool {
    let sentence = runup
        .rsplit(['.', '!', '?', ';', '\n'])
        .next()
        .unwrap_or(runup);
    words(sentence)
        .split_whitespace()
        .any(|word| ATTRIBUTIONS.contains(&word))
}

/// The constructions that put words in a source, in the three shapes a Judge
/// and a gaming call actually write: a source saying them, a source holding
/// them, and a source doing something to them.
///
/// Lowercase and whole words, because [`words`] has already dropped the case
/// and the punctuation — so `serving.rs:825 reads` and `Scope notes` both land.
const ATTRIBUTIONS: &[&str] = &[
    // A source saying it.
    "according",
    "calls",
    "called",
    "cites",
    "cited",
    "citing",
    "claims",
    "claimed",
    "describes",
    "described",
    "notes",
    "noted",
    "per",
    "puts",
    "quotes",
    "quoted",
    "quoting",
    "read",
    "reads",
    "records",
    "recorded",
    "reports",
    "reported",
    "said",
    "says",
    "stated",
    "states",
    "writes",
    "written",
    "wrote",
    // A source holding it.
    "carries",
    "contains",
    "containing",
    "includes",
    "including",
    "showed",
    "showing",
    "shows",
    // A source doing something it is the object of, which is how a gaming
    // citation points at a hunk: *the suite drops "…"*, *it replaces "…"*.
    "adds",
    "added",
    "asserts",
    "asserted",
    "changes",
    "changed",
    "drops",
    "dropped",
    "removes",
    "removed",
    "replaces",
    "replaced",
];

/// Every span between a pair of quotation marks, straight or typographic, with
/// what was written in front of it.
///
/// Marks are paired in the order they appear rather than by kind, so a curly
/// open closed by a straight mark still reads as one span — a model mixing them
/// mid-line is not an escape from the check. An unclosed final mark opens
/// nothing.
fn quotations(cited: &str) -> Vec<Quotation> {
    let marks: Vec<usize> = cited
        .char_indices()
        .filter(|(_, c)| matches!(c, '"' | '\u{201c}' | '\u{201d}'))
        .map(|(at, _)| at)
        .collect();
    let mut since = 0;
    let mut found = Vec::new();
    for pair in marks.chunks_exact(2) {
        let opened = pair[0] + after(cited, pair[0]);
        let span = cited[opened..pair[1]].trim().to_string();
        let runup = cited[since..pair[0]].to_string();
        since = pair[1] + after(cited, pair[1]);
        if !span.is_empty() {
            found.push(Quotation { span, runup });
        }
    }
    found
}

/// How far past `at` the next byte is, for a mark that may be one byte or three.
fn after(cited: &str, at: usize) -> usize {
    cited[at..].chars().next().map_or(1, char::len_utf8)
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
