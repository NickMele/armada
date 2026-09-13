//! A Helm conversation's process, asserted on the rendering. **Nothing here
//! starts anything**, for `harness`'s reason.

use adapter_traits::{Environment, McpConfig, Model};

use super::harness::value_after;
use crate::conversing::{door_tools, ConversationRefused, Conversing};
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
fn a_new_conversation_opens_in_the_repository_holding_only_the_door() {
    let launch = rendered(&fresh());
    let args = launch.args();

    assert_eq!(launch.directory(), "/repos/armada");
    assert_eq!(launch.environment(), &environment());
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    assert_eq!(
        value_after(args, "--mcp-config").as_deref(),
        Some("/var/armada/helm-mcp.json")
    );
    assert_eq!(value_after(args, "--allowedTools"), Some(door_tools()));
    assert_eq!(door_tools(), "mcp__armada-fleet");
    assert_eq!(
        value_after(args, "--permission-mode").as_deref(),
        Some("dontAsk")
    );
    assert!(!args.iter().any(|arg| arg == "--resume"));
    assert!(!args.iter().any(|arg| arg == "--permission-prompt-tool"));
}

#[test]
fn nothing_that_writes_the_checkout_is_callable() {
    let args = rendered(&fresh()).args().to_vec();
    let denied = value_after(&args, "--disallowedTools").expect("a deny list");
    for tool in ["Bash", "Edit", "Write", "NotebookEdit"] {
        assert!(
            denied.split(',').any(|named| named == tool),
            "{tool} denied"
        );
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
