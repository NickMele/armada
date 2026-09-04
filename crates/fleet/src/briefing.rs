//! The first turn a Drone is given, assembled from the Job it is being put on.
//!
//! **The baseline is quoted, not written here.** `docs/contracts/agent-prompt.md`
//! section 5 carries the M1 rendering as sanctioned copy, and each clause in it
//! passed a membership test this module is in no position to re-run. [`BASELINE`]
//! is that text transcribed, and the rest of this module assembles the per-Job
//! blocks around it. A wording change belongs in the contract; a copy here that
//! drifted from it would be the second-vocabulary defect in prose.
//! [`notekeeping`] and the gaming half of [`Stopped`] are **drafted** for the
//! same reason: sanctioned copy is the contract's and it has none for them yet.
//!
//! **What is not assembled is missing rather than empty.** The contract names
//! six layers and M1 has no Kit and no Manifest, which its own M1 rendering
//! says, so what is assembled is baseline, job brief, where-you-are and step.
//! The exemplar corpus and a review step's injected reference material each
//! need a record M1 has no type for, and a block rendered empty reads to a
//! Drone as a block that was answered. What the previous step produced was on
//! that list and is not — [`crossing`](mod@crate::crossing) is what does.
//!
//! **No block here is written from a `ResolvedCheck`'s `run` string.** The step
//! block comes from [`ResolvedStep::label`] and the Job's own fields, and the
//! blocks that outlive the turn are not here at all — the sentence this module
//! was split on. [`crate::terms`] holds the four a step puts to whoever works
//! it; [`Stopped`] stayed, because it is why a resuming brief differs from a
//! first one and the one turn it was built for consumes it.

use adapter_traits::{Prompt, SpawnConfigRefused};
use core_model::{
    EscalationTrigger, FrozenWorkflow, GamingFlag, Job, JobId, Judgment, ResolvedStep, StepId,
    StepVerdict,
};
use verification::TheBaseMoved;

use crate::crossing::{Crossed, Produced, Reconciling, Redirected};
use crate::terms::{Checking, Declaring, Delivering};

/// Layer 1, verbatim from the Agent Prompt Contract's M1 rendering: **mechanics,
/// never task content**, identical on every step of every Job, which is what
/// makes it a constant rather than something assembled.
///
/// **Its third paragraph names which outcome comes back and which does not.** It
/// used to promise a later turn carrying the reason whenever work did not pass,
/// and exactly one outcome keeps that promise:
/// [`HandedBack`](crate::Ruling::HandedBack) carries a `tell` built from what
/// each Check expected and produced. [`Refused`](crate::Ruling::Refused),
/// [`Suspect`](crate::Ruling::Suspect) and [`CouldNotDecide`](crate::Ruling::CouldNotDecide)
/// have no message field at all — the step stops, the Job escalates, `dispatch`
/// terminates without a turn, and a Drone told to wait waited holding a live
/// session until a person noticed. Twice on one Job: nineteen hours, nearly all
/// of it that wait.
///
/// **The promise narrowed rather than the system gaining a message.** Telling a
/// refused Drone changes nothing it can act on — a refusal says the work runs
/// and is not what was asked for, which resubmitting under the same instructions
/// cannot answer — and `#140` ends a Drone once its step's work clears the
/// machine gates, so the process is ending either way.
///
/// **Stated positively, over both halves.** "You may not hear back" tells a Drone
/// nothing, and a rule about refusals alone would leave a Drone whose work
/// *passed* reading its own silence as one: a step that advances ends its Drone
/// too, and sends nothing.
pub const BASELINE: &str = "\
You are working in a git worktree on a branch of your own. You cannot push, \
open a pull request, or run commands this repository has not declared.

When you have finished the work described below, you must report it using the \
evidence submission tool you have been given. It is the only way to report. \
Work you do not submit is work no one sees, and the task will not move on.

Submitting returns \"recorded\". That is a receipt, not a verdict — your work \
is checked after you submit. A later turn comes only where the part is coming \
back to you: a check failed and there is another attempt, or what you \
submitted could not be read. Wait for it rather than submitting again. Every \
other outcome ends your part where it stands and sends you nothing — work \
that runs and is refused for what it is goes to a person, and you are not \
asked about it again.";

/// What a Drone being put on a worktree is opening with.
///
/// **The two first turns as a value rather than as two calls**, so that the one
/// funnel every spawn goes through — [`put_a_drone_on`] — can decide the brief
/// *after* the rebase it runs has answered. A caller that assembled the prompt
/// itself would have had to be handed the catch-up back, which is the fourth
/// call site `#180` exists to avoid.
///
/// **A struct rather than the enum it was**, because what a boundary carries is
/// orthogonal to whether the step has been attempted. A restarted Drone on part
/// three needs to know what part two produced exactly as a fresh one does, and
/// an enum would have had to carry [`Crossed`] in both arms. It is also what
/// keeps [`put_a_drone_on`]'s signature closed: the carried items ride the
/// `Opening` a caller already passes, which is how `#207`'s waiting redirect
/// arrived without the spawn funnel changing shape.
///
/// **An opening turn is asked for rather than assembled by the caller.** Every
/// spawn rebases first, so what a Drone is told about its branch is not known
/// at the moment the caller would have built the prompt.
///
/// [`put_a_drone_on`]: crate::daemon::Fleet::put_a_drone_on
#[derive(Clone, Debug)]
pub struct Opening {
    attempted: Attempted,
    crossed: Crossed,
}

/// Whether the step being opened has been worked before.
#[derive(Clone, Debug)]
enum Attempted {
    /// A step no Drone has attempted yet — a Job's first, or the one an
    /// override advanced to.
    No,
    /// A step that stopped, and what the record says stopped it.
    Before(Stopped),
}

impl Opening {
    /// A step no Drone has attempted yet, carrying nothing.
    pub fn fresh() -> Opening {
        Opening {
            attempted: Attempted::No,
            crossed: Crossed::nothing(),
        }
    }

    /// A step that stopped, carrying nothing but what stopped it.
    pub fn resuming(stopped: Stopped) -> Opening {
        Opening {
            attempted: Attempted::Before(stopped),
            crossed: Crossed::nothing(),
        }
    }

    /// The same opening, with what the boundary handed across.
    ///
    /// **Separate from both constructors on purpose.** A spawn that carries
    /// nothing is a legitimate spawn — a Job's first step carries nothing — so
    /// this cannot be a required argument, and a caller that has something to
    /// carry says so in one call rather than picking between four
    /// constructors.
    pub fn carrying(self, crossed: Crossed) -> Opening {
        Opening { crossed, ..self }
    }

    /// The same opening, plus the note a person left where there was no Drone
    /// to take it.
    ///
    /// **Folded in rather than passed in**, and it is the one carried item no
    /// caller supplies. `crate::spawning` asks the record for it on every
    /// spawn, because "the very next opening brief" is a fact about the Job
    /// and not about the act that happened to reach the spawn — a redirect
    /// that waited through a boundary because the caller building the
    /// `Crossed` did not know about it is the whole failure `#207` exists to
    /// close.
    pub(crate) fn also_carrying(self, redirect: Option<Redirected>) -> Opening {
        Opening {
            crossed: self.crossed.and_redirect(redirect),
            ..self
        }
    }

    /// The whole opening turn: the four blocks, what stopped the last attempt
    /// where there was one, and what the rebase came to where it came to
    /// anything.
    ///
    /// **The branch block is last.** It is the only block describing something
    /// that happened after the work was described, and a Drone that stops
    /// reading has read the task rather than the git.
    pub fn turn(
        &self,
        job: &Job,
        workflow: &FrozenWorkflow,
        at: &StepId,
        moved: Option<&TheBaseMoved>,
    ) -> Result<Brief, SpawnConfigRefused> {
        let mut blocks = match &self.attempted {
            Attempted::No => assemble(job, workflow, at, &self.crossed),
            Attempted::Before(stopped) => {
                let mut blocks = assemble(job, workflow, at, &self.crossed);
                blocks.headed(&stopped.block());
                blocks
            }
        };
        if let Some(moved) = moved {
            blocks.headed(Reconciling::of(moved).text());
        }
        blocks.brief()
    }
}

/// A brief as it is sent: the turn, and which of its lines are block headings.
///
/// **The headings travel because the shape is known only where it is written.**
/// `ipc::Saw::Instructed::headings` carries the argument, and what goes wrong
/// for a reader that guesses instead.
pub struct Brief {
    prompt: Prompt,
    headings: Vec<usize>,
}

impl Brief {
    /// The turn, exactly as it reaches a Drone.
    pub fn as_str(&self) -> &str {
        self.prompt.as_str()
    }

    /// The lines that are block headings, zero-based into the turn's lines.
    pub fn headings(&self) -> &[usize] {
        &self.headings
    }

    /// The turn, for the harness that spawns on it.
    pub fn prompt(self) -> Prompt {
        self.prompt
    }
}

/// A brief under assembly, block by block.
///
/// **A block says whether it is headed rather than being asked.** Every heading
/// in this module is written as `HEADING\n\n` at the top of its block, so a
/// reader could take the first line of every block and be right about most of
/// them — and wrong about [`BASELINE`], which opens with prose, and wrong about
/// what the part before produced, which opens with a sentence. The two
/// constructors are the difference, stated by the call that appends.
struct Blocks {
    text: String,
    headings: Vec<usize>,
    /// Lines written so far, which is the line the next block starts on.
    lines: usize,
}

impl Blocks {
    fn opening(baseline: &str) -> Blocks {
        let mut blocks = Blocks {
            text: String::new(),
            headings: Vec::new(),
            lines: 0,
        };
        blocks.prose(baseline);
        blocks
    }

    /// A block with no heading of its own.
    fn prose(&mut self, block: &str) {
        self.push(block, false);
    }

    /// A block whose first line is its heading.
    fn headed(&mut self, block: &str) {
        self.push(block, true);
    }

    fn push(&mut self, block: &str, headed: bool) {
        if !self.text.is_empty() {
            // The blank line between blocks, which is two more lines gone by.
            self.text.push_str("\n\n");
            self.lines += 2;
        }
        if headed {
            self.headings.push(self.lines);
        }
        self.text.push_str(block);
        self.lines += block.matches('\n').count();
    }

    /// Refuses only where the assembled text is empty, which
    /// [`Prompt::assembled`] decides and this does not restate.
    fn brief(self) -> Result<Brief, SpawnConfigRefused> {
        Ok(Brief {
            prompt: Prompt::assembled(&self.text)?,
            headings: self.headings,
        })
    }
}

/// Assemble the first turn for a Job standing at one step of its workflow.
///
/// **There is no argument through which arbitrary text reaches a Drone.** The
/// blocks are built from the Job record and the resolved workflow, and a caller
/// that wanted to say something else would have to add a block here — which is
/// the same refusal `Turn` makes about a prepared string.
///
/// `crossed` is what the boundary handed across — [`Crossed::nothing`] on a
/// Job's first step, and on every spawn that has nothing to hand.
///
/// Refuses only where the assembled text is empty, which
/// [`Prompt::assembled`] decides and this does not restate.
pub fn first_turn(
    job: &Job,
    workflow: &FrozenWorkflow,
    at: &StepId,
    crossed: &Crossed,
) -> Result<Brief, SpawnConfigRefused> {
    assemble(job, workflow, at, crossed).brief()
}

/// Assemble the first turn for a Drone taking over a step that stopped.
///
/// **The reason is not optional and there is no constructor without one.** A
/// restarted Drone has no session and no history: it knows nothing about the
/// attempt it is replacing, and a brief that did not say what stopped would
/// send it to reproduce the work that was refused.
///
/// [`Stopped`] is read off the record rather than composed by a caller, so
/// nothing here is a claim about the Job that the log does not already carry.
pub fn resuming_turn(
    job: &Job,
    workflow: &FrozenWorkflow,
    at: &StepId,
    stopped: &Stopped,
    crossed: &Crossed,
) -> Result<Brief, SpawnConfigRefused> {
    let mut blocks = assemble(job, workflow, at, crossed);
    blocks.headed(&stopped.block());
    blocks.brief()
}

/// Why the step a Drone is being put on stopped, as the record holds it.
///
/// Built by `crate::resume` from `last_verdict`, `job_step_judgments` and
/// `job_step_gaming_flags` — the same three a person reads on the detail view.
///
/// **All three are rendered.** The verdict used to be carried and never read,
/// which left the briefing holding the true answer and stating a different
/// one — see [`Stopped::why`].
#[derive(Clone, Debug, Default)]
pub struct Stopped {
    /// What the gate said stopped it, spelled as the registry spells it. It
    /// decides the block's first sentence, and no two triggers get the same
    /// one.
    pub verdict: Option<StepVerdict>,
    /// Every criterion the Judge answered on the step. Only the refused ones
    /// are rendered.
    pub judged: Vec<Judgment>,
    /// Every gaming pattern the step's evidence tripped.
    pub flagged: Vec<GamingFlag>,
}

impl Stopped {
    /// The block, in the shape the Agent Prompt Contract's refusal reprompt
    /// specifies: `expected` and `produced`, **never `consequence`**, which is
    /// written for a person deciding whether to care, and **never a counter**.
    ///
    /// The gaming half has no sanctioned wording and is drafted. It renders
    /// the pattern and what it cited, which is the same two-column shape and
    /// the whole of what a flag is.
    ///
    /// **The closing line follows what was cited rather than being fixed.**
    /// "Address this" names the rows above it, and there are stops that leave
    /// no rows at all — a Drone told to address nothing goes looking for it.
    fn block(&self) -> String {
        let mut block = format!(
            "WHY THIS PART IS BEING DONE AGAIN\n\n{} Its work is on the branch you are in.",
            self.why()
        );
        let mut cited = false;
        for judgment in self.judged.iter().filter(|judged| judged.verdict.refuses()) {
            if let (Some(expected), Some(produced)) = (&judgment.expected, &judgment.produced) {
                cited = true;
                block.push_str(&format!(
                    "\n\n  Expected   {expected}\n  Produced   {produced}"
                ));
            }
        }
        for flag in &self.flagged {
            cited = true;
            block.push_str(&format!(
                "\n\n  Pattern    {}\n  Found in   {}",
                flag.pattern.as_wire(),
                flag.cited
            ));
        }
        block.push_str(match cited {
            true => {
                "\n\nAddress this and submit again. Say what changed since the last \
                 submission."
            }
            false => "\n\nNothing was cited for you to answer. Finish this part and submit.",
        });
        block
    }

    /// What stopped the earlier attempt, in one sentence, from the verdict the
    /// record holds.
    ///
    /// **Read rather than assumed.** [`Stopped::verdict`] was carried here and
    /// never rendered, and the block said "checked and did not pass" whatever
    /// it held — which is false after `gate_undecided`, where nothing was
    /// checked and the gate said so. A Drone told its work did not pass goes
    /// looking for what was wrong with work that was right, and the second
    /// attempt is then worse than the first for a reason nothing records.
    ///
    /// **One sentence per trigger, matched exhaustively**, and no two triggers
    /// share one. A Drone acts on what it is told, so a trigger added to the
    /// registry is a compile error here rather than a Drone quietly handed the
    /// nearest sentence — and the nearest sentence is the trap, not the
    /// missing one. `drone_killed` sat under `thrashing`'s line for a while
    /// because both are true of a step stopped mid-run; one of them sends a
    /// Drone hunting for what was wrong with work nothing had measured.
    ///
    /// Every sentence says what Fleet knows and stops. What the gate could not
    /// read, and what a Judge would make of it, are not Fleet's to speculate
    /// about — `docs/concepts/drone.md`'s rule about self-report, pointed the
    /// other way.
    fn why(&self) -> &'static str {
        let Some(StepVerdict::Failed(stopped_by)) = self.verdict else {
            // No verdict against the work at all. `passed` and `not_reached`
            // are not refusals either, and rendering any of the three as one
            // is the defect this function exists to close.
            return "An earlier attempt at this part stopped. The record holds no verdict \
                    against its work.";
        };
        match stopped_by.trigger() {
            EscalationTrigger::GateFailure => {
                "An earlier attempt at this part was checked and did not pass."
            }
            EscalationTrigger::EvidenceSuspect => {
                "An earlier attempt at this part passed its checks, and what it submitted \
                 was not accepted as evidence that the work was done."
            }
            EscalationTrigger::GateUndecided => {
                "An earlier attempt at this part was never checked. Something the check \
                 needed could not be read, so nothing was decided about the work itself."
            }
            EscalationTrigger::Thrashing => {
                "An earlier attempt at this part was stopped while it was still running. \
                 Nothing it did was checked."
            }
            // **Not `Thrashing`'s line, though it is true of this too.** That
            // one is a machine finding the work was going nowhere, and a Drone
            // told it goes looking for what was wrong with what it produced.
            // Nothing was wrong with it here and nothing was measured; a
            // person took the process away. Saying who is the whole of the
            // difference, and it is said without a reason because the record
            // holds none — why a person did it is theirs and is not in this.
            EscalationTrigger::DroneKilled => {
                "An earlier attempt at this part was ended by a person while it was still \
                 running. Nothing it did was checked, and nothing about it was judged."
            }
            // **Not `DroneKilled`'s line, and the difference is who acted.**
            // There a person took the process away mid-run; here the Drone
            // said its own run was over and Fleet took it at its word. Told
            // the wrong one, a Drone goes looking for the instruction that
            // stopped it and there was none. The second sentence is the
            // accepted cost said out loud: a run that reports it has ended and
            // then carries on loses whatever it did after saying so.
            EscalationTrigger::RunEnded => {
                "An earlier attempt at this part said its run was over without having \
                 submitted anything, and was stopped there. Nothing it did was checked, \
                 and anything it did after saying so was not kept."
            }
            // **Not `BlockedByPolicy`'s line**, which says a tool or a
            // command was denied and sends the next attempt looking for a
            // setting to work around. Nothing was denied here: the earlier
            // attempt was told the paths it wanted are not part of this part
            // of the work. The second sentence is what this attempt can do
            // differently, and it is the only one of these lines that names
            // the scope at all — a Drone that reads only the first will ask
            // for the same paths again.
            EscalationTrigger::ScopeRefused => {
                "An earlier attempt at this part asked to write files outside what this \
                 task says it changes, and was told they are not part of it. Nothing it \
                 did was checked. Do this part inside the files the task already names."
            }
            // `Thrashing`'s line is true of this too and leaves out the part
            // this attempt can do differently.
            EscalationTrigger::NoReport => {
                "An earlier attempt at this part was told to stop and report where it had \
                 got to, and did not answer. It was stopped there, and nothing it did was \
                 checked."
            }
            EscalationTrigger::CheckTimeout => {
                "An earlier attempt at this part was stopped because a check did not \
                 finish. Nothing was decided about the work itself."
            }
            EscalationTrigger::EvidenceTooLarge => {
                "An earlier attempt at this part submitted more than could be read, so it \
                 was never checked."
            }
            EscalationTrigger::BlockedByPolicy => {
                "An earlier attempt at this part was refused a tool or a command it needed, \
                 and stopped without submitting anything."
            }
            EscalationTrigger::LoopCap => {
                "An earlier attempt at this part used every round it is allowed. Nothing it \
                 did was refused."
            }
            // Job-level triggers, which `StepLevelTrigger::of` will not build,
            // so none of these is reachable through a stopped step. Named
            // rather than swept into a wildcard: the narrowing is what makes
            // them unreachable, and a wildcard would hide a registry change
            // that moved one of them to step level.
            EscalationTrigger::DependencyFailed
            | EscalationTrigger::FanOut
            | EscalationTrigger::HatchUnbidden
            | EscalationTrigger::Interrupted
            | EscalationTrigger::NoWorktree
            | EscalationTrigger::NotConfigurable
            | EscalationTrigger::NotPrepared
            | EscalationTrigger::ResourceExhausted
            | EscalationTrigger::Silent
            | EscalationTrigger::Stalled
            | EscalationTrigger::WouldNotStart => {
                "An earlier attempt at this part stopped. The record holds no verdict \
                 against its work."
            }
        }
    }
}

fn assemble(job: &Job, workflow: &FrozenWorkflow, at: &StepId, crossed: &Crossed) -> Blocks {
    // The baseline is the one block with no heading of its own.
    let mut blocks = Blocks::opening(BASELINE);
    blocks.headed(&notekeeping(job.id()));
    blocks.headed(&job_brief(job));
    blocks.headed(&where_you_are(workflow, at, crossed.produced()));
    // **After the rail and before the step.** The rail is what establishes
    // that there is a part before this one at all, and this says that part is
    // closed — which is only meaningful once a Drone knows it exists.
    if let Some(cleared) = crossed.cleared() {
        blocks.headed(&cleared.text());
    }
    // **Before the step and not after it.** A Drone that stops reading at the
    // step block has read the instruction, and the block itself says which of
    // the two comes first — an instruction placed after the definition it
    // overrides reads as a footnote to it.
    if let Some(redirect) = crossed.redirect() {
        blocks.headed(&redirect.text());
    }
    // **Before the step block, with the other things the boundary carried.**
    // It is what the part is about rather than a footnote to it: a Drone after
    // a dispatch has no other way to learn that the Jobs exist.
    if let Some(dispatched) = crossed.dispatched() {
        blocks.headed(dispatched.text());
    }
    if let Some(step) = workflow.steps().iter().find(|step| step.id() == at) {
        blocks.headed(&step_block(step));
        if let Some(delivers) = Delivering::at(step) {
            blocks.headed(delivers.text());
        }
        if let Some(asked) = Declaring::at(step) {
            blocks.headed(asked.text());
        }
        if let Some(offered) = Checking::at(step) {
            blocks.headed(offered.text());
        }
    }
    blocks
}

/// Where a file a Drone writes for itself goes, which is not the repository
/// root. A private function rather than one of the types above: it is rendered
/// where the turn is assembled, and does not outlive it.
///
/// **Every other per-Job artifact is already keyed by Job id** — the log, a
/// Check's output, the worktree, the brief's attachments. A plan was the one
/// piece that was not, and one shared slot at the root cannot say whether what
/// is in it is still live: the `PLAN.md` this repository carried belonged to a
/// Job that finished and merged, and every worktree cut after it inherited a
/// confident plan for work nobody had asked for. **No Drone is known to have
/// been misled by it** — nothing pointed at that file, so one reached it only
/// by going looking. The Job-keyed path is the correct home; harm was not
/// observed.
///
/// **It offers a place; it does not ask for a plan.** A block a Drone reads as
/// an instruction puts every Job through a planning step nobody requested, so
/// the whole of it is conditional and the last sentence says so outright. A
/// plan is no longer one of its examples either: on a `plan` step that made it
/// read as an instruction to put the step's deliverable in the one directory
/// the Judge cannot see, and [`Delivering`] names that file instead.
///
/// **"Plan" already means something else here.** [`Declaring`] calls the
/// declared scope "the plan you declared", and a second meaning in one turn is
/// the second-vocabulary defect in prose — so this block leads with what a
/// Drone is actually holding, a file it wrote for itself.
fn notekeeping(job: &JobId) -> String {
    format!(
        "FILES YOU WRITE FOR YOURSELF\n\nA checklist, notes you want to keep \
         between turns — anything you write for yourself rather than for the \
         work goes under .armada/{}/, which is this task's alone. None of it \
         belongs at the repository root, where a file outlives the task that \
         wrote it with nothing to say that task is over. Nothing here is \
         asking you to write any of it, and a file this part is asked to \
         deliver is not one of them — that one is named where it is asked \
         for, at the path that is read, and it does not go here.",
        job.as_str()
    )
}

/// What the Job is about, in the requester's own words.
///
/// `facts` is the context the Job carries and `acceptance_criteria` is what the
/// requester said "done" means. Both are the requester's text and both go in:
/// the criteria in particular are layer 5 in the contract's order, and a Drone
/// that cannot see them is being asked to hit a bar it was not shown.
///
/// Between the two, a line per attachment — a screenshot, a log capture,
/// whatever a person picked when the brief was written. Naming the
/// worktree-relative path is the whole of what this owes a Drone: `dispatch`
/// already copied the file to that path, and a Drone opens it with its own
/// tools rather than being handed anything more than where to look.
fn job_brief(job: &Job) -> String {
    let mut brief = format!("JOB BRIEF\n\n{}", job.title().as_str());
    if !job.facts().as_str().is_empty() {
        brief.push_str("\n\n");
        brief.push_str(job.facts().as_str());
    }
    if !job.attachments().is_empty() {
        brief.push_str("\n\nFiles attached to this brief, copied into your worktree:");
        for attachment in job.attachments() {
            brief.push_str("\n  - .armada/attachments/");
            brief.push_str(&attachment.filename);
        }
    }
    if !job.acceptance_criteria().is_empty() {
        brief.push_str("\n\nThis is done when:");
        for criterion in job.acceptance_criteria() {
            brief.push_str("\n  - ");
            brief.push_str(&criterion.text);
        }
    }
    brief
}

/// The rail, with the stop inside it.
///
/// **"Parts", not "steps"** — the contract is explicit that `step` is Armada's
/// word for a plan artifact and that a Drone which learns the system's
/// vocabulary can reason about the machinery.
///
/// **The stop sits inside the list rather than after it**, because where the
/// line falls is the boundary, and later parts carry the specific prohibition
/// rather than a general one.
///
/// **`produced` goes between the rail and the closing paragraph**, which is
/// where `docs/contracts/agent-prompt.md` draws it. It is inside this block
/// rather than beside it because it is positional — "part 1" only means
/// anything to a Drone that has just read the numbered list — and the contract
/// draws it inside.
fn where_you_are(workflow: &FrozenWorkflow, at: &StepId, produced: Option<&Produced>) -> String {
    let steps = workflow.steps();
    let position = steps.iter().position(|step| step.id() == at);
    let mut block = format!("WHERE YOU ARE\n\nThis task runs in {} parts.", steps.len());
    if let Some(index) = position {
        block.push_str(&format!(" You are on part {}.\n", index + 1));
    } else {
        block.push('\n');
    }
    for (index, step) in steps.iter().enumerate() {
        let mark = match position {
            Some(here) if index < here => "done",
            Some(here) if index == here => "you are here",
            _ => "not yours — do not do it",
        };
        block.push_str(&format!("\n  {}. {} — {mark}", index + 1, step.label()));
        if position == Some(index) {
            block.push_str("\n     STOP. Submit when this part is done, then wait.");
        }
    }
    if let Some(produced) = produced {
        block.push_str("\n\n");
        block.push_str(&produced.text());
    }
    block.push_str(
        "\n\nThe parts after this one happen after you submit, and doing them \
         yourself does not move this task forward. Leave the branch in a state \
         they can start from.",
    );
    block
}

/// The step itself, and the one instruction that is the same on every step:
/// what to claim.
///
/// The closing line is where a work submission's `not_claimed` field comes
/// from — an adjacent problem noticed and left alone has somewhere to land.
fn step_block(step: &ResolvedStep) -> String {
    format!(
        "STEP: {}\n\nWhat you claim should be what the work now does, not that \
         you finished. An adjacent problem you notice and leave alone goes \
         under Not claimed.",
        step.label()
    )
}
