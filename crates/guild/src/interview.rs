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

/// One question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    /// Its position, one-based. Shown as `n/5`.
    pub number: usize,
    /// The question itself.
    pub prompt: &'static str,
    /// What pressing enter does. **Always present**, because a question whose
    /// default is invisible is a question you cannot skip at one in the
    /// morning.
    pub hint: &'static str,
    /// The guild file the answer lands in.
    pub writes: &'static str,
}

/// The five, in order.
pub const QUESTIONS: [Question; 5] = [
    Question {
        number: 1,
        prompt: "How should agents write to you?",
        hint: "(enter to keep what import found)",
        writes: "voice.md",
    },
    Question {
        number: 2,
        prompt: "What does \"done\" mean — coverage, review, commit style?",
        hint: "(enter to keep what import found)",
        writes: "expectations.md",
    },
    Question {
        number: 3,
        prompt: "How do you work? Branching, when to ask versus decide, parallelism.",
        hint: "(enter to keep what import found)",
        writes: "how-i-work.md",
    },
    Question {
        number: 4,
        prompt: "Default budget ceilings — iterations, tokens, wall clock?",
        hint: "(enter for the per-workflow ceilings, e.g. 20, 600k, 90m)",
        writes: "workflows/*.yml",
    },
    Question {
        number: 5,
        prompt: "A private git remote to sync your guild to?",
        hint: "(enter to leave sync off — export still works)",
        writes: "machine.yml",
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

    /// How many of the five were left at their default.
    pub fn skipped(&self) -> usize {
        [
            self.voice.is_none(),
            self.expectations.is_none(),
            self.how_i_work.is_none(),
            self.ceilings.is_none(),
            self.remote.is_none(),
        ]
        .iter()
        .filter(|defaulted| **defaulted)
        .count()
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
                !question.hint.is_empty(),
                "question {} has no visible default, so it cannot be skipped \
                 by anyone who has not read the source",
                question.number
            );
            assert!(!question.writes.is_empty());
        }
    }

    /// The first question, exactly as `tests/golden/render/init-machine.plain`
    /// spells it. The fixture is the specification and this is the text it
    /// froze.
    #[test]
    fn the_first_question_is_the_one_the_agreed_layout_shows() {
        assert_eq!(QUESTIONS[0].prompt, "How should agents write to you?");
        assert_eq!(QUESTIONS[0].hint, "(enter to keep what import found)");
    }

    /// `--defaults` leaves a working guild and a report that says so.
    #[test]
    fn defaulting_everything_is_five_skipped_and_not_an_error() {
        let answers = Answers::all_defaulted();
        assert_eq!(answers.skipped(), COUNT);
        assert_eq!(answers.fragment("voice.md"), None);
        assert_eq!(answers.ceilings_for("bug"), Ceilings::AUTONOMOUS);
        assert_eq!(answers.ceilings_for("plan"), Ceilings::ADVISORY);
    }

    /// An answered interview is nothing skipped — the case the agreed layout's
    /// `5 questions, 0 skipped` summary shows.
    #[test]
    fn answering_everything_is_nothing_skipped() {
        let answers = Answers {
            voice: Some("Lead with the answer.".to_string()),
            expectations: Some("Tests pass.".to_string()),
            how_i_work: Some("Trunk based.".to_string()),
            ceilings: Some(Ceilings::AUTONOMOUS),
            remote: Some("git@example.com:me/guild.git".to_string()),
        };
        assert_eq!(answers.skipped(), 0);
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
