//! The seven questions, and the default behind every one of them.
//!
//! `PLAN.md` §13.4 fixes both the set and the shape:
//!
//! | # | Asks | Writes | Default if skipped |
//! |---|---|---|---|
//! | 1 | How should agents write to you? | `voice.md` | what import wrote |
//! | 2 | What does "done" mean? | `expectations.md` | what import wrote |
//! | 3 | How do you work? | `how-i-work.md` | what import wrote |
//! | 4 | Iterations before a Job stops | `workflows/*.yml` | `20` |
//! | 5 | Tokens before a Job stops | `workflows/*.yml` | `600k` |
//! | 6 | Minutes before a Job stops | `workflows/*.yml` | `90m` |
//! | 7 | A private git remote to sync to | `machine.yml` | none — sync off, `export` still works |
//!
//! # One limit per question, because one question could not carry three
//!
//! Questions 4–6 were **one** question until this split, and it asked for
//! `20, 600k, 90m` — a positional triple, comma separated, in which `m` meant
//! *minutes* in the third slot and *nothing at all* in the second, where `k`
//! meant thousands. It was reported as confusing, a first pass made only its
//! default typeable, and it was reported as confusing again. That is the
//! diagnosis: the shape was wrong, not the wording. Three decisions in one
//! answer means a reader who knows exactly what he wants for one of them still
//! cannot answer without deciding the other two, and positional units mean he
//! cannot check his answer by reading it back.
//!
//! So each limit is now its own question taking **one number**, with its own
//! visible default, and each says in its own prompt what happens when the
//! number is reached.
//!
//! # What they do when reached: stop and ask, never abort
//!
//! Every workflow's budget carries `on_exhausted: needs_human`
//! (`armada_core::fleet::workflow::OnExhausted`), and that is the only value it
//! has. `armada_core::fleet::job::exhausted` returns which ceiling was reached;
//! the Drone stops, the Job records what it spent and where it got to, and it is
//! raised to the inbox as `NEEDS_HUMAN`, which settles the Job to `PAUSED`.
//!
//! **Nothing is discarded and nothing is rolled back**, so the prompts say
//! *stops and asks you* rather than *aborts*. The two are different products and
//! a person choosing a number is entitled to know which one he is buying.
//!
//! # Why there is no question about money
//!
//! A Job's `Spend` does carry `cost_usd`, summed from each turn's
//! `total_cost_usd` — but `Budget` has no dollar field and
//! [`exhausted`](armada_core::fleet::job::exhausted) checks iterations, tokens
//! and wall clock and nothing else. A dollar ceiling asked for here would be a
//! number the engine never reads, which is worse than not asking: it would read
//! as a spending cap and stop nothing.
//!
//! Dollars are **reported** rather than **capped**, and the token limit is the
//! one that actually bites. Adding a real money ceiling is a change to `Budget`
//! and to `exhausted`, and it belongs there rather than in a question.
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
//! same seven questions be driven from an interactive session and from a fixture
//! with no branching in between.

use serde::{Deserialize, Serialize};

/// How the answer is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Paragraphs. Gets a real editor: wrapping, arrow keys, and a paste of
    /// several paragraphs that arrives intact.
    Prose,
    /// One short structured value — one ceiling, a remote. A single line,
    /// because an editor for eleven characters is ceremony.
    Line,
}

/// One question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    /// Its position, one-based. Shown as `n/7`.
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
    /// import found*, *600k*. Always present, because a question whose default
    /// is invisible is a question you cannot skip at one in the morning.
    ///
    /// **For a question taking a number this is the number itself**, typeable
    /// as it stands, and it is also what the `now` line shows — one constant,
    /// so the hint and the standing value cannot drift apart.
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

/// The seven, in order.
pub const QUESTIONS: [Question; 7] = [
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
    // **Three questions, one number each, and the first of them teaches the
    // word.** They were a single question taking `20, 600k, 90m` — see the
    // module docs for why a positional triple could not be rescued by rewording
    // it. Each prompt ends in *stops and asks you*, because that is what
    // `on_exhausted: needs_human` actually does and *aborts* would be a
    // different product.
    Question {
        number: 4,
        prompt: "How many iterations should one Job run before it stops and asks you?",
        purpose: "A Job is one piece of work you asked for — it gets its own \
                  branch and its own limits. One whole number here: at this \
                  many turns it stops, keeps everything it has done, and waits \
                  in your inbox for you to say whether to carry on.",
        // **The default is a value, not a description.** "the per-workflow
        // ceilings" is a sentence you cannot type; `20` is the answer *and* the
        // format, which is what a reader who has never set a ceiling needs.
        keeps: "20",
        writes: "workflows/*.yml",
        shape: Shape::Line,
    },
    Question {
        number: 5,
        prompt: "How many tokens should one Job spend before it stops and asks you?",
        purpose: "One number — write it as 600k or as 600000, both read the \
                  same. It counts every token that Job spends, and at this many \
                  it stops and waits for you exactly as the iteration limit \
                  does. This is the limit that bounds what a Job costs.",
        keeps: "600k",
        writes: "workflows/*.yml",
        shape: Shape::Line,
    },
    Question {
        number: 6,
        prompt: "How long should one Job run before it stops and asks you?",
        purpose: "One number, in minutes — 90 is an hour and a half. It is \
                  clock time from the moment the Job starts, not time spent \
                  thinking, so a Job left waiting overnight reaches it. It then \
                  stops and waits for you like the other two.",
        keeps: "90m",
        writes: "workflows/*.yml",
        shape: Shape::Line,
    },
    Question {
        number: 7,
        prompt: "Where should your guild sync to?",
        purpose: "A git URL, or a folder — iCloud Drive, a NAS, a drive you \
                  plug in. Given a folder, Armada makes it a git remote for you.",
        keeps: "sync off, export still works",
        writes: "machine.yml",
        shape: Shape::Line,
    },
];

/// How many there are. Named rather than spelled `7` at four call sites,
/// because the `n/7` in the render and the `7 questions` in the summary are the
/// same number and drifting apart is the whole failure. It was `5` until
/// questions 4–6 stopped being one question.
pub const COUNT: usize = QUESTIONS.len();

/// The numbers of the three questions that each set one ceiling, in the order
/// the ceilings are written. Named so that the interview driver, the `now` line
/// and the tests all agree on which question is which without spelling `4`,
/// `5` and `6` in four places.
pub const CEILING_QUESTIONS: [usize; 3] = [4, 5, 6];

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

/// Read question 4's answer — a count of iterations, as `20`.
///
/// **Refuses rather than guesses.** A ceiling silently read as `0` is a fleet
/// that stops after no iterations, and the caller can re-ask a question far
/// more cheaply than a user can debug that.
pub fn parse_iterations(answer: &str) -> Result<u32, String> {
    let value = number(answer, "iterations", "20")?;
    u32::try_from(value).map_err(|_| refusal(answer, "iterations", "20"))
}

/// Read question 5's answer — a token count, as `600k` or `600000`.
///
/// **`k` is thousands and it is the only suffix**, because there is exactly one
/// unit here. The old triple read `m` as *minutes* in one slot and as nothing in
/// another, which is precisely the ambiguity the split removes.
pub fn parse_tokens(answer: &str) -> Result<u64, String> {
    let text = answer.trim();
    let value = match text.strip_suffix(['k', 'K']) {
        Some(head) => number(head, "tokens", "600k")?
            .checked_mul(1_000)
            .ok_or_else(|| refusal(answer, "tokens", "600k"))?,
        None => number(text, "tokens", "600k")?,
    };
    // **A ceiling under a thousand tokens is refused rather than obeyed.** It is
    // less than a single turn, so it would exhaust every Job on its first one —
    // and the likeliest way to type it is meaning `600k` and writing `600`.
    if value < 1_000 {
        return Err(format!(
            "`{}` is fewer tokens than one turn spends — did you mean `{}k`?",
            answer.trim(),
            answer.trim()
        ));
    }
    Ok(value)
}

/// Read question 6's answer — minutes of wall clock, as `90` or `90m`.
///
/// The trailing `m` is accepted because the hint shows `90m`, and a hint you
/// cannot type back is a lie for exactly one keystroke.
pub fn parse_minutes(answer: &str) -> Result<u32, String> {
    let text = answer.trim();
    let digits = text.strip_suffix(['m', 'M']).unwrap_or(text);
    let value = number(digits, "minutes", "90")?;
    u32::try_from(value).map_err(|_| refusal(answer, "minutes", "90"))
}

/// One whole number, refused if it is not one or if it is zero.
///
/// `unit` and `example` are what the refusal says, so a person who mistyped is
/// told what this question wanted rather than that a parse failed.
fn number(text: &str, unit: &str, example: &str) -> Result<u64, String> {
    let value: u64 = text
        .trim()
        .parse()
        .map_err(|_| refusal(text, unit, example))?;
    if value == 0 {
        return Err(format!(
            "`{}` is zero, which is not a ceiling — try `{example}`",
            text.trim()
        ));
    }
    Ok(value)
}

/// What a refused answer says: what was wanted, and something typeable.
fn refusal(text: &str, unit: &str, example: &str) -> String {
    format!(
        "`{}` is not a number of {unit} — try `{example}`",
        text.trim()
    )
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
    /// Question 4 — iterations, on its own.
    ///
    /// **The three ceilings are three fields rather than one `Ceilings`**, and
    /// that is the whole gain of splitting the question: each is independently
    /// answered or defaulted, so a person who cares only about tokens keeps
    /// `design`'s 15 iterations and `bug`'s 20 rather than flattening both to
    /// whichever number he had to invent to get past the other two slots.
    pub iterations: Option<u32>,
    /// Question 5 — tokens, on its own.
    pub tokens: Option<u64>,
    /// Question 6 — wall clock in minutes, on its own.
    pub wall_clock_minutes: Option<u32>,
    /// Question 7.
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

    /// How many of the seven you typed an answer to.
    pub fn answered(&self) -> usize {
        [
            self.voice.is_some(),
            self.expectations.is_some(),
            self.how_i_work.is_some(),
            self.iterations.is_some(),
            self.tokens.is_some(),
            self.wall_clock_minutes.is_some(),
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
    ///
    /// **Per field, not all-or-nothing.** Each of questions 4–6 that was
    /// answered replaces that one number in every workflow; each that was left
    /// at its default keeps the per-workflow value of §14.6 — so answering only
    /// the token question does not quietly also set `design`'s iterations to
    /// `bug`'s.
    pub fn ceilings_for(&self, workflow: &str) -> Ceilings {
        let per_workflow = Ceilings::for_workflow(workflow);
        Ceilings {
            iterations: self.iterations.unwrap_or(per_workflow.iterations),
            tokens: self.tokens.unwrap_or(per_workflow.tokens),
            wall_clock_minutes: self
                .wall_clock_minutes
                .unwrap_or(per_workflow.wall_clock_minutes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Seven, numbered, each with a default.** The count is load-bearing: it
    /// is the `n/7` in the render and the `7 questions` in the summary.
    #[test]
    fn there_are_seven_questions_each_numbered_and_each_skippable() {
        assert_eq!(COUNT, 7);
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

    /// **Each limit question asks for exactly one number, and its default is
    /// that number.** This is the split, stated as a test: the three questions
    /// carry no comma between them, each shows a default you could type, and
    /// each default parses with the parser that question's answer goes to.
    #[test]
    fn each_limit_question_takes_one_number_and_shows_it_as_its_default() {
        assert_eq!(CEILING_QUESTIONS, [4, 5, 6]);
        assert_eq!(parse_iterations(QUESTIONS[3].keeps), Ok(20));
        assert_eq!(parse_tokens(QUESTIONS[4].keeps), Ok(600_000));
        assert_eq!(parse_minutes(QUESTIONS[5].keeps), Ok(90));
        for number in CEILING_QUESTIONS {
            let question = QUESTIONS[number - 1];
            assert!(
                !question.keeps.contains(','),
                "question {number} still offers a list as its default: {}",
                question.keeps
            );
            assert_eq!(
                question.shape,
                Shape::Line,
                "question {number} wants one number and should not open an editor"
            );
            assert_eq!(question.writes, "workflows/*.yml");
        }
    }

    /// **Each of the three says what happens when the number is reached, and
    /// says the true thing.** `on_exhausted` has one value — `needs_human` —
    /// and it pauses the Job and raises it. A prompt that said *aborts* would
    /// be describing a different product to a person choosing a number.
    #[test]
    fn every_limit_question_says_it_stops_and_asks_rather_than_aborts() {
        for number in CEILING_QUESTIONS {
            let question = QUESTIONS[number - 1];
            assert!(
                question.prompt.contains("stops and asks you"),
                "question {number} does not say what the limit does: {}",
                question.prompt
            );
            let says = format!("{} {}", question.prompt, question.purpose).to_lowercase();
            assert!(
                !says.contains("abort") && !says.contains("give up"),
                "question {number} promises an abort, which is not what happens"
            );
            assert!(
                says.contains("waits") || says.contains("inbox"),
                "question {number} does not say the work is kept and waits for you"
            );
        }
    }

    /// **No question asks for money.** `Budget` has no dollar field and
    /// `exhausted` never looks at `cost_usd`, so a money ceiling asked here
    /// would be a number nothing reads — it would look like a spending cap and
    /// stop nothing.
    #[test]
    fn nothing_asks_for_a_dollar_ceiling_because_none_is_enforced() {
        for question in QUESTIONS {
            let says = format!("{} {}", question.prompt, question.purpose).to_lowercase();
            for money in ["dollar", "usd", "$", " spend per", "per month"] {
                assert!(
                    !says.contains(money),
                    "question {} asks about money: {money}",
                    question.number
                );
            }
        }
    }

    /// Prose gets the editor and a short structured value does not.
    #[test]
    fn the_three_fragments_are_prose_and_the_other_four_are_lines() {
        let shapes: Vec<Shape> = QUESTIONS.iter().map(|q| q.shape).collect();
        assert_eq!(
            shapes,
            vec![
                Shape::Prose,
                Shape::Prose,
                Shape::Prose,
                Shape::Line,
                Shape::Line,
                Shape::Line,
                Shape::Line
            ]
        );
    }

    /// `--defaults` leaves a working guild and a report that says so.
    #[test]
    fn defaulting_everything_keeps_all_seven_and_is_not_an_error() {
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
            iterations: Some(20),
            tokens: Some(600_000),
            wall_clock_minutes: Some(90),
            remote: Some("git@example.com:me/guild.git".to_string()),
        };
        assert_eq!(answers.answered(), COUNT);
        assert_eq!(answers.kept(), 0);
        assert_eq!(answers.fragment("voice.md"), Some("Lead with the answer."));
        assert_eq!(answers.ceilings_for("plan"), Ceilings::AUTONOMOUS);
    }

    /// **An answered limit overrides that one number in every workflow.**
    #[test]
    fn an_answered_ceiling_replaces_the_per_workflow_default_everywhere() {
        let answers = Answers {
            iterations: Some(8),
            tokens: Some(200_000),
            wall_clock_minutes: Some(30),
            ..Answers::default()
        };
        for workflow in ["design", "plan", "feature", "bug"] {
            assert_eq!(answers.ceilings_for(workflow).iterations, 8, "{workflow}");
            assert_eq!(answers.ceilings_for(workflow).tokens, 200_000, "{workflow}");
        }
    }

    /// **The gain of splitting, as a test.** One answered limit changes that
    /// number and nothing else — `design` keeps its 15 iterations and `bug`
    /// keeps its 20, where the old single question forced a person who only
    /// wanted a token ceiling to flatten both.
    #[test]
    fn answering_one_limit_leaves_the_other_two_per_workflow() {
        let answers = Answers {
            tokens: Some(200_000),
            ..Answers::default()
        };
        assert_eq!(answers.answered(), 1);
        for workflow in ["design", "plan", "feature", "bug"] {
            let ceilings = answers.ceilings_for(workflow);
            assert_eq!(ceilings.tokens, 200_000, "{workflow}");
            assert_eq!(
                ceilings.iterations,
                Ceilings::for_workflow(workflow).iterations,
                "{workflow} lost its own iteration ceiling"
            );
            assert_eq!(
                ceilings.wall_clock_minutes,
                Ceilings::for_workflow(workflow).wall_clock_minutes,
                "{workflow} lost its own wall clock"
            );
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
    /// hint is a lie for exactly one keystroke. The triple spelling survives for
    /// reports that show all three at once; it is no longer anything anybody has
    /// to type.
    #[test]
    fn the_spelling_in_the_hint_round_trips() {
        assert_eq!(Ceilings::AUTONOMOUS.written(), "20, 600k, 90m");
        assert_eq!(Ceilings::ADVISORY.written(), "15, 500k, 90m");
        let defaulted = Answers::all_defaulted();
        assert_eq!(defaulted.ceilings_for("bug").written(), "20, 600k, 90m");
        // Every hint is typeable into its own question, and reads back as the
        // number the defaulted interview would have used anyway.
        assert_eq!(
            Ceilings {
                iterations: parse_iterations(QUESTIONS[3].keeps).unwrap(),
                tokens: parse_tokens(QUESTIONS[4].keeps).unwrap(),
                wall_clock_minutes: parse_minutes(QUESTIONS[5].keeps).unwrap(),
            },
            Ceilings::AUTONOMOUS
        );
    }

    /// **Tokens read the same written either way**, because the hint says `600k`
    /// and the purpose says `600000` and a reader may type back either.
    #[test]
    fn a_token_ceiling_reads_the_same_in_both_spellings() {
        assert_eq!(parse_tokens("600k"), parse_tokens("600000"));
        assert_eq!(parse_tokens("600K"), Ok(600_000));
        assert_eq!(parse_tokens(" 200k "), Ok(200_000));
    }

    /// The minutes question shows `90m` and must accept it back, as well as the
    /// bare `90` its purpose asks for.
    #[test]
    fn minutes_are_read_with_or_without_the_suffix_the_hint_shows() {
        assert_eq!(parse_minutes("90m"), Ok(90));
        assert_eq!(parse_minutes("90"), Ok(90));
        assert_eq!(parse_minutes("30M"), Ok(30));
    }

    /// **Refused rather than guessed.** A ceiling misread as zero is a fleet
    /// that stops immediately, and re-asking a question is far cheaper than
    /// debugging that. Every refusal names the unit and offers something
    /// typeable.
    #[test]
    fn an_unreadable_ceiling_is_refused_and_says_what_was_expected() {
        for answer in ["", "lots", "0", "20, 600k, 90m", "-4"] {
            let refusal = parse_iterations(answer).unwrap_err();
            assert!(refusal.contains("20"), "`{answer}`: {refusal}");
        }
        for answer in ["", "heaps", "0", "600k, 90m"] {
            let refusal = parse_tokens(answer).unwrap_err();
            assert!(refusal.contains("600k"), "`{answer}`: {refusal}");
        }
        for answer in ["", "ages", "0m", "90m, 20"] {
            let refusal = parse_minutes(answer).unwrap_err();
            assert!(refusal.contains("90"), "`{answer}`: {refusal}");
        }
    }

    /// **The old triple is refused by all three, rather than half-read by one.**
    /// Somebody who did this interview before will type `20, 600k, 90m` out of
    /// habit, and reading the leading `20` as his answer would silently set a
    /// ceiling he did not choose for the other two.
    #[test]
    fn the_old_triple_is_refused_rather_than_partly_understood() {
        assert!(parse_iterations("20, 600k, 90m").is_err());
        assert!(parse_tokens("20, 600k, 90m").is_err());
        assert!(parse_minutes("20, 600k, 90m").is_err());
    }

    /// **A token ceiling under one turn is refused**, because the way it gets
    /// typed is meaning `600k` and writing `600`, and obeying it would exhaust
    /// every Job on its first turn.
    #[test]
    fn a_token_ceiling_smaller_than_a_turn_is_refused_with_the_likely_fix() {
        let refusal = parse_tokens("600").unwrap_err();
        assert!(refusal.contains("600k"), "{refusal}");
        assert_eq!(parse_tokens("1000"), Ok(1_000), "a thousand is allowed");
    }
}
