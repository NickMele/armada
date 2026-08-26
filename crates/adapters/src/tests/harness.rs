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

use crate::harness::{evidence_tool, HarnessRefused, HeadlessAgent};

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

#[test]
fn the_drone_is_never_asked_to_confirm_anything() {
    // Leaving the mode off does not mean "no mode": it inherits the operator's
    // own default, measured as `auto` — a Drone that approves itself.
    let args = rendered(Toolbelt::evidence_only());
    assert_eq!(
        value_after(&args, "--permission-mode").as_deref(),
        Some("dontAsk")
    );
}

#[test]
fn the_evidence_tool_is_in_a_toolbelt_that_was_granted_nothing() {
    let args = rendered(Toolbelt::evidence_only());
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    assert_eq!(
        allowed,
        evidence_tool(),
        "a Drone granted nothing else still reports, because the tool is not \
         one of the grants"
    );
}

#[test]
fn a_grant_becomes_the_tools_it_needs_and_the_evidence_tool_stays_first() {
    let args = rendered(
        Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ChangeTheWorktree)
            .and(Grant::RunADeclaredCommand(String::from("cargo test"))),
    );
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    let entries: Vec<&str> = allowed.split(',').collect();

    assert_eq!(entries.first(), Some(&evidence_tool()));
    assert!(entries.contains(&"Read"), "{allowed}");
    assert!(entries.contains(&"Edit"), "{allowed}");
    assert!(entries.contains(&"Bash(cargo test:*)"), "{allowed}");
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
    // could put a Drone somewhere else or hand it something else.
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render(&config(Toolbelt::evidence_only()))
        .expect("a legal configuration renders");

    assert_eq!(launch.program(), "/usr/local/bin/agent");
    assert_eq!(launch.directory(), worktree().path());
    assert_eq!(launch.environment(), &environment());
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
