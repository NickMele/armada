//! `secrets:` and `secret_providers:`, end to end — **and the one assertion the
//! whole feature exists for: a resolved value appears nowhere on disk.**
//!
//! **No provider here is real and none of them costs anything.** A provider is
//! *"a command that prints a secret to stdout"* (`PLAN.md` §4.7), which is
//! exactly what `printf` is — so the provider under test is four lines of `sh`
//! that talks to nothing, in the same spirit as the stub `claude` this suite
//! already puts on `PATH`. Nothing here spends a token, starts a session or
//! touches a keychain.
//!
//! **Every planted value is chosen to be invisible to the shape detector.**
//! `armada_helm::redact::scrub` already catches anything that *looks* like a
//! credential — a vendor prefix, a JWT, a long opaque run — so a fixture shaped
//! like a real token would be redacted whether or not this feature worked, and
//! the test would pass against a broken implementation. `hunter2-planted` is a
//! password no shape test will ever recognise, which is what makes the greps
//! below evidence rather than decoration.
//!
//! **Every group started here is stopped here**, for the reason `detach.rs`
//! gives: a detached run outlives the invocation that started it.

mod support;

use std::path::Path;
use std::time::{Duration, Instant};
use support::Machine;

/// The value planted in every test that greps for one.
///
/// **Deliberately not credential-shaped** — see the module header. Asserted to
/// be invisible to the shape detector by
/// [`the_planted_value_is_one_the_shape_detector_cannot_see`], so this cannot
/// drift into a token by accident and quietly make every grep below vacuous.
const PLANTED: &str = "hunter2-planted";

/// The environment variable it is granted as.
///
/// **Not credential-shaped either.** `SECRET_TOKEN=hunter2-planted` would be
/// redacted by the *name*, which is the same vacuity in the other direction.
const NAME: &str = "PLANTED";

/// A repository whose one check is granted the secret and prints it.
///
/// **The check both leaks it and fails**, on purpose: a failing check is what
/// writes the failure journal, the run's log file and Armada's own failure
/// record, which is three of the four places the grep has to look.
fn leaky_config() -> String {
    format!(
        "\
manifest:
  version: 1
  secrets:
    {NAME}: stub://one
  secret_providers:
    stub: {{ cmd: \"./provider.sh ${{ref}}\" }}
  components:
    app:
      root: src
      checks:
        leaks: {{ cmd: \"./printer.sh\", scope: component, secrets: [{NAME}] }}
"
    )
}

/// One granted check and one that is not, so the grant can be shown to be
/// per-entry rather than per-run (`PLAN.md` §4.7 rule 5).
fn two_checks_one_grant() -> String {
    format!(
        "\
manifest:
  version: 1
  secrets:
    {NAME}: stub://one
  secret_providers:
    stub: {{ cmd: \"./provider.sh ${{ref}}\" }}
  components:
    app:
      root: src
      checks:
        granted: {{ cmd: \"./reporter.sh granted\", scope: component, secrets: [{NAME}] }}
        ungranted: {{ cmd: \"./reporter.sh ungranted\", scope: component }}
"
    )
}

/// Write the stub providers and the checks that report on them.
///
/// | script | what it is |
/// |---|---|
/// | `provider.sh` | the provider under test: records that it ran, refuses to run without a terminal, prints the secret |
/// | `printer.sh` | a check that leaks its grant into its own output, then fails |
/// | `reporter.sh` | a check that writes what it can see of the environment to a file named after itself |
fn scripts(repo: &Path) {
    // **The provider, and the third line is the whole of `013`.**
    //
    // `ARMADA_DETACH_RUN` is set on the detached child and inherited by
    // everything below it (`PLAN.md` §4.5), and it is *not* set on the parent
    // that detaches. So a provider invoked from the run loop — the design `013`
    // was written to forbid — sees it and fails, and a provider invoked in the
    // caller's terminal does not and succeeds. That stands in for the real
    // symptom, which is `op` waiting on a biometric against a session nobody is
    // looking at; a test cannot wait two minutes for a hang, and it does not
    // need to, because the *cause* is observable and the hang is only its
    // consequence.
    write(
        repo,
        "provider.sh",
        &format!(
            "#!/bin/sh\n\
             echo \"$1\" >> \"$(dirname \"$0\")/provider.calls\"\n\
             if [ -n \"$ARMADA_DETACH_RUN\" ]; then\n\
             \x20 echo 'no terminal here to prompt on' >&2\n\
             \x20 exit 1\n\
             fi\n\
             printf '%s\\n' '{PLANTED}'\n"
        ),
    );
    // Leaks its grant the way a real check does — `set -x`, a debug log, an
    // error that echoes the environment — and then fails.
    write(
        repo,
        "printer.sh",
        &format!("#!/bin/sh\necho \"resolved {NAME}=${NAME}\"\nexit 3\n"),
    );
    // Writes down exactly what it could see, so the assertion is about the
    // child's environment rather than about anything Armada reported.
    write(
        repo,
        "reporter.sh",
        &format!("#!/bin/sh\nprintf '%s' \"${NAME}\" > \"$(dirname \"$0\")/saw.$1\"\n"),
    );
}

fn write(repo: &Path, name: &str, body: &str) {
    let path = repo.join(name);
    std::fs::write(&path, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn envelope(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!("not JSON: {e}\n{}", support::why(output));
    })
}

/// Every regular file under `root`, read as bytes.
///
/// **Bytes rather than text**, because a file this misses is a file the grep
/// cannot search, and a run directory holds logs whose encoding nobody
/// promised.
fn files_under(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => stack.push(path),
                Ok(kind) if kind.is_file() => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        found.push((path, bytes));
                    }
                }
                _ => {}
            }
        }
    }
    found
}

/// Fail naming the file, because *"a value leaked"* without the path is a
/// finding nobody can act on.
fn assert_absent(root: &Path, needle: &str, what: &str) {
    let mut searched = 0usize;
    for (path, bytes) in files_under(root) {
        searched += 1;
        let haystack = String::from_utf8_lossy(&bytes);
        assert!(
            !haystack.contains(needle),
            "`{needle}` reached {} ({what}):\n{haystack}",
            path.display()
        );
    }
    // **The grep has to have looked at something.** A walk that found nothing
    // passes every assertion above it and proves nothing — which is exactly the
    // vacuous-assertion failure `AGENTS.md` says to invert and watch fail.
    assert!(searched > 0, "nothing was searched under {}", root.display());
}

/// Stop whatever a detached run left behind — `detach.rs`'s rule, and this
/// suite starts groups too.
fn stop(machine: &Machine, repo: &Path) {
    let _ = machine.run(repo, &["manifest", "clean", "--force", "--json"]);
}

fn poll_until_done(machine: &Machine, repo: &Path, run: &str) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = envelope(&machine.run(repo, &["manifest", "check", "--status", run, "--json"]));
        if last["status"] != "RUNNING" {
            return last;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("run {run} never left RUNNING; last poll was {last}");
}

/// **The invariant this whole file exists to check that the fixture can
/// check.** If the planted value were credential-shaped, every grep below would
/// pass against an implementation that did nothing at all, because
/// `redact::scrub` would have removed it on shape alone.
#[test]
fn the_planted_value_is_one_the_shape_detector_cannot_see() {
    assert!(
        !armada_guild::secrets::value_is_credential_shaped(PLANTED),
        "`{PLANTED}` is credential-shaped, so every grep in this file is vacuous"
    );
    assert!(
        !armada_guild::secrets::key_is_credential_shaped(NAME),
        "`{NAME}` is credential-shaped, so `{NAME}=…` would be redacted by name alone"
    );
    assert_eq!(
        armada_helm::redact::scrub(&format!("{NAME}={PLANTED}")),
        format!("{NAME}={PLANTED}"),
        "the shape redactor already removes the fixture; it cannot prove anything"
    );
}

/// **The deliverable.** Plant a value, run a check that leaks it, and require it
/// to be absent from every file Armada wrote — the run directory, the failure
/// journal, `~/.armada/failures.jsonl` and the ring buffer — and from the
/// envelope on stdout.
#[test]
fn a_resolved_value_appears_nowhere_armada_wrote() {
    let machine = Machine::new();
    let repo = machine.repo("leaky", &leaky_config());
    scripts(&repo);

    let output = machine.run(&repo, &["manifest", "check", "--json"]);
    let json = envelope(&output);
    assert_eq!(json["status"], "FAILED", "{}", support::why(&output));

    // The check really did receive it — otherwise the greps below are about a
    // value that never existed. The *masked* line proves both halves at once:
    // the variable reached the child, and what it printed did not reach the log.
    let log = std::fs::read_to_string(
        repo.join(json["data"]["results"][0]["log"].as_str().expect("a log path")),
    )
    .expect("the check's log");
    assert_eq!(
        log.trim(),
        format!("resolved {NAME}={}", armada_helm::redact::MASK),
        "the child either never got the secret or the log kept it"
    );

    // The four places. **`.armada/` rather than the whole workspace**, because
    // the stub provider's own source is in the workspace and has to contain the
    // value it prints — grepping it would only ever find the fixture. Everything
    // *Armada* wrote about this run is under `.armada/` and under `$HOME`, and
    // that is the claim being made.
    assert_absent(
        &repo.join(".armada"),
        PLANTED,
        "the run directory: logs, the record and the failure journal",
    );
    assert_absent(
        machine.home.path(),
        PLANTED,
        "~/.armada — manifest.db, failures.jsonl and the ring buffer",
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains(PLANTED),
        "the `--json` envelope carried it"
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains(PLANTED),
        "the progress stream carried it"
    );
}

/// **`PLAN.md` §4.7 rule 5, by name:** *"a check with no grant sees no secret in
/// its environment during a run where a sibling check has one."*
#[test]
fn only_the_check_that_declared_the_grant_can_see_it() {
    let machine = Machine::new();
    let repo = machine.repo("grants", &two_checks_one_grant());
    scripts(&repo);

    let output = machine.run(&repo, &["manifest", "check", "--json"]);
    assert_eq!(envelope(&output)["status"], "PASS", "{}", support::why(&output));

    assert_eq!(
        std::fs::read_to_string(repo.join("saw.granted")).unwrap(),
        PLANTED,
        "the check that declared the grant did not get it"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("saw.ungranted")).unwrap(),
        "",
        "a check with no `secrets:` saw a sibling's token"
    );
}

/// **`013`, and the case that fails silently without it.** A provider that only
/// works where the terminal is must still work under `--detach`, because the
/// parent resolved it before the child ever existed.
///
/// Inverted below by [`the_same_provider_fails_if_it_is_run_from_the_detached_child`],
/// which is what makes this an assertion about *where* resolution happened
/// rather than about the run merely passing.
#[test]
fn a_detached_run_works_with_a_provider_only_the_parent_could_have_run() {
    let machine = Machine::new();
    let repo = machine.repo("detached", &two_checks_one_grant());
    scripts(&repo);

    let started = machine.run(&repo, &["manifest", "check", "--detach", "--json"]);
    let json = envelope(&started);
    assert_eq!(json["status"], "RUNNING", "{}", support::why(&started));
    let run = json["data"]["run_id"].as_str().unwrap().to_string();

    let done = poll_until_done(&machine, &repo, &run);
    stop(&machine, &repo);
    assert_eq!(done["status"], "PASS", "the detached run did not pass: {done}");

    assert_eq!(
        std::fs::read_to_string(repo.join("saw.granted")).unwrap(),
        PLANTED,
        "the detached child never received the value its parent resolved"
    );
    // **Once, in the parent.** Twenty grants of one secret are one call
    // (`PLAN.md` §4.7's in-memory cache), and a second line here would mean the
    // child resolved it again — which is the design `013` forbids.
    assert_eq!(
        std::fs::read_to_string(repo.join("provider.calls")).unwrap(),
        "one\n",
        "the provider ran somewhere other than exactly once in the parent"
    );
    // **`.armada/` rather than the whole repo here**, because `saw.granted` is
    // the fixture's own record of what the check could see — the value in it is
    // the assertion above, not a leak. Everything Armada itself wrote about this
    // run is under `.armada/`, and that is the claim.
    assert_absent(
        &repo.join(".armada"),
        PLANTED,
        "the detached run's own directory",
    );
    assert_absent(machine.home.path(), PLANTED, "~/.armada");
}

/// The inversion: the same provider, run where `013` says it must not be, fails.
///
/// **Without this the test above is vacuous** — it would pass just as happily
/// against a provider that works everywhere. This proves the fixture can tell
/// the two placements apart, by putting the provider in the environment the
/// detached child has and watching it refuse.
#[test]
fn the_same_provider_fails_if_it_is_run_from_the_detached_child() {
    let machine = Machine::new();
    let repo = machine.repo("inverted", &two_checks_one_grant());
    scripts(&repo);

    // Standing in for the detached child's environment. Armada treats an
    // unparseable value as "not a detached run" (`adopted_run`), so this adopts
    // nothing and takes the ordinary resolving path — it only shows the provider
    // what the detached child would have shown it.
    let output = machine.run_with_env(
        &repo,
        &["manifest", "check", "--json"],
        &[(armada_helm::verbs::check::DETACH_RUN_VAR, "not-a-run-id")],
    );

    let json = envelope(&output);
    assert_eq!(json["status"], "FAILED", "{}", support::why(&output));
    let message = json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains(NAME), "the error names no secret: {json}");
    assert!(message.contains("stub"), "the error names no provider: {json}");
}

/// **Reporting what would run is not a reason to touch a hardware key.**
/// `--dry-run` invokes nothing, and the sentinel the provider writes on every
/// call is how that is checked rather than asserted.
#[test]
fn a_dry_run_never_invokes_a_provider() {
    let machine = Machine::new();
    let repo = machine.repo("preview", &two_checks_one_grant());
    scripts(&repo);

    let output = machine.run(&repo, &["manifest", "check", "--dry-run", "--json"]);
    let json = envelope(&output);
    assert_eq!(json["status"], "SKIPPED", "{}", support::why(&output));
    assert!(
        !repo.join("provider.calls").exists(),
        "a preview made somebody touch their hardware key"
    );
    // And the preview still said what it would have done, so nothing was
    // skipped to get here.
    assert!(
        json["data"]["would_run"]
            .as_array()
            .is_some_and(|rows| rows.len() == 2),
        "the preview reported nothing: {json}"
    );
}

/// A secret no selected check was granted is never resolved: `--component web`
/// must not prompt for `api`'s token.
#[test]
fn a_secret_outside_the_selection_is_never_resolved() {
    let machine = Machine::new();
    let repo = machine.repo(
        "scoped",
        &format!(
            "\
manifest:
  version: 1
  secrets:
    {NAME}: stub://one
  secret_providers:
    stub: {{ cmd: \"./provider.sh ${{ref}}\" }}
  components:
    api:
      root: api
      checks:
        needs: {{ cmd: \"./reporter.sh api\", scope: component, secrets: [{NAME}] }}
    web:
      root: web
      checks:
        plain: {{ cmd: \"./reporter.sh web\", scope: component }}
"
        ),
    );
    scripts(&repo);

    let output = machine.run(
        &repo,
        &["manifest", "check", "--component", "web", "--json"],
    );
    assert_eq!(envelope(&output)["status"], "PASS", "{}", support::why(&output));
    assert!(
        !repo.join("provider.calls").exists(),
        "linting `web` resolved `api`'s secret"
    );
}

/// A provider that fails says which secret and which provider — and **never
/// repeats what the provider printed** (`PLAN.md` §4.7 rule 3), because a
/// chatty provider is the one path structurally incapable of redaction.
#[test]
fn a_failing_provider_is_named_and_its_output_is_not_repeated() {
    let machine = Machine::new();
    let repo = machine.repo("broken", &two_checks_one_grant());
    scripts(&repo);
    // `set -x` and a debug line: what a real provider does when it goes wrong.
    write(
        &repo,
        "provider.sh",
        &format!("#!/bin/sh\necho 'DEBUG token was {PLANTED}' >&2\nexit 7\n"),
    );

    let output = machine.run(&repo, &["manifest", "check", "--json"]);
    let json = envelope(&output);
    assert_eq!(json["status"], "FAILED", "{}", support::why(&output));
    assert_eq!(json["error"]["class"], "tool_failed", "{json}");

    let message = json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains(NAME), "no secret named: {json}");
    assert!(message.contains("stub"), "no provider named: {json}");
    assert!(message.contains('7'), "no exit code: {json}");
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains(PLANTED),
        "the provider's own output was repeated back"
    );
    assert_absent(machine.home.path(), PLANTED, "~/.armada");
}

/// A provider naming something that does not exist is a failure and never an
/// empty variable — a variable set to `""` reads as configured and is not.
#[test]
fn a_provider_that_prints_nothing_fails_the_run() {
    let machine = Machine::new();
    let repo = machine.repo("silent", &two_checks_one_grant());
    scripts(&repo);
    write(&repo, "provider.sh", "#!/bin/sh\nexit 0\n");

    let json = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    assert_eq!(json["status"], "FAILED", "{json}");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("printed nothing")),
        "{json}"
    );
}

/// **`PLAN.md` §4.7 rule 1, end to end.** A reference that reads as an inert URI
/// in review and is command injection in a shell arrives as one argument.
#[test]
fn an_injection_in_a_reference_arrives_as_one_argument() {
    let machine = Machine::new();
    let repo = machine.repo(
        "injected",
        &format!(
            "\
manifest:
  version: 1
  secrets:
    {NAME}: \"stub://a; touch INJECTED; echo $(whoami)\"
  secret_providers:
    stub: {{ cmd: \"./provider.sh ${{ref}}\" }}
  components:
    app:
      root: src
      checks:
        one: {{ cmd: \"./reporter.sh one\", scope: component, secrets: [{NAME}] }}
"
        ),
    );
    scripts(&repo);
    // Prints its argument count and its one argument, so the assertion is about
    // argv rather than about what the provider chose to do with it.
    write(
        &repo,
        "provider.sh",
        "#!/bin/sh\nprintf 'argc=%s arg=%s' \"$#\" \"$1\"\n",
    );

    let output = machine.run(&repo, &["manifest", "check", "--json"]);
    assert_eq!(envelope(&output)["status"], "PASS", "{}", support::why(&output));
    assert!(
        !repo.join("INJECTED").exists(),
        "the reference was interpreted by a shell"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("saw.one")).unwrap(),
        "argc=1 arg=a; touch INJECTED; echo $(whoami)",
        "the reference was word-split or expanded"
    );
}
