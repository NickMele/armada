//! Leases: how long-running work holds machine-wide claims (PLAN.md §4.3),
//! written as a reducer.
//!
//! `ARCHITECTURE.md` §1.2 scopes reducers to the scheduler **and this loop**,
//! and the reason is the same in both places: acquisition is a state machine
//! whose interesting states are the ones that are hard to reach on purpose —
//! the holder that died between heartbeat and reap, the ceiling that expires
//! while a legitimate 30-minute hold is still running, the race lost on the
//! insert. A reducer puts all of them inside a unit test.
//!
//! ```text
//! acquire   insert a lease row
//! hold      renew the heartbeat from the shell's event loop
//! release   delete the row on exit
//! reclaim   a lease whose heartbeat has gone cold is dead — take it
//! ```
//!
//! **There is deliberately no TTL.** The fixture set contains a 1800-second
//! check holding `exclusive: [gpu]`, so a TTL long enough not to fire on a
//! healthy job is useless as wedge detection — and when it fires wrongly on an
//! exclusive, two workspaces hold one mutex with no error anywhere. What bounds
//! the wait instead is *silence*: a renewal happens in the loop that steps the
//! reducer, never on a background timer, so a wedged loop is a loop that stopped
//! renewing.

use crate::error::{CharError, ErrClass};
use crate::id::WorkspaceId;
use serde::Serialize;
use std::fmt;

/// Milliseconds of silence after which a lease is dead and may be taken.
///
/// Twelve missed renewals at [`RENEW_INTERVAL_MS`], because the cost of being
/// wrong is a stolen exclusive and the cost of waiting is one minute. **This is
/// not a TTL: it bounds silence, not the hold.**
pub const COLD_MS: u64 = 60_000;

/// How often a holder renews its heartbeat, from the shell's event loop.
pub const RENEW_INTERVAL_MS: u64 = 5_000;

/// How long the shell sleeps between attempts while blocked on a lease.
///
/// Well under [`RENEW_INTERVAL_MS`] so a released lease is picked up promptly,
/// and far enough above zero that a fifteen-minute wait is not a spin.
pub const POLL_INTERVAL_MS: u64 = 500;

/// What a lease is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LeaseKind {
    /// One per workspace, held by everything that mutates: `init`, `up`,
    /// `down`, `clean`, `check`, and every `commands:` entry.
    Run,
    /// One per machine, held by `clean --all` — which runs from outside any
    /// workspace and would otherwise be the only mutating verb with no lock.
    /// Its row carries a NULL workspace.
    Machine,
    /// A unit of the machine's CPU budget. Machine-wide, not per-run: five
    /// concurrent workspaces each granting themselves the full budget is
    /// sustained 5× oversubscription on exactly the case this project is
    /// built around.
    CpuSlot,
    /// A named mutex — `browser`, `gpu`. Machine-wide for the same reason.
    Exclusive,
}

impl fmt::Display for LeaseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LeaseKind::Run => "run",
            LeaseKind::Machine => "machine",
            LeaseKind::CpuSlot => "cpu-slot",
            LeaseKind::Exclusive => "exclusive",
        })
    }
}

/// Which lease. The key distinguishes two leases of one kind — `browser` from
/// `gpu`, slot 3 from slot 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseId {
    /// `None` only for [`LeaseKind::Machine`].
    pub workspace: Option<WorkspaceId>,
    /// Which kind.
    pub kind: LeaseKind,
    /// The name within the kind.
    pub key: String,
}

impl LeaseId {
    /// The run lease for one workspace.
    ///
    /// **The key is the workspace id, and that is load-bearing.** `char.db`
    /// keys leases by `(kind, key)` because a cpu-slot and an exclusive are
    /// machine-wide budgets — slot 3 is slot 3 for everyone, `browser` is one
    /// browser. The run lease is the opposite: it is *per workspace*, and five
    /// agents in five worktrees must never contend on it. A constant key here
    /// makes the second worktree's `char init` fail with "a run is already in
    /// flight" against a run in a different directory, which is the exact
    /// silent-serialisation failure the flat-siblings model exists to prevent.
    pub fn run(workspace: WorkspaceId) -> Self {
        LeaseId {
            key: workspace.as_str().to_string(),
            workspace: Some(workspace),
            kind: LeaseKind::Run,
        }
    }

    /// The machine lease `clean --all` takes.
    pub fn machine() -> Self {
        LeaseId {
            workspace: None,
            kind: LeaseKind::Machine,
            key: "all".to_string(),
        }
    }

    /// A named exclusive.
    pub fn exclusive(workspace: WorkspaceId, name: impl Into<String>) -> Self {
        LeaseId {
            workspace: Some(workspace),
            kind: LeaseKind::Exclusive,
            key: name.into(),
        }
    }
}

/// Who is holding a lease char wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The workspace holding it, when it has one.
    pub workspace: Option<WorkspaceId>,
    /// The holding process.
    pub pid: i32,
    /// How long it has held it, in milliseconds.
    pub held_ms: u64,
}

/// What a waiting result reports, and it carries the distinction that matters:
/// whether the cause is inside this workspace or outside it (PLAN.md §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum WaitingOn {
    /// This run's own budget. It will clear on its own.
    CpuSlot {
        /// Slots this check needs.
        cpu_slot: u32,
        /// Slots free right now.
        available: u32,
    },
    /// **Another workspace** is the reason — the thing an agent cannot work
    /// out for itself, and the only useful answer to "why has this taken
    /// fifteen minutes".
    Exclusive {
        /// The exclusive's name.
        exclusive: String,
        /// The workspace holding it.
        held_by: WorkspaceId,
        /// How long it has held it.
        since_ms: u64,
    },
    /// A run in this workspace, or the machine lease.
    Run {
        /// The lease kind, spelled as it is in `char.db`.
        run: String,
        /// The holding pid.
        pid: i32,
        /// How long it has held it.
        since_ms: u64,
    },
}

/// Whether a claim that cannot be satisfied waits or gives up.
///
/// The two differ because the questions differ: "is another run already going
/// in *my* workspace?" is almost always a mistake worth reporting, while "is
/// the machine busy?" is a budget, and a budget that errors instead of queueing
/// is not a budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Report the holder immediately (the run and machine leases).
    FailFast,
    /// Queue, visibly (cpu-slots and exclusives).
    Block,
}

/// Where a claim has got to.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Nothing attempted yet.
    Start,
    /// An insert is in flight.
    Attempting,
    /// Someone live holds it.
    Waiting(Holder),
    /// Held by this process.
    Granted,
    /// Given up, with the reason.
    Failed(CharError),
}

/// One claim in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimState {
    /// Which lease.
    pub lease: LeaseId,
    /// Wait or fail fast.
    pub policy: Policy,
    /// The acquisition ceiling in milliseconds, or `None` under `--wait`.
    ///
    /// **`--wait` is exempt from the ceiling.** `acquire_timeout` bounds waits
    /// char imposes; `--wait` is the caller asking, and the fixtures contain a
    /// 1800-second check, so a ceiling there turns "queue behind that run" into
    /// "fail after fifteen minutes" for a run behaving exactly as specified.
    pub ceiling_ms: Option<u64>,
    /// Cumulative time spent waiting to acquire. It counts waiting, never
    /// running: a check that waits 14 minutes and then runs for an hour is not
    /// affected.
    pub waited_ms: u64,
    /// Where the claim has got to.
    pub phase: Phase,
}

impl ClaimState {
    /// A fresh claim.
    pub fn new(lease: LeaseId, policy: Policy, ceiling_ms: Option<u64>) -> Self {
        ClaimState {
            lease,
            policy,
            ceiling_ms,
            waited_ms: 0,
            phase: Phase::Start,
        }
    }
}

/// What the shell observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimEvent {
    /// Begin.
    Start,
    /// The insert succeeded — the lease is ours.
    Granted,
    /// A live holder is in the way. Losing an insert race lands here too: the
    /// winner is a live holder like any other.
    Held(Holder),
    /// The holder's heartbeat has gone cold, or its row predates this boot.
    HolderCold(Holder),
    /// The shell slept for this long.
    Slept {
        /// Milliseconds actually slept.
        ms: u64,
    },
    /// SIGINT.
    Interrupted,
}

/// What the shell should do next.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimAction {
    /// Re-decide and try the insert, inside one `BEGIN IMMEDIATE`.
    ///
    /// Re-decide is deliberate: for a port block the choice depends on what is
    /// already claimed, and losing a race means the answer changed.
    Attempt,
    /// Delete the dead row, then attempt again.
    Reclaim(Holder),
    /// Sleep, then report back with [`ClaimEvent::Slept`].
    Sleep {
        /// Milliseconds.
        ms: u64,
    },
    /// Emit a `WAITING` result naming what is in the way.
    Report(WaitingOn),
    /// Stop: the lease is held.
    Granted,
    /// Stop: give up, with this error.
    Failed(CharError),
}

/// The claim loop, as a reducer.
///
/// **Never add a `_ =>` arm.** It converts a compile error into silence, which
/// is the entire reason the language decision landed where it did.
pub fn step(state: ClaimState, event: ClaimEvent) -> (ClaimState, Vec<ClaimAction>) {
    match event {
        ClaimEvent::Start => (
            ClaimState {
                phase: Phase::Attempting,
                ..state
            },
            vec![ClaimAction::Attempt],
        ),

        ClaimEvent::Granted => (
            ClaimState {
                phase: Phase::Granted,
                ..state
            },
            vec![ClaimAction::Granted],
        ),

        ClaimEvent::Held(holder) => match state.policy {
            Policy::FailFast => {
                let error = already_held(&state.lease, &holder);
                (
                    ClaimState {
                        phase: Phase::Failed(error.clone()),
                        ..state
                    },
                    vec![ClaimAction::Failed(error)],
                )
            }
            Policy::Block => {
                let waiting_on = waiting_on(&state.lease, &holder);
                (
                    ClaimState {
                        phase: Phase::Waiting(holder),
                        ..state
                    },
                    vec![
                        ClaimAction::Report(waiting_on),
                        ClaimAction::Sleep {
                            ms: POLL_INTERVAL_MS,
                        },
                    ],
                )
            }
        },

        // A cold holder is dead by the only definition available: it stopped
        // renewing. Reclaiming is what makes crash recovery fall out of the
        // design rather than needing a mechanism of its own.
        ClaimEvent::HolderCold(holder) => (
            ClaimState {
                phase: Phase::Attempting,
                ..state
            },
            vec![ClaimAction::Reclaim(holder), ClaimAction::Attempt],
        ),

        ClaimEvent::Slept { ms } => {
            let waited_ms = state.waited_ms.saturating_add(ms);
            match state.ceiling_ms {
                Some(ceiling) if waited_ms >= ceiling => {
                    let error = ceiling_expired(&state.phase, waited_ms);
                    (
                        ClaimState {
                            waited_ms,
                            phase: Phase::Failed(error.clone()),
                            ..state
                        },
                        vec![ClaimAction::Failed(error)],
                    )
                }
                Some(_) | None => (
                    ClaimState {
                        waited_ms,
                        phase: Phase::Attempting,
                        ..state
                    },
                    vec![ClaimAction::Attempt],
                ),
            }
        }

        ClaimEvent::Interrupted => {
            let error = CharError {
                class: ErrClass::Aborted,
                r#where: state.lease.key.clone(),
                message: "interrupted while waiting for a lease".to_string(),
                next_action: None,
            };
            (
                ClaimState {
                    phase: Phase::Failed(error.clone()),
                    ..state
                },
                vec![ClaimAction::Failed(error)],
            )
        }
    }
}

/// Whether a lease row is dead and may be taken.
///
/// A row from a different boot is stale **by definition** — a monotonic reading
/// is meaningless across a reboot, which is precisely what `boot_id` is for.
/// Without it every row survives a reboot as a permanent phantom that
/// `status --all` reports forever.
pub fn is_cold(heartbeat_mono_ms: u64, now_mono_ms: u64, same_boot: bool) -> bool {
    if !same_boot {
        return true;
    }
    now_mono_ms.saturating_sub(heartbeat_mono_ms) >= COLD_MS
}

/// The order leases must be acquired in, and it is a correctness property
/// rather than a preference.
///
/// > **Acquire `exclusive:` first, in sorted name order, then `cost:` slots —
/// > and never hold a slot while waiting on an exclusive.**
///
/// **Both halves matter.** Sorting orders exclusives against each other, but
/// `cost:` slots are *also* machine-wide leases, so ordering within one class
/// leaves a cross-class cycle open: A holds eight slots and waits on `browser`
/// while B holds `browser` and waits on slots. Acquiring exclusives first
/// closes it — a run waiting on an exclusive holds nothing a slot-waiter needs.
///
/// That makes a cycle impossible rather than unlikely, for every interleaving:
/// if a worker waits for X, everything it holds is below X, so following a
/// supposed cycle the awaited resource strictly increases and returning to the
/// start would need X > X. No step in that argument mentions timing, which is
/// why it is preferred over detection. **Release in reverse.**
pub fn acquisition_order(
    workspace: &WorkspaceId,
    exclusives: &[String],
    cost: u32,
) -> Vec<LeaseId> {
    let mut sorted: Vec<&String> = exclusives.iter().collect();
    sorted.sort();
    sorted.dedup();

    let mut order: Vec<LeaseId> = sorted
        .into_iter()
        .map(|name| LeaseId::exclusive(workspace.clone(), name.clone()))
        .collect();

    for slot in 0..cost {
        order.push(LeaseId {
            workspace: Some(workspace.clone()),
            kind: LeaseKind::CpuSlot,
            key: slot.to_string(),
        });
    }
    order
}

fn waiting_on(lease: &LeaseId, holder: &Holder) -> WaitingOn {
    match lease.kind {
        LeaseKind::Exclusive => WaitingOn::Exclusive {
            exclusive: lease.key.clone(),
            held_by: holder
                .workspace
                .clone()
                .unwrap_or_else(|| WorkspaceId::from_stored("machine")),
            since_ms: holder.held_ms,
        },
        LeaseKind::CpuSlot => WaitingOn::CpuSlot {
            cpu_slot: 1,
            available: 0,
        },
        LeaseKind::Run | LeaseKind::Machine => WaitingOn::Run {
            run: lease.kind.to_string(),
            pid: holder.pid,
            since_ms: holder.held_ms,
        },
    }
}

fn already_held(lease: &LeaseId, holder: &Holder) -> CharError {
    CharError {
        class: ErrClass::BadInvocation,
        r#where: lease.kind.to_string(),
        message: format!(
            "a run is already in flight: pid {}, started {}s ago",
            holder.pid,
            holder.held_ms / 1000
        ),
        next_action: Some(
            "`char check --wait` to queue, or `char check --status` to watch it".to_string(),
        ),
    }
}

fn ceiling_expired(phase: &Phase, waited_ms: u64) -> CharError {
    // Retryable, because the actionable fact is that the machine was busy
    // rather than that this check is slow.
    let held_by = match phase {
        Phase::Waiting(holder) => holder.workspace.as_ref().map(|w| format!("held by {w} ")),
        Phase::Start | Phase::Attempting | Phase::Granted | Phase::Failed(_) => None,
    }
    .unwrap_or_default();
    CharError {
        class: ErrClass::Aborted,
        r#where: "lease".to_string(),
        message: format!(
            "{held_by}for {}m — acquisition ceiling reached",
            waited_ms / 60_000
        ),
        next_action: Some("retry; `char status --all` names what is holding it".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceId {
        WorkspaceId::from_stored("a3f91c02")
    }

    fn holder() -> Holder {
        Holder {
            workspace: Some(WorkspaceId::from_stored("7c21ab90")),
            pid: 4212,
            held_ms: 44_000,
        }
    }

    /// Five agents in five worktrees never contend on the run lease; a second
    /// run in the *same* workspace is the only fail-fast case.
    #[test]
    fn the_run_lease_is_keyed_per_workspace_and_never_machine_wide() {
        let mine = LeaseId::run(WorkspaceId::from_stored("a3f91c02"));
        let theirs = LeaseId::run(WorkspaceId::from_stored("7c21ab90"));
        assert_ne!(mine.key, theirs.key);
        assert_eq!(mine.key, "a3f91c02");

        // Where the opposite is true, and deliberately so: a named exclusive
        // is one mutex for the whole machine.
        assert_eq!(
            LeaseId::exclusive(WorkspaceId::from_stored("a3f91c02"), "browser").key,
            LeaseId::exclusive(WorkspaceId::from_stored("7c21ab90"), "browser").key
        );
    }

    #[test]
    fn a_claim_starts_by_attempting() {
        let state = ClaimState::new(LeaseId::run(ws()), Policy::FailFast, None);
        let (state, actions) = step(state, ClaimEvent::Start);
        assert_eq!(actions, vec![ClaimAction::Attempt]);
        assert_eq!(state.phase, Phase::Attempting);
    }

    #[test]
    fn a_granted_claim_stops() {
        let state = ClaimState::new(LeaseId::run(ws()), Policy::FailFast, None);
        let (state, actions) = step(state, ClaimEvent::Granted);
        assert_eq!(actions, vec![ClaimAction::Granted]);
        assert_eq!(state.phase, Phase::Granted);
    }

    /// The run lease fails fast and its `next_action` names `--wait`, because
    /// waiting on it means waiting for an entire other run.
    #[test]
    fn the_run_lease_fails_fast_and_names_the_holder() {
        let state = ClaimState::new(LeaseId::run(ws()), Policy::FailFast, None);
        let (state, actions) = step(state, ClaimEvent::Held(holder()));
        match &actions[..] {
            [ClaimAction::Failed(err)] => {
                assert_eq!(err.class, ErrClass::BadInvocation);
                assert!(err.message.contains("4212"));
                assert!(err.next_action.as_deref().unwrap().contains("--wait"));
            }
            other => panic!("expected one Failed action, got {other:?}"),
        }
        assert!(matches!(state.phase, Phase::Failed(_)));
    }

    /// An exclusive blocks, and the wait is **visible**: the defect was never
    /// blocking, it was blocking invisibly and without a ceiling.
    #[test]
    fn an_exclusive_blocks_visibly_naming_the_workspace_that_holds_it() {
        let state = ClaimState::new(
            LeaseId::exclusive(ws(), "browser"),
            Policy::Block,
            Some(2_400_000),
        );
        let (state, actions) = step(state, ClaimEvent::Held(holder()));
        assert_eq!(
            actions[0],
            ClaimAction::Report(WaitingOn::Exclusive {
                exclusive: "browser".to_string(),
                held_by: WorkspaceId::from_stored("7c21ab90"),
                since_ms: 44_000,
            })
        );
        assert_eq!(
            actions[1],
            ClaimAction::Sleep {
                ms: POLL_INTERVAL_MS
            }
        );
        assert!(matches!(state.phase, Phase::Waiting(_)));
    }

    #[test]
    fn a_cold_holder_is_reclaimed_and_then_retried() {
        let state = ClaimState::new(LeaseId::exclusive(ws(), "gpu"), Policy::Block, None);
        let (state, actions) = step(state, ClaimEvent::HolderCold(holder()));
        assert_eq!(
            actions,
            vec![ClaimAction::Reclaim(holder()), ClaimAction::Attempt]
        );
        assert_eq!(state.phase, Phase::Attempting);
    }

    #[test]
    fn sleeping_accumulates_toward_the_ceiling_and_then_fails_retryably() {
        let mut state = ClaimState::new(
            LeaseId::exclusive(ws(), "browser"),
            Policy::Block,
            Some(1_000),
        );
        state.phase = Phase::Waiting(holder());
        let (state, actions) = step(state, ClaimEvent::Slept { ms: 400 });
        assert_eq!(state.waited_ms, 400);
        assert_eq!(actions, vec![ClaimAction::Attempt]);

        let (state, actions) = step(state, ClaimEvent::Slept { ms: 600 });
        assert_eq!(state.waited_ms, 1_000);
        match &actions[..] {
            [ClaimAction::Failed(err)] => assert_eq!(err.class, ErrClass::Aborted),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    /// `--wait` is the caller asking, and the fixtures contain a 1800-second
    /// check — so a ceiling there would fail a run behaving exactly as
    /// specified.
    #[test]
    fn wait_is_exempt_from_the_ceiling() {
        let mut state = ClaimState::new(LeaseId::exclusive(ws(), "gpu"), Policy::Block, None);
        for _ in 0..10_000 {
            let (next, actions) = step(state, ClaimEvent::Slept { ms: 500 });
            state = next;
            assert_eq!(actions, vec![ClaimAction::Attempt]);
        }
        assert!(!matches!(state.phase, Phase::Failed(_)));
    }

    #[test]
    fn an_interrupt_ends_the_claim_as_aborted() {
        let state = ClaimState::new(LeaseId::run(ws()), Policy::Block, None);
        let (_, actions) = step(state, ClaimEvent::Interrupted);
        match &actions[..] {
            [ClaimAction::Failed(err)] => assert_eq!(err.class, ErrClass::Aborted),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_lease_goes_cold_after_sixty_seconds_of_silence() {
        assert!(!is_cold(1_000, 1_000 + COLD_MS - 1, true));
        assert!(is_cold(1_000, 1_000 + COLD_MS, true));
    }

    #[test]
    fn a_row_from_a_previous_boot_is_stale_however_fresh_its_heartbeat_looks() {
        assert!(is_cold(u64::MAX, 0, false));
    }

    #[test]
    fn exclusives_are_acquired_before_slots_and_in_sorted_order() {
        let order = acquisition_order(
            &ws(),
            &["gpu".to_string(), "browser".to_string(), "gpu".to_string()],
            2,
        );
        let kinds: Vec<(LeaseKind, &str)> =
            order.iter().map(|l| (l.kind, l.key.as_str())).collect();
        assert_eq!(
            kinds,
            vec![
                (LeaseKind::Exclusive, "browser"),
                (LeaseKind::Exclusive, "gpu"),
                (LeaseKind::CpuSlot, "0"),
                (LeaseKind::CpuSlot, "1"),
            ]
        );
    }

    /// Two runs declaring the same two exclusives in opposite order must
    /// produce the *same* acquisition sequence — that identity is the whole
    /// proof, and it is easy to write down and easy to forget.
    #[test]
    fn opposite_declaration_orders_produce_one_acquisition_order() {
        let a = acquisition_order(&ws(), &["browser".into(), "gpu".into()], 0);
        let b = acquisition_order(&ws(), &["gpu".into(), "browser".into()], 0);
        assert_eq!(a, b);
    }

    #[test]
    fn the_lease_kinds_spell_themselves_the_way_the_column_does() {
        assert_eq!(LeaseKind::CpuSlot.to_string(), "cpu-slot");
        assert_eq!(
            serde_json::to_string(&LeaseKind::CpuSlot).unwrap(),
            "\"cpu-slot\""
        );
    }
}
