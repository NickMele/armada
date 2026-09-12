//! What crosses a step boundary when the process does not.
//!
//! **A value, not three arguments** — a Drone belongs to a step, so part two never saw part one
//! worked, and everything the gone process held has to arrive in the next Drone's opening brief.
//! That list is not closed: [`Crossed`] carries three, and the third — [`Redirected`] — arrived
//! as `#139` said it would: a field, a method, no caller of the first two touched.
//!
//! **An injected turn does not survive being moved into an opening brief.** Every sentence of
//! `verification::OutcomeTurn` addresses the Drone that did the work — a string cannot be
//! re-tensed — so what crosses is the **facts**, and [`Cleared`] is the fresh Drone's rendering
//! of them, exactly as [`Reconciling`] is of `verification::TheBaseMoved`. [`Reconciling`] moved
//! here from [`briefing`](mod@crate::briefing) for that reason.
//!
//! **Drafted wording** (`docs/contracts/agent-prompt.md` section 4a); the product block is not
//! — the contract draws it, and [`Produced::text`] follows it.
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
    dispatched: Option<Dispatched>,
    overtaken: Option<Overtaken>,
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
    /// That a Job from the same request landed work while this one was
    /// between steps.
    ///
    /// **Folded in rather than passed in, like [`and_redirect`]** and for the
    /// same reason: it is a fact about the Job at the moment a Drone starts,
    /// not about the act that reached the spawn. `crate::spawning` asks the
    /// record for it on every spawn.
    ///
    /// [`and_redirect`]: Crossed::and_redirect
    pub fn and_overtaken(self, overtaken: Option<Overtaken>) -> Crossed {
        Crossed { overtaken, ..self }
    }

    pub fn and_redirect(self, redirect: Option<Redirected>) -> Crossed {
        Crossed { redirect, ..self }
    }

    /// The Jobs this Job dispatched, and where each of them got to.
    ///
    /// **The one block a Drone could not have found out for itself.** A Drone
    /// has no tool that reads the Board and no way to reach another Job's
    /// record, so a step asked to act on what its Job created has nothing to
    /// act on without this — which is why it is carried across the boundary
    /// rather than left to be asked for.
    pub fn and_dispatched(self, dispatched: Option<Dispatched>) -> Crossed {
        Crossed { dispatched, ..self }
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

    pub(crate) fn dispatched(&self) -> Option<&Dispatched> {
        self.dispatched.as_ref()
    }

    pub(crate) fn overtaken(&self) -> Option<&Overtaken> {
        self.overtaken.as_ref()
    }
}

/// That a Job read off the same request landed work while this one was between
/// steps.
///
/// **Why a Drone is told rather than the Job stopped.** Two Jobs from one reading are unordered
/// by construction, so either may land the other's work. Before dispatch `crate::superseding`
/// closes a Job with nothing left to do — it can, because no Drone has started and no branch
/// holds anything. **Once a Drone has worked, that answer costs too much**: closing would throw
/// away work nobody has read, and killing mid-step is a different decision again, so the
/// boundary says what happened and lets the Drone decide.
///
/// **It gates nothing** — no Check reads it, no Judge sees it, and a Drone that ignores it is
/// refused by exactly what would have refused it anyway, which is what makes telling safe where
/// `crate::gate`'s rule makes believing unsafe.
///
/// **Drafted wording**, like [`Cleared`] and [`Reconciling`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overtaken {
    title: String,
    claimed: String,
}

impl Overtaken {
    /// What a landed sibling was called, and what its Evidence claimed the work
    /// now does.
    ///
    /// **The sibling's own `claimed`**, quoted and never paraphrased, for
    /// [`Redirected`]'s reason: it is a statement somebody else's Drone made
    /// about work this one is about to repeat, and Fleet summarising it would
    /// be Fleet deciding what they meant.
    pub fn of(title: &str, claimed: &str) -> Overtaken {
        Overtaken {
            title: title.to_string(),
            claimed: claimed.to_string(),
        }
    }

    /// The block, as it reaches a Drone.
    pub(crate) fn text(&self) -> String {
        let Overtaken { title, claimed } = self;
        format!(
            "WHAT LANDED WHILE YOU WERE WORKING

This Job and another were read off one              request, so neither waits on the other. That one — \"{title}\" — has since              landed, and its own evidence says:\n\n  \"{claimed}\"\n\nSome of what this              step was going to do may already be in your base. Read before you write, and say              in your evidence what you found rather than doing it twice. Where nothing is              left, that is a finding and not a failure."
        )
    }
}

/// What became of the Jobs this Job dispatched.
///
/// # It is a rendered block and not a list of records
///
/// [`Cleared`] and [`Reconciling`] are the same shape and for the same reason:
/// what a Drone needs is a paragraph it can read at the top of a brief, and a
/// list of typed rows would make `briefing` the second place the wording of a
/// Job's outcome is decided. The record is `store`'s and stays there.
///
/// **Every child is named, including the ones that failed.** A block that
/// listed only what succeeded would be the one nobody needs — the failures are
/// what the next step is there for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dispatched(String);

impl Dispatched {
    /// One line per child: the id, what it was called, and where it ended.
    ///
    /// `None` where the Job dispatched nothing, which renders no block at all
    /// rather than a block saying there were none — `Crossed`'s rule, and the
    /// same argument the empty boundary makes.
    pub fn of(children: &[(String, String, &'static str)]) -> Option<Dispatched> {
        if children.is_empty() {
            return None;
        }
        let mut said = String::from(
            "THE JOBS YOU DISPATCHED\n\nEvery one of these has finished. This is \
             the whole of what you have to report on, and it is not visible to \
             you anywhere else.\n",
        );
        for (id, title, ended) in children {
            said.push_str(&format!(
                "
  {id}  {ended}  {title}"
            ));
        }
        Some(Dispatched(said))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a person said while nobody was there to hear it.
///
/// **Why a type and not the string on the record.** The string is a person's own words and
/// passes through untouched — the Agent Prompt Contract's table says `redirect_drone`'s content
/// comes from a person, not a verdict. What this adds is the frame the words need in an
/// *opening* turn and did not need injected: injected, the words arrive mid-conversation; here
/// they arrive at the top of a brief, addressed to a process that never worked this part.
///
/// **It says who wrote them** — "a person read the work and asked for this" is the difference
/// between an instruction and the step's own definition, and a fresh Drone has nothing else to
/// tell the two apart.
///
/// **Drafted wording**, like [`Cleared`] and [`Reconciling`] — `docs/contracts/agent-prompt.md`
/// sanctions no copy for it, section 4a's table giving `redirect_drone` no Fleet wording at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redirected(String);

impl Redirected {
    /// The note the record is holding.
    pub fn of(waiting: &RedirectWaiting) -> Redirected {
        Redirected(waiting.text().to_string())
    }

    /// The block, as it reaches a Drone.
    ///
    /// **Quoted and never paraphrased**, for the reason `fleet::resume::redirect` gives about
    /// the Judge's citation: the person read the work and left this, and Fleet summarising it
    /// would be Fleet deciding what they meant.
    ///
    /// **Framed as work, not context** — the whole difference from [`Produced`]'s `not_claimed`
    /// block, which says outright it is not work this part owes; this is the reason the part is
    /// being worked at all.
    ///
    /// **"left this" and not "wrote this", since `#526`** — a person typing a note wrote the
    /// words, but one picking comments off a pull request picked somebody else's
    /// (`fleet::remarks` is the second act writing this column), and the assembled note itself
    /// says which kind it is.
    pub(crate) fn text(&self) -> String {
        let said = &self.0;
        format!(
            "WHAT A PERSON ASKED FOR\n\nA person read this work and left this before you \
             started. It is not part of the step's definition and it is not something an \
             earlier part claimed — it is an instruction, and it is why this part is being \
             worked:\n\n  \"{said}\"\n\nDo what it asks. Where it and the step below disagree \
             about what to do first, this comes first."
        )
    }
}

/// What the part immediately before this one produced, as the record holds it.
///
/// **The part immediately before, no further back.** The contract draws one block naming one
/// part; reaching an arbitrary earlier step is what `baseline_ref` and `reference_docs` are for
/// (the Judge's yardstick, kept from a Drone by `docs/concepts/drone.md`, refused by `config::scope`).
///
/// **Both the quotation and the path, neither alone.** Quoting the claim alone reproduces what
/// `#138` closed: a sentence about a file rather than the file. Naming the path alone spends a
/// tool call to read two lines and says nothing on a step whose product is the diff.
///
/// **Two of the three evidence strings cross.** `shown_by` does not: `#138` is explicit the
/// brief points at a path Fleet resolved, not whatever the previous Drone typed. `not_claimed`
/// does, on the owner's ruling of 31 Aug 2026 — the path is right there in this worktree, so a
/// Drone wanting the whole opens the file, bearing on what this part must not redo.
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
/// **A different block from the one a live Drone gets, in tense.** `verification::TheBaseMoved`
/// renders "while you worked", true at a step boundary and false in an opening turn: this Drone
/// did not work, and a first turn describing work it has no memory of has to reconcile before
/// it can start.
///
/// **The conflicted variant is the reader `#180` had to find** — a rebase runs where there is no
/// session to inject a turn into, so the conflict rides the brief as the Drone's opening work.
///
/// **Drafted wording**, like [`Cleared`] and `terms::Redeclaring` — `docs/contracts/agent-prompt.md`
/// has no sanctioned copy for it.
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
