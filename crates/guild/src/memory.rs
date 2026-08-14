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
/// **The header is the whole reason a fragment is safe to write without asking
/// anyone.** It states that a machine carved this out, that the carving is a
/// guess, and that editing the file is the supported path — the rule
/// `PLAN.md` §13.4 states as *"a tool that can only be configured through a
/// wizard is a tool you cannot fix at one in the morning."*
fn fragment(which: Fragment, parts: &[String]) -> String {
    let subject = match which {
        Fragment::Voice => "how you want to be spoken to",
        Fragment::Expectations => "what \"done\" means",
        Fragment::HowIWork => "how you work — process, tooling, and what to decide without asking",
    };
    let mut out = format!(
        "<!-- Imported from CLAUDE.md by `armada guild init`. This is {subject}.\n\
         \n\
         The split between voice.md, expectations.md and how-i-work.md is a guess made by\n\
         reading headings, and nothing asked you to confirm it. Edit this file directly —\n\
         that is the supported path, not a workaround. Anything the interview asked you\n\
         wins over anything here. -->\n"
    );
    if parts.is_empty() {
        out.push_str(
            "\n<!-- Import found nothing here. `armada doctor` reports this fragment as\n\
             still-as-imported until you write something. -->\n",
        );
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

    /// **Every fragment is written, including the empty ones.** `armada doctor`
    /// reports a fragment that is still whatever import produced, and it needs
    /// a file to compare against to do that.
    #[test]
    fn all_three_fragments_come_back_even_when_two_are_empty() {
        let split = split("## Branching\n\nTrunk based.\n");
        assert_eq!(split.len(), 3);
        for (name, body) in &split {
            assert!(!body.is_empty(), "{name} came back with nothing at all");
            assert!(
                body.contains("Imported from CLAUDE.md"),
                "{name} does not say where it came from"
            );
        }
        assert!(split[0].1.contains("Import found nothing here"));
    }

    /// An empty input is not an error: a machine with no `CLAUDE.md` is
    /// ordinary, and it gets three headers and no content.
    #[test]
    fn an_empty_memory_file_produces_three_empty_fragments() {
        let split = split("");
        assert_eq!(split.len(), 3);
        for (_, body) in &split {
            assert!(body.contains("Import found nothing here"));
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
