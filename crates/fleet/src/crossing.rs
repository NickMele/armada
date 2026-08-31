//! What crosses a step boundary when the process does not.
//!
//! # Why this is a value and not three arguments
//!
//! A Drone belongs to a step, so the one starting part two never saw part one
//! worked. Everything the process that is gone would have held has to be handed
//! over in the next Drone's opening brief, and that list is not closed:
//! [`Crossed`] carries two, and `#207` adds a redirect that arrived while no
//! Drone was there to take it. A third item adds a field and a method, and no
//! caller carrying the first two is touched.
//!
//! # An injected turn does not survive being moved into an opening brief
//!
//! Every sentence of `verification::OutcomeTurn` is addressed to the Drone that
//! did the work: "Go on to Implement" is a continuation, and `Verified` says
//! "the checks that cover what you changed" to a Drone that changed nothing. So
//! what crosses is the **facts** of the outcome and not the rendered turn — a
//! string cannot be re-tensed. [`Cleared`] is the fresh Drone's rendering of
//! them, exactly as [`Reconciling`] is of `verification::TheBaseMoved`: same
//! fact, different tense, two types. [`Reconciling`] moved here from
//! [`briefing`](mod@crate::briefing) for that reason.
//!
//! **Drafted wording**, which `docs/contracts/agent-prompt.md` section 4a says.
//! The product block is not — the contract draws it, and [`Produced::text`]
//! follows what is drawn.
use core_model::{FrozenWorkflow, ResolvedStep, StepEvidence, StepId};
use verification::TheBaseMoved;

/// What a Drone that was not there has to be handed, because the process that
/// would have held it is gone.
///
/// **Everything is optional and nothing is defaulted into prose.** A boundary
/// that carries nothing is the Job's first step, and it renders no block at all
/// rather than a block saying there is nothing — the rail above already says
/// "You are on part 1", and a constant sentence telling a Drone what it can see
/// costs a constant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Crossed {
    produced: Option<Produced>,
    cleared: Option<Cleared>,
}

impl Crossed {
    /// A boundary that carries nothing: a Job's first step, and every spawn
    /// that has not been taught to carry anything yet.
    pub fn nothing() -> Crossed {
        Crossed::default()
    }

    /// What the part before this one produced, where there is a part before
    /// this one.
    ///
    /// Takes the `Option` [`Produced::before`] answers with, so that "there is
    /// no earlier part" stays a case the caller does not have to spell.
    pub fn and_produced(self, produced: Option<Produced>) -> Crossed {
        Crossed { produced, ..self }
    }

    /// That the part before this one got past its gate, and by whose decision.
    pub fn and_cleared(self, cleared: Cleared) -> Crossed {
        Crossed {
            cleared: Some(cleared),
            ..self
        }
    }

    pub(crate) fn produced(&self) -> Option<&Produced> {
        self.produced.as_ref()
    }

    pub(crate) fn cleared(&self) -> Option<&Cleared> {
        self.cleared.as_ref()
    }
}

/// What the part immediately before this one produced, as the record holds it.
///
/// **The part immediately before, and no further back.** The contract draws one
/// block and it names one part. Reaching an arbitrary earlier step's evidence
/// is what `baseline_ref` and `reference_docs` are for, and `reference_docs` is
/// the Judge's yardstick — `docs/concepts/drone.md` keeps it away from a Drone
/// and `config::scope` refuses it.
///
/// **Both the quotation and the path, and neither alone.** Quoting the claim
/// alone reproduces exactly what `#138` closed: the next Drone is handed a
/// sentence a Drone typed about a file, rather than the file. Naming the path
/// alone spends a tool call to read two lines, on every step of every Job, and
/// says nothing at all on a step whose product is the diff. So the claim is
/// quoted because it is free to read, and the path is named because the claim
/// is a summary of something and the something is on disk beside it.
///
/// **`shown_by` is not carried.** `#138` is explicit that the brief points at a
/// path Fleet resolved rather than at whatever the previous Drone typed, and
/// `shown_by` is the string it was talking about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Produced {
    /// One-based, and it is the part number the rail counts with — "part 1",
    /// never a step id.
    part: usize,
    /// What the earlier part claimed its work now does. `None` where the step
    /// is on the record and its evidence is not, which an override advance
    /// leaves behind.
    claimed: Option<String>,
    /// The file that part was asked to write, where it declared one. Resolved
    /// by Fleet from the frozen definition, so no Drone chose it.
    at: Option<String>,
}

impl Produced {
    /// What the part before `at` produced. `None` where `at` is the first part
    /// of the workflow, or is not in it at all.
    ///
    /// **`None` renders nothing and `Some` with no claim renders a sentence.**
    /// The two absences are different and the difference is the one this whole
    /// block was deferred over: a Drone on part 1 sees a rail with nothing
    /// marked done, so silence is accurate; a Drone on part 3 sees part 2
    /// marked done, and silence there reads as a block that was answered.
    ///
    /// **Not built on `AtStep::baseline`,** which the issue proposed. That
    /// resolves an `EvidenceRef` a definition names, and it needs an `AtStep`,
    /// which carries a `Worktree` a brief has no use for. The property it
    /// exists to enforce — strictly earlier — is a property of "the part before
    /// this one" by construction rather than a check that could fail.
    pub fn before(
        workflow: &FrozenWorkflow,
        at: &StepId,
        recorded: &[(StepId, StepEvidence)],
    ) -> Option<Produced> {
        let steps = workflow.steps();
        let here = steps.iter().position(|step| step.id() == at)?;
        let earlier = steps.get(here.checked_sub(1)?)?;
        Some(Produced {
            part: here,
            claimed: recorded
                .iter()
                .find(|(step, _)| step == earlier.id())
                .map(|(_, evidence)| evidence.claimed.clone()),
            at: earlier.deliverable().map(str::to_string),
        })
    }

    /// The block, in the shape `docs/contracts/agent-prompt.md` draws it:
    /// a heading naming the part, and the claim indented under it.
    pub(crate) fn text(&self) -> String {
        let part = self.part;
        let mut block = match &self.claimed {
            Some(claimed) => format!("What part {part} produced:\n  \"{claimed}\""),
            // Said rather than left out, which is `verification::GamingBrief`'s
            // answer to the same absence — it tells the Judge there is no
            // earlier step to measure against rather than handing it a blank.
            None => format!("What part {part} produced:\n  There is no record of what it claimed."),
        };
        block.push_str(&match &self.at {
            Some(at) => format!(
                "\n\nIt wrote that part's finding to {at}, in the worktree you \
                 are in. Read it before you start. What is quoted above \
                 summarises it and does not replace it."
            ),
            None => String::from("\n\nIts work is on the branch you are in."),
        });
        block
    }
}

/// That the part before this one got past its gate, and by whose decision.
///
/// **Two constructors and no enum in the signature**, mirroring the pair on
/// `verification::OutcomeTurn` — `advanced` is the mechanical tier ruling and
/// `approved` is a person at a human gate. The two call sites that pick between
/// those pick between these, which is what keeps the pair from drifting apart:
/// `fleet::gate` builds one and `fleet::overruling` builds the other.
///
/// # What this block is for
///
/// Not to report a verdict — the rail already marks the part done. It is to say
/// the part is **closed**, because the failure a fresh Drone is exposed to is
/// arriving on an unfamiliar branch and re-opening work somebody already
/// accepted. So both renderings end on the same instruction, and neither
/// carries a count of anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cleared {
    label: String,
    by: ByWhom,
}

/// Which gate let the earlier part through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ByWhom {
    /// The mechanical tier ran what the step declared and it passed.
    Checks,
    /// A person read the work and took it.
    Person,
}

impl Cleared {
    /// The step's own checks ran and it passed them.
    ///
    /// **It does not say which checks, or how many were skipped.**
    /// `verification::Verified` splits that three ways for a live Drone, and
    /// two of the three sentences are about "what you changed" — which is a
    /// thing this Drone did not do. A fresh Drone told a check covering paths
    /// it never touched was skipped has been told about a change it cannot see.
    pub fn checked(passed: &ResolvedStep) -> Cleared {
        Cleared {
            label: passed.label().to_string(),
            by: ByWhom::Checks,
        }
    }

    /// A person read the work at a human gate and took it.
    ///
    /// It carries no part of what the person said, for
    /// `OutcomeTurn::approved`'s reason: where there is something to change the
    /// act is `request_changes` and the words go with it.
    pub fn reviewed(passed: &ResolvedStep) -> Cleared {
        Cleared {
            label: passed.label().to_string(),
            by: ByWhom::Person,
        }
    }

    /// The block, exactly as it reaches a Drone.
    pub(crate) fn text(&self) -> String {
        let label = &self.label;
        let how = match self.by {
            ByWhom::Checks => format!("{label} passed the checks that gate it"),
            ByWhom::Person => format!("{label} was read by a person and accepted"),
        };
        format!(
            "THE PART BEFORE THIS ONE\n\n{how}, and its work is on the branch \
             you are in. It is settled: it is not yours to do again, to review \
             or to improve on. Start this part from it."
        )
    }
}

/// What Fleet did to this branch before the Drone reading it existed.
///
/// **A different block from the one a live Drone gets**, and the difference is
/// the tense. `verification::TheBaseMoved` renders "while you worked", which is
/// true at a step boundary and false in an opening turn: this Drone did not
/// work, and a first turn that opens by describing work it has no memory of is
/// a first turn it has to reconcile before it can start.
///
/// **The conflicted variant is the reader `#180` had to find.** A rebase runs
/// where there is no session to inject a turn into, so the conflict rides the
/// brief and is the Drone's opening piece of work — which is the whole of what
/// "the Drone is asked to resolve them before continuing" means on a path with
/// no Drone yet.
///
/// **Drafted wording**, like [`Cleared`] and like `briefing::Redeclaring`.
/// `docs/contracts/agent-prompt.md` has no sanctioned copy for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reconciling(String);

impl Reconciling {
    /// The block, from what the catch-up came to.
    pub fn of(moved: &TheBaseMoved) -> Reconciling {
        let said = match moved {
            TheBaseMoved::BroughtUpToDate { base, commits } => format!(
                "`{base}` moved on by {commits} commit(s) since this branch was cut, and the \
                 branch has been brought up to it before you started. The worktree is current. \
                 Work already on the branch may now sit on top of code that changed underneath \
                 it — read a file before you edit it."
            ),
            TheBaseMoved::Conflicted { base, files } => format!(
                "`{base}` moved on since this branch was cut, and the branch has been brought \
                 up to it before you started. These files were left with conflict markers in \
                 them, and resolving them is the first piece of your work:\n\n{}\n\nOpen each \
                 one, keep what belongs, and remove every marker before you submit.",
                files
                    .iter()
                    .map(|file| format!("- {file}"))
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
            TheBaseMoved::CouldNotFollow { base } => format!(
                "`{base}` moved on since this branch was cut, and the branch could not be put \
                 on top of it. It is exactly where it was. Nothing here is yours to fix — do \
                 the work described above, and somebody will reconcile the two."
            ),
        };
        Reconciling(format!("THE BRANCH YOU ARE ON\n\n{said}"))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}
