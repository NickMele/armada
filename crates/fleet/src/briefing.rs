//! The first turn a Drone is given, assembled from the Job it is being put on.
//!
//! # The baseline is quoted, not written here
//!
//! `docs/contracts/agent-prompt.md` section 5 carries the M1 rendering as
//! sanctioned copy, and the three clauses it contains each passed a membership
//! test this module is in no position to re-run. So [`BASELINE`] is that text,
//! transcribed, and the rest of this module assembles the per-Job blocks around
//! it. **A wording change belongs in the contract**, and a copy here that
//! drifted from it would be the second-vocabulary defect in prose.
//!
//! # Four blocks of six layers, and the two that are missing are missing
//!
//! The contract names six layers: baseline, Kit, Manifest, WorkflowDef framing,
//! the Job brief, and the step. **M1 has no Kit and no Manifest layer**, which
//! the contract's own M1 rendering says, so what is assembled here is baseline,
//! job brief, where-you-are, step — the four blocks the rendering shows.
//!
//! What is not assembled: the exemplar corpus, the injected reference material
//! a review step needs, and what the previous step produced. Each needs a
//! record M1 has no type for, and a block rendered empty reads to a Drone as a
//! block that was answered. The scope block is written **only** where the step
//! declares one — the only step where the call exists.
//!
//! One block is assembled that the rendering does not show: [`notekeeping`],
//! which names the one path a Drone's own files belong under. Its wording is
//! **drafted**, like the gaming half of [`Stopped`] and the whole of
//! [`Redeclaring`] — the contract has no sanctioned copy for it, and sanctioned
//! copy is the contract's to write.
//!
//! # A Drone is not told what the Checks are. It is told it can run them
//!
//! This module used to say the first half and stop there: telling a Drone the
//! Check would let it satisfy the Check rather than do the work. **The owner
//! overruled that on 2026-08-28** — *"this is what the judge is for and the
//! gaming checks"* — because the defence against a Drone satisfying the bar
//! instead of doing the work is `docs/concepts/judge.md` and the gaming
//! patterns, `check_config_edited` among them, and not keeping the Drone
//! ignorant. What the old rule actually cost is in `crate::dry_run`: a Drone
//! that could not run a single command that would tell it anything, hand-
//! checked its work, said so honestly, and failed a Check it had no way to see
//! coming.
//!
//! So the narrowing that remains is narrower and is about this module rather
//! than about the system. **No block here is written from a `ResolvedCheck`'s
//! `run` string** — the step block is written from [`ResolvedStep::label`] and
//! the Job's own fields, and [`Checking`] says a tool exists rather than what
//! it will run. A Drone that wants to know what the Checks are calls the tool
//! and reads what they printed, which is Fleet running them rather than a
//! Drone reading a command out of a prompt.
//!
//! Five blocks outlive the turn they were written for and are types rather
//! than paragraphs: [`Declaring`], [`Redeclaring`], [`Checking`], [`Stopped`]
//! and [`Reconciling`]. An opening turn is asked for as [`Opening`] rather than
//! assembled by the caller: every spawn rebases first, so what a Drone is told
//! about its branch is not known when the caller would have built the prompt.

use adapter_traits::{Prompt, SpawnConfigRefused};
use core_model::{
    EscalationTrigger, FrozenWorkflow, GamingFlag, Job, JobId, Judgment, RepoPath, ResolvedStep,
    StepId, StepVerdict,
};
use verification::TheBaseMoved;

/// Layer 1, verbatim from the Agent Prompt Contract's M1 rendering.
///
/// **Mechanics, never task content**, and identical on every step of every Job
/// — which is what makes it a constant rather than something assembled.
pub const BASELINE: &str = "\
You are working in a git worktree on a branch of your own. You cannot push, \
open a pull request, or run commands this repository has not declared.

When you have finished the work described below, you must report it using the \
evidence submission tool you have been given. It is the only way to report. \
Work you do not submit is work no one sees, and the task will not move on.

Submitting returns \"recorded\". That is a receipt, not a verdict — your work \
is checked after you submit. If it does not pass you will be told in a later \
turn, with the reason. Wait for that turn.";

/// What a Drone being put on a worktree is opening with.
///
/// **The two first turns as a value rather than as two calls**, so that the one
/// funnel every spawn goes through — [`put_a_drone_on`] — can decide the brief
/// *after* the rebase it runs has answered. A caller that assembled the prompt
/// itself would have had to be handed the catch-up back, which is the fourth
/// call site `#180` exists to avoid.
///
/// [`put_a_drone_on`]: crate::daemon::Fleet::put_a_drone_on
#[derive(Clone, Debug)]
pub enum Opening {
    /// A step no Drone has attempted yet — a Job's first, or the one an
    /// override advanced to.
    Fresh,
    /// A step that stopped, and what the record says stopped it.
    Resuming(Stopped),
}

impl Opening {
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
    ) -> Result<Prompt, SpawnConfigRefused> {
        let brief = match self {
            Opening::Fresh => first_turn(job, workflow, at)?,
            Opening::Resuming(stopped) => resuming_turn(job, workflow, at, stopped)?,
        };
        let Some(moved) = moved else {
            return Ok(brief);
        };
        Prompt::assembled(&format!(
            "{}\n\n{}",
            brief.as_str(),
            Reconciling::of(moved).text()
        ))
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
/// **Drafted wording**, like [`Redeclaring`] and the gaming half of [`Stopped`].
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

/// Assemble the first turn for a Job standing at one step of its workflow.
///
/// **There is no argument through which arbitrary text reaches a Drone.** The
/// blocks are built from the Job record and the resolved workflow, and a caller
/// that wanted to say something else would have to add a block here — which is
/// the same refusal `Turn` makes about a prepared string.
///
/// Refuses only where the assembled text is empty, which
/// [`Prompt::assembled`] decides and this does not restate.
pub fn first_turn(
    job: &Job,
    workflow: &FrozenWorkflow,
    at: &StepId,
) -> Result<Prompt, SpawnConfigRefused> {
    Prompt::assembled(&assemble(job, workflow, at))
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
) -> Result<Prompt, SpawnConfigRefused> {
    let mut text = assemble(job, workflow, at);
    text.push_str("\n\n");
    text.push_str(&stopped.block());
    Prompt::assembled(&text)
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
    /// **One sentence per trigger, matched exhaustively.** `gate_failure`,
    /// `evidence_suspect`, `gate_undecided` and `thrashing` are four different
    /// things to be told and a Drone acts on what it is told, so a trigger
    /// added to the registry is a compile error here rather than a Drone
    /// quietly handed the nearest sentence.
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
            | EscalationTrigger::ResourceExhausted
            | EscalationTrigger::Silent
            | EscalationTrigger::Stalled => {
                "An earlier attempt at this part stopped. The record holds no verdict \
                 against its work."
            }
        }
    }
}

fn assemble(job: &Job, workflow: &FrozenWorkflow, at: &StepId) -> String {
    let mut text = String::from(BASELINE);
    text.push_str("\n\n");
    text.push_str(&notekeeping(job.id()));
    text.push_str("\n\n");
    text.push_str(&job_brief(job));
    text.push_str("\n\n");
    text.push_str(&where_you_are(workflow, at));
    if let Some(step) = workflow.steps().iter().find(|step| step.id() == at) {
        text.push_str("\n\n");
        text.push_str(&step_block(step));
        if let Some(asked) = Declaring::at(step) {
            text.push_str("\n\n");
            text.push_str(asked.text());
        }
        if let Some(offered) = Checking::at(step) {
            text.push_str("\n\n");
            text.push_str(offered.text());
        }
    }
    text
}

/// What a step tells its Drone about the dry run, where it has Checks to run.
///
/// **A tool nothing points at is the defect this whole capability is about.**
/// Spike 6 measured that a description alone does not make a Drone call a tool,
/// which is why the Evidence obligation is in the baseline and the scope ask is
/// in [`Declaring`]; this is the same fact applied to an offer rather than an
/// obligation.
///
/// **It offers and does not instruct.** A block a Drone reads as a requirement
/// puts every step through a build nobody asked for, and the cost of one is
/// minutes. So the whole of it is conditional, and the last sentence says
/// outright that not calling it is a legitimate way to work.
///
/// **It names no number.** The allowance is Fleet's and is named in the refusal
/// a Drone gets when it is spent — `docs/concepts/drone.md` keeps counters out
/// of what a Drone is told, because a counter is a bar, and "two runs left" is
/// a thing to optimise against rather than information about the work.
///
/// **It says twice that this is not the gate**, in the two places a Drone could
/// stop reading: that a pass here is not a pass, and that submitting is still
/// the only way to report. A Drone that read a green dry run as a finished step
/// would have been made worse off by being offered this at all.
///
/// **Drafted wording**, like [`Redeclaring`] and the gaming half of [`Stopped`].
/// `docs/contracts/agent-prompt.md` has no sanctioned copy for it, and
/// sanctioned copy is the contract's to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checking(String);

impl Checking {
    /// The offer this step makes, or `None` where it has nothing to run.
    ///
    /// **The step's own declared Checks are the switch**, and it is the same
    /// switch `crate::dry_run` refuses on: a step declaring none would answer
    /// every call with a refusal, and a Drone pointed at a tool that refuses it
    /// is a Drone reading a denial as a broken system. That is the defect this
    /// capability exists to close, arriving from the other side.
    pub fn at(step: &ResolvedStep) -> Option<Checking> {
        if step.checks().is_empty() {
            return None;
        }
        Some(Checking(String::from(
            "FINDING OUT WHERE YOU STAND\n\nYou can ask for the checks that \
             gate this part to be run against your worktree, and you will be \
             told what each one did and where its output was written. Use it \
             when you want to know whether the work holds up rather than \
             guessing — and note that it takes as long as the checks take.\n\n\
             It is not a verdict and it advances nothing. A run in which \
             everything passes does not finish this part; the checks are run \
             again when you submit, and that run is the one that decides. \
             Submitting is still the only way to report. There is a limit on \
             how many times one part may ask, and you do not have to ask at \
             all.",
        )))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a step asks its Drone to declare before starting, where it asks at all.
///
/// **The obligation is here rather than in the tool's description**, for the
/// reason the baseline carries the Evidence obligation: spike 6 measured that a
/// description alone does not make a Drone call a tool.
///
/// **And it is a value rather than a private paragraph**, because the ask is
/// made more than once. The first turn carries it, and so does the turn a Drone
/// gets when a step advances underneath it — see [`Declaring::at`] for why the
/// second one is not optional.
///
/// The consequence is stated plainly and without a threat: a plan that turns
/// out wrong is fixed by declaring again, and work belonging to a later part
/// does not become this part's by being named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaring(String);

impl Declaring {
    /// The ask this step makes, or `None` where it makes none.
    ///
    /// **`wants_a_declaration` decides it and nothing else does.** That is the
    /// cold switch `core_model::EvidenceScope` describes: a step with no
    /// evidence scope, or one whose context comes from somewhere other than the
    /// Drone, is told exactly what it was told before any of this existed.
    /// Inferring the ask from `declare_plan_at` or from `scope_diff_check`
    /// would put a tool call in front of a Drone that has no tool to make it
    /// with.
    ///
    /// **Every step that wants one is asked, including the fourth step of a
    /// Job whose Drone declared correctly on the first.** Job
    /// `01M14F8VFA00189ZBMF0HXE607` declared its scope on the step that asked
    /// for it, advanced, worked the next step for twenty-two minutes, and
    /// failed `evidence_scope` on a call nobody had requested: the declaration
    /// is cleared at the boundary and the ask was not repeated there.
    pub fn at(step: &ResolvedStep) -> Option<Declaring> {
        let scope = step.evidence_scope()?;
        if !scope.wants_a_declaration() {
            return None;
        }
        let mut block = String::from(
            "BEFORE YOU START\n\nCall the scope tool with the repository-relative \
             paths this part's work will be in. Include what you will change and \
             what has to be read to judge the change. Each part is checked \
             against what was declared for it, and what you declared for an \
             earlier part does not carry over.",
        );
        if scope.scope_diff_check() {
            block.push_str(
                " Files you change outside them are compared against what you \
                 declared. If the work turns out to be somewhere else, call the \
                 tool again — a plan that changed is fine, and a file changed \
                 for the next part is not.",
            );
        }
        if !scope.exclude_paths().is_empty() {
            block.push_str("\n\nDo not name these, and do not change them:");
            for path in scope.exclude_paths() {
                block.push_str("\n  - ");
                block.push_str(path.as_str());
            }
        }
        Some(Declaring(block))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a Drone is told when its work turns up outside the plan it declared.
///
/// **The other half of [`Declaring`], and it was missing.** The live check in
/// `crate::scope` has compared edits against the plan since the scope tool
/// existed, and everything it found went to the Job's log. The Drone was never
/// told, so the one call that fixes a plan that turned out wrong was a call it
/// had no reason to make: Job `01M14HZ8ND001FYT6264WZJFPB` drifted onto
/// `crates/ipc/src/lib.rs`, carried on for seven minutes and reached its gate
/// still holding a declaration it had outgrown.
///
/// **It offers rather than demands, and the wording is the whole mechanism.**
/// `docs/concepts/judge.md` keeps drift a signal because legitimate
/// investigation moves the work, so a Drone that reads this as an accusation
/// and apologises, or as a stop-work order and downs tools over a file it was
/// right to touch, has been made worse off by being told. Three sentences carry
/// that: nothing has failed, you are not being asked to stop, and here is the
/// call that makes the plan true. The stop-and-report directive is
/// `crate::converging::ReportNow` and is a different act.
///
/// **Drafted wording**, like the gaming half of [`Stopped`].
/// `docs/contracts/agent-prompt.md` has no sanctioned copy for a mid-step
/// scope notice, and the phrasing here follows [`Declaring`]'s so a Drone
/// reads one vocabulary rather than two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redeclaring(String);

impl Redeclaring {
    /// The notice for paths seen outside the plan, or `None` where there is
    /// nothing to say.
    ///
    /// **There is no constructor taking a string**, and the step decides
    /// whether one exists at all — the same narrowing [`Declaring::at`] makes.
    /// `watches_live_edits` is the switch rather than `scope_diff_check`: a
    /// step whose plan arrives at the gate has no live plan to correct, and a
    /// Drone told to call a tool it was never asked to call goes looking for
    /// one.
    ///
    /// **`drifted` is what was seen for the first time**, which
    /// [`Working::drifting`](crate::working::Working::drifting) already
    /// answers. Passing everything seen so far would say the same thing every
    /// turn, and a notice a Drone has already acted on is one it reads as
    /// having been ignored.
    pub fn at(step: &ResolvedStep, drifted: &[RepoPath]) -> Option<Redeclaring> {
        if drifted.is_empty() || !step.evidence_scope()?.watches_live_edits() {
            return None;
        }
        let mut block = String::from(
            "FILES OUTSIDE WHAT YOU DECLARED\n\nThe plan you declared for this \
             part does not cover everything that has changed:",
        );
        for path in drifted {
            block.push_str("\n  - ");
            block.push_str(path.as_str());
        }
        block.push_str(
            "\n\nNothing has failed and you are not being asked to stop. If this \
             part's work is there, call the scope tool again with every path the \
             work is in. The new call replaces the plan, and that is how a plan \
             that turned out wrong is corrected. If that work belongs to a later \
             part, leave it to that part.",
        );
        Some(Redeclaring(block))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Where a file a Drone writes for itself goes, which is not the repository
/// root.
///
/// **Every other per-Job artifact is already keyed by Job id**: the log at
/// `.armada/logs/<job-id>.jsonl`, a Check's output under
/// `.armada/checks/<job-id>/`, the worktree itself, and the brief's
/// attachments, which `drafting` keeps under `<attachments_dir>/<job_id>/` and
/// [`job_brief`] names a line at a time. A plan was the one piece that was
/// not, and one shared slot at the root cannot say whether what is in it is
/// still live — the `PLAN.md` this repository carried belonged to a Job that
/// finished and merged, and every worktree cut after it inherited a confident
/// plan for work nobody had asked for.
///
/// **No Drone is known to have been misled by it.** Nothing pointed at that
/// file — not this module, not the Manifest, not the agent files — so a Drone
/// reached it only by going looking. The reason for this block is that the
/// Job-keyed path is the correct home, not that harm was observed.
///
/// **It offers a place; it does not ask for a plan.** A block a Drone reads as
/// an instruction puts every Job through a planning step nobody requested, so
/// the whole of it is conditional and the last sentence says so outright.
///
/// **"Plan" already means something else here.** [`Declaring`] calls the
/// declared scope "the plan you declared", and a second meaning in the same
/// turn is the second-vocabulary defect in prose. So this block leads with what
/// a Drone is actually holding — a file it wrote for itself — and a plan is one
/// kind of it rather than the subject.
///
/// A private function rather than one of the three types above: it is rendered
/// where the turn is assembled and nowhere else, and does not outlive it.
fn notekeeping(job: &JobId) -> String {
    format!(
        "FILES YOU WRITE FOR YOURSELF\n\nA plan, a checklist, notes you want \
         to keep between turns — anything you write for yourself rather than \
         for the work goes under .armada/{}/, which is this task's alone. \
         None of it belongs at the repository root, where a file outlives the \
         task that wrote it with nothing to say that task is over. Nothing \
         here is asking you to write any of it.",
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
fn where_you_are(workflow: &FrozenWorkflow, at: &StepId) -> String {
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
