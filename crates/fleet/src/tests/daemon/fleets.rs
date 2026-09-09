//! A Fleet assembled over fakes, and the [`Fittings`] it is assembled from.
//!
//! **One set of fittings, varied by one field.** Every builder here calls
//! [`fitted_with`] and then changes the single thing its cases are about — the
//! version control, the workflow, the Judge, the mint. A second full set would
//! be a second answer to what a fixture Fleet is, and the two would drift.
//!
//! **A dial that ships is planted at the number that ships; a dial that would
//! make an unrelated case flaky is planted out of reach.** Which of the two
//! each one is, and why, is written on the field in [`fitted_with`] rather than
//! here, because that is where somebody changing it is standing.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Model;
use config::Manifest;
use store::Store;
use testkit::{FakeHarness, FakeJudge, FakeLinkLookup, FakeVcs, FakeWorkProduct};

use super::workflows::{
    manifest, one, two_steps, two_steps_delivering_nothing, two_steps_gated_on_a_manifest_rule,
    two_steps_gated_on_a_manifest_rule_judged, two_steps_gated_on_a_person,
};
// Through the parent's re-exports, which are what every other module in the
// suite reaches these four by.
use super::{Counted, Ticking, NEVER_QUIET, UNTRIPPABLE};
use crate::allowance::{Allowance, Micros};
use crate::daemon::{Fittings, Fleet, Host};
use crate::dry_run::DryRuns;
use crate::gate::CheckBudget;
use crate::headroom::{Bytes, Headroom, Polling, Spare};
use crate::holding::Reclaiming;
use crate::judging::JudgeBudget;
use crate::noticing::Noticing;
use crate::slots::Concurrency;
use crate::tests::tmp::TempDir;

/// Everything a Fleet is assembled from, over one temporary directory.
pub fn fittings(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    fitted_with(home, work, FakeHarness::that_listens())
}

/// A budget no fixture can reach.
///
/// **The opposite call to `headroom`'s**, which plants the production
/// threshold: a cap that ships is worth tripping in a fixture, and a budget
/// that ships is not, because the fakes report no cost at all. Every case about
/// the budget plants its own — see `crate::tests::allowance`.
const UNSPENDABLE: Allowance = Allowance::of(Micros::dollars(1_000_000), u64::MAX);

pub fn fitted_with(
    home: &TempDir,
    work: FakeWorkProduct,
    harness: FakeHarness,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let root = home.path().to_string_lossy().to_string();
    Fittings {
        store: Store::open(&home.path().join("armada.db")).expect("a store"),
        harness,
        vcs: FakeVcs::new(),
        work,
        clock: Arc::new(Ticking::from_nine()),
        mint: Arc::new(Counted::from_one()),
        workflows: one(two_steps()),
        manifest: manifest(),
        host: Host {
            user: String::from("someone"),
            repo_root: root.clone(),
            path: "/usr/bin:/bin".to_string(),
            home: root,
            mcp_config: "/etc/armada/mcp.json".to_string(),
            // A port nothing is listening on. Attribution is planted in these
            // fixtures, so this is the other half of a pair no fake ever
            // matches on.
            port: 47821,
            attachments_dir: home
                .path()
                .join("attachments")
                .to_string_lossy()
                .to_string(),
        },
        // Nothing to place. A fake harness opens no sockets, so a fixture that
        // answered otherwise would be asserting against the machine rather than
        // against Fleet. `crate::tests::peer` plants one where the subject is
        // attribution.
        peers: Arc::new(crate::tests::peer::TheOnlyDrone),
        // **One, so every fixture but the concurrency cases behaves exactly as
        // it did when there was one slot.** A bound of two here would make every
        // test that queues a second Job assert a different thing than it was
        // written to.
        concurrency: Concurrency::of(1),
        // A machine with plenty of everything, so no fixture but `headroom`'s
        // own is ever held back by one. Reading the real machine here would
        // make every test in this crate pass or fail on what else is running.
        machine: Arc::new(crate::tests::headroom::Plentiful),
        // The production threshold, so the cases that trip it trip the one that
        // ships. See `armada::serve::PROVISIONAL_HEADROOM`.
        headroom: Headroom::of(Spare::percent(15), Bytes::gibibytes(10)),
        // No interval, so a fixture that moves the machine sees it move. The
        // one case about the interval sets its own.
        polling: Polling::every(Duration::ZERO),
        // Never due, so no test asks a fake forge anything unless it says to.
        noticing: Noticing::every(Duration::from_secs(86_400)),
        // A day, for `noticing`'s reason: a fixture that swept on its own would
        // have every other test in this crate racing a worktree removal. The
        // tests that want a sweep ask for one.
        reclaiming: Reclaiming::every(Duration::from_secs(86_400)),
        // **Far more than any fixture spends**, so no test but `allowance`'s
        // own is ever held back by the budget. The fakes report a `cost_micros`
        // of zero, and a cap set at the shipped number would still be reached
        // by a fixture that turned three hundred times.
        allowance: UNSPENDABLE,
        budget: CheckBudget::of(Duration::from_secs(5)),
        norms: UNTRIPPABLE,
        liveness: NEVER_QUIET,
        // The production allowance, so the cases that spend it spend the number
        // that ships. A fixture with its own would prove a cap and not the cap.
        dry_runs: DryRuns::of(3),
        // A Judge that fails every call, because no step in these fixtures
        // declares a criterion. One that answered would let a cold-by-default
        // regression pass unseen.
        judge: Arc::new(FakeJudge::that_fails("a Judge that should never be asked")),
        judge_budget: JudgeBudget::of(Duration::from_secs(5)),
        // The same five seconds. A suite's proposals answer in milliseconds, so
        // the split that matters in production — a watched call outliving an
        // unwatched one — buys a test nothing but a slower failure when one
        // hangs.
        proposer_budget: JudgeBudget::of(Duration::from_secs(5)),
        judge_model: Model::named("the-cheap-model").expect("a model name"),
        proposer_model: Model::named("the-cheap-model").expect("a model name"),
        // Resolves nothing, so every fixture but `proposing`'s own behaves
        // exactly as it did before this seam existed. The cases about it
        // plant their own.
        links: Arc::new(FakeLinkLookup::resolving_nothing()),
        // Planted, not read. The composition root resolves these from the
        // environment and the adapter; a test that read the same sources would
        // be asserting against a machine rather than against Fleet.
        models: ipc::ModelChoices {
            models: vec!["a-model".to_string(), "another-model".to_string()],
            default: "a-model".to_string(),
        },
        events: api::Broadcaster::new(),
    }
}

/// A Fleet whose Drone holds its input open, so the gate has something to speak
/// to when a step advances.
pub fn a_fleet(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    Fleet::assembled(fittings(home, work))
}

/// A Fleet whose version control is scripted. What `landing` needs: the commit
/// a finished Job gets is the fake's to record, refuse, or answer as nothing,
/// and what `delivery` needs: where the branch stands and what a push answers.
pub fn a_fleet_committing_through(
    home: &TempDir,
    work: FakeWorkProduct,
    vcs: FakeVcs,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// The same, on a workflow **neither of whose steps sends the work out**. What
/// it buys is a step boundary with nothing published on it, which is what a
/// case about the rebase alone wants — see `two_steps_delivering_nothing`.
pub fn a_fleet_delivering_nothing(
    home: &TempDir,
    work: FakeWorkProduct,
    vcs: FakeVcs,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(two_steps_delivering_nothing());
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// A Fleet whose workflow puts a person on one step's gate, committing through
/// this version control — the second half is what the last-step case needs,
/// since approving there lands the work.
pub fn a_fleet_gated_on_a_person(
    home: &TempDir,
    work: FakeWorkProduct,
    gate_on: &str,
    vcs: FakeVcs,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(two_steps_gated_on_a_person(
        gate_on,
        None,
        Some("summarise"),
    ));
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// The same, with the step's gate naming a Manifest policy rather than a
/// person. **Delivering nothing**, for the sibling below's reason.
pub fn a_fleet_gated_on_a_manifest_rule(
    home: &TempDir,
    work: FakeWorkProduct,
    gate_on: &str,
    key: &str,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(two_steps_gated_on_a_manifest_rule(gate_on, key, None));
    Fleet::assembled(fittings)
}

/// The same, over an `armada.yml` that says something about the policy — and,
/// where a question is given, over a step that asks the Judge it.
///
/// **The Manifest and the workflow move together on purpose.** What a
/// `manifest_rule:` gate comes to is the pair, and a fixture that could set one
/// without the other would let a case assert against half of the resolution.
pub fn a_fleet_gated_on_a_manifest_rule_saying(
    home: &TempDir,
    work: FakeWorkProduct,
    gate_on: &str,
    key: &str,
    says: &str,
    question: Option<&str>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    // **A Judge that objects to nothing, where there is a question at all.**
    // What is under test is what the policy does with a step that *has* a
    // judgment, and the default fixture Judge refuses every call — which would
    // make the case pass for the wrong reason, on a ruling that never reached
    // the gate.
    fittings.judge = Arc::new(FakeJudge::with_no_objection());
    fittings.workflows = one(match question {
        None => two_steps_gated_on_a_manifest_rule(gate_on, key, None),
        Some(question) => two_steps_gated_on_a_manifest_rule_judged(gate_on, key, question),
    });
    fittings.manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{says}"),
    )
    .expect("a manifest that parses");
    Fleet::assembled(fittings)
}

/// The same, on a workflow **neither of whose steps sends the work out**, for
/// `a_fleet_delivering_nothing`'s reason: a case about what a human boundary
/// does to a branch would otherwise read a commit, a second rebase, a push and
/// a pull request out of the same delta.
pub fn a_fleet_gated_on_a_person_delivering_nothing(
    home: &TempDir,
    work: FakeWorkProduct,
    gate_on: &str,
    vcs: FakeVcs,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(two_steps_gated_on_a_person(gate_on, None, None));
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// A Fleet over an `armada.yml` that names the branch its work merges into.
pub fn a_fleet_whose_manifest_declares_a_base(
    home: &TempDir,
    work: FakeWorkProduct,
    vcs: FakeVcs,
    base: &str,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.vcs = vcs;
    fittings.manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\nbase: {base}\n"),
    )
    .expect("a manifest that parses");
    Fleet::assembled(fittings)
}

/// A Fleet over a store another Fleet wrote to, holding a workflow of its own.
///
/// The pair of arguments is what an edited `.armada/workflows/` looks like from
/// inside the process: Fleet reads the file once at assembly, so a different
/// definition and a restart are the same event.
pub fn a_fleet_holding(
    home: &TempDir,
    work: FakeWorkProduct,
    workflow: config::ResolvedWorkflow,
    next: u64,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(workflow);
    fittings.mint = Arc::new(Counted::from_next(next));
    Fleet::assembled(fittings)
}

/// A Fleet holding every one of these workflows, keyed by each one's own id —
/// what an `.armada/workflows/` with more than one definition looks like from
/// inside the process.
pub fn a_fleet_holding_all(
    home: &TempDir,
    work: FakeWorkProduct,
    workflows: Vec<config::ResolvedWorkflow>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = workflows
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    Fleet::assembled(fittings)
}

/// A Fleet whose steps are judged, and by whom.
///
/// The workflow is an argument because the Judge is cold by default: the
/// fixture workflow declares no criterion, so a Fleet that only swapped the
/// client would never make a call.
/// `impl Into<Arc<_>>` so a case that asserts **what the call was asked** can
/// keep a handle on the judge, which records the questions.
pub fn a_fleet_judged_by(
    home: &TempDir,
    work: FakeWorkProduct,
    workflow: config::ResolvedWorkflow,
    judge: impl Into<Arc<FakeJudge>>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(workflow);
    fittings.judge = judge.into();
    Fleet::assembled(fittings)
}

/// A Fleet whose dispatch requests are read by this client, holding these
/// workflows.
///
/// The workflows are an argument because they are half of what the proposer is
/// told: a catalogue it cannot choose from proves nothing about it choosing.
pub fn a_fleet_proposing_through(
    home: &TempDir,
    work: FakeWorkProduct,
    workflows: Vec<config::ResolvedWorkflow>,
    proposer: FakeJudge,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = workflows
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(proposer);
    Fleet::assembled(fittings)
}

/// A Fleet over a store another Fleet wrote to. See [`Counted::from_next`].
pub fn a_fleet_minting_from(
    home: &TempDir,
    work: FakeWorkProduct,
    next: u64,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.mint = Arc::new(Counted::from_next(next));
    Fleet::assembled(fittings)
}

/// A Fleet whose Drone reads its first turn, prints it back and exits — which
/// is a Drone that finished having submitted nothing.
pub(super) fn a_fleet_whose_drone_leaves(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    Fleet::assembled(fitted_with(
        home,
        work,
        FakeHarness::that_echoes_its_first_turn(),
    ))
}
