//! What a Drone is confined to, asserted on the rendering rather than on a
//! process.
//!
//! **No test here starts anything.** That is the point of the harness rendering
//! instead of spawning: every property this milestone step is about — the
//! strict flag, the one server, the Evidence tool, an argument list with
//! nothing readable on it, an environment nobody inherited — is a value, so a
//! suite can hold the whole confinement posture without a credential, a network
//! or a dollar.

use adapter_traits::{
    AgentHarness, DroneSpawnConfig, Environment, Grant, McpConfig, Model, Prompt, Toolbelt,
    Worktree,
};

use crate::harness::{
    ask_tool, checks_tool, dispatch_tool, evidence_tool, permission_tool, scope_tool, widen_tool,
    HarnessRefused, HeadlessAgent,
};

const SECRET_LOOKING_TASK: &str = "fix the parser, the token is hunter2";

fn worktree() -> Worktree {
    Worktree::at("/repos/armada/.armada/worktrees/01AAA", "armada/01AAA")
}

fn environment() -> Environment {
    Environment::nothing()
        .and("PATH", "/usr/bin:/bin")
        .expect("a legal name")
        .and("HOME", "/Users/user")
        .expect("a legal name")
}

fn config(toolbelt: Toolbelt) -> DroneSpawnConfig {
    DroneSpawnConfig::spawn_in(
        &worktree(),
        Model::named("a-model").expect("a named model"),
        Prompt::assembled(SECRET_LOOKING_TASK).expect("an assembled prompt"),
        McpConfig::only_these("/var/armada/01AAA/mcp.json").expect("an absolute path"),
        toolbelt,
        environment(),
    )
}

fn rendered(toolbelt: Toolbelt) -> Vec<String> {
    HeadlessAgent::at("/usr/local/bin/agent")
        .render(&config(toolbelt))
        .expect("a legal configuration renders")
        .args()
        .to_vec()
}

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

#[test]
fn every_rendering_carries_the_strict_flag_and_the_file_together() {
    // The v1 defect, in one assertion. Spawning without the flag brought the
    // operator's seven connected servers and ninety-five tools into the
    // session, and the run looked like a success throughout.
    for toolbelt in [
        Toolbelt::evidence_only(),
        Toolbelt::evidence_only().and(Grant::ReadTheWorktree),
        Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ChangeTheWorktree)
            .and(Grant::RunADeclaredCommand(String::from("cargo test"))),
    ] {
        let args = rendered(toolbelt);
        assert!(
            args.iter().any(|arg| arg == "--strict-mcp-config"),
            "no ambient server may appear under any condition: {args:?}"
        );
        assert_eq!(
            value_after(&args, "--mcp-config").as_deref(),
            Some("/var/armada/01AAA/mcp.json"),
            "the file that says which server, beside the flag that says only \
             that one: {args:?}"
        );
    }
}

/// Every belt a Drone can be given, for the properties no grant may change.
fn every_kind_of_toolbelt() -> [Toolbelt; 3] {
    [
        Toolbelt::evidence_only(),
        Toolbelt::evidence_only().and(Grant::ReadTheWorktree),
        Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ChangeTheWorktree)
            .and(Grant::RunADeclaredCommand(String::from("cargo test")))
            .and(Grant::DispatchAJob),
    ]
}

/// **Asked, and Armada is who is asked.** Leaving the mode off inherits the
/// operator's own default, measured as `auto` — a Drone that approves itself —
/// and `dontAsk` never consults the tool, so nothing could be held for a person.
#[test]
fn every_question_a_drone_raises_is_put_to_armada() {
    for belt in every_kind_of_toolbelt() {
        let args = rendered(belt);
        assert_eq!(
            value_after(&args, "--permission-mode").as_deref(),
            Some("default"),
            "{args:?}"
        );
        assert_eq!(
            value_after(&args, "--permission-prompt-tool").as_deref(),
            Some(permission_tool()),
            "{args:?}"
        );
    }
}

/// The tool answers for the Drone and is not one of the Drone's, so it is on
/// no allowlist however much was granted.
#[test]
fn the_permission_tool_is_on_no_allowlist() {
    for belt in every_kind_of_toolbelt() {
        let args = rendered(belt);
        let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
        assert!(
            !allowed.split(',').any(|entry| entry == permission_tool()),
            "{allowed}"
        );
    }
}

#[test]
fn armadas_own_tools_are_in_a_toolbelt_that_was_granted_nothing() {
    let args = rendered(Toolbelt::evidence_only());
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    assert_eq!(
        allowed,
        format!(
            "{},{},{},{},{}",
            evidence_tool(),
            scope_tool(),
            checks_tool(),
            widen_tool(),
            ask_tool()
        ),
        "a Drone granted nothing else still reports, still declares its scope, \
         can still ask whether its work passes, can still ask for a path the \
         task does not name and can still ask a person, because none of the \
         five is one of the grants — and a Drone denied one is denied silently"
    );
}

#[test]
fn a_grant_becomes_the_tools_it_needs_and_armadas_own_stay_first() {
    let args = rendered(
        Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ChangeTheWorktree)
            .and(Grant::RunADeclaredCommand(String::from("cargo test"))),
    );
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    let entries: Vec<&str> = allowed.split(',').collect();

    assert_eq!(entries.first(), Some(&evidence_tool()));
    assert_eq!(entries.get(1), Some(&scope_tool()));
    assert_eq!(entries.get(2), Some(&checks_tool()));
    assert_eq!(entries.get(3), Some(&widen_tool()));
    assert_eq!(entries.get(4), Some(&ask_tool()));
    assert!(entries.contains(&"Read"), "{allowed}");
    assert!(entries.contains(&"Edit"), "{allowed}");
    assert!(entries.contains(&"Bash(cargo test:*)"), "{allowed}");
}

/// **The scope-request tool is given, not granted**, for the asking tool's
/// reason and with a sharper failure. A Drone denied it does not stop needing
/// the path — it writes the file, and the first anybody hears is the drift
/// look at the gate, which is the outcome asking first exists to replace.
#[test]
fn every_toolbelt_carries_the_scope_request_tool() {
    for belt in [
        Toolbelt::evidence_only(),
        Toolbelt::evidence_only().and(Grant::ReadTheWorktree),
        Toolbelt::evidence_only().and(Grant::DispatchAJob),
    ] {
        let args = rendered(belt);
        let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
        assert!(
            allowed.split(',').any(|entry| entry == widen_tool()),
            "a Drone that cannot ask for a path writes it anyway: {allowed}"
        );
    }
}

/// **The asking tool is given, not granted**, and this is the case that says
/// so — the mirror of the dispatch pair below.
///
/// Asking costs nothing and creates nothing, so there is no spend to gate it
/// behind. And the denial is worse than the usual silence: a Drone that cannot
/// ask does not go quiet, it **guesses**, and on a step whose output is other
/// Jobs a guess is work nobody chose.
#[test]
fn every_toolbelt_carries_the_asking_tool_however_little_was_granted() {
    for belt in [
        Toolbelt::evidence_only(),
        Toolbelt::evidence_only().and(Grant::ReadTheWorktree),
        Toolbelt::evidence_only().and(Grant::DispatchAJob),
    ] {
        let args = rendered(belt);
        let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
        assert!(
            allowed.split(',').any(|entry| entry == ask_tool()),
            "a Drone that cannot ask guesses: {allowed}"
        );
    }
}

/// **The dispatch tool is granted, not given.** A Drone that was not granted it
/// has no entry for it at all, which is what makes creating Jobs a call that is
/// not on its list rather than one somebody remembered to refuse.
#[test]
fn a_toolbelt_without_the_grant_carries_no_dispatch_tool() {
    let args = rendered(
        Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ChangeTheWorktree),
    );
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    assert!(
        !allowed.contains(dispatch_tool()),
        "a Drone on an ordinary step may not create Jobs: {allowed}"
    );
}

#[test]
fn the_grant_puts_the_dispatch_tool_on_the_list() {
    let args = rendered(Toolbelt::evidence_only().and(Grant::DispatchAJob));
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    let entries: Vec<&str> = allowed.split(',').collect();
    assert!(entries.contains(&dispatch_tool()), "{allowed}");
    // Armada's own three still lead, which is the ordering the file states and
    // the thing a reader checks an argument list by eye against.
    assert_eq!(entries.first(), Some(&evidence_tool()));
}

#[test]
fn nothing_readable_is_on_the_argument_list() {
    // `ps` prints a same-uid child's arguments on darwin 27 and does not print
    // its environment, so anything here is public to every process on the
    // machine. The prompt goes in on stdin instead.
    let args = rendered(Toolbelt::evidence_only().and(Grant::ReadTheWorktree));
    for arg in &args {
        assert!(
            !arg.contains("hunter2") && !arg.contains("fix the parser"),
            "the task text reached argv: {arg}"
        );
    }
}

#[test]
fn the_launch_takes_its_directory_and_environment_from_the_config() {
    // Not from the harness. An implementation has no parameter through which it
    // could put a Drone somewhere else or hand it something else — the one
    // variable it adds is the permission wait, and that value is not its own.
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render(&config(Toolbelt::evidence_only()))
        .expect("a legal configuration renders");

    assert_eq!(launch.program(), "/usr/local/bin/agent");
    assert_eq!(launch.directory(), worktree().path());
    let given = environment();
    let (from_the_config, added) = launch.environment().vars().split_at(given.vars().len());
    assert_eq!(from_the_config, given.vars());
    assert_eq!(added.len(), 1, "one variable past the config's: {added:?}");
}

/// Over HTTP the CLI abandons a tool call after about a minute unless told
/// otherwise, and after five minutes whatever it is told — measured, spike 15.
#[test]
fn the_drone_waits_on_a_permission_answer_longer_than_fleet_holds_one() {
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render(&config(Toolbelt::evidence_only()))
        .expect("a legal configuration renders");

    let wait = launch
        .environment()
        .vars()
        .iter()
        .find(|(name, _)| name == "MCP_TOOL_TIMEOUT")
        .map(|(_, value)| value.clone());
    assert_eq!(
        wait,
        Some(adapter_traits::PERMISSION_WAIT.as_millis().to_string())
    );
    assert_eq!(wait.as_deref(), Some("300000"), "five minutes");
}

/// Which of two values wins is a rule nobody would find, so a config that
/// already names the wait is refused at render rather than overwritten.
#[test]
fn a_config_that_already_names_the_wait_is_refused() {
    let already = Environment::nothing()
        .and("MCP_TOOL_TIMEOUT", "1")
        .expect("a legal name");
    let config = DroneSpawnConfig::spawn_in(
        &worktree(),
        Model::named("a-model").expect("a named model"),
        Prompt::assembled(SECRET_LOOKING_TASK).expect("an assembled prompt"),
        McpConfig::only_these("/var/armada/01AAA/mcp.json").expect("an absolute path"),
        Toolbelt::evidence_only(),
        already,
    );

    let refused = HeadlessAgent::at("/usr/local/bin/agent").render(&config);
    assert!(
        matches!(refused, Err(HarnessRefused::PermissionWaitNotSet(_))),
        "{refused:?}"
    );
}

#[test]
fn no_credential_is_anywhere_in_a_rendered_drone() {
    // The other half of "a Drone cannot push". Nothing on either channel
    // authenticates it to anything.
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render(&config(Toolbelt::evidence_only()))
        .expect("a legal configuration renders");

    for name in launch.environment().names() {
        assert!(
            !["SSH_AUTH_SOCK", "GITHUB_TOKEN", "GIT_ASKPASS", "GH_TOKEN"].contains(&name),
            "a credential-bearing variable reached a Drone: {name}"
        );
    }
}

#[test]
fn a_command_is_grantable_exactly_when_it_would_render() {
    let harness = HeadlessAgent::at("/usr/local/bin/agent");
    assert!(harness.grantable("npm publish --access public").is_ok());
    assert!(matches!(
        harness.grantable("git push origin HEAD"),
        Err(HarnessRefused::CommandWouldPush { .. })
    ));
    assert!(matches!(
        harness.grantable("echo (a, b)"),
        Err(HarnessRefused::CommandNotExpressibleAsARule { .. })
    ));
}

#[test]
fn a_declared_command_that_would_push_is_refused_by_name() {
    for run in [
        "git push",
        "git push --force origin main",
        "/usr/bin/git push",
    ] {
        let refused = HeadlessAgent::at("/usr/local/bin/agent").render(&config(
            Toolbelt::evidence_only().and(Grant::RunADeclaredCommand(String::from(run))),
        ));
        assert!(
            matches!(refused, Err(HarnessRefused::CommandWouldPush { .. })),
            "`{run}` was not refused: {refused:?}"
        );
    }
}

#[test]
fn a_command_that_only_looks_like_a_push_is_not_refused() {
    // The control. A refusal that fired on the word alone would deny
    // `cargo test --features push` and the Job would go quiet.
    for run in [
        "cargo test --features push",
        "make push-image",
        "npm run push",
    ] {
        let rendered = HeadlessAgent::at("/usr/local/bin/agent").render(&config(
            Toolbelt::evidence_only().and(Grant::RunADeclaredCommand(String::from(run))),
        ));
        assert!(rendered.is_ok(), "`{run}` was refused and should not be");
    }
}

#[test]
fn a_command_that_would_break_the_rule_is_refused_rather_than_rendered() {
    // A malformed rule does not fail — it allows nothing, the Drone is denied a
    // command it was told it had, and the Job goes quiet looking like a prompt
    // problem. Refusing at render is what makes that failure loud.
    let refused =
        HeadlessAgent::at("/usr/local/bin/agent").render(&config(Toolbelt::evidence_only().and(
            Grant::RunADeclaredCommand(String::from("sh -c (cargo test)")),
        )));
    assert!(
        matches!(
            refused,
            Err(HarnessRefused::CommandNotExpressibleAsARule { .. })
        ),
        "{refused:?}"
    );
}

/// **A command a person allowed is spelled as a declared one, and refused as
/// one.** A push allowed at a question is still not a push a Drone can make.
#[test]
fn a_command_a_person_allowed_renders_as_a_declared_one_and_a_push_is_still_refused() {
    let declared = rendered(
        Toolbelt::evidence_only().and(Grant::RunADeclaredCommand(String::from("npm install"))),
    );
    let allowed = rendered(
        Toolbelt::evidence_only().and(Grant::RunAnAllowedCommand(String::from("npm install"))),
    );
    assert_eq!(allowed, declared);
    let list = value_after(&allowed, "--allowedTools").expect("an allowlist is rendered");
    assert!(
        list.split(',').any(|entry| entry == "Bash(npm install:*)"),
        "{list}"
    );

    let refused =
        HeadlessAgent::at("/usr/local/bin/agent").render(&config(Toolbelt::evidence_only().and(
            Grant::RunAnAllowedCommand(String::from("git push origin main")),
        )));
    assert!(
        matches!(refused, Err(HarnessRefused::CommandWouldPush { .. })),
        "{refused:?}"
    );
}

#[test]
fn a_relative_mcp_path_is_refused_where_it_is_written() {
    // It would resolve against the Drone's own working directory, which is the
    // worktree — so the file Fleet wrote would not be the file the Drone read,
    // and the Drone would come up with no Evidence tool and no way to say so.
    assert!(McpConfig::only_these("mcp.json").is_err());
    assert!(McpConfig::only_these("").is_err());
}

#[test]
fn two_drones_render_the_same_way_from_the_same_configuration() {
    let first = rendered(Toolbelt::evidence_only().and(Grant::ReadTheWorktree));
    let second = rendered(Toolbelt::evidence_only().and(Grant::ReadTheWorktree));
    assert_eq!(first, second);
}
