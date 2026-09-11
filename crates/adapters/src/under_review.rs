//! What the forge says about a pull request nobody has merged yet.
//!
//! # One process, and a record per line
//!
//! `crate::landing`'s read asks four scalars and takes the one line they come
//! back on. This asks for three lists, so `--jq` prints a record per line and
//! every line leads with a word saying which kind it is — which is what lets
//! the reader drop what it has no word for, including whatever notice `gh`
//! printed above the answer. That is `last_line`'s hazard, guarded differently.
//!
//! # Still no parse
//!
//! jq's `@tsv` escapes a tab, a newline, a carriage return and a backslash on
//! the way out, so a comment of four paragraphs is one line and putting it back
//! is [`as_written`] undoing four sequences. Not a deserialisation and not
//! wanting one: `store` and `ipc` stay the only crates that read a document.
//!
//! # The vendor's words stop here
//!
//! `APPROVED`, `SUCCESS`, `TIMED_OUT` and the rest are this forge's vocabulary
//! and no other crate sees one. A word this forge has and Armada has not is
//! silence rather than a guess — `crate::landing::read`'s own rule.

use adapter_traits::{
    FromOutside, Remark, ReviewVerdict, ReviewedBy, UnderReview, WhatPeopleSaid, WhatTheForgeRan,
};

use crate::delivery::asked_lines;

/// The fields one read of a pull request under review needs.
///
/// `reviews` beside `comments` because a reviewer's note is written in the
/// review rather than in the conversation, and a read that took only
/// `comments` would miss the sentence a person wrote while asking for changes.
/// `statusCheckRollup` is the forge's own total over both kinds of check it
/// runs — a check run and a status context are different shapes and it returns
/// them in one list.
const FIELDS: &str = "reviewDecision,statusCheckRollup,comments,reviews";

/// The reduction, run on the forge's side.
///
/// **Five streams into one `@tsv`.** In jq, `|` binds looser than `,`, so every
/// record built above reaches the encoder — the parentheses are for a reader
/// rather than for the parser.
///
/// **`// ""` on every field, and a leading tag on every record.** A forge that
/// answers `null` where this expected a string would otherwise stop jq
/// mid-stream and lose the records already built; a missing field arriving as
/// empty is a record the reader drops on its own.
///
/// **A remark with no body is not a remark**, on either stream. A review's
/// state is already counted in `reviewDecision`, and an empty comment is not
/// something to offer a person to pick or to hand a Drone.
///
/// **`.id` leads every remark record**, because a person picks comments off
/// this reading and the pull request is read again when they press. The forge
/// mints it and it is the only handle that survives an edit between the two
/// reads.
///
/// **A review reaches this reduction twice**, its body deciding a `remark` row
/// and its `.state` a `verdict` row, independently. **`.url` trails every
/// `remark` row**, empty the same way any missing field here is.
const REDUCTION: &str = "\
    ([\"said\", (.reviewDecision // \"\")]), \
    (.statusCheckRollup // [] | .[] | \
        [\"check\", (.name // .context // \"\"), (.status // \"\"), \
         (.conclusion // .state // \"\")]), \
    (.comments // [] | .[] | select((.body // \"\") != \"\") | \
        [\"remark\", (.id // \"\"), (.author.login // \"\"), (.createdAt // \"\"), \
         (.body // \"\"), (.url // \"\")]), \
    (.reviews // [] | .[] | select((.body // \"\") != \"\") | \
        [\"remark\", (.id // \"\"), (.author.login // \"\"), (.submittedAt // \"\"), \
         (.body // \"\"), (.url // \"\")]), \
    (.reviews // [] | .[] | \
        select(.state == \"APPROVED\" or .state == \"CHANGES_REQUESTED\") | \
        [\"verdict\", (.author.login // \"\"), (.state // \"\")]) \
    | @tsv";

/// Ask the forge who has looked at one pull request, what ran against it, and
/// what anybody wrote on it.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> UnderReview {
    match asked_lines(in_repo, pull_request, FIELDS, REDUCTION) {
        Some(lines) => folded(&lines),
        None => UnderReview::unreadable(),
    }
}

/// The records the reduction printed, folded into the one value.
///
/// **Split from [`read`] so that a test can hand it what jq prints.** Every
/// decision worth getting wrong is here — which tag is which, an escape undone,
/// a rollup totalled — and none of it needs a forge, an account or a network.
pub(crate) fn folded(lines: &[String]) -> UnderReview {
    // **Unreadable until a `said` record says otherwise.** A forge that
    // answered lines this has no word for has not said what people said, and
    // the absence of that record is exactly the silence the variant means.
    let mut people = WhatPeopleSaid::Unreadable;
    let mut checks = Checks::default();
    let mut remarks = Vec::new();
    let mut verdicts = Vec::new();
    for line in lines {
        let mut field = line.split('\t');
        match field.next() {
            Some("said") => people = what_people_said(field.next().unwrap_or_default()),
            Some("check") => checks.saw(
                field.next().unwrap_or_default(),
                field.next().unwrap_or_default(),
                field.next().unwrap_or_default(),
            ),
            Some("remark") => {
                let id = field.next().unwrap_or_default();
                let by = field.next().unwrap_or_default();
                let at = field.next().unwrap_or_default();
                let said = field.next().unwrap_or_default();
                let url = field.next().unwrap_or_default();
                // **A comment the forge named nothing is dropped.** The whole
                // road this reading feeds is a person picking comments and
                // pressing, and a comment with no handle cannot be picked
                // without the press meaning some other one. Counting it would
                // put a row on a screen that no act could reach.
                if !id.is_empty() {
                    let mut remark = Remark::written(
                        as_written(id),
                        as_written(by),
                        as_written(at),
                        as_written(said),
                    );
                    if !url.is_empty() {
                        remark = remark.with_url(as_written(url));
                    }
                    remarks.push(remark);
                }
            }
            Some("verdict") => {
                let by = field.next().unwrap_or_default();
                let state = field.next().unwrap_or_default();
                // The `select` in the reduction already narrowed `.state` to
                // the two words this reads, so `what_review_verdict` failing
                // here means the forge printed neither — nothing this build
                // could have asked for, and the row is dropped rather than
                // guessed at.
                if let Some(verdict) = what_review_verdict(state) {
                    verdicts.push(ReviewedBy {
                        by: FromOutside::verbatim(as_written(by)),
                        verdict,
                    });
                }
            }
            // A notice `gh` printed, or a record a later version of this
            // reduction emits and this build has no word for. Dropped rather
            // than guessed at, which is what the tag is for.
            _ => {}
        }
    }
    UnderReview {
        people,
        // **Only where the reduction was read at all.** A `people` still
        // unreadable means no record of any kind arrived that this understood,
        // so an empty rollup here would be reported as `NothingRan` — a forge
        // that could not be asked rendered as a repository with no automation.
        checks: match people {
            WhatPeopleSaid::Unreadable => WhatTheForgeRan::Unreadable,
            _ => checks.totalled(),
        },
        remarks,
        verdicts,
    }
}

/// This forge's word for what the people looking at it have said.
///
/// **A word this build has no name for is silence**, never
/// [`WhatPeopleSaid::NobodyHasLooked`] — the empty string is the forge saying
/// nobody has, and anything else is this vocabulary being behind the forge's.
pub(crate) fn what_people_said(said: &str) -> WhatPeopleSaid {
    match said {
        "APPROVED" => WhatPeopleSaid::Approved,
        "CHANGES_REQUESTED" => WhatPeopleSaid::ChangesRequested,
        "REVIEW_REQUIRED" => WhatPeopleSaid::Awaited,
        "" => WhatPeopleSaid::NobodyHasLooked,
        _ => WhatPeopleSaid::Unreadable,
    }
}

/// This forge's word for one reviewer's verdict, narrowed to the two this
/// vocabulary names.
///
/// **`None` rather than a third variant.** The reduction's own `select`
/// already keeps every other state — `COMMENTED`, `DISMISSED`, `PENDING` — out
/// of this stream, so `None` here means the forge printed neither word this
/// build asked it to, and the row is dropped exactly as an unrecognised tag
/// is.
pub(crate) fn what_review_verdict(state: &str) -> Option<ReviewVerdict> {
    match state {
        "APPROVED" => Some(ReviewVerdict::Approved),
        "CHANGES_REQUESTED" => Some(ReviewVerdict::ChangesRequested),
        _ => None,
    }
}

/// The rollup, totalled as it is read.
///
/// **Counted rather than collected.** What leaves is three numbers and the
/// names of the ones that did not pass, so a pull request with two hundred
/// green checks costs two hundred increments and holds nothing.
#[derive(Default)]
pub(crate) struct Checks {
    seen: usize,
    finished: usize,
    failed: Vec<FromOutside>,
}

impl Checks {
    /// One check, from the two words the forge uses for it.
    ///
    /// A check run carries `status` and, once it is over, `conclusion`. A
    /// status context carries neither and answers `state` instead, which the
    /// reduction has already folded into the same field. So the pair is read
    /// together: a conclusion this knows decides it, and anything else turns on
    /// whether the forge says it has finished.
    pub(crate) fn saw(&mut self, name: &str, status: &str, conclusion: &str) {
        self.seen += 1;
        match conclusion {
            "SUCCESS" | "NEUTRAL" | "SKIPPED" => self.finished += 1,
            "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED"
            | "STARTUP_FAILURE" | "STALE" => {
                self.finished += 1;
                self.failed.push(FromOutside::verbatim(as_written(name)));
            }
            // **Finished in a way this build has no word for counts as not
            // passing**, and the direction is the whole of the choice: the
            // caller of this reading is deciding whether work may go out, so a
            // pass it cannot vouch for is the one answer it must not give.
            _ if status == "COMPLETED" => {
                self.finished += 1;
                self.failed.push(FromOutside::verbatim(as_written(name)));
            }
            _ => {}
        }
    }

    /// The three numbers as the one answer a caller acts on.
    ///
    /// **Failure is read before unfinished**, which is the rule
    /// [`WhatTheForgeRan::SomeFailed`] carries: a check that has already failed
    /// does not become a pass when the rest finish.
    pub(crate) fn totalled(self) -> WhatTheForgeRan {
        if self.seen == 0 {
            return WhatTheForgeRan::NothingRan;
        }
        if !self.failed.is_empty() {
            return WhatTheForgeRan::SomeFailed {
                failed: self.failed,
                checks: self.seen,
            };
        }
        match self.finished == self.seen {
            true => WhatTheForgeRan::AllPassed { checks: self.seen },
            false => WhatTheForgeRan::StillWaiting {
                finished: self.finished,
                checks: self.seen,
            },
        }
    }
}

/// One `@tsv` field, back as the text somebody typed.
///
/// **Four sequences and no others**, because four is all jq's `@tsv` writes:
/// `\t`, `\n`, `\r` and `\\`. A backslash before anything else was a backslash
/// somebody typed and stays one — guessing at a fifth escape would rewrite a
/// person's comment.
pub(crate) fn as_written(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut rest = field.chars();
    while let Some(one) = rest.next() {
        if one != '\\' {
            out.push(one);
            continue;
        }
        match rest.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
