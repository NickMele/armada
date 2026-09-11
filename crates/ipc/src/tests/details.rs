//! What a Job's detail says that its Board row cannot.
//!
//! The step rows, what each gate did, what the Judge cited and the note
//! waiting for the next Drone reach a client only through
//! [`JobDetail`](crate::JobDetail), so this is where they are held true.
//!
//! Most cases here turn on one distinction: **empty is an answer and absent is
//! a different answer.** An ungated step declares no Checks; a step this Fleet
//! holds no workflow for cannot say. A client that read a gap either way could
//! not tell those apart, which is the whole reason the optional fields are
//! skipped rather than written `null`.

use crate::tests::{detail_of, job};
use crate::{
    decode, encode, CheckRun, DeclaredCheck, JobDetail, Judged, Refusal, StepFacts, Stuck,
};

/// **The distinction the whole field exists for.** An ungated step says so with
/// an empty list; a step Fleet cannot answer for carries no key at all. A
/// client that saw a gap either way could not tell them apart.
#[test]
fn an_ungated_step_says_so_and_an_unanswerable_one_carries_no_key() {
    let job = job();
    let ungated = detail_of(
        &job,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("Reproduce it".to_string()),
            declares: Some(Vec::new()),
            ran: Vec::new(),
            judged: Vec::new(),
            flagged: Vec::new(),
            deliverables: Vec::new(),
            frames: Vec::new(),
            attempts: Vec::new(),
            verdicts: Vec::new(),
            judging: None,
        }],
    );
    let json = encode(&ungated).expect("a detail is plain data");
    assert!(json.contains("\"checks\":[]"), "declares none: {json}");

    let unanswerable = detail_of(
        &job,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: None,
            declares: None,
            ran: Vec::new(),
            judged: Vec::new(),
            flagged: Vec::new(),
            deliverables: Vec::new(),
            frames: Vec::new(),
            attempts: Vec::new(),
            verdicts: Vec::new(),
            judging: None,
        }],
    );
    let json = encode(&unanswerable).expect("a detail is plain data");
    assert!(
        !json.contains("\"checks\""),
        "absent, never present-and-null: {json}"
    );
    assert!(
        json.contains("\"check_runs\":[]"),
        "what ran is always a list — nothing ran: {json}"
    );
}

/// **The fallback is deliberate, and a blank is not a state.** A step Fleet
/// cannot name reads as its id, so nothing downstream has to decide what an
/// empty label draws as.
#[test]
fn a_step_with_no_label_reads_as_its_id() {
    let detail = detail_of(
        &job(),
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("   ".to_string()),
            declares: None,
            ran: Vec::new(),
            judged: Vec::new(),
            flagged: Vec::new(),
            deliverables: Vec::new(),
            frames: Vec::new(),
            attempts: Vec::new(),
            verdicts: Vec::new(),
            judging: None,
        }],
    );
    assert_eq!(detail.steps[0].label, "repro");

    let unanswerable = detail_of(&job(), &[]);
    assert_eq!(unanswerable.steps[0].label, "repro");
}

/// A recorded run round-trips, and a pass carries neither sentence.
#[test]
fn a_check_run_crosses_with_which_of_the_five_outcomes_it_was() {
    let detail = detail_of(
        &job(),
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("Reproduce it".to_string()),
            declares: Some(vec![DeclaredCheck {
                kind: "manifest_check".to_string(),
                name: Some("suite".to_string()),
                run: Some("cargo nextest run --workspace".to_string()),
                expect_exit_code: Some(0),
                when: Some(vec!["crates/**".to_string()]),
            }]),
            ran: vec![CheckRun {
                attempt: 1,
                name: "suite".to_string(),
                outcome: core_model::CheckOutcome::NeverRan.into(),
                expected: Some("`suite` can be run".to_string()),
                produced: Some("`suite` is not installed".to_string()),
                output_path: Some(".armada/checks/01JOB/repro.0.log".to_string()),
            }],
            judged: Vec::new(),
            flagged: Vec::new(),
            deliverables: Vec::new(),
            frames: Vec::new(),
            attempts: Vec::new(),
            verdicts: Vec::new(),
            judging: None,
        }],
    );
    let json = encode(&detail).expect("a detail is plain data");

    assert!(json.contains("\"outcome\":\"never_ran\""), "{json}");
    assert!(
        json.contains("\"run\":\"cargo nextest run --workspace\""),
        "the command the workflow froze crosses whole: {json}"
    );
    assert!(
        json.contains("\"when\":[\"crates/**\"]"),
        "and so do the paths it covers, which are only useful before it runs: {json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        detail
    );
}

/// **A refusal's citation crosses, and a no-objection carries none.**
///
/// This is what makes escalating a refusal worth more than ending the Job: the
/// escalation trigger says the gate stopped, and only these three lines say
/// what was wrong with the work. A person reading the Job is the audience.
#[test]
fn a_judge_refusal_crosses_with_the_three_lines_it_cited() {
    let detail = detail_of(
        &job(),
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("Reproduce it".to_string()),
            declares: Some(Vec::new()),
            ran: Vec::new(),
            judged: vec![
                Judged {
                    attempt: 1,
                    criterion_id: crate::CriterionId::carried("c1"),
                    member: None,
                    verdict: core_model::JudgeVerdict::NotMet.into(),
                    expected: Some("the caller's bound narrowed".to_string()),
                    produced: Some("the reader's bound widened".to_string()),
                    consequence: Some("every other caller reads one row too many".to_string()),
                    brief_path: Some(".armada/briefs/01JOB/repro.1.c1.txt".to_string()),
                    cited: Some(vec![crate::Citation {
                        region: "check:test_suite".to_string(),
                        from_line: 2007,
                        to_line: 2008,
                    }]),
                    given: Some(crate::Given {
                        digest: "3f7a10c2b40de991".to_string(),
                        size: 41_204,
                        model: "haiku".to_string(),
                    }),
                },
                Judged {
                    attempt: 1,
                    criterion_id: crate::CriterionId::carried("c2"),
                    member: None,
                    verdict: core_model::JudgeVerdict::Met.into(),
                    expected: None,
                    produced: None,
                    consequence: None,
                    brief_path: None,
                    cited: None,
                    given: None,
                },
            ],
            flagged: Vec::new(),
            deliverables: Vec::new(),
            frames: Vec::new(),
            attempts: Vec::new(),
            verdicts: Vec::new(),
            judging: None,
        }],
    );
    let json = encode(&detail).expect("a detail is plain data");

    assert!(json.contains("\"verdict\":\"not_met\""), "{json}");
    assert!(
        json.contains("every other caller reads one row too many"),
        "the line a person triages on crosses: {json}"
    );
    assert!(
        !json.contains("\"expected\":null"),
        "a no-objection cites nothing, and absent is not null: {json}"
    );
    assert!(
        json.contains("\"region\":\"check:test_suite\"") && json.contains("\"from_line\":2007"),
        "where in the brief the refusal quoted from crosses: {json}"
    );
    assert!(
        json.contains("\"digest\":\"3f7a10c2b40de991\""),
        "and what that member was handed, which is what a client compares \
         across a panel: {json}"
    );
    assert!(
        !json.contains("\"cited\":null") && !json.contains("\"given\":null"),
        "a row nobody recorded either for sends no key at all, which is what \
         the `?:` on the TypeScript side reads — where `[]` would have said \
         the member quoted nothing: {json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        detail
    );
}

/// A step nothing asked the Judge about says so with an empty list, the way an
/// ungated step says so about its Checks.
#[test]
fn a_step_the_judge_was_never_asked_about_carries_an_empty_list() {
    let detail = detail_of(&job(), &[]);
    let json = encode(&detail).expect("a detail is plain data");
    assert!(json.contains("\"judged\":[]"), "{json}");
}

/// **A note with nowhere to go crosses, and stops crossing when it goes.**
///
/// The whole of `#212`: between a person asking for changes at a gate and the
/// next Drone opening with their words, the Job is `queued` and the wire said
/// nothing that a Job nobody typed anything into does not also say. The field is
/// read off the record rather than handed in, so the two readings below are the
/// record's own two states and cannot drift from them.
#[test]
fn a_note_waiting_for_the_next_drone_crosses_until_it_is_delivered() {
    let waiting = job()
        .redirect_waits(
            core_model::RedirectWaiting::saying("name the cause, not the symptom")
                .expect("a note with something in it"),
        )
        .expect("nothing was waiting");

    let held = detail_of(&waiting, &[]);
    let json = encode(&held).expect("a detail is plain data");
    assert_eq!(
        held.redirect_waiting
            .as_ref()
            .map(|note| note.note.as_str()),
        Some("name the cause, not the symptom"),
        "the person's own words, quoted rather than counted"
    );
    assert!(
        json.contains("name the cause, not the symptom"),
        "and they reach the wire: {json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        held
    );

    // Delivery is what clears it, so the field cannot be a badge that goes
    // stale: there is no third state for a surface to keep drawing.
    let delivered = detail_of(&waiting.redirect_delivered(), &[]);
    assert!(
        delivered.redirect_waiting.is_none(),
        "a delivered note stops saying it is waiting"
    );
    assert!(
        !encode(&delivered)
            .expect("a detail is plain data")
            .contains("redirect_waiting"),
        "and absent is absent, never present-and-null"
    );
}

/// **A Check that declares no `when` sends no key at all.**
///
/// Absent and empty would be one value with opposite meanings — always, and
/// never — which is the collision `core_model::Covers` refuses at the other end
/// of the wire. A client that received `[]` would have to know which one this
/// build meant.
#[test]
fn a_check_covering_everything_carries_no_when() {
    let declared = DeclaredCheck {
        kind: "manifest_check".to_string(),
        name: Some("build".to_string()),
        run: Some("cargo build --workspace --locked".to_string()),
        expect_exit_code: Some(0),
        when: None,
    };
    let json = encode(&declared).expect("a declaration is plain data");
    assert!(!json.contains("when"), "{json}");
    assert_eq!(
        decode::<DeclaredCheck>("a declared check", json.as_bytes()).expect("it round-trips"),
        declared
    );
}

/// The classification of the real Job this was written against: a policy
/// stopped one `Bash` call, and the harness said nothing about why.
fn blocked() -> Stuck {
    Stuck {
        stopped_by: Some(String::from("blocked_by_policy")),
        step_id: None,
        recourse: Vec::new(),
        worktree_on_disk: true,
        drone_unheard: false,
        refused: vec![Refusal {
            tool: String::from("Bash"),
            call: String::from("toolu_01B13LL"),
            detail: String::from("cargo nextest run --package ipc"),
            truncated: false,
            length: Some(30),
            because: String::new(),
        }],
        refusals: 1,
        undecided: None,
    }
}

/// **The command is what crosses, because the reason does not exist.** Every
/// `permission_denied` line observed carried an empty `decision_reason`, so a
/// client drawing `because` alone would draw the same blank a person was
/// already looking at.
#[test]
fn a_refusal_crosses_with_its_command_and_keeps_its_empty_reason() {
    let json = encode(&blocked()).expect("a classification is plain data");

    assert!(
        json.contains("\"detail\":\"cargo nextest run --package ipc\""),
        "the command, joined off the `called` row: {json}"
    );
    assert!(
        json.contains("\"because\":\"\""),
        "an empty reason crosses as one rather than being dropped, so a client \
         can tell it apart from a key it did not read: {json}"
    );
    assert!(
        json.contains("\"call\":\"toolu_01B13LL\""),
        "and the id `get_call` serves the whole argument by: {json}"
    );
    assert_eq!(
        decode::<Stuck>("a classification", json.as_bytes()).expect("it reads back"),
        blocked()
    );
}

/// **A capped list says how many there were.** A client shown fifty of a
/// hundred and thirty-seven and told nothing would render the fifty as the
/// whole list, which is the defect one level down from the one this closes.
#[test]
fn the_count_crosses_beside_the_list_and_a_job_with_none_says_so() {
    let many = Stuck {
        refusals: 137,
        ..blocked()
    };
    let json = encode(&many).expect("a classification is plain data");
    assert!(json.contains("\"refusals\":137"), "{json}");

    let quiet = Stuck {
        refused: Vec::new(),
        refusals: 0,
        ..blocked()
    };
    let json = encode(&quiet).expect("a classification is plain data");
    assert!(
        json.contains("\"refused\":[]") && json.contains("\"refusals\":0"),
        "empty is a Drone that was refused nothing, and it is stated rather \
         than left to a missing key: {json}"
    );
}

/// **Absent, not `null`.** Every trigger but `gate_undecided` carries no
/// sentence to say, and a client reading `undecided: null` could not tell that
/// from a Fleet that simply had not sent the field yet.
#[test]
fn the_gates_sentence_crosses_only_where_it_timed_out() {
    let json = encode(&blocked()).expect("a classification is plain data");
    assert!(
        !json.contains("undecided"),
        "gate_failure carries no sentence about a timeout: {json}"
    );

    let timed_out = Stuck {
        stopped_by: Some(String::from("gate_undecided")),
        undecided: Some(String::from("the Judge did not answer inside its budget")),
        ..blocked()
    };
    let json = encode(&timed_out).expect("a classification is plain data");
    assert!(
        json.contains("\"undecided\":\"the Judge did not answer inside its budget\""),
        "{json}"
    );
    assert_eq!(
        decode::<Stuck>("a classification", json.as_bytes()).expect("it reads back"),
        timed_out
    );
}

/// **A cut command has to say it was cut.** The whole of a heredoc stays in the
/// file, so what crosses is a line — and a client that drew that line as the
/// entire command would have somebody paste a truncated one into an allowlist,
/// which is the failure this whole field exists to prevent, one step further
/// on.
#[test]
fn a_cut_argument_says_so_and_how_much_there_was() {
    let mut cut = blocked();
    cut.refused[0].detail = String::from("cat <<EOF > out.txt word word word");
    cut.refused[0].truncated = true;
    cut.refused[0].length = Some(14_320);

    let json = encode(&cut).expect("a classification is plain data");
    assert!(json.contains("\"truncated\":true"), "{json}");
    assert!(
        json.contains("\"length\":14320"),
        "the size of what there was, so a surface says a proportion rather \
         than reporting that something was taken away: {json}"
    );
    assert_eq!(
        decode::<Stuck>("a classification", json.as_bytes()).expect("it reads back"),
        cut
    );
}

/// A row written before the file recorded the size carries no length, and that
/// is a different answer from an argument of no length.
#[test]
fn an_unmeasured_argument_carries_no_length_rather_than_nought() {
    let mut old = blocked();
    old.refused[0].length = None;

    let json = encode(&old).expect("a classification is plain data");
    assert!(
        !json.contains("\"length\""),
        "absent, never present-and-null: {json}"
    );
    assert_eq!(
        decode::<Stuck>("a classification", json.as_bytes()).expect("it reads back"),
        old
    );
}
