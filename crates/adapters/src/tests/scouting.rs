//! A scout's process, asserted on the rendering, and the checkout reading it
//! records. **The launch starts nothing**, for `harness`'s reason; the checkout
//! is a real repository, for `repo`'s.

use adapter_traits::{CallDetail, DroneEvent, Environment, McpConfig, Model};

use super::harness::value_after;
use super::repo::TempRepo;
use crate::conversing::door_tools;
use crate::harness::HeadlessAgent;
use crate::scouting::{checkout_as_it_stands, denied_to_a_scout, Looked, Scouting};

fn scouting() -> Scouting {
    Scouting::in_checkout(
        "/repos/armada/",
        Model::named("a-model").expect("a named model"),
        McpConfig::only_these("/var/armada/scout-mcp.json").expect("an absolute path"),
        Environment::nothing()
            .and("PATH", "/usr/bin:/bin")
            .expect("a legal name"),
    )
    .expect("an absolute checkout")
}

fn launch() -> adapter_traits::Launch {
    HeadlessAgent::at("/usr/local/bin/agent")
        .render_scout(&scouting())
        .expect("renders")
}

/// `#1292`'s watch-for: **the rendered launch carries no write tool.** Not in
/// the toolset, not in the allowlist, and every tool the CLI offers that
/// edits, runs a command, spawns an agent or reaches the network denied by
/// name — so an operator's own allow of one does not reach a scout.
#[test]
fn a_scouts_launch_allows_only_reads_and_denies_every_write() {
    let launch = launch();
    let args = launch.args();
    let reads = Some("Read,Grep,Glob".to_string());
    assert_eq!(value_after(args, "--tools"), reads);
    assert_eq!(value_after(args, "--allowedTools"), reads);

    let denied = value_after(args, "--disallowedTools").expect("a deny list");
    let denied: Vec<&str> = denied.split(',').collect();
    for writes in [
        "Bash",
        "Edit",
        "Write",
        "NotebookEdit",
        "Task",
        "WebFetch",
        "WebSearch",
        "EnterWorktree",
        "RemoteTrigger",
        "SendMessage",
        "PushNotification",
    ] {
        assert!(
            denied.contains(&writes),
            "`{writes}` is not denied: {denied:?}"
        );
    }
    assert_eq!(denied, denied_to_a_scout());
    for read in ["Read", "Grep", "Glob"] {
        assert!(!denied.contains(&read), "a scout reads: {read}");
    }
    assert!(
        !args.iter().any(|arg| arg.contains(&door_tools())),
        "a scout is not given Fleet's door: {args:?}"
    );
}

/// **Confined to one checkout, holding no server, asked nothing.**
#[test]
fn a_scout_runs_in_its_checkout_restricted_with_no_server_and_no_prompt_tool() {
    let launch = launch();
    let args = launch.args();
    assert_eq!(launch.directory(), "/repos/armada");
    assert!(args.iter().any(|arg| arg == "--restricted"));
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    assert_eq!(
        value_after(args, "--mcp-config").as_deref(),
        Some("/var/armada/scout-mcp.json")
    );
    assert_eq!(
        value_after(args, "--permission-mode").as_deref(),
        Some("dontAsk")
    );
    assert!(!args.iter().any(|arg| arg == "--permission-prompt-tool"));
    assert!(!args.iter().any(|arg| arg == "--resume"));
    assert!(args.iter().any(|arg| arg == "--no-session-persistence"));
}

#[test]
fn a_relative_checkout_is_refused() {
    let refused = Scouting::in_checkout(
        "repos/armada",
        Model::named("a-model").expect("a named model"),
        McpConfig::only_these("/var/armada/scout-mcp.json").expect("an absolute path"),
        Environment::nothing(),
    );
    assert!(refused.is_err());
}

fn called(tool: &str, detail: &str) -> DroneEvent {
    DroneEvent::Called {
        tool: tool.to_string(),
        call: "toolu_1".to_string(),
        detail: CallDetail::of(detail),
    }
}

/// What a Finding lists: **a file read, relative to the checkout, and a
/// search as its pattern and where**. A call that is not a read is not listed.
#[test]
fn a_read_and_a_search_are_named_relative_to_the_checkout() {
    let agent = HeadlessAgent::at("/usr/local/bin/agent");
    let looked = |event: DroneEvent| agent.scout_looked(&event, "/repos/armada");
    assert_eq!(
        looked(called("Read", "/repos/armada/crates/fleet/src/routing.rs")),
        Some((
            "toolu_1".to_string(),
            Looked::File("crates/fleet/src/routing.rs".to_string())
        ))
    );
    assert_eq!(
        looked(called("Grep", "weight in /repos/armada/crates")),
        Some((
            "toolu_1".to_string(),
            Looked::Search("weight in crates".to_string())
        ))
    );
    assert_eq!(
        looked(called("Read", "/elsewhere/notes.txt")),
        Some((
            "toolu_1".to_string(),
            Looked::File("/elsewhere/notes.txt".to_string())
        )),
        "a path outside is kept whole, so a Finding never passes it off as the repository's"
    );
    assert_eq!(looked(called("Bash", "cat routing.rs")), None);
}

/// **Untracked files are uncommitted**, since a scout reads them.
#[test]
fn a_checkout_is_read_as_its_commit_and_whether_anything_is_uncommitted() {
    let repo = TempRepo::with_a_commit();
    let clean = checkout_as_it_stands(&repo.root_str()).expect("a repository");
    assert_eq!(clean.commit, repo.head_str());
    assert!(!clean.uncommitted);

    repo.write("scratch.txt", "a note nobody committed");
    let dirty = checkout_as_it_stands(&repo.root_str()).expect("a repository");
    assert_eq!(dirty.commit, repo.head_str());
    assert!(dirty.uncommitted);

    assert!(checkout_as_it_stands("/nowhere/at/all").is_err());
}
