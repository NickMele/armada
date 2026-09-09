//! What a person calls a Job: a number that counts within its Manifest, and
//! the title reduced to something a branch name can hold.
//!
//! # The id is not the handle, and neither replaces the other
//!
//! [`JobId`](crate::JobId) stays the record's key. It is unique across every
//! Manifest, it is minted without asking anything, and nothing that joins to a
//! Job is written in terms of anything else. What it is bad at is being said,
//! typed or told apart, and that is the whole of what this is for.
//!
//! **ULIDs sort by time, so the two Jobs hardest to tell apart are the two you
//! most need to** — one proposal's Jobs are minted in the same millisecond and
//! share every character but a few. `01M222EJWH004FA17WAMCQEKXR` and
//! `01M222EJWJ0058XKVA2NB4JGMD` agree on nine.
//!
//! # The number counts within a Manifest, and the store allocates it
//!
//! Not globally: a person works in one repository at a time and *job 12* is
//! shorter than anything that has to be unique across all of them. Nothing here
//! mints one — the count is a fact about a table, so `store` is the authority
//! and this only carries what it allocated.
//!
//! **Both directions, in one file.** Only [`handle_of`] existed for a while, so
//! what a person could say was what nothing accepted. [`JobReference`] is
//! beside it so a change to a handle cannot land without the reader.

use alloc::format;
use alloc::string::{String, ToString};

use crate::envelope::Ulid;
use crate::job::ids::{JobId, Title};
use crate::job::record::Job;

/// A Job's number within its Manifest. **Allocated by the store at insert**,
/// monotonic per Manifest, and never reused — a killed Job keeps its number the
/// way a closed issue keeps its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobNumber(u32);

impl JobNumber {
    /// Carry a number the store allocated. Nothing else may make one.
    pub fn carried(number: u32) -> JobNumber {
        JobNumber(number)
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl core::fmt::Display for JobNumber {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "{}", self.0)
    }
}

/// A name whose value is a credential. Matched case-insensitively as a
/// substring, so `AWS_SECRET_ACCESS_KEY` is caught by `secret`.
///
/// **One list, two readers.** `fleet::redaction` scrubs a filed report with it
/// and [`handle_of`] refuses to slug a title that trips it. It was the
/// redactor's alone until a handle carried into a branch name what the report
/// had just redacted out of the title beside it — so it is a fact about what a
/// credential is called, which is the domain's, rather than a fact about
/// reports.
///
/// **Deliberately narrow, and `auth` is not here.** `auth` catches `author`,
/// and a report about a Job whose commit author was redacted is a report
/// missing the fact somebody filed it to explain. `authorization` is the
/// spelling that means the header. No vendor prefixes: that list is a better
/// detector *and* a list of credential shapes committed to a public
/// repository, which this repository's own guard refuses.
pub const CREDENTIAL_NAMES: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "passphrase",
    "credential",
    "apikey",
    "api_key",
    "access_key",
    "private_key",
    "authorization",
];

/// Whether this text names a credential, and so must not be carried anywhere
/// that is not scrubbed.
pub fn names_a_credential(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    CREDENTIAL_NAMES.iter().any(|name| lowered.contains(name))
}

/// How much of a title a handle carries.
///
/// **Long enough to tell two Jobs of one request apart, short enough to read at
/// the end of `Merge pull request #549 from NickMele/armada/…`.** The number
/// alone is already unique, so nothing depends on this bound — a title cut
/// mid-word loses nothing but a word.
const SLUG_LIMIT: usize = 42;

/// What a Job is called where a person reads or types it: `12-the-drone-count`.
///
/// **Derived, never stored.** Both halves are frozen at creation — the store
/// allocates the number once and a Job's title cannot change — so deriving it
/// is stable, and a second copy on the record is a second thing to keep true.
/// It is the directory a worktree goes in, the branch a Job is delivered on,
/// and the name every path under `.armada/` is keyed by.
///
/// **Portable by construction.** `WorktreeSpec::for_job` refuses a character
/// that names no directory and no branch; this emits `a-z`, `0-9` and `-`, so
/// that refusal is unreachable from here rather than merely unlikely.
pub fn handle_of(number: JobNumber, title: &Title) -> String {
    // **A title naming a credential is carried by its number alone.**
    //
    // A handle is a branch name and a directory name, and a branch is pushed to
    // a forge. Neither passes through `fleet::redaction`, which is the one
    // chokepoint a *report's* text is scrubbed at — so a title reading
    // `AGENT_API_TOKEN=hunter2` was redacted on the report's Title line and
    // written out in full on the Branch line beside it.
    //
    // **Refused rather than scrubbed.** Redacting here would put a second
    // scrubber in a second crate, and the two would drift; and a handle is a
    // convenience the number does not need. What is lost is readability on the
    // rare Job whose title names a secret, which is a Job whose title should be
    // rewritten anyway.
    if names_a_credential(title.as_str()) {
        return number.to_string();
    }
    let slug = slug_of(title.as_str());
    match slug.is_empty() {
        // A title of nothing but punctuation. The number is what is unique, so
        // this reads oddly and still names exactly one Job.
        true => number.to_string(),
        false => format!("{number}-{slug}"),
    }
}

/// A title reduced to the characters a branch name may hold.
///
/// Runs of anything else become one `-`, and leading and trailing ones are
/// dropped — a `-` at either end of a git ref is legal and reads as a typo.
fn slug_of(title: &str) -> String {
    let mut slug = String::with_capacity(SLUG_LIMIT);
    let mut pending = false;
    for character in title.chars() {
        if slug.len() >= SLUG_LIMIT {
            break;
        }
        match character.is_ascii_alphanumeric() {
            // Not `to_lowercase`: a handle is compared as text in a branch name
            // and on a case-insensitive filesystem, so two titles differing
            // only in case must not make two handles that are one directory.
            true => {
                if pending && !slug.is_empty() {
                    slug.push('-');
                }
                pending = false;
                slug.push(character.to_ascii_lowercase());
            }
            false => pending = true,
        }
    }
    slug
}

/// A Job named by something a person can say: the record's key, a whole
/// handle, or the number at the front of one.
///
/// **Reading is [`handle_of`] backwards, and the number is the only half that
/// resolves.** A handle is a number and a slug, so its number is the run of
/// digits before the first `-`; the slug is carried to be checked against the
/// title it came from, never to be searched on. Which Job a number names stays
/// outside this type — that is a fact about a Manifest's table, and
/// `store::Store::resolve_job` is the authority for it.
///
/// **Three forms and no fourth.** A title is not one: two Jobs may share one,
/// and a reference that sometimes names two Jobs is one a caller has to
/// disambiguate rather than resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobReference {
    /// The `jobs` row's own key. Unique across every Manifest, so it names one
    /// Job with nothing else supplied.
    Id(JobId),
    /// A whole handle, as Bridge shows it and a person pastes it.
    Handle {
        number: JobNumber,
        /// The text as it was given, kept so the title behind it can be
        /// checked. A number that matches under a slug that does not names no
        /// Job — most often a handle pasted out of some other repository.
        said: String,
    },
    /// The number alone — the shortest thing a person can type, and the one
    /// form that means nothing without a Manifest to count it within.
    Number(JobNumber),
}

impl JobReference {
    /// What this text names, or `None` where it names nothing at all.
    ///
    /// The three cannot be confused: a handle always holds a `-` and a ULID
    /// never does, and a number is digits to the end.
    pub fn read(said: &str) -> Option<JobReference> {
        let said = said.trim();
        let digits: String = said.chars().take_while(char::is_ascii_digit).collect();
        let rest = &said[digits.len()..];
        match (digits.is_empty(), rest.chars().next()) {
            (false, None) => number(&digits).map(JobReference::Number),
            // The slug is held to what `slug_of` emits rather than merely to
            // being present. A trailing `-` is the shape of a handle somebody
            // cut short, and resolving it on its number alone would answer for
            // a Job the text does not name.
            (false, Some('-')) if is_a_slug(&rest[1..]) => {
                number(&digits).map(|number| JobReference::Handle {
                    number,
                    said: said.to_string(),
                })
            }
            // A ULID is Crockford base32 and holds no separator, so text that
            // is neither of the two above is either an id or nothing.
            _ => (!said.is_empty() && said.chars().all(|one| one.is_ascii_alphanumeric()))
                .then(|| JobReference::Id(JobId::carried(Ulid::carried(said.to_string())))),
        }
    }

    /// Whether this names that Job.
    ///
    /// **For a caller already holding the Jobs it is choosing between** — a
    /// Drone naming a sibling, where the set is small, known and in memory. A
    /// caller holding only a store asks the store, which is these same three
    /// answers against an index.
    pub fn names(&self, job: &Job) -> bool {
        match self {
            JobReference::Id(id) => job.id() == id,
            JobReference::Number(number) => job.number() == *number,
            JobReference::Handle { number, said } => {
                job.number() == *number && &job.handle() == said
            }
        }
    }
}

/// A number a `u32` can hold. A run of digits longer than that is a typo, and
/// no Manifest has counted that far.
fn number(digits: &str) -> Option<JobNumber> {
    digits.parse().ok().map(JobNumber)
}

/// What [`slug_of`] emits, and nothing else.
fn is_a_slug(said: &str) -> bool {
    !said.is_empty()
        && said
            .chars()
            .all(|one| one.is_ascii_lowercase() || one.is_ascii_digit() || one == '-')
        && !said.ends_with('-')
}

impl core::fmt::Display for JobReference {
    /// What was said, back as it was said. Every refusal downstream quotes it.
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JobReference::Id(id) => write!(out, "{}", id.as_str()),
            JobReference::Handle { said, .. } => write!(out, "{said}"),
            JobReference::Number(number) => write!(out, "{number}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn handle(number: u32, title: &str) -> String {
        handle_of(
            JobNumber::carried(number),
            &Title::new(title).expect("a title"),
        )
    }

    #[test]
    fn reads_as_a_number_and_the_words_of_the_title() {
        assert_eq!(
            handle(12, "The drone count is wrong"),
            "12-the-drone-count-is-wrong"
        );
    }

    #[test]
    fn holds_only_what_a_branch_and_a_directory_may_hold() {
        let said = handle(3, "Fix `Shell.tsx` — the \"1 of 2\" reading (again)!");

        assert!(
            said.chars()
                .all(|one| one.is_ascii_lowercase() || one.is_ascii_digit() || one == '-'),
            "{said}"
        );
        assert!(!said.contains("--"), "runs collapse: {said}");
        assert!(!said.ends_with('-'), "and nothing trails: {said}");
    }

    // The failure the handle exists for: two Jobs of one request, told apart at
    // a glance rather than at the tenth character.
    #[test]
    fn tells_two_jobs_of_one_request_apart() {
        assert_eq!(
            handle(12, "The drone count is wrong"),
            "12-the-drone-count-is-wrong"
        );
        assert_eq!(
            handle(13, "Say why a Job is waiting"),
            "13-say-why-a-job-is-waiting"
        );
    }

    #[test]
    fn is_cut_at_a_bound_rather_than_growing_without_one() {
        let said = handle(9, &"word ".repeat(40));

        assert!(said.len() <= SLUG_LIMIT + 3, "{} chars: {said}", said.len());
        assert!(!said.ends_with('-'));
    }

    #[test]
    fn is_the_number_alone_where_a_title_reduces_to_nothing() {
        assert_eq!(handle(7, "— ⟨⟩ …"), "7");
    }

    // A case-insensitive filesystem is what makes this a defect rather than a
    // preference: two directories that differ only in case are one directory.
    #[test]
    fn does_not_depend_on_the_case_the_title_was_written_in() {
        assert_eq!(handle(4, "Fix The Reader"), handle(4, "fix the reader"));
    }

    // **The defect this guard exists for.** A branch is pushed to a forge and
    // never passes through `fleet::redaction`, so a title the report redacted
    // was written out in full on the Branch line beside it.
    #[test]
    fn carries_no_part_of_a_title_that_names_a_credential() {
        let said = handle(5, "run it with AGENT_API_TOKEN=hunter2 against the reader");

        assert_eq!(said, "5");
        assert!(!said.contains("hunter2"));
    }

    #[test]
    fn catches_a_credential_name_however_it_is_written() {
        for title in [
            "set AWS_SECRET_ACCESS_KEY and go",
            "the password field is blank",
            "Authorization header is dropped",
            "fix api_key handling",
        ] {
            assert_eq!(handle(6, title), "6", "{title}");
        }
    }

    // `auth` catching `author` is the reason the list says `authorization`, and
    // a Job about commit authorship is an ordinary Job with an ordinary handle.
    #[test]
    fn does_not_catch_an_author() {
        assert_eq!(
            handle(8, "The commit author is dropped"),
            "8-the-commit-author-is-dropped"
        );
    }

    #[test]
    fn numbers_carry_and_read_back() {
        assert_eq!(JobNumber::carried(12).get(), 12);
        assert_eq!(JobNumber::carried(12).to_string(), "12");
    }

    // The defect this half exists for: what Bridge shows is what a person
    // pastes, and until now nothing read it back.
    #[test]
    fn reads_back_the_handle_bridge_shows() {
        assert_eq!(
            JobReference::read("1-board-s-clear-button-should-reclaim-worktr"),
            Some(JobReference::Handle {
                number: JobNumber::carried(1),
                said: "1-board-s-clear-button-should-reclaim-worktr".to_string(),
            })
        );
    }

    #[test]
    fn reads_a_bare_number_and_a_ulid_as_themselves() {
        assert_eq!(
            JobReference::read("12"),
            Some(JobReference::Number(JobNumber::carried(12)))
        );
        assert_eq!(
            JobReference::read("01M22TYSAE0023MADDP5ZQEYGW"),
            Some(JobReference::Id(JobId::carried(Ulid::carried(
                "01M22TYSAE0023MADDP5ZQEYGW".to_string()
            ))))
        );
    }

    // A title reducing to nothing makes a handle that is a number, so the two
    // forms meet — and both resolve the same way, which is why this is not an
    // ambiguity to break.
    #[test]
    fn reads_a_handle_of_nothing_but_a_number_as_a_number() {
        assert_eq!(
            JobReference::read(&handle(7, "— ⟨⟩ …")),
            Some(JobReference::Number(JobNumber::carried(7)))
        );
    }

    #[test]
    fn names_nothing_where_the_text_could_be_no_form() {
        for said in ["", "   ", "-3", "a b", "12-", "much/too/pathlike"] {
            assert_eq!(JobReference::read(said), None, "{said:?}");
        }
    }

    #[test]
    fn says_back_what_was_said() {
        for said in ["1-board-s-clear-button", "12", "01M22TYSAE0023MADDP5ZQEYGW"] {
            assert_eq!(
                JobReference::read(said).expect("a reference").to_string(),
                said
            );
        }
    }

    #[test]
    fn is_trimmed_because_a_paste_carries_what_surrounded_it() {
        assert_eq!(JobReference::read("  12\n"), JobReference::read("12"));
    }
}
