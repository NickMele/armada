//! `armada untried` end to end, against a scratch `$HOME`.
//!
//! **What this holds is the one claim that cannot be checked in a unit test**:
//! that the counter is written by the real entrypoint, on the real path, for
//! every verb — including the ones that fail. A fold tested against a
//! hand-written file proves the arithmetic and nothing about whether anything
//! ever calls it (`AGENTS.md`: *a green unit test on a reducer does not prove
//! the driver ever feeds it the event*).

mod support;

use support::{why, Machine};

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn listing(machine: &Machine, at: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut line = vec!["untried"];
    line.extend_from_slice(args);
    line.push("--json");
    let out = machine.run(at, &line);
    assert!(out.status.success(), "{}", why(&out));
    serde_json::from_str(&stdout(&out)).expect("an envelope")
}

fn row<'a>(payload: &'a serde_json::Value, verb: &str) -> Option<&'a serde_json::Value> {
    payload["data"]["results"]
        .as_array()
        .expect("results[]")
        .iter()
        .find(|row| row["verb"] == verb)
}

/// **The whole feature.** On a machine that has run nothing, every verb Armada
/// owns is on the list; run one, and it leaves.
#[test]
fn a_verb_leaves_the_untried_list_the_moment_it_is_run() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");

    // One run, so the file exists — and `manifest status` is the verb it counts.
    let first = machine.run(&repo, &["manifest", "status"]);
    assert!(first.status.success(), "{}", why(&first));

    let untried = listing(&machine, &repo, &[]);
    assert!(
        row(&untried, "manifest status").is_none(),
        "a verb that has been run is still listed as never run: {untried}"
    );
    assert!(
        row(&untried, "guild export").is_some(),
        "a verb nobody has run has to be on the list, or the list says nothing: {untried}"
    );
    assert!(
        untried["data"]["untried"].as_u64().unwrap() > 1,
        "{untried}"
    );
    assert_eq!(
        untried["data"]["tried"], 0,
        "the listing shows only untried"
    );

    // `--all` is the whole roster, and it carries the count and the age.
    let all = listing(&machine, &repo, &["--all"]);
    let ran = row(&all, "manifest status").expect("the run is counted");
    assert_eq!(ran["count"], 1, "{all}");
    assert_eq!(ran["ok"], true, "{all}");
    assert!(ran["age_s"].is_u64(), "{all}");
    // **Never is null, not zero.** The two answers this listing exists to tell
    // apart must not render the same.
    let never = row(&all, "guild export").expect("on the roster");
    assert!(never["age_s"].is_null(), "{all}");
    assert_eq!(never["count"], 0, "{all}");
    assert!(all["data"]["tried"].as_u64().unwrap() >= 1, "{all}");
}

/// **A failing run still counts, and says it failed.** A verb you tried once and
/// that broke is exactly the one that still needs testing, and a counter that
/// only recorded successes would hide it.
#[test]
fn a_verb_that_failed_is_counted_and_the_failure_is_on_the_row() {
    let machine = Machine::new();
    let outside = machine.outside();
    // `manifest status` outside a workspace is a real refusal.
    let broke = machine.run(&outside, &["manifest", "status"]);
    assert!(!broke.status.success(), "{}", why(&broke));

    let all = listing(&machine, &outside, &["--all"]);
    let ran = row(&all, "manifest status").expect("counted even though it failed");
    assert_eq!(ran["count"], 1, "{all}");
    assert_eq!(ran["ok"], false, "{all}");
}

/// **A sub-verb counts against its parent**, because `tasks start` is not a page
/// `--help` reaches and a row for it could never be satisfied.
#[test]
fn a_sub_verb_counts_against_the_verb_that_owns_the_page() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let written = machine.run(&repo, &["task", "rename the port allocator"]);
    assert!(written.status.success(), "{}", why(&written));
    machine.run(&repo, &["tasks", "--json"]);

    let all = listing(&machine, &repo, &["--all"]);
    assert!(
        row(&all, "tasks").expect("on the roster")["count"] == 1,
        "{all}"
    );
    assert!(row(&all, "tasks start").is_none(), "{all}");
}

/// **Nothing a person typed reaches the file.** A repository's declared command
/// and an unknown verb are both off Armada's roster, so both are counted as
/// nothing at all — which is why this file needs no scrubber of its own.
#[test]
fn nothing_off_armadas_own_roster_is_ever_written_down() {
    let machine = Machine::new();
    let repo = machine.repo(
        "orders",
        "version: 1\nname: orders\ncommands:\n  deploy: ./echoer.sh\n",
    );
    machine.run(&repo, &["manifest", "deploy"]);
    machine.run(&repo, &["ghp_notarealtoken"]);
    machine.run(&repo, &["manifest", "status"]);

    let written = std::fs::read_to_string(machine.home.path().join(".armada/untried.jsonl"))
        .expect("written");
    assert!(!written.contains("deploy"), "{written}");
    assert!(!written.contains("ghp_"), "{written}");
    assert!(
        written.contains("manifest status"),
        "the roster verb is still counted: {written}"
    );
}

/// **The file is bounded by the roster.** Every run rewrites it, so a machine
/// used all day holds one line per verb rather than one per invocation.
#[test]
fn the_file_holds_one_line_per_verb_however_much_armada_is_used() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    for _ in 0..6 {
        machine.run(&repo, &["manifest", "status"]);
    }
    machine.run(&repo, &["doctor"]);
    let written = std::fs::read_to_string(machine.home.path().join(".armada/untried.jsonl"))
        .expect("written");
    assert_eq!(written.lines().count(), 2, "{written}");

    let all = listing(&machine, &repo, &["--all"]);
    assert_eq!(row(&all, "manifest status").unwrap()["count"], 6, "{all}");
}

/// **`armada untried` answers on a machine that has never run anything.** It is
/// the verb you reach for when you do not know what works yet, so it must not
/// need a workspace, a guild or a boot id.
#[test]
fn untried_answers_from_nowhere_on_a_machine_that_has_done_nothing() {
    let machine = Machine::new();
    let outside = machine.outside();
    let listed = listing(&machine, &outside, &[]);
    assert!(
        listed["data"]["untried"].as_u64().unwrap() > 5,
        "the whole roster is untried: {listed}"
    );
    assert_eq!(listed["data"]["tried"], 0, "{listed}");
}
