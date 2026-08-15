//! The five questions, and the default behind every one of them.
//!
//! `PLAN.md` §13.4 fixes both the set and the shape:
//!
//! | # | Asks | Writes | Default if skipped |
//! |---|---|---|---|
//! | 1 | How should agents write to you? | `voice.md` | what import wrote |
//! | 2 | What does "done" mean? | `expectations.md` | what import wrote |
//! | 3 | How do you work? | `how-i-work.md` | what import wrote |
//! | 4 | Default budget ceilings | `workflows/*.yml` | the per-workflow ceilings of §14.6 |
//! | 5 | A private git remote to sync to | `machine.yml` | none — sync off, `export` still works |
//!
//! # Three rules, each of which was decided rather than fallen into
//!
//! **It asks only what it cannot read from the machine.** Import has already
//! run and has done most of the work; asking you to confirm what it found would
//! be asking you to review a machine's reading of your own memory file, which
//! is more work than answering and produces a worse answer.
//!
//! **Questions are not pre-filled with the import's guess.** They ask fresh.
//! Import populates the files, these questions ask, and your answer wins where
//! the two overlap.
//!
//! **Every question has a default and `--defaults` takes all of them.** A
//! skipped interview leaves a *working* guild, and `armada doctor` reports it as
//! incomplete, naming the fragments that are still whatever import produced.
//! What the interview must never do is finish silently in a state that looks
//! configured and is not — the same rule the privacy gate follows
//! (`ARCHITECTURE.md` §2.4).
//!
//! # Nothing here reads or writes anything
//!
//! [`Question`] is data and [`Answers`] is a value. Who asks — a terminal, a
//! `--defaults` flag, a test — is the caller's business, which is what lets the
//! same five questions be driven from an interactive session and from a fixture
//! with no branching in between.

use serde::{Deserialize, Serialize};

/// How the answer is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Paragraphs. Gets a real editor: wrapping, arrow keys, and a paste of
    /// several paragraphs that arrives intact.
    Prose,
    /// One short structured value — a triple of ceilings, a remote. A single
    /// line, because an editor for eleven characters is ceremony.
    Line,
}

/// One question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    /// Its position, one-based. Shown as `n/5`.
    pub number: usize,
    /// The question itself, in one line.
    pub prompt: &'static str,
    /// **What answer is wanted, and what the answer is for.**
    ///
    /// The prompt alone was not enough: a real first run produced *"What does
    /// "done" mean — coverage, review, commit style?"* and the person answering
    /// could not tell what shape of sentence was wanted or where it would end
    /// up. Each question writes one specific file for one specific purpose, and
    /// this is where it says so.
    pub purpose: &'static str,
    /// **What the default answer is**, as the object of a sentence — *what
    /// import found*, *20, 600k, 90m*. Always present, because a question whose
    /// default is invisible is a question you cannot skip at one in the morning.
    ///
    /// The *key* that takes it is not here: it is `enter` at a single-line
    /// prompt and `esc` in the text area, and only the render knows which one
    /// this question got.
    pub keeps: &'static str,
    /// The guild file the answer lands in.
    pub writes: &'static str,
    /// How it is typed.
    pub shape: Shape,
}

/// The five, in order.
pub const QUESTIONS: [Question; 5] = [
    Question {
        number: 1,
        prompt: "How should agents write to you?",
        purpose: "Tone, length, and what to lead with. Every agent reads this \
                  before it says anything.",
        keeps: "what import found",
        writes: "voice.md",
        shape: Shape::Prose,
    },
    Question {
        number: 2,
        prompt: "When is work actually finished?",
        purpose: "What must be true before an agent tells you it is done: tests \
                  passing, a review, a branch, a changelog entry. Workflows gate \
                  on this.",
        keeps: "what import found",
        writes: "expectations.md",
        shape: Shape::Prose,
    },
    Question {
        number: 3,
        prompt: "How should agents work in your repos?",
        purpose: "Branching, what to do without asking, what to always ask about \
                  first.",
        keeps: "what import found",
        writes: "how-i-work.md",
        shape: Shape::Prose,
    },
    Question {
        number: 4,
        prompt: "How much should one Job spend before it stops and asks you?",
        purpose: "Iterations, tokens and wall clock, in that order, like \
                  8, 200k, 30m. A Job that hits any one of them stops and \
                  reports rather than carrying on.",
        // **The default is spelled out rather than described.** "the
        // per-workflow ceilings" is a sentence you cannot type; `20, 600k, 90m`
        // is the answer *and* the format, which is what a reader who has never
        // seen a ceiling before actually needs.
        keeps: "20, 600k, 90m",
        writes: "workflows/*.yml",
        shape: Shape::Line,
    },
    Question {
        number: 5,
        prompt: "Where should your guild sync to?",
        purpose: "A git URL, or a folder — iCloud Drive, a NAS, a drive you \
                  plug in. Given a folder, Armada makes it a git remote for you.",
        keeps: "sync off, export still works",
        writes: "machine.yml",
        shape: Shape::Line,
    },
];

/// How many there are. Named rather than spelled `5` at four call sites,
/// because the `n/5` in the render and the `5 questions` in the summary are the
/// same number and drifting apart is the whole failure.
pub const COUNT: usize = QUESTIONS.len();

/// A budget ceiling triple (`PLAN.md` §14.3, §14.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ceilings {
    /// How many times the loop may go round.
    pub iterations: u32,
    /// Tokens, whole.
    pub tokens: u64,
    /// Wall clock, in minutes.
    pub wall_clock_minutes: u32,
}

impl Ceilings {
    /// `design` and `plan`: they end at you regardless, so this is a runaway
    /// guard rather than a budget (`PLAN.md` §14.6).
    pub const ADVISORY: Ceilings = Ceilings {
        iterations: 15,
        tokens: 500_000,
        wall_clock_minutes: 90,
    };

    /// `feature` and `bug`: they can close autonomously, so this bounds real
    /// spend.
    pub const AUTONOMOUS: Ceilings = Ceilings {
        iterations: 20,
        tokens: 600_000,
        wall_clock_minutes: 90,
    };

    /// The default for a named starter workflow.
    pub fn for_workflow(name: &str) -> Ceilings {
        match name {
            "design" | "plan" => Ceilings::ADVISORY,
            _ => Ceilings::AUTONOMOUS,
        }
    }

    /// `20, 600k, 90m` — the spelling the hint offers and the one the answer is
    /// read back in.
    pub fn written(&self) -> String {
        format!(
            "{}, {}, {}m",
            self.iterations,
            thousands(self.tokens),
            self.wall_clock_minutes
        )
    }
}

/// `600000` as `600k`, and `1500000` as `1500k`. Whole thousands only, because
/// a ceiling is a round number and `600.5k` reads as a measurement.
fn thousands(tokens: u64) -> String {
    if tokens.is_multiple_of(1_000) && tokens >= 1_000 {
        format!("{}k", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}

/// Read `20, 600k, 90m` back.
///
/// **Refuses rather than guesses.** A ceiling silently read as `0` is a fleet
/// that stops after no iterations, and the caller can re-ask a question far
/// more cheaply than a user can debug that.
pub fn parse_ceilings(answer: &str) -> Result<Ceilings, String> {
    let parts: Vec<&str> = answer.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!(
            "expected three values — iterations, tokens, wall clock — as `{}`",
            Ceilings::AUTONOMOUS.written()
        ));
    }
    Ok(Ceilings {
        iterations: number(parts[0], 1)? as u32,
        tokens: number(parts[1], 1_000)?,
        wall_clock_minutes: number(parts[2], 1)? as u32,
    })
}

/// One value, with `k`/`m` read as thousands and `m` also read as the minutes
/// suffix. The unit is decided by which position it is in, so `90m` is ninety
/// minutes and `600k` is six hundred thousand.
fn number(text: &str, scale: u64) -> Result<u64, String> {
    let (digits, multiplier) = match text.strip_suffix(['k', 'K']) {
        Some(head) => (head, scale.max(1_000)),
        None => (text.trim_end_matches(['m', 'M']), 1),
    };
    let value: u64 = digits
        .trim()
        .parse()
        .map_err(|_| format!("`{text}` is not a number"))?;
    let scaled = value * multiplier;
    if scaled == 0 {
        return Err(format!("`{text}` is zero, which is not a ceiling"));
    }
    Ok(scaled)
}

/// What the interview came back with. `None` on a field means the default was
/// taken, which is what `armada doctor` reports as still-as-imported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    /// Question 1.
    pub voice: Option<String>,
    /// Question 2.
    pub expectations: Option<String>,
    /// Question 3.
    pub how_i_work: Option<String>,
    /// Question 4.
    pub ceilings: Option<Ceilings>,
    /// Question 5.
    pub remote: Option<String>,
}

impl Answers {
    /// Every question defaulted — what `--defaults` produces.
    ///
    /// **A working guild, not an empty one.** Import has already written the
    /// three fragments and the starters are already copied; what defaulting
    /// leaves behind is a guild whose fragments are a machine's reading of your
    /// memory file rather than your own words, which is exactly what `doctor`
    /// reports.
    pub fn all_defaulted() -> Answers {
        Answers::default()
    }

    /// How many of the five you typed an answer to.
    pub fn answered(&self) -> usize {
        [
            self.voice.is_some(),
            self.expectations.is_some(),
            self.how_i_work.is_some(),
            self.ceilings.is_some(),
            self.remote.is_some(),
        ]
        .iter()
        .filter(|given| **given)
        .count()
    }

    /// How many were left at what import found.
    ///
    /// **Not "skipped".** Pressing enter is what the hint instructs and it
    /// accepts a value; a report that called it skipping told a person who had
    /// followed the instructions that he had done nothing. `armada doctor` still
    /// names the fragments that are a machine's reading of your memory file
    /// rather than your own words, which is the fact worth carrying.
    pub fn kept(&self) -> usize {
        COUNT - self.answered()
    }

    /// The answer for a fragment, if the interview got one.
    ///
    /// **Your answer wins over the import where they overlap** (`PLAN.md`
    /// §13.4) — which is this function returning `Some` and the caller
    /// overwriting what import wrote.
    pub fn fragment(&self, file: &str) -> Option<&str> {
        match file {
            "voice.md" => self.voice.as_deref(),
            "expectations.md" => self.expectations.as_deref(),
            "how-i-work.md" => self.how_i_work.as_deref(),
            _ => None,
        }
    }

    /// The ceilings a named workflow should be written with.
    pub fn ceilings_for(&self, workflow: &str) -> Ceilings {
        self.ceilings
            .unwrap_or_else(|| Ceilings::for_workflow(workflow))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Five, numbered, each with a default.** The count is load-bearing: it
    /// is the `n/5` in the render and the `5 questions` in the summary.
    #[test]
    fn there_are_five_questions_each_numbered_and_each_skippable() {
        assert_eq!(COUNT, 5);
        for (index, question) in QUESTIONS.iter().enumerate() {
            assert_eq!(question.number, index + 1);
            assert!(
                !question.keeps.is_empty(),
                "question {} has no visible default, so it cannot be skipped \
                 by anyone who has not read the source",
                question.number
            );
            assert!(!question.writes.is_empty());
        }
    }

    /// **Every question says what answer is wanted.** The first run of this
    /// interview came back with "the questions do not say what answer is
    /// wanted", so this is the rule stated as a test: a prompt is a question and
    /// the purpose is the answer's shape, and neither may be missing.
    #[test]
    fn every_question_says_what_answer_it_wants() {
        for question in QUESTIONS {
            assert!(
                question.prompt.ends_with('?'),
                "question {} is not a question",
                question.number
            );
            assert!(
                question.purpose.len() > question.prompt.len(),
                "question {}'s purpose says less than its prompt does",
                question.number
            );
        }
    }

    /// **Question 4's default is a value, not a description.** He had no idea
    /// what to type, and "the per-workflow ceilings" is not a thing you can
    /// type — the default is now the value itself, and the purpose carries the
    /// format.
    #[test]
    fn the_budget_question_shows_its_default_as_something_you_could_type() {
        assert_eq!(QUESTIONS[3].keeps, Ceilings::AUTONOMOUS.written());
        assert!(parse_ceilings(QUESTIONS[3].keeps).is_ok());
        assert!(
            parse_ceilings("8, 200k, 30m").is_ok(),
            "the purpose offers a shape the parser refuses"
        );
        assert!(QUESTIONS[3].purpose.contains("8, 200k, 30m"));
    }

    /// Prose gets the editor and a short structured value does not.
    #[test]
    fn the_three_fragments_are_prose_and_the_other_two_are_lines() {
        let shapes: Vec<Shape> = QUESTIONS.iter().map(|q| q.shape).collect();
        assert_eq!(
            shapes,
            vec![
                Shape::Prose,
                Shape::Prose,
                Shape::Prose,
                Shape::Line,
                Shape::Line
            ]
        );
    }

    /// `--defaults` leaves a working guild and a report that says so.
    #[test]
    fn defaulting_everything_keeps_all_five_and_is_not_an_error() {
        let answers = Answers::all_defaulted();
        assert_eq!(answers.answered(), 0);
        assert_eq!(answers.kept(), COUNT);
        assert_eq!(answers.fragment("voice.md"), None);
        assert_eq!(answers.ceilings_for("bug"), Ceilings::AUTONOMOUS);
        assert_eq!(answers.ceilings_for("plan"), Ceilings::ADVISORY);
    }

    /// An answered interview keeps nothing.
    #[test]
    fn answering_everything_keeps_nothing() {
        let answers = Answers {
            voice: Some("Lead with the answer.".to_string()),
            expectations: Some("Tests pass.".to_string()),
            how_i_work: Some("Trunk based.".to_string()),
            ceilings: Some(Ceilings::AUTONOMOUS),
            remote: Some("git@example.com:me/guild.git".to_string()),
        };
        assert_eq!(answers.answered(), COUNT);
        assert_eq!(answers.kept(), 0);
        assert_eq!(answers.fragment("voice.md"), Some("Lead with the answer."));
    }

    /// **An answer overrides the per-workflow default for every workflow.**
    /// Question 4 asks one triple; §14.6's two triples are what it replaces.
    #[test]
    fn an_answered_ceiling_replaces_the_per_workflow_default_everywhere() {
        let answers = Answers {
            ceilings: Some(Ceilings {
                iterations: 8,
                tokens: 200_000,
                wall_clock_minutes: 30,
            }),
            ..Answers::default()
        };
        for workflow in ["design", "plan", "feature", "bug"] {
            assert_eq!(answers.ceilings_for(workflow).iterations, 8, "{workflow}");
        }
    }

    /// The two triples of §14.6, and the reason they differ: design and plan
    /// end at you regardless, feature and bug can close on their own.
    #[test]
    fn the_starter_ceilings_are_the_ones_the_plan_specifies() {
        assert_eq!(
            Ceilings::for_workflow("design"),
            Ceilings {
                iterations: 15,
                tokens: 500_000,
                wall_clock_minutes: 90
            }
        );
        assert_eq!(
            Ceilings::for_workflow("feature"),
            Ceilings {
                iterations: 20,
                tokens: 600_000,
                wall_clock_minutes: 90
            }
        );
    }

    /// What the hint offers has to be what the answer is read back in, or the
    /// hint is a lie for exactly one keystroke.
    #[test]
    fn the_spelling_in_the_hint_round_trips() {
        let written = Ceilings::AUTONOMOUS.written();
        assert_eq!(written, "20, 600k, 90m");
        assert_eq!(parse_ceilings(&written), Ok(Ceilings::AUTONOMOUS));
        assert_eq!(
            parse_ceilings("15, 500k, 90m"),
            Ok(Ceilings::ADVISORY),
            "the other triple round-trips too"
        );
    }

    /// **Refused rather than guessed.** A ceiling misread as zero is a fleet
    /// that stops immediately, and re-asking a question is far cheaper than
    /// debugging that.
    #[test]
    fn an_unreadable_ceiling_is_refused_and_says_what_was_expected() {
        for answer in ["", "20", "20, 600k", "lots, 600k, 90m", "0, 600k, 90m"] {
            let refusal = parse_ceilings(answer).unwrap_err();
            assert!(!refusal.is_empty(), "`{answer}` was accepted");
        }
        assert!(parse_ceilings("20").unwrap_err().contains("20, 600k, 90m"));
    }
}
