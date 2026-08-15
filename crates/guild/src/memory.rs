//! Splitting an imported `CLAUDE.md` into the three fragments of
//! `PLAN.md` §13.1.
//!
//! | Fragment | Answers |
//! |---|---|
//! | `voice.md` | how to talk to you |
//! | `expectations.md` | what "done" means |
//! | `how-i-work.md` | process and tooling |
//!
//! **Three files rather than one, because each is separately editable and
//! separately projected** — a distinction one file could not express.
//!
//! # The split is a guess, and it is never presented as anything else
//!
//! `PLAN.md` §13.4 settles what happens next: *"It does not show you a parsed
//! split and ask you to correct it."* Reviewing a machine's reading of your own
//! memory file is more work than answering the question and produces a worse
//! answer, because you end up editing its interpretation rather than saying
//! what you mean. So this function's output is a **starting point the interview
//! then overwrites where the two overlap**, and every fragment it writes says
//! so at the top.
//!
//! That is also why the classifier is keyword matching and not something
//! cleverer. A better guess would still be a guess, and a guess nobody is asked
//! to confirm does not need to be good — it needs to be *harmless*, which means
//! never dropping a line. [`split`] is total: every section of the input lands
//! in exactly one fragment, and `how-i-work.md` is where the unclassifiable
//! goes.

/// The three fragments, in the order `PLAN.md` §13.1 lists them.
pub const FRAGMENTS: [&str; 3] = ["voice.md", "expectations.md", "how-i-work.md"];

/// The token every fragment carries while it is still Armada's words.
///
/// Machine-readable and deliberately ugly, so that `armada doctor` matches on
/// something nobody types by accident — it used to match on the prose *"Imported
/// from CLAUDE.md"*, which is a sentence a person could reasonably keep after
/// rewriting the file underneath it.
pub const MARKER: &str = "armada:unedited";

/// Why a fragment is not yet yours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unedited {
    /// Armada carved it out of your `CLAUDE.md` and nobody confirmed the split.
    Imported,
    /// Import found nothing, so the file holds the examples Armada wrote.
    Example,
}

impl Unedited {
    /// The word `armada doctor` puts in its detail.
    pub const fn said(self) -> &'static str {
        match self {
            Unedited::Imported => "still as imported",
            Unedited::Example => "still Armada's example text",
        }
    }
}

/// Whether a fragment is still Armada's words, and which kind.
///
/// **Read from the file rather than remembered**, because the thing being asked
/// about is what is on disk after a person has had a chance to edit it.
pub fn state(body: &str) -> Option<Unedited> {
    if !body.contains(MARKER) {
        return None;
    }
    if body.contains("armada:unedited example") {
        return Some(Unedited::Example);
    }
    Some(Unedited::Imported)
}

/// Which fragment a section of the memory file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fragment {
    /// How to talk to you.
    Voice,
    /// What "done" means.
    Expectations,
    /// Process and tooling — **and everything unclassifiable**.
    HowIWork,
}

impl Fragment {
    /// The file this fragment is written to.
    pub const fn file(self) -> &'static str {
        match self {
            Fragment::Voice => "voice.md",
            Fragment::Expectations => "expectations.md",
            Fragment::HowIWork => "how-i-work.md",
        }
    }

    /// The fragment a guild-relative filename names.
    pub fn of(file: &str) -> Option<Fragment> {
        [Fragment::Voice, Fragment::Expectations, Fragment::HowIWork]
            .into_iter()
            .find(|fragment| fragment.file() == file)
    }

    /// The file's own heading.
    pub const fn title(self) -> &'static str {
        match self {
            Fragment::Voice => "Voice",
            Fragment::Expectations => "Expectations",
            Fragment::HowIWork => "How I work",
        }
    }

    /// **Who reads this file, and when.**
    ///
    /// The one line that was missing. A fragment used to open with two comments
    /// about where it came from and nothing about what it was for, and a real
    /// reader opened `expectations.md` after a default run and said he had no
    /// idea what it meant. Provenance is not purpose.
    pub const fn read_by(self) -> &'static str {
        match self {
            Fragment::Voice => {
                "Every agent reads this before it writes you a word — Helm in \
                 conversation, and every Drone that reports back."
            }
            Fragment::Expectations => {
                "Every agent reads this before it tells you a Job is done, and \
                 a workflow's verify step gates on it."
            }
            Fragment::HowIWork => {
                "Every agent reads this before it touches one of your \
                 repositories."
            }
        }
    }

    /// What to write, in one sentence.
    pub const fn asks(self) -> &'static str {
        match self {
            Fragment::Voice => "Write the tone, the length, and what to lead with.",
            Fragment::Expectations => "Write what has to be true before it may say so.",
            Fragment::HowIWork => {
                "Write your branching, what to decide without asking, and what \
                 to always ask about first."
            }
        }
    }

    /// A shape to copy, for the file import had nothing to put in.
    ///
    /// **Real lines rather than a blank you have to invent from nothing.** They
    /// sit under a heading that says what they are, because an example a reader
    /// mistakes for your answer is worse than no example — and `armada doctor`
    /// names the file until they are gone.
    pub const fn examples(self) -> &'static [&'static str] {
        match self {
            Fragment::Voice => &[
                "Lead with the answer. The reasoning goes after it, for when I want it.",
                "150 words unless I ask for more.",
                "Tables for anything comparative, prose for a single fact.",
                "No preamble, no recap, no \"let me know if\".",
            ],
            Fragment::Expectations => &[
                "Tests pass, and new behaviour has a test that fails without it.",
                "The linter is clean at the settings the repository ships.",
                "The change is on a branch, never committed to the default one.",
                "The commit message says why, not what.",
            ],
            Fragment::HowIWork => &[
                "Branch from the default branch; never commit to it directly.",
                "Small commits, each one green on its own.",
                "Formatting, lint fixes and test scaffolding: just do them.",
                "Refactors, dependency bumps and schema changes: ask first.",
            ],
        }
    }
}

/// Words that place a section in [`Fragment::Voice`].
const VOICE: [&str; 12] = [
    "voice",
    "tone",
    "brevity",
    "verbosity",
    "concise",
    "how you talk",
    "speak",
    "writing style",
    "response",
    "bluf",
    "bottom line",
    "asking me",
];

/// Words that place a section in [`Fragment::Expectations`].
const EXPECTATIONS: [&str; 11] = [
    "expectation",
    "definition of done",
    "done means",
    "when is work finished",
    "acceptance",
    "quality bar",
    "coverage",
    "review",
    "commit message",
    "before you finish",
    "checklist",
];

/// A section of the memory file: its heading, if it had one, and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Section {
    heading: Option<String>,
    lines: Vec<String>,
}

/// Split a memory file into the three fragments.
///
/// Returns them in [`FRAGMENTS`] order, each already carrying the header that
/// says where it came from. **A fragment with nothing in it still comes back**,
/// with the header and no body: `armada doctor` reports a fragment that is
/// still whatever import produced (`PLAN.md` §13.4), and it can only do that if
/// the file exists to be compared against.
pub fn split(memory: &str) -> Vec<(&'static str, String)> {
    let mut voice = Vec::new();
    let mut expectations = Vec::new();
    let mut how_i_work = Vec::new();

    for section in sections(memory) {
        let body = render(&section);
        if body.trim().is_empty() {
            continue;
        }
        match classify(&section) {
            Fragment::Voice => voice.push(body),
            Fragment::Expectations => expectations.push(body),
            Fragment::HowIWork => how_i_work.push(body),
        }
    }

    vec![
        (Fragment::Voice.file(), fragment(Fragment::Voice, &voice)),
        (
            Fragment::Expectations.file(),
            fragment(Fragment::Expectations, &expectations),
        ),
        (
            Fragment::HowIWork.file(),
            fragment(Fragment::HowIWork, &how_i_work),
        ),
    ]
}

/// What each fragment says about itself.
///
/// # Purpose first, provenance second
///
/// A fragment used to open with two comments about where it came from and
/// nothing about what it was for. After a default run `expectations.md` was
/// *only* those two comments, and the person who opened it said he had no idea
/// what it meant — which is a fair reading of a file that says a machine wrote
/// it and stops.
///
/// So every fragment now opens the same way whether import found anything or
/// not: its own heading, **who reads it and when**, and what to write. The
/// difference is what follows — the sections import carved out, or a set of
/// examples under a heading that says they are examples.
///
/// # The header comment is still the reason this is safe to write unasked
///
/// It says the split is a guess, that editing the file directly is the supported
/// path, and that deleting the comment is what marks the file as yours — the
/// rule `PLAN.md` §13.4 states as *"a tool that can only be configured through a
/// wizard is a tool you cannot fix at one in the morning."*
fn fragment(which: Fragment, parts: &[String]) -> String {
    let mut out = String::new();

    if parts.is_empty() {
        out.push_str(&format!(
            "<!-- {MARKER} example\n\
             \n\
             \x20    Import found nothing for this file, so Armada wrote the examples below.\n\
             \x20    They are a shape to copy, not your answer.\n\
             \n\
             \x20    Replace them and delete this comment. `armada doctor` names this file\n\
             \x20    until you do. -->\n"
        ));
    } else {
        out.push_str(&format!(
            "<!-- {MARKER} imported\n\
             \n\
             \x20    `armada guild init` carved this out of your CLAUDE.md by reading\n\
             \x20    headings. The split between voice.md, expectations.md and\n\
             \x20    how-i-work.md is a guess, and nothing asked you to confirm it.\n\
             \n\
             \x20    Edit this file directly — that is the supported path, not a\n\
             \x20    workaround. Delete this comment once the words below are yours;\n\
             \x20    `armada doctor` names this file until you do. -->\n"
        ));
    }

    out.push_str(&format!(
        "\n# {}\n\n{}\n\n{}\n",
        which.title(),
        which.read_by(),
        which.asks()
    ));

    if parts.is_empty() {
        out.push_str("\n## Examples, not your answer — replace them\n\n");
        for example in which.examples() {
            out.push_str(&format!("- {example}\n"));
        }
        return out;
    }

    for part in parts {
        out.push('\n');
        out.push_str(part);
        if !part.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Cut the file at its headings. Text before the first heading is a section
/// with no heading of its own, which is how a memory file that is just prose
/// still classifies rather than being dropped.
fn sections(memory: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut current = Section {
        heading: None,
        lines: Vec::new(),
    };
    let mut fenced = false;

    for line in memory.lines() {
        // A `#` inside a fenced block is a shell comment, not a heading.
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if !fenced && is_heading(line) {
            if !current.lines.is_empty() || current.heading.is_some() {
                out.push(current);
            }
            current = Section {
                heading: Some(line.to_string()),
                lines: Vec::new(),
            };
            continue;
        }
        current.lines.push(line.to_string());
    }
    if !current.lines.is_empty() || current.heading.is_some() {
        out.push(current);
    }
    out
}

fn is_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('#') && trimmed.trim_start_matches('#').starts_with(' ')
}

fn render(section: &Section) -> String {
    let mut out = String::new();
    if let Some(heading) = &section.heading {
        out.push_str(heading);
        out.push('\n');
    }
    for line in &section.lines {
        out.push_str(line);
        out.push('\n');
    }
    out.trim_end().to_string()
}

/// **The heading decides; the body only breaks a tie.**
///
/// A heading is what its author wrote to say what a section is about, so it is
/// the strongest signal available and it is read first. The body is consulted
/// only for a section with no heading at all — matching on the body generally
/// would let one sentence about "review" drag a whole section about branching
/// into the wrong fragment.
fn classify(section: &Section) -> Fragment {
    if let Some(heading) = &section.heading {
        if let Some(fragment) = keyword(heading) {
            return fragment;
        }
        return Fragment::HowIWork;
    }
    keyword(&section.lines.join("\n")).unwrap_or(Fragment::HowIWork)
}

fn keyword(text: &str) -> Option<Fragment> {
    let lowered = text.to_ascii_lowercase();
    if VOICE.iter().any(|word| lowered.contains(word)) {
        return Some(Fragment::Voice);
    }
    if EXPECTATIONS.iter().any(|word| lowered.contains(word)) {
        return Some(Fragment::Expectations);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMORY: &str = "\
# My preferences

## Verbosity

150 words maximum per response.

## What done means

Tests pass and the diff is reviewed.

## Branching

Never commit to main.
";

    #[test]
    fn a_heading_about_voice_lands_in_voice_md() {
        let split = split(MEMORY);
        let voice = &split[0].1;
        assert_eq!(split[0].0, "voice.md");
        assert!(voice.contains("150 words maximum"), "{voice}");
        assert!(!voice.contains("Never commit to main"), "{voice}");
    }

    #[test]
    fn a_heading_about_done_lands_in_expectations_md() {
        let split = split(MEMORY);
        assert_eq!(split[1].0, "expectations.md");
        assert!(split[1].1.contains("Tests pass"), "{}", split[1].1);
    }

    /// **The catch-all is `how-i-work.md`, and the split is total.** Every line
    /// of the input reaches exactly one fragment; a classifier that dropped the
    /// sections it did not recognise would silently lose the half of a memory
    /// file that is neither voice nor a definition of done.
    #[test]
    fn everything_unrecognised_lands_in_how_i_work_and_nothing_is_dropped() {
        let split = split(MEMORY);
        assert_eq!(split[2].0, "how-i-work.md");
        assert!(split[2].1.contains("Never commit to main"));
        assert!(split[2].1.contains("# My preferences"));

        let joined: String = split.iter().map(|(_, body)| body.as_str()).collect();
        for line in MEMORY.lines().filter(|l| !l.trim().is_empty()) {
            assert!(joined.contains(line), "`{line}` was dropped by the split");
        }
    }

    /// A memory file that is one long paragraph has no headings at all, and is
    /// the shape a first-time user is most likely to have.
    #[test]
    fn a_memory_file_with_no_headings_still_classifies() {
        let split = split("Keep your responses concise and lead with the answer.\n");
        assert!(split[0].1.contains("Keep your responses concise"));
    }

    /// **Every fragment is written, including the ones import had nothing for.**
    /// `armada doctor` reports a fragment that is still Armada's words, and it
    /// needs a file to read to do that.
    #[test]
    fn all_three_fragments_come_back_even_when_two_have_nothing_imported() {
        let split = split("## Branching\n\nTrunk based.\n");
        assert_eq!(split.len(), 3);
        for (name, body) in &split {
            assert!(!body.is_empty(), "{name} came back with nothing at all");
            assert!(
                state(body).is_some(),
                "{name} does not say it is still Armada's words"
            );
        }
        assert_eq!(state(&split[0].1), Some(Unedited::Example));
        assert_eq!(state(&split[2].1), Some(Unedited::Imported));
    }

    /// **The file a real reader opened and could not interpret.**
    ///
    /// After a default run `expectations.md` was two comments about where it had
    /// come from and nothing else. It now says who reads it, when, and what to
    /// write, and carries examples under a heading that says they are examples —
    /// because a file that explains its own provenance and not its purpose is a
    /// file nobody can act on.
    #[test]
    fn a_fragment_import_found_nothing_for_says_what_to_write_in_it() {
        let expectations = &split("")[1].1;

        assert!(expectations.contains("# Expectations"), "{expectations}");
        assert!(
            expectations.contains("before it tells you a Job is done"),
            "it does not say who reads it and when: {expectations}"
        );
        assert!(
            expectations.contains("Write what has to be true"),
            "it does not say what to write: {expectations}"
        );
        assert!(
            expectations.contains("## Examples, not your answer — replace them"),
            "the examples are not marked as replaceable: {expectations}"
        );
        for example in Fragment::Expectations.examples() {
            assert!(expectations.contains(example), "{example} is missing");
        }

        // And it is still reported as not yours, which is the half that makes
        // writing examples unasked safe.
        assert_eq!(state(expectations), Some(Unedited::Example));
    }

    /// All three, not just the one that was complained about.
    #[test]
    fn every_fragment_says_who_reads_it_and_what_to_write() {
        for (index, (name, body)) in split("").iter().enumerate() {
            let which = Fragment::of(name).expect("a fragment names itself");
            assert!(body.contains(which.title()), "{name} has no heading");
            assert!(body.contains(which.read_by()), "{name}: who reads it?");
            assert!(body.contains(which.asks()), "{name}: what do I write?");
            assert!(!which.examples().is_empty(), "{name} has no examples");
            assert_eq!(index, FRAGMENTS.iter().position(|f| f == name).unwrap());
        }
    }

    /// **An imported fragment says what it is for too.** The purpose line is not
    /// a consolation for an empty file — a reader who has just had his memory
    /// file carved into three needs it more, not less.
    #[test]
    fn an_imported_fragment_carries_the_purpose_as_well_as_the_content() {
        let voice = &split("## Verbosity\n\n150 words maximum per response.\n")[0].1;
        assert!(voice.contains("# Voice"), "{voice}");
        assert!(voice.contains(Fragment::Voice.read_by()), "{voice}");
        assert!(voice.contains("150 words maximum per response."), "{voice}");
        assert!(
            !voice.contains("Lead with the answer. The reasoning"),
            "an imported fragment was given examples as well: {voice}"
        );
        assert_eq!(state(voice), Some(Unedited::Imported));
    }

    /// **The marker is machine-readable, and a rewritten file has none.**
    ///
    /// `armada doctor` used to match on the prose *"Imported from CLAUDE.md"*,
    /// which is a sentence somebody could reasonably keep after replacing
    /// everything under it — and then be told for ever that the file was not
    /// his.
    #[test]
    fn a_file_with_no_marker_is_yours() {
        assert_eq!(state("# Voice\n\nAnswer first.\n"), None);
        assert_eq!(state(""), None);
        assert_eq!(Unedited::Example.said(), "still Armada's example text");
        assert_eq!(Unedited::Imported.said(), "still as imported");
    }

    /// An empty input is not an error: a machine with no `CLAUDE.md` is
    /// ordinary, and it gets three usable templates.
    #[test]
    fn an_empty_memory_file_produces_three_templates() {
        let split = split("");
        assert_eq!(split.len(), 3);
        for (_, body) in &split {
            assert_eq!(state(body), Some(Unedited::Example));
        }
    }

    /// A `#` inside a fenced block is a shell comment. Treating it as a heading
    /// cuts a code sample in half and files the two halves separately.
    #[test]
    fn a_hash_inside_a_fenced_block_is_not_a_heading() {
        let split = split("## Branching\n\n```sh\n# not a heading\ngit switch -c work\n```\n");
        let how = &split[2].1;
        assert!(how.contains("# not a heading"), "{how}");
        assert!(how.contains("git switch -c work"), "{how}");
    }
}
