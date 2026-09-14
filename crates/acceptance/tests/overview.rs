//! Overview's claim: **everything in flight and everything waiting on me,
//! across my repositories, on one surface — and I can answer it, or ask Helm
//! about it, without leaving.**
//!
//! **The screen half is not a Rust question.** What is reachable is the seam
//! the surface is drawn from: that a received Job names its Manifest, so a
//! client can sort a mixed list without a second query, that the tiles'
//! readings survive [`ipc::encode`] and back, and that each kind of open
//! question is carried on the `JobDetail` it belongs to and named by the id an
//! answer would send back. The apparatus is [`bench::overview`]. **A green run
//! here is not the milestone** — the two tables below name what still is not
//! proved.

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the four below.
#[allow(dead_code)]
mod bench;

use core_model::AdmissionHold;
use ipc::{
    AnswerCommand, AskedOption, ChosenAnswer, CommandAnswer, CommandInFlight, Declaration, Drift,
    FleetCapacity, FleetHealth, JobList, JobSummary, JudgeAnswer, JudgeAnswered, JudgeQuestion,
    ManifestDrift, Probe, QuestionId, QuestionInFlight, Unfollowed, Unprobed,
};

use bench::overview::{
    created_under, detail_of, done_under, queued_under, round_trip_answer_command,
    round_trip_capacity, round_trip_chosen_answer, round_trip_detail, round_trip_drift,
    round_trip_health, round_trip_jobs, round_trip_judge_answered, running_under, section_of,
    NEEDS_YOU, QUEUED, RUNNING,
};

// | Not proved here | Why not, and what would prove it |
// |---|---|
// | That answering names a stale id and is refused — `NotAnswered::Superseded`, `NotPermitted::NothingToAnswer`, `Adrift::NotAnswerable` | Each is read off a live slot and `self.store()` inside `fleet::questioning`, `fleet::permitting::holding` and `fleet::asking` — a real `Fleet` over a real store. `crates/acceptance/Cargo.toml`'s own header says why this crate holds neither: "the loop cannot admit a Job without spawning a Drone, so the loop is proven in `fleet`'s own tests and not here". `fleet`'s own tests prove the three: `crates/fleet/src/tests/questioning.rs::an_answer_to_a_question_that_is_not_the_one_outstanding_is_refused`, `crates/fleet/src/tests/permitting.rs::an_answer_naming_another_call_is_refused_and_the_question_stands`, `crates/fleet/src/tests/asking.rs::answering_a_stale_question_is_refused` |
// | A Helm session taking two turns, the second remembering the first, and a resumed one remembering both | `crates/fleet/src/tests/helm_conversation.rs` proves exactly this, against a stand-in agent run as a real child process (`ProcessHost` over `HeadlessAgent`) — but it lives behind `#[cfg(test)]` in `fleet`, unreachable from here, and this crate's own manifest forbids spawning one regardless. Reaching it would mean changing `fleet` to expose test-only apparatus, which is outside this file's scope — reported rather than done. `fleet`'s own tests: `two_messages_in_one_conversation_and_the_second_remembers_the_first`, `the_conversation_resumes_after_fleet_restarts` |
// | That a person reading the surface learns anything, and Bridge's own pick | Nothing here renders — `needs-you.ts` and `board.ts` own the rule; `ofPicked` is not a Rust question |

// ---------------------------------------------------------------------------
// Every Job carries the Manifest it belongs to
// ---------------------------------------------------------------------------

/// A new status, or a misspelling in one of the three lists `bench::overview`
/// hand keeps, fails this rather than passing silently: every non-terminal
/// status in `core_model::JobStatus::ALL` lands in exactly one of the three,
/// every terminal one lands in none, and every name the lists carry is a real
/// status's wire spelling.
#[test]
fn every_status_the_registry_names_is_classified_or_excluded_as_terminal() {
    for status in core_model::JobStatus::ALL {
        let wire = status.as_wire();
        let placed = [NEEDS_YOU, RUNNING, QUEUED]
            .iter()
            .filter(|list| list.contains(&wire))
            .count();
        if status.is_terminal() {
            assert_eq!(
                placed, 0,
                "{wire} is terminal and belongs in none of the three"
            );
        } else {
            assert_eq!(placed, 1, "{wire} must land in exactly one of the three");
        }
    }
    for name in NEEDS_YOU.iter().chain(RUNNING).chain(QUEUED) {
        assert!(
            core_model::JobStatus::from_wire(name).is_some(),
            "{name} is not a status this build's registry carries"
        );
    }
}

/// Two Manifests, one list, and each Manifest's own sections read back out of
/// it — without a second query and without one Manifest's Jobs leaking into
/// the other's count.
///
/// **The failure this is against is a client that reads the whole list once
/// it has one Manifest's Jobs in hand.** A Board narrowed by `ofPicked`
/// filters on `owner_manifest_id`; a client that instead trusted "the list I
/// have is the Manifest I asked for" would show a second repository's Jobs
/// under the first the moment Fleet served two.
#[test]
fn every_jobs_manifest_survives_the_wire_and_sorts_two_repositories_apart() {
    let storefront = "01MANIFESTOVERVIEWSTOREFRONT";
    let mailer = "01MANIFESTOVERVIEWMAILERXXXX";

    let gate = created_under(storefront, "01JOBOVERVIEWGATE0000000A", "wait for a person");
    let running = running_under(storefront, "01JOBOVERVIEWRUNNING000A", "run the migration");
    let asking = running_under(
        storefront,
        "01JOBOVERVIEWASKING0000A",
        "ask before it writes",
    );
    let queued = queued_under(storefront, "01JOBOVERVIEWQUEUED0000A", "wait its turn");
    let done = done_under(storefront, "01JOBOVERVIEWDONE00000A", "already over");

    let mailer_gate = created_under(
        mailer,
        "01JOBOVERVIEWGATE0000000B",
        "a mailer Job waits too",
    );
    let mailer_queued = queued_under(mailer, "01JOBOVERVIEWQUEUED0000B", "and one is queued");

    let list = JobList {
        jobs: vec![
            JobSummary::of(&gate, None, None, None, false, None, None),
            JobSummary::of(&running, None, None, None, false, None, None),
            // The one row `asking` overrides: a running Job whose Drone is
            // waiting on an answer reads as Needs you, not Running.
            JobSummary::of(&asking, None, None, None, true, None, None),
            JobSummary::of(&queued, None, None, None, false, None, None),
            JobSummary::of(&done, None, None, None, false, None, None),
            JobSummary::of(&mailer_gate, None, None, None, false, None, None),
            JobSummary::of(&mailer_queued, None, None, None, false, None, None),
        ],
        unreadable: Vec::new(),
    };
    let received: JobList = round_trip_jobs(&list);
    assert_eq!(
        received.jobs.len(),
        7,
        "the whole of what the machine moved"
    );

    let of = |manifest: &str, section: &str| -> Vec<&str> {
        received
            .jobs
            .iter()
            .filter(|row| row.owner_manifest_id.as_str() == manifest)
            .filter(|row| section_of(row) == Some(section))
            .map(|row| row.title.as_str())
            .collect()
    };

    assert_eq!(
        of(storefront, "needs-you"),
        ["wait for a person", "ask before it writes"]
    );
    assert_eq!(of(storefront, "running"), ["run the migration"]);
    assert_eq!(of(storefront, "queued"), ["wait its turn"]);
    assert_eq!(
        of(mailer, "needs-you"),
        ["a mailer Job waits too"],
        "the second Manifest's own Needs you row, not the first's"
    );
    assert_eq!(of(mailer, "queued"), ["and one is queued"]);
    assert!(
        of(mailer, "running").is_empty(),
        "nothing here dispatched a Drone on the mailer"
    );

    // Done, and the fourth section: every non-terminal status in
    // `job-statuses.toml` is `who_is_acting = "Person"` or `"Drone"`, so
    // Other is empty by construction rather than by omission here.
    let placed: Vec<&JobSummary> = received
        .jobs
        .iter()
        .filter(|row| row.title != "already over")
        .collect();
    assert!(placed.iter().all(|row| section_of(row).is_some()));
    let over = received
        .jobs
        .iter()
        .find(|row| row.title == "already over")
        .expect("the finished Job rode with the rest");
    assert!(
        section_of(over).is_none(),
        "a Job that is over is in none of the three sections a person works down"
    );
}

// ---------------------------------------------------------------------------
// The readings the tiles draw
// ---------------------------------------------------------------------------

// | Not proved here | Why not, and what would prove it |
// |---|---|
// | Health, capacity and drift saying anything true about a real machine | Hermetic — every value below is built and round-tripped by hand, not answered by a live Fleet |
// | `?manifest_id=` on `get_manifest_drift` reaching the repository it names | `InManifest` in `crates/api/src/scoped.rs` is `pub(crate)` — `api`'s own router tests own that seam |
// | A model's reply, to anything | This file calls no model and opens no session |

/// Capacity, as the status bar and the tile both read it: the bound, what is
/// occupying it, and — only when something is — what is holding the next
/// Drone back.
///
/// **Fleet-wide**, unlike the drift tile below: `docs/concepts/fleet.md`
/// names capacity as a reading a scoped surface still shows unscoped, because
/// the bound is the machine's and not a repository's.
#[test]
fn capacity_survives_the_wire_and_says_nothing_where_nothing_holds_it_back() {
    let open = FleetCapacity::of(3, 1, None);
    let held = FleetCapacity::of(3, 3, Some(AdmissionHold::Disk));

    let (open, open_body) = round_trip_capacity(&open);
    let (held, _) = round_trip_capacity(&held);

    assert_eq!((open.bound, open.occupied), (3, 1));
    assert!(open.held_by.is_none());
    assert!(
        !open_body.contains("held_by"),
        "nothing holding admission back is absent, not null: {open_body}"
    );
    assert_eq!(
        held.held_by.as_ref().map(ipc::AdmissionHold::as_wire),
        Some("disk")
    );
}

/// Health, as far as Fleet can answer for itself: every probe it ran, and
/// everything it could not — named, never silently missing.
///
/// **Not Doctor.** `crates/ipc/src/health.rs` says so in its own header: this
/// is the rows Fleet itself holds, and the rest is `not_probed`.
#[test]
fn health_survives_the_wire_with_the_unprobed_half_still_on_it() {
    let health = FleetHealth {
        probes: vec![Probe {
            module: "Fleet".to_string(),
            outcome: "pass".to_string(),
            detail: "one Drone, one working slot".to_string(),
        }],
        not_probed: vec![Unprobed {
            owner: "adapters".to_string(),
            because: "Doctor's grid is not built".to_string(),
        }],
    };
    let received: FleetHealth = round_trip_health(&health);
    assert_eq!(received.probes.len(), 1);
    assert_eq!(received.probes[0].outcome, "pass");
    assert_eq!(
        received.not_probed[0].because, "Doctor's grid is not built",
        "a report with rows and nothing beside them reads as a healthy \
         machine, which is the failure `not_probed` exists to name"
    );
}

/// Drift, for one repository — never for the machine. Two Manifests read two
/// answers, because a `run` line's target is a fact about a checkout and not
/// about Fleet.
#[test]
fn drift_survives_the_wire_and_is_read_per_repository() {
    let storefront = ManifestDrift {
        path: "storefront/armada.yml".to_string(),
        checkout: "/repos/storefront".to_string(),
        declarations: vec![Declaration {
            section: "checks".to_string(),
            name: "e2e".to_string(),
            key: "run".to_string(),
            run: "pnpm playwright test".to_string(),
            drift: Drift::Current { checked: 1 },
            unfollowed: Vec::new(),
        }],
    };
    let mailer = ManifestDrift {
        path: "mailer/armada.yml".to_string(),
        checkout: "/repos/mailer".to_string(),
        declarations: vec![Declaration {
            section: "checks".to_string(),
            name: "lint".to_string(),
            key: "run".to_string(),
            run: "golangci-lint run".to_string(),
            drift: Drift::Gone {
                missing: vec![".golangci.yml".to_string()],
            },
            unfollowed: vec![Unfollowed {
                word: "golangci-lint".to_string(),
                why: "not a tool this read follows".to_string(),
            }],
        }],
    };

    let storefront: ManifestDrift = round_trip_drift(&storefront);
    let mailer: ManifestDrift = round_trip_drift(&mailer);

    assert!(matches!(
        storefront.declarations[0].drift,
        Drift::Current { checked: 1 }
    ));
    let Drift::Gone { missing } = &mailer.declarations[0].drift else {
        panic!("the mailer's line is missing what it names");
    };
    assert_eq!(missing, &vec![".golangci.yml".to_string()]);
    assert_eq!(mailer.declarations[0].unfollowed[0].word, "golangci-lint");
    assert_ne!(
        storefront.checkout, mailer.checkout,
        "one repository's drift, not the machine's"
    );
}

// ---------------------------------------------------------------------------
// Each kind of question, served on the Job it belongs to
// ---------------------------------------------------------------------------

// | Not proved here | Why not, and what would prove it |
// |---|---|
// | That a person reading a card learns anything, or that pressing an answer reaches Fleet | Nothing here renders and nothing here opens a connection — see the file header |
// | That the three are mutually exclusive on one Job | Nothing in `core_model` or `ipc` says so; `JobDetail` carries all three as independent optional fields, and each test below sets exactly one |

/// A Drone's own question, a command it reached for and was not given, and a
/// Judge criterion that refused — each carried on the `JobDetail` of the Job
/// it belongs to, under whichever Manifest that Job is, and surviving
/// `ipc::encode` and back with the id an answer would name.
///
/// **`command_waiting` and `judge_question` are filled in after
/// [`ipc::JobDetail::of`], never through it.** `crates/ipc/src/detail.rs`
/// says so on both fields: `of` hardcodes them absent, because it is called
/// with only what a Job's record and its frozen workflow say, and both of
/// these live on a slot or a store row instead. This is that filling — the
/// same shape `api`'s own daemon uses — not a second constructor.
#[test]
fn every_kind_of_question_is_served_on_the_job_it_belongs_to_across_two_manifests() {
    let storefront = "01MANIFESTOVERVIEWSTOREFRONT";
    let mailer = "01MANIFESTOVERVIEWMAILERXXXX";

    // A Drone's own question, on a storefront Job — `asking` is a constructor
    // argument, per `JobDetail::of`'s own comment: "the question lives on the
    // working slot for as long as it is unanswered".
    let job = running_under(storefront, "01JOBOVERVIEWASKEDQ0000A", "waiting on a drone");
    let question = QuestionInFlight {
        question_id: QuestionId::carried("01QUESTIONOVERVIEWDRONEA"),
        step_id: ipc::StepId::carried("fix"),
        asked_at: ipc::Instant::carried("2026-09-13T09:05:00.000Z"),
        question: "which cursor should the fix touch?".to_string(),
        options: vec![
            AskedOption {
                label: "the read cursor".to_string(),
                consequence: "fixes the reader only".to_string(),
            },
            AskedOption {
                label: "both".to_string(),
                consequence: "fixes the reader and the writer".to_string(),
            },
        ],
    };
    let detail = detail_of(&job, Some(question.clone()));
    let received = round_trip_detail(&detail);
    assert_eq!(received.job.owner_manifest_id.as_str(), storefront);
    let served = received
        .asking
        .expect("the drone's own question is served on its Job");
    assert_eq!(served.question_id, question.question_id);
    assert_eq!(served.question, question.question);

    // A command a Drone reached for and was not given, on a mailer Job.
    let job = running_under(mailer, "01JOBOVERVIEWHELDCMD000B", "waiting on a command");
    let mut detail = detail_of(&job, None);
    detail.command_waiting = Some(CommandInFlight {
        call: "call-01QUESTIONOVERVIEWCMDB".to_string(),
        step_id: ipc::StepId::carried("fix"),
        asked_at: ipc::Instant::carried("2026-09-13T09:06:00.000Z"),
        tool: "Bash".to_string(),
        detail: "rm -rf node_modules".to_string(),
        truncated: false,
        length: None,
        offers: vec![CommandAnswer::AllowForJob, CommandAnswer::Reject],
        rules: Vec::new(),
        suggested_rule: None,
    });
    let received = round_trip_detail(&detail);
    assert_eq!(received.job.owner_manifest_id.as_str(), mailer);
    let waiting = received
        .command_waiting
        .expect("the held command is served on its Job");
    assert_eq!(waiting.call, "call-01QUESTIONOVERVIEWCMDB");

    // A Judge criterion that refused, on a second storefront Job.
    let job = running_under(
        storefront,
        "01JOBOVERVIEWJUDGEQ0000C",
        "waiting on a judge answer",
    );
    let mut detail = detail_of(&job, None);
    detail.judge_question = Some(JudgeQuestion {
        step_id: ipc::StepId::carried("fix"),
        criterion_id: ipc::CriterionId::carried("c1"),
        question: "does the fix address the cause the note named?".to_string(),
        expected: "a change to read.rs's bound".to_string(),
        produced: "a change to an unrelated bound".to_string(),
        consequence: "the reported symptom still occurs".to_string(),
        asked_at: ipc::Instant::carried("2026-09-13T09:07:00.000Z"),
        brief_path: None,
    });
    let received = round_trip_detail(&detail);
    assert_eq!(received.job.owner_manifest_id.as_str(), storefront);
    let asked = received
        .judge_question
        .expect("the judge question is served on its Job");
    assert_eq!(asked.criterion_id.as_str(), "c1");
}

/// Each answer names the id it was served with — `ChosenAnswer::question_id`,
/// `AnswerCommand::call`, `JudgeAnswered::asked_at` — and every one of the
/// three routes is a route Fleet serves. **Not that a stale one is refused**:
/// see the file header.
#[test]
fn each_answer_names_the_id_it_was_served_with_and_the_route_is_served() {
    for operation in ["answer_question", "answer_command", "answer_judge"] {
        assert!(
            api::SERVED.iter().any(|route| route.operation == operation),
            "`{operation}` is offered on a Job's detail and nothing serves it"
        );
    }

    let chosen = ChosenAnswer {
        question_id: QuestionId::carried("01QUESTIONOVERVIEWDRONEA"),
        chose: "the read cursor".to_string(),
    };
    let received = round_trip_chosen_answer(&chosen);
    assert_eq!(received.question_id, chosen.question_id);

    let answered_command = AnswerCommand {
        call: "call-01QUESTIONOVERVIEWCMDB".to_string(),
        answer: CommandAnswer::Reject,
        note: Some("not on this job".to_string()),
        rule: None,
    };
    let received = round_trip_answer_command(&answered_command);
    assert_eq!(received.call, answered_command.call);

    let answered_judge = JudgeAnswered {
        answer: JudgeAnswer::Agree,
        asked_at: Some(ipc::Instant::carried("2026-09-13T09:07:00.000Z")),
        note: None,
    };
    let received = round_trip_judge_answered(&answered_judge);
    assert_eq!(
        received.asked_at, answered_judge.asked_at,
        "the id the answer names back, for `answer_judge`'s own Superseded check"
    );
}

// `detail_of` and every `round_trip_*` helper the tests above call are
// `bench::overview`'s — see the module header on why the bench and the claim
// are kept apart.
