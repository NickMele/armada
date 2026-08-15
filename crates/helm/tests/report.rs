//! `armada report` end to end, against a scratch `$HOME`.
//!
//! **The suite that matters here is the secret one.** *"But don't include
//! secrets"* is the constraint this feature is built under, and a claim of
//! scrubbing without a test is a promise. So a credential-shaped value is
//! planted in all three places one really arrives from — **an environment
//! variable, a command line, and the output of an earlier run** — and every one
//! of them is asserted absent from what was written to disk.
//!
//! **Nothing here spends a token or starts a `claude` session.** The stub on
//! `PATH` answers `--version`, and every verb driven is a local one.

mod support;

use support::{why, Machine};

/// A GitHub personal access token's shape, with no account behind it. **Not a
/// real credential** — this repository is public permanently, so a fixture may
/// only ever have the *shape* of one.
const TOKEN: &str = "ghp_A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8";

/// Everything Armada wrote about this machine, as one string.
///
/// **Both files, because a leak in either is a leak.** The ring buffer is
/// written on every run and the failures store on every report, and asserting
/// only on the one being tested is how the other becomes the hole.
fn everything_written(machine: &Machine) -> String {
    let armada = machine.home.path().join(".armada");
    ["failures.jsonl", "recent.jsonl"]
        .iter()
        .map(|name| std::fs::read_to_string(armada.join(name)).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// **The evidence for the whole feature.** A report is filed about the run that
/// prompted it, the description comes back, the diagnostics are attached, and
/// the same entry lists and shows under `armada failures` — one store, one id.
#[test]
fn a_report_is_filed_with_its_diagnostics_and_lists_beside_what_armada_caught() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");

    // The run being reported. It succeeds, which is the whole point: no failure
    // recorder could ever hold it.
    let before = machine.run(&repo, &["manifest", "status"]);
    assert!(before.status.success(), "{}", why(&before));

    let filed = machine.run(
        &repo,
        &[
            "report",
            "the dry-run said CREATED worktree and made nothing",
        ],
    );
    assert!(filed.status.success(), "{}", why(&filed));
    let said = stdout(&filed);
    assert!(
        said.contains("the dry-run said CREATED worktree and made nothing"),
        "the description comes back: {said}"
    );
    assert!(
        said.contains("REPORTED") || said.contains("reported"),
        "{said}"
    );
    // The diagnostics are attached rather than asked for.
    assert!(said.contains("what was run before it"), "{said}");
    assert!(
        said.contains("manifest status"),
        "the run before it: {said}"
    );
    assert!(said.contains("armada"), "{said}");

    // **One store.** The same entry is in `armada failures`, and `show` opens
    // it — no second listing, no second id space.
    let listed = machine.run(&repo, &["failures", "--json"]);
    assert!(listed.status.success(), "{}", why(&listed));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&listed)).expect("an envelope");
    let rows = payload["data"]["results"].as_array().expect("results[]");
    let row = rows
        .iter()
        .find(|row| row["origin"] == "reported")
        .expect("the filed report is in the listing");
    assert_eq!(row["state"], "OPEN");
    assert!(
        row["class"].is_null(),
        "a report carries no class: Armada attributed nothing"
    );
    let id = row["id"].as_str().expect("an id").to_string();

    let shown = machine.run(&repo, &["failures", "show", &id[..4], "--json"]);
    assert!(shown.status.success(), "{}", why(&shown));
    let one: serde_json::Value = serde_json::from_str(&stdout(&shown)).expect("an envelope");
    let entry = &one["data"]["results"][0];
    assert_eq!(entry["id"], id.as_str(), "the same entry, by prefix");
    // **`--json` carries all of it**, so an agent can read a report without a
    // human relaying it.
    let diagnostics = &entry["diagnostics"];
    assert!(diagnostics.is_object(), "the diagnostics travel: {entry}");
    assert!(!diagnostics["armada"].as_str().unwrap().is_empty());
    assert!(!diagnostics["system"].as_str().unwrap().is_empty());
    assert!(
        diagnostics["recent"]
            .as_array()
            .expect("recent[]")
            .iter()
            .any(|run| run["verb"] == "manifest status"),
        "the run being reported is attached: {diagnostics}"
    );
    assert!(
        one["data"]["task"]
            .as_str()
            .unwrap()
            .contains("reported by hand"),
        "the prompt a Job would get says which kind of record this is"
    );
}

/// **The deliverable.** A credential planted in an environment variable, on a
/// command line, and in the captured output of an earlier run reaches none of
/// what Armada writes down.
///
/// Each half is asserted separately: that the secret is gone, **and** that the
/// thing it was attached to is still there. A redactor that dropped the whole
/// line would pass the first assertion and make every attachment useless, which
/// is the failure mode that ends with the guard being turned off.
#[test]
fn no_credential_survives_a_report_from_the_environment_the_argv_or_the_output() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");

    // Shape one: the environment. It reaches the record through the envelope of
    // any verb that reports what it was given.
    let with_env = machine.run_with_env(
        &repo,
        &["manifest", "status"],
        &[("GITHUB_TOKEN", TOKEN), ("ANTHROPIC_API_KEY", TOKEN)],
    );
    assert!(with_env.status.success(), "{}", why(&with_env));

    // Shape two: the command line. `--token` is not a flag Armada takes, so the
    // run is refused — and a refused run is still a run the buffer keeps, which
    // is exactly why it has to be scrubbed.
    machine.run(&repo, &["manifest", "status", "--token", TOKEN]);
    machine.run(&repo, &["manifest", "status", &format!("--token={TOKEN}")]);

    // Shape three: captured output. A repository whose config names the token
    // puts it in the envelope of the verb that reads the config.
    let leaky = machine.repo(
        "leaky",
        &format!("version: 1\nname: leaky\ncommands:\n  deploy: echo {TOKEN}\n"),
    );
    machine.run(&leaky, &["manifest", "commands"]);

    let filed = machine.run(
        &repo,
        &["report", "the output carried something it should not have"],
    );
    assert!(filed.status.success(), "{}", why(&filed));

    let written = everything_written(&machine);
    assert!(!written.is_empty(), "something was written to assert on");
    assert!(
        !written.contains(TOKEN),
        "a credential reached what Armada wrote down:\n{written}"
    );
    assert!(
        !written.contains("ghp_"),
        "not even the prefix survives:\n{written}"
    );
    assert!(
        written.contains("[redacted]"),
        "the mask says a value was removed rather than never being there:\n{written}"
    );
    // The other half: the record is still a record. The flag survives even
    // though its value did not, because *which* flag was given is the
    // diagnostic.
    assert!(
        written.contains("--token"),
        "the flag names itself, and only the value goes:\n{written}"
    );
    assert!(
        written.contains("manifest status"),
        "the runs are still identifiable:\n{written}"
    );

    // And the report a person reads carries none of it either.
    assert!(!stdout(&filed).contains("ghp_"), "{}", stdout(&filed));
}

/// **No absolute `$HOME` reaches disk**, the rule `docs/reserved/010` states for
/// the failure log, holding for the two files this feature adds to and writes.
///
/// This repository is public permanently and the eventual GitHub-issue path
/// makes that worse rather than better: a home path cannot be un-published once
/// it is.
#[test]
fn no_absolute_home_path_reaches_either_file() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    machine.run(&repo, &["manifest", "status"]);
    let filed = machine.run(&repo, &["report", "the paths were absolute"]);
    assert!(filed.status.success(), "{}", why(&filed));

    let home = machine.home.path().display().to_string();
    let written = everything_written(&machine);
    assert!(
        !written.contains(&home),
        "an absolute $HOME reached disk:\n{written}"
    );
    assert!(written.contains('~'), "and it was abbreviated:\n{written}");
}

/// **The ring buffer is bounded on a real machine**, which is half the argument
/// for it existing at all.
#[test]
fn the_ring_buffer_never_grows_past_what_it_keeps() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    for _ in 0..(armada_core::recent::KEEP + 3) {
        machine.run(&repo, &["manifest", "status"]);
    }
    let text =
        std::fs::read_to_string(machine.home.path().join(".armada/recent.jsonl")).expect("written");
    assert_eq!(text.lines().count(), armada_core::recent::KEEP);
}

/// **A report without a sentence is refused rather than filed empty**, and the
/// refusal shows the line that would have worked.
#[test]
fn a_report_with_nothing_to_say_is_refused_with_the_shape_that_works() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let refused = machine.run(&repo, &["report"]);
    assert!(!refused.status.success(), "{}", why(&refused));
    let said = why(&refused);
    assert!(said.contains("armada report \""), "{said}");
}

/// **A report can be filed from a directory that is no workspace**, which is
/// where a good deal of what is worth reporting happens — including the failure
/// that prompted the failure log in the first place.
#[test]
fn a_report_can_be_filed_from_outside_any_workspace() {
    let machine = Machine::new();
    let outside = machine.outside();
    let filed = machine.run(&outside, &["report", "armada would not start here"]);
    assert!(filed.status.success(), "{}", why(&filed));
    assert!(
        stdout(&filed).contains("no workspace here"),
        "the absence is reported as a finding: {}",
        stdout(&filed)
    );
}

/// **A filed report is promoted into a Job exactly as a recorded failure is** —
/// one promotion path, and `--dry-run` proves it without spending anything.
#[test]
fn a_filed_report_promotes_into_a_job_through_the_same_verb() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    // `fix` names the `bug` workflow rather than classifying it, which is what
    // keeps this path free of a model call — so the guild has to have one.
    let workflows = machine.home.path().join(".armada/guild/workflows");
    std::fs::create_dir_all(&workflows).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/guild/workflows/bug.yml"),
        workflows.join("bug.yml"),
    )
    .unwrap();
    let filed = machine.run(&repo, &["report", "the dry-run made nothing", "--json"]);
    assert!(filed.status.success(), "{}", why(&filed));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&filed)).expect("an envelope");
    let id = payload["data"]["results"][0]["id"]
        .as_str()
        .expect("an id")
        .to_string();

    let fixed = machine.run(&repo, &["failures", "fix", &id, "--dry-run", "--json"]);
    assert!(fixed.status.success(), "{}", why(&fixed));
    let spawned: serde_json::Value = serde_json::from_str(&stdout(&fixed)).expect("an envelope");
    assert_eq!(
        spawned["data"]["workflow"], "bug",
        "named rather than classified, so nothing is spent: {spawned}"
    );
}
