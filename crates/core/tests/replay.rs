//! **The replay property: a recorded `Event` sequence replayed through `step`
//! must equal the persisted `State`.**
//!
//! This is the strongest single assertion phase 3 can make, and it is the second
//! dividend from choosing a reducer (PLAN.md §3.4). Every other test in the
//! engine drives one transition; a lost verdict, a leaked slot or a run that
//! never finishes lives in the *composition* of hundreds of them, and the only
//! way to reach that composition is to run one and check the whole answer.
//!
//! **What would be caught here and nowhere else.** Any state the reducer keeps
//! that it did not derive from `(state, event)` — a field the shell filled in, a
//! cached decision, a counter incremented in an action rather than in a
//! transition. All of those pass the unit tests and all of them make the record
//! `char explain` reads a fiction, because a record that does not replay is a
//! record of something that did not happen.
//!
//! **The round trip is part of the property, deliberately.** The record is read
//! back off disk by a later process, so "replays in memory" is not the claim
//! being made — the claim is that what is written down is enough.

use charkit_core::dispatch::Journal;
use charkit_core::error::ErrClass;
use charkit_core::id::WorkspaceId;
use charkit_core::lease::LeaseKind;
use charkit_core::run::{RunId, RunRecord};
use charkit_core::schedule::{replay, step, Action, CheckId, EnvDelta, Event, Phase, Plan, State};
use std::path::PathBuf;

/// A deterministic generator, so a failure is reproducible from its seed.
///
/// Hand-rolled rather than a crate: `ARCHITECTURE.md` §1.5 keeps `core` to pure
/// data crates, and a dev-dependency that exists to produce eleven bits of
/// choice is a dependency to review, audit and upgrade for the rest of the
/// project.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        // xorshift64*, which is short, has no state to get wrong, and is not
        // being asked to be a random number generator — only to vary.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

fn plan(name: &str, cost: u32, exclusives: &[&str], needs: &[&str]) -> Plan {
    Plan {
        id: CheckId::new(name),
        argv: vec!["true".to_string(), name.to_string()],
        env: EnvDelta::default(),
        files: Vec::new(),
        timeout_ms: 900_000,
        cost,
        exclusives: exclusives.iter().map(|e| (*e).to_string()).collect(),
        needs: needs.iter().map(|n| CheckId::new(*n)).collect(),
        log: Some(format!("logs/{}.log", name.replace(':', "."))),
        skip: None,
    }
}

/// A shell that answers whatever the reducer proposes, with the choices it has
/// varied by the seed: grant or deny a lease, exit zero or not, run past a
/// deadline, be interrupted.
///
/// It is a *plausible* shell rather than a faithful one — it does not enforce
/// that a lease is free before granting it — and that is deliberate. The
/// property under test is that the reducer is a function of its inputs, and a
/// driver that only ever produced well-behaved sequences would exercise the
/// well-behaved half of the state space.
fn drive(seed: u64, plans: Vec<Plan>, slots: u32) -> (State, Vec<Event>) {
    let mut random = Seeded(seed.max(1));
    let mut state = State::new(PathBuf::from("/srv/repo"), slots, plans);
    let mut events: Vec<Event> = Vec::new();
    let mut now: u64 = 1_000;

    let feed = |state: State, events: &mut Vec<Event>, event: Event| {
        events.push(event.clone());
        step(state, event)
    };

    let (next, mut pending) = feed(state, &mut events, Event::Started);
    state = next;

    // A bound rather than a `while !finished`: a reducer bug that never
    // finishes must fail the test rather than hang the suite.
    for _ in 0..2_000 {
        if state.finished {
            break;
        }

        let mut answers: Vec<Event> = Vec::new();
        for action in &pending {
            match action {
                Action::Acquire { check, kind } => {
                    if random.below(4) == 0 {
                        answers.push(Event::LeaseDenied {
                            check: check.clone(),
                            kind: *kind,
                            holder: WorkspaceId::from_stored("7c21ab90"),
                        });
                    }
                    answers.push(Event::LeaseGranted {
                        check: check.clone(),
                        kind: *kind,
                    });
                }
                Action::Spawn { check, .. } => {
                    if let Some(pgid) =
                        charkit_core::schedule::Pgid::new(4_000 + random.below(50) as i32)
                    {
                        answers.push(Event::ChildSpawned {
                            check: check.clone(),
                            pgid,
                        });
                    }
                    answers.push(Event::ChildOutput {
                        check: check.clone(),
                        bytes: random.below(9_000) as usize,
                    });
                    match random.below(8) {
                        0 => answers.push(Event::Deadline {
                            check: check.clone(),
                        }),
                        1 => answers.push(Event::SpawnFailed {
                            check: check.clone(),
                            err: ErrClass::BadConfig,
                        }),
                        2 => answers.push(Event::AcquireCeiling {
                            check: check.clone(),
                        }),
                        n => answers.push(Event::ChildExited {
                            check: check.clone(),
                            code: (n % 2) as i32,
                        }),
                    }
                }
                Action::Kill { check, .. } => answers.push(Event::ChildExited {
                    check: check.clone(),
                    code: 143,
                }),
                Action::Release { .. }
                | Action::Renew
                | Action::Sleep { .. }
                | Action::Emit { .. }
                | Action::Finish { .. }
                | Action::Reap => {}
            }
        }

        now += 1 + random.below(400);
        answers.push(Event::Tick { now_mono: now });
        if random.below(120) == 0 {
            answers.push(Event::Interrupted);
        }
        if random.below(400) == 0 {
            answers.push(Event::WorkspaceGone);
        }

        pending = Vec::new();
        for event in answers {
            let (next, actions) = feed(state, &mut events, event);
            state = next;
            pending.extend(actions);
        }
    }

    (state, events)
}

/// The shapes worth generating: one check, a graph, a contended exclusive, a
/// budget that cannot fit everything, and a check that costs more than the
/// machine.
fn shapes() -> Vec<(&'static str, Vec<Plan>, u32)> {
    vec![
        ("one check", vec![plan("api:lint", 1, &[], &[])], 6),
        (
            "a graph",
            vec![
                plan("core:build", 2, &[], &[]),
                plan("ui:types", 1, &[], &["core:build"]),
                plan("ui:test", 2, &[], &["core:build"]),
                plan("api:lint", 1, &[], &[]),
            ],
            6,
        ),
        (
            "two exclusives, declared in opposite orders",
            vec![
                plan("web:e2e", 4, &["browser", "gpu"], &[]),
                plan("train:test", 4, &["gpu", "browser"], &[]),
            ],
            8,
        ),
        (
            "a budget that cannot fit everything",
            vec![
                plan("a:one", 2, &[], &[]),
                plan("b:two", 2, &[], &[]),
                plan("c:three", 2, &[], &[]),
            ],
            2,
        ),
        (
            "a check costing more than the machine",
            vec![
                plan("train:test", 8, &["gpu"], &[]),
                plan("api:lint", 1, &[], &[]),
            ],
            2,
        ),
    ]
}

/// **The property.** Every generated run, replayed from its own event sequence,
/// arrives at the state that was persisted.
#[test]
fn every_recorded_run_replays_to_the_state_that_was_persisted() {
    for (label, plans, slots) in shapes() {
        for seed in 1..=40u64 {
            let (lived, events) = drive(seed, plans.clone(), slots);
            let replayed = replay(lived.restart(), &events);
            assert_eq!(
                replayed, lived,
                "{label}, seed {seed}: the replay diverged from the run"
            );
        }
    }
}

/// The same property across the boundary that actually matters: the record is
/// read back by a **later process**, so "replays in memory" is not the claim.
#[test]
fn a_run_replays_after_the_round_trip_through_state_json() {
    for (label, plans, slots) in shapes() {
        for seed in 1..=20u64 {
            let (lived, events) = drive(seed, plans.clone(), slots);

            let mut record = RunRecord::new(
                RunId::mint(1_786_000_000_000, seed),
                WorkspaceId::from_stored("a3f91c02"),
                "2026-08-11T14:02:11Z".to_string(),
                lived.clone(),
            );
            record.journal = Journal::default();
            for event in &events {
                record.journal.observed(event);
            }

            let written = serde_json::to_string(&record).expect("a record serializes");
            let read: RunRecord = serde_json::from_str(&written).expect("a record reads back");

            assert_eq!(read.state, lived, "{label}, seed {seed}: the state changed");
            assert_eq!(
                read.journal.events, events,
                "{label}, seed {seed}: the sequence changed"
            );
            assert_eq!(
                replay(read.state.restart(), &read.journal.events),
                lived,
                "{label}, seed {seed}: the record did not replay"
            );
        }
    }
}

/// **The inversion, without which the property above is decoration.** If a
/// perturbed sequence replayed to the same state, then the state was not a
/// function of the sequence and the property proved nothing about either.
///
/// **Three perturbations, each provably load-bearing rather than merely likely
/// to be.** An earlier version of this test dropped every event in turn and
/// asserted each drop mattered, which is false for two honest reasons and
/// finding them was worth more than the test was:
///
/// - `Event::Started` changes no state of its own. It exists to trigger the
///   first scheduling pass, and the next event triggers one too — so dropping it
///   from a sequence that has any other event is genuinely inert.
/// - `Event::ChildOutput` **used to be** inert, because the byte count lived on
///   `Running` and was discarded when the check finished. That was a real
///   defect: char recorded an event no persisted state reflected, so a record
///   could disagree with its run in the one dimension nothing checked. The count
///   now survives into `Outcome`, which is why the third case below can assert
///   what it does.
#[test]
fn a_perturbed_sequence_replays_to_a_different_state() {
    let (lived, events) = drive(7, shapes()[1].1.clone(), 6);
    assert!(
        events.len() > 12,
        "the generated run is too short to perturb"
    );

    // 1. Truncation. A run that was cut short did not reach the state that was
    //    persisted, whatever the last few events were.
    for back in 1..=8 {
        let shorter = &events[..events.len() - back];
        assert_ne!(
            replay(lived.restart(), shorter),
            lived,
            "dropping the last {back} event(s) changed nothing"
        );
    }

    // 2. A verdict. Flipping one exit code has to change the run's answer, or
    //    the persisted state is not recording what the checks did.
    let exit = events
        .iter()
        .position(|event| matches!(event, Event::ChildExited { code: 0, .. }))
        .expect("a check exited zero in this run");
    let mut flipped = events.clone();
    if let Event::ChildExited { check, .. } = &events[exit] {
        flipped[exit] = Event::ChildExited {
            check: check.clone(),
            code: 1,
        };
    }
    assert_ne!(
        replay(lived.restart(), &flipped),
        lived,
        "a check that failed instead of passing produced the same state"
    );

    // 3. Output. The count reaches the persisted outcome, so the event is not
    //    written down for nothing.
    let output = events
        .iter()
        .position(|event| matches!(event, Event::ChildOutput { bytes, .. } if *bytes > 0))
        .expect("a check produced output in this run");
    let mut quieter = events.clone();
    if let Event::ChildOutput { check, bytes } = &events[output] {
        quieter[output] = Event::ChildOutput {
            check: check.clone(),
            bytes: bytes / 2,
        };
    }
    assert_ne!(
        replay(lived.restart(), &quieter),
        lived,
        "a check's output volume left no trace in the persisted state"
    );

    // 4. The clock. Durations come off the tick readings, so moving one has to
    //    move the answer. The *last* tick, and forward, so the perturbed
    //    sequence is still one a real shell could have produced: a monotonic
    //    clock does not go backwards, and a test that asserts on a sequence
    //    nothing can generate is asserting about nothing.
    let tick = events
        .iter()
        .rposition(|event| matches!(event, Event::Tick { .. }))
        .expect("the run ticked");
    let last = match &events[tick] {
        Event::Tick { now_mono } => *now_mono,
        other => panic!("expected a tick, got {other:?}"),
    };
    let mut later = events.clone();
    later[tick] = Event::Tick {
        now_mono: last + 5_000,
    };
    assert_ne!(
        replay(lived.restart(), &later),
        lived,
        "the clock left no trace in the persisted state"
    );
}

/// A replay is a pure fold, so replaying twice is the same as replaying once.
/// If it were not, the reducer would be reading something that is not its
/// arguments.
#[test]
fn replaying_the_same_sequence_twice_gives_the_same_answer() {
    for seed in [3u64, 11, 29] {
        let (lived, events) = drive(seed, shapes()[2].1.clone(), 8);
        let once = replay(lived.restart(), &events);
        let twice = replay(lived.restart(), &events);
        assert_eq!(once, twice);
        assert_eq!(once, lived);
    }
}

/// The starting state is a projection of the ending one, and the projection has
/// to be right or the replay begins somewhere the run never was.
#[test]
fn restarting_a_finished_run_returns_it_to_the_state_it_began_in() {
    let plans = shapes()[1].1.clone();
    let fresh = State::new(PathBuf::from("/srv/repo"), 6, plans.clone());
    let (lived, _) = drive(5, plans, 6);

    assert_eq!(lived.restart(), fresh);
    assert!(lived
        .restart()
        .checks
        .values()
        .all(|check| check.phase == Phase::Pending && check.held.is_empty()));
    assert_eq!(lived.restart().budget.in_use, 0);
}

/// The generator has to reach the interesting half of the state space, or the
/// property above is a statement about happy paths. Asserted rather than
/// assumed, because a driver that quietly stopped producing denials would
/// weaken every test in this file without failing any of them.
#[test]
fn the_generated_runs_actually_reach_the_hard_states() {
    let mut denials = 0;
    let mut deadlines = 0;
    let mut ceilings = 0;
    let mut interrupts = 0;
    let mut exclusives = 0;

    for (_, plans, slots) in shapes() {
        for seed in 1..=40u64 {
            let (_, events) = drive(seed, plans.clone(), slots);
            for event in &events {
                match event {
                    Event::LeaseDenied { kind, .. } => {
                        denials += 1;
                        if *kind == LeaseKind::Exclusive {
                            exclusives += 1;
                        }
                    }
                    Event::Deadline { .. } => deadlines += 1,
                    Event::AcquireCeiling { .. } => ceilings += 1,
                    Event::Interrupted => interrupts += 1,
                    _ => {}
                }
            }
        }
    }

    assert!(denials > 50, "only {denials} lease denials were generated");
    assert!(
        exclusives > 5,
        "only {exclusives} exclusives were contended"
    );
    assert!(deadlines > 5, "only {deadlines} deadlines were generated");
    assert!(ceilings > 5, "only {ceilings} ceilings were generated");
    assert!(interrupts > 0, "no run was ever interrupted");
}
