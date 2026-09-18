//! A Helm conversation's process, asserted on the rendering. **Nothing here
//! starts anything**, for `harness`'s reason.

use adapter_traits::{Environment, McpConfig, Model};

use super::harness::value_after;
use crate::conversing::{door_tools, wrote_the_checkout, ConversationRefused, Conversing};
use crate::harness::HeadlessAgent;

fn environment() -> Environment {
    Environment::nothing()
        .and("PATH", "/usr/bin:/bin")
        .expect("a legal name")
        .and("HOME", "/Users/user")
        .expect("a legal name")
}

fn fresh() -> Conversing {
    Conversing::in_repository(
        "/repos/armada",
        Model::named("a-model").expect("a named model"),
        McpConfig::only_these("/var/armada/helm-mcp.json").expect("an absolute path"),
        environment(),
    )
    .expect("an absolute root")
}

fn rendered(conversing: &Conversing) -> adapter_traits::Launch {
    HeadlessAgent::at("/usr/local/bin/agent")
        .render_conversation(conversing)
        .expect("renders")
}

#[test]
fn a_new_conversation_opens_in_the_repository_with_the_door_beside_what_a_person_has() {
    let launch = rendered(&fresh());
    let args = launch.args();

    assert_eq!(launch.directory(), "/repos/armada");
    assert_eq!(launch.environment(), &environment());
    assert_eq!(
        value_after(args, "--mcp-config").as_deref(),
        Some("/var/armada/helm-mcp.json")
    );
    assert_eq!(door_tools(), "mcp__armada-fleet");
    assert!(!args.iter().any(|arg| arg == "--resume"));
}

/// `#1373`: each of these withholds something a person has in a terminal, and
/// spike 018 measured what. A conversation passes none of them.
#[test]
fn nothing_narrows_what_a_conversation_resolves() {
    let args = rendered(&fresh()).args().to_vec();
    for withheld in [
        "--strict-mcp-config",
        "--tools",
        "--allowedTools",
        "--disallowedTools",
        "--restricted",
    ] {
        assert!(
            !args.iter().any(|arg| arg == withheld),
            "{withheld} is not on a conversation: {args:?}"
        );
    }
    assert_eq!(
        value_after(&args, "--permission-mode").as_deref(),
        Some("default"),
        "the person's own settings decide, which is what a terminal session runs"
    );
}

/// `#1389`: **the mode alone would refuse everything the settings do not
/// cover.** What makes `default` the terminal's behaviour rather than a
/// narrowing is the tool the uncovered call is put to, and it is the door's own
/// — spike 19.
#[test]
fn every_call_the_settings_do_not_cover_is_put_to_the_person_at_the_door() {
    let args = rendered(&fresh()).args().to_vec();
    assert_eq!(
        value_after(&args, "--permission-prompt-tool").as_deref(),
        Some("mcp__armada-fleet__ask_the_person")
    );
    assert!(
        !args.iter().any(|arg| arg == "--permission-prompts"),
        "`host` is the default, and naming it would be a second spelling of it"
    );
}

/// `#1389`: auto mode is what the owner asked for and is not reachable for a
/// spawned session — spike 19 measured it reporting `default` on `init` with no
/// classifier call. **A value the CLI accepts and ignores is worse than one it
/// refuses**, so this holds the rendering to what was measured.
#[test]
fn a_conversation_does_not_ask_for_a_mode_a_spawned_session_cannot_have() {
    let args = rendered(&fresh()).args().to_vec();
    for unreachable in ["auto", "manual"] {
        assert!(
            !args.iter().any(|arg| arg == unreachable),
            "{unreachable} is not in force for a `-p` session"
        );
    }
}

/// `#1373`: **widening Helm must not widen a Drone.** Rendered side by side,
/// because the two are the same file's argument and the danger is that a flag
/// taken off one comes off both.
#[test]
fn a_drones_launch_is_untouched_by_what_a_conversation_resolves() {
    let drone = super::harness::rendered(adapter_traits::Toolbelt::evidence_only());
    assert!(drone.iter().any(|arg| arg == "--strict-mcp-config"));
    assert_eq!(
        value_after(&drone, "--permission-mode").as_deref(),
        Some("default")
    );
    assert_eq!(
        value_after(&drone, "--permission-prompt-tool"),
        Some(crate::harness::permission_tool().to_string())
    );
    assert!(drone.iter().any(|arg| arg == "--allowedTools"));
    assert!(drone.iter().any(|arg| arg == "--disallowedTools"));

    let helm = rendered(&fresh()).args().to_vec();
    assert!(!helm.iter().any(|arg| arg == "--strict-mcp-config"));
}

/// A write is recognised so it can be recorded, and `Bash` is deliberately not
/// one: nothing in the stream says whether a shell line wrote a file.
#[test]
fn the_tools_that_change_a_checkout_are_named_and_bash_is_not_among_them() {
    for tool in ["Edit", "Write", "NotebookEdit"] {
        assert!(wrote_the_checkout(tool), "{tool} changes the checkout");
    }
    for tool in ["Bash", "Read", "Grep", "Glob", "WebFetch"] {
        assert!(!wrote_the_checkout(tool), "{tool} is not a write");
    }
}

/// The detail a write call leaves on a row is the path and how much moved, and
/// the event carries the path alone.
#[test]
fn the_file_a_write_named_is_read_back_out_of_its_row() {
    for (detail, path) in [
        (
            "~/armada/crates/api/src/lib.rs +3 -1",
            "~/armada/crates/api/src/lib.rs",
        ),
        ("~/armada/README.md +12", "~/armada/README.md"),
        ("~/armada/README.md", "~/armada/README.md"),
        ("a name with spaces.md +1 -0", "a name with spaces.md"),
        // Not a size: left alone rather than guessed at.
        ("notes -draft.md", "notes -draft.md"),
    ] {
        assert_eq!(crate::conversing::path_written(detail), path);
    }
}

#[test]
fn a_later_message_resumes_the_session_by_id() {
    let resuming = fresh()
        .resuming("4e0fb413-aaaa-bbbb")
        .expect("a portable id");
    let args = rendered(&resuming).args().to_vec();
    assert_eq!(
        value_after(&args, "--resume").as_deref(),
        Some("4e0fb413-aaaa-bbbb")
    );
    assert_eq!(args.iter().filter(|arg| *arg == "--resume").count(), 1);
}

#[test]
fn an_id_that_argv_would_misread_is_refused() {
    for given in ["", "--model", "an id", "id;rm"] {
        assert_eq!(
            fresh().resuming(given),
            Err(ConversationRefused::SessionNotPortable {
                given: given.to_string()
            })
        );
    }
}

#[test]
fn a_relative_root_is_refused() {
    let refused = Conversing::in_repository(
        "repos/armada",
        Model::named("a-model").expect("a named model"),
        McpConfig::only_these("/var/armada/helm-mcp.json").expect("an absolute path"),
        environment(),
    );
    assert!(matches!(
        refused,
        Err(ConversationRefused::DirectoryNotAbsolute { .. })
    ));
}
