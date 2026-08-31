//! What crosses a step boundary when the process does not.
//!
//! # Why this is a value and not three arguments
//!
//! A Drone belongs to a step, so the one starting part two never saw part one
//! worked. Everything the process that is gone would have held has to be handed
//! over in the next Drone's opening brief, and that list is not closed:
//! [`Crossed`] carries three, and the third — [`Redirected`], a person's note
//! written where no Drone was there — arrived as `#139` said it would: a field,
//! a method, no caller of the first two touched. A fourth costs the same.
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
//! follows it.
use core_model::{FrozenWorkflow, RedirectWaiting, ResolvedStep, StepEvidence, StepId};
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
    redirect: Option<Redirected>,
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

    /// A person's note, written at a boundary where there was no Drone to
    /// take it.
    ///
    /// Takes the `Option` the record answers with, like
    /// [`and_produced`](Crossed::and_produced) — "nobody has said anything to
    /// this Job" is the ordinary case and not one a caller should have to
    /// spell.
    ///
    /// **Not built by the caller that crosses the boundary.**
    /// `fleet::spawning` folds it in, because it is the one funnel every spawn
    /// goes through and the note is owed to *the next Drone* rather than to the
    /// next Drone of one act. A caller that had to remember it is a caller that
    /// could forget.
    pub fn and_redirect(self, redirect: Option<Redirected>) -> Crossed {
        Crossed { redirect, ..self }
    }

    pub(crate) fn produced(&self) -> Option<&Produced> {
        self.produced.as_ref()
    }

    pub(crate) fn cleared(&self) -> Option<&Cleared> {
        self.cleared.as_ref()
    }

    pub(crate) fn redirect(&self) -> Option<&Redirected> {
        self.redirect.as_ref()
    }
}

/// What a person said while nobody was there to hear it.
///
/// # Why this is a type and not the string on the record
///
/// The string is a person's own words and passes through untouched — the Agent
/// Prompt Contract's table says outright that `redirect_drone`'s content comes
/// from a person rather than from a verdict. What this adds is the frame the
/// words need in an *opening* turn and did not need in an injected one, which
/// is the same re-tensing [`Cleared`] and [`Reconciling`] exist for: injected,
/// the words arrive in the middle of a conversation the Drone remembers; here
/// they arrive at the top of a brief, addressed to a process that has never
/// worked this part and would otherwise read them as part of the task.
///
/// **It says who wrote them.** "A person read the work and asked for this" is
/// the difference between an instruction and the step's own definition, and a
/// fresh Drone has nothing else to tell the two apart.
///
/// **Drafted wording**, like [`Cleared`] and [`Reconciling`].
/// `docs/contracts/agent-prompt.md` sanctions no copy for it — section 4a's
/// table gives `redirect_drone` no Fleet wording at all, which is a statement
/// about the note and not about the frame around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redirected(String);

impl Redirected {
    /// The note the record is holding.
    pub fn of(waiting: &RedirectWaiting) -> Redirected {
        Redirected(waiting.text().to_string())
    }

    /// The block, as it reaches a Drone.
    ///
    /// **The words are quoted and never paraphrased**, for the reason
    /// `fleet::resume::redirect` gives about the Judge's citation: the person
    /// read the work and wrote this from it, and Fleet summarising it would be
    /// Fleet deciding what they meant.
    ///
    /// **It is framed as work and not as context**, which is the whole of the
    /// difference between it and [`Produced`]'s `not_claimed` block. That one
    /// says outright that it is not work this part owes; this one is the
    /// reason this part is being worked at all.
    pub(crate) fn text(&self) -> String {
        let said = &self.0;
        format!(
            "WHAT A PERSON ASKED FOR\n\nA person read this work and wrote this before you \
             started. It is not part of the step's definition and it is not something an \
             earlier part claimed — it is an instruction, and it is why this part is being \
             worked:\n\n  \"{said}\"\n\nDo what it asks. Where it and the step below disagree \
             about what to do first, this comes first."
        )
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
/// alone reproduces what `#138` closed: the next Drone is handed a sentence a
/// Drone typed about a file, rather than the file. Naming the path alone spends
/// a tool call to read two lines, and says nothing on a step whose product is
/// the diff.
///
/// **Two of the three evidence strings cross, and which two follows from
/// that.** `shown_by` does not: `#138` is explicit that the brief points at a
/// path Fleet resolved rather than at whatever the previous Drone typed.
/// `not_claimed` does, on the owner's ruling of 31 Aug 2026 — `claimed`
/// summarises a file in this same worktree and the path is right there, so a
/// Drone wanting the whole of it opens the file, and `not_claimed` is nowhere
/// else. It also bears most directly on what this part must not spend its turn
/// on: "the writer has the same bound and is untouched" is exactly what a fix
/// step should not go and re-do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Produced {
    /// One-based, and it is the part number the rail counts with — "part 1",
    /// never a step id.
    part: usize,
    /// What the earlier part claimed its work now does. `None` where the step
    /// is on the record and its evidence is not, which an override advance
    /// leaves behind.
    claimed: Option<String>,
    /// Everything that claim does not assert — the gap it left and the thing
    /// it changed that nobody asked for.
    ///
    /// **`None` where the field is empty, and empty is legal.**
    /// `docs/contracts/agent-copy.md` says so outright: a Drone reporting that
    /// it left nothing behind is not a Drone declining to answer. A label with
    /// nothing under it would turn the first of those into the second.
    not_claimed: Option<String>,
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
        let evidence = recorded
            .iter()
            .find(|(step, _)| step == earlier.id())
            .map(|(_, evidence)| evidence);
        Some(Produced {
            part: here,
            claimed: evidence.map(|evidence| evidence.claimed.clone()),
            // **The filter is where "empty renders nothing" lives**, and it is
            // here rather than in the rendering so that the value cannot reach
            // a caller holding a blank it would have to check again.
            not_claimed: evidence
                .map(|evidence| evidence.not_claimed.clone())
                .filter(|left_alone| !left_alone.trim().is_empty()),
            at: earlier.deliverable().map(str::to_string),
        })
    }

    /// The block, in the shape `docs/contracts/agent-prompt.md` draws it:
    /// a heading naming the part, and the claim indented under it.
    ///
    /// **The path sentence stays next to the claim it is about.** What was left
    /// alone follows both, so "what is quoted above summarises it" can only be
    /// read as the claim — a second quotation between the two would make that
    /// sentence ambiguous about which quotation it means.
    ///
    /// **What was left alone is framed as context and never as work.** It is
    /// the one block here a Drone could read as a to-do list, and doing so
    /// would produce exactly the failure the field exists to prevent: a part
    /// doing the next part's work, having been handed a list of it. So the
    /// sentence under it says what the field is — everything the claim does not
    /// cover — and says outright that it is not work this part owes.
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
        if let Some(left_alone) = &self.not_claimed {
            block.push_str(&format!(
                "\n\nWhat part {part} did not claim:\n  \"{left_alone}\"\n\nThat is \
                 everything its claim does not cover — a gap it left on purpose, \
                 or something it changed that nobody asked for. It is context \
                 for this part and not a list of work this part owes."
            ));
        }
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
