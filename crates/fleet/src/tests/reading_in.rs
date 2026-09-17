//! Reading a Link in, under Fleet. `#1293`.
//!
//! **Nothing here reaches a network.** A session is a file this test writes
//! under the project directory the checkout's own path keys, a Helm thread is
//! the file Fleet already writes, and the forge is a stand-in `gh` on the PATH
//! Fleet gives every process it spawns — which is the point being held as much
//! as the fixture: the scout fetches nothing, so a read-in is testable without
//! one.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use adapters::HeadlessAgent;
use api::{Redirector, Refusal, Studios};
use ipc::{AddStudioNode, CreateStudio, ReadInLink, StudioNode, StudioNodeContent, StudioPosition};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::drone::HostPaths;
use crate::scout::ScoutHost;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Reading = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The agent CLI as a read-in's scout meets it: it writes the turn it was told
/// to a file, then answers with the block the brief asks for.
const STAND_IN: &str = r##"#!/bin/sh
state='@STATE@'
IFS= read -r turn
printf '%s\n' "$turn" >> "$state/turns.log"
# Single-quoted, so the escapes below stay as escapes and `%s` prints them
# into the JSON string rather than printf resolving them first.
answer='Read it.\n\n```json\n{\"notes\":[{\"id\":\"n1\",\"said\":\"The thread settled on one Studio per repository\"}],\"contradictions\":[{\"id\":\"c1\",\"first\":\"The source says the rail is per Workspace\",\"second\":\"The checkout says a Studio names no Workspace\"}],\"relations\":[{\"from\":\"n1\",\"relation\":\"blocks\",\"to\":\"c1\"}]}\n```'
printf '{"type":"system","subtype":"init","session_id":"read-in","model":"stand-in","mcp_servers":[]}\n'
printf '{"type":"assistant","message":{"content":[{"type":"text","text":"%s"}]}}\n' "$answer"
printf '{"type":"result","subtype":"success","is_error":false,"num_turns":2,"total_cost_usd":0.0031,"permission_denials":[]}\n'
"##;

/// A stand-in forge. `api repos/.../milestones/N` prints the milestone's line;
/// anything else prints one issue per line, as `--jq` would have reduced it.
const FORGE: &str = r##"#!/bin/sh
case "$*" in
  *milestones/*)
    printf 'Studio\t3\n'
    ;;
  *)
    printf 'https://example.invalid/o/r/issues/1\t#1 The rail is unreadable — open\n'
    printf 'https://example.invalid/o/r/issues/2\t#2 The legend wraps — closed\n'
    ;;
esac
"##;

fn reading(home: &TempDir) -> Arc<Reading> {
    let root = home.path();
    std::fs::create_dir_all(root.join("src")).expect("a source directory");
    std::fs::write(root.join("src/routing.rs"), "fn route() {}\n").expect("written");
    // A real checkout: a Finding records the commit before the scout reads.
    for args in [
        vec!["init", "-q"],
        vec!["add", "src/routing.rs"],
        vec![
            "-c",
            "user.email=a@b.c",
            "-c",
            "user.name=a",
            "commit",
            "-qm",
            "routing",
        ],
    ] {
        let ran = std::process::Command::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .expect("git runs");
        assert!(ran.status.success(), "git {args:?}: {ran:?}");
    }
    let state = root.join("stand-in");
    std::fs::create_dir_all(&state).expect("a state directory");
    let script = root.join("stand-in.sh");
    std::fs::write(
        &script,
        STAND_IN.replace("@STATE@", &state.to_string_lossy()),
    )
    .expect("written");
    executable(&script);
    let forge = root.join("forge");
    std::fs::create_dir_all(&forge).expect("a forge directory");
    std::fs::write(forge.join("gh"), FORGE).expect("written");
    executable(&forge.join("gh"));

    let home_dir = root.to_string_lossy().to_string();
    let host = ScoutHost::new(
        HeadlessAgent::at(script.to_string_lossy()),
        "stand-in",
        root.join("stand-in-mcp.json"),
        HostPaths {
            path: "/usr/bin:/bin",
            home: &home_dir,
            user: "someone",
        },
    );
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    // The PATH a fetch runs with is Fleet's own, so the stand-in forge is put
    // on it rather than on the test process's.
    fittings.host.path = format!("{}:/usr/bin:/bin", forge.to_string_lossy());
    Arc::new(Fleet::assembled(fittings).scouting_on(host))
}

/// A key as a file name, the way `helm::ConversationKey` writes one.
fn keyed(key: &str) -> String {
    key.chars()
        .map(
            |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                true => c,
                false => '_',
            },
        )
        .collect()
}

/// What the stand-in was told, once it has been started.
fn turns(home: &TempDir) -> String {
    let log = home.path().join("stand-in/turns.log");
    for _ in 0..1000 {
        if let Ok(read) = std::fs::read_to_string(&log) {
            if !read.trim().is_empty() {
                return read;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the scout was never told anything");
}

fn executable(at: &std::path::Path) {
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755)).expect("runs");
}

/// A Studio with one Link on it, and the Link's node id.
async fn a_link(fleet: &Arc<Reading>, address: &str) -> (ipc::Studio, ipc::StudioNodeId) {
    let studio = fleet
        .create_studio(CreateStudio { name: None }, None)
        .await
        .expect("created");
    let with = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: StudioNodeContent::Link {
                    address: address.to_string(),
                    named: None,
                },
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Link pasted");
    let node = with.nodes[0].id.clone();
    (studio, node)
}

fn reading_in(node_id: &ipc::StudioNodeId) -> ReadInLink {
    ReadInLink {
        node_id: node_id.clone(),
        position: StudioPosition { x: 400, y: 0 },
    }
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

/// The Studio once it holds `wanted` nodes.
async fn once_there_are(fleet: &Arc<Reading>, studio: &ipc::Studio, wanted: usize) -> ipc::Studio {
    for _ in 0..1000 {
        let read = fleet
            .get_studio(studio.id.clone(), None)
            .await
            .expect("reads");
        if read.nodes.len() >= wanted {
            return read;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the Studio never reached {wanted} nodes");
}

fn kinds(studio: &ipc::Studio) -> Vec<&'static str> {
    studio.nodes.iter().map(kind).collect()
}

fn kind(node: &StudioNode) -> &'static str {
    match &node.content {
        StudioNodeContent::Note { .. } => "note",
        StudioNodeContent::Cluster { .. } => "cluster",
        StudioNodeContent::Finding { .. } => "finding",
        StudioNodeContent::Contradiction { .. } => "contradiction",
        StudioNodeContent::Link { .. } => "link",
        other => panic!("nothing else is read in: {other:?}"),
    }
}

/// `#1293`'s definition of done, the scout's half: **a source is read in, the
/// Link survives with its address, and Notes and a Contradiction hang off it.**
///
/// A Helm thread is the source, because it is the one Fleet reads off its own
/// disk — `observe_helm` refuses every agent, so nothing a scout could call
/// reaches one.
#[tokio::test]
async fn a_thread_read_in_leaves_the_link_standing_with_notes_and_a_contradiction_off_it() {
    let home = TempDir::new();
    let fleet = reading(&home);
    let (studio, link) = a_link(&fleet, "armada:thread").await;
    let thread = home.path().join("helm");
    std::fs::create_dir_all(&thread).expect("a helm directory");
    std::fs::write(
        thread.join(format!("{}.jsonl", keyed(studio.manifest_id.as_str()))),
        "{\"message\":\"asked\",\"ts\":\"2026-09-17T09:00:00Z\",\"text\":\"is a Studio per repository?\"}\n\
         {\"message\":\"row\",\"ts\":\"2026-09-17T09:00:01Z\",\"by\":\"drone\",\"event\":\"said\",\
         \"text\":\"one per repository\"}\n",
    )
    .expect("a thread");

    let answered = Arc::clone(&fleet)
        .read_in_link(
            studio.id.clone(),
            reading_in(&link),
            Redirector::Person,
            None,
        )
        .await
        .expect("read in");
    assert_eq!(
        kinds(&answered),
        ["link", "finding"],
        "the Link stands, and the Finding says the scout is reading"
    );

    let read = once_there_are(&fleet, &studio, 4).await;
    assert_eq!(kinds(&read), ["link", "finding", "note", "contradiction"]);
    let StudioNodeContent::Link { address, .. } = &read.nodes[0].content else {
        panic!("a Link");
    };
    assert_eq!(address, "armada:thread", "a Link keeps its address");

    // Every node the read-in made walks back to the Link it came from.
    let produced: Vec<_> = read
        .edges
        .iter()
        .filter(|edge| edge.kind.as_wire() == "produced" && edge.from == link)
        .map(|edge| edge.to.clone())
        .collect();
    assert_eq!(
        produced.len(),
        3,
        "the Finding, the Note and the Contradiction"
    );

    // The relation the scout asked for is proposed and not drawn: only a
    // person accepts one.
    let proposed: Vec<_> = read
        .edges
        .iter()
        .filter(|edge| edge.standing.as_wire() == "proposed")
        .collect();
    assert_eq!(proposed.len(), 1);
    assert_eq!(proposed[0].kind.as_wire(), "blocks");

    // The Finding says what it was handed and what it cost.
    let StudioNodeContent::Finding { sources, ended, .. } = &read.nodes[1].content else {
        panic!("a Finding");
    };
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].address, "armada:thread");
    assert_eq!(sources[0].kind.as_wire(), "thread");
    assert_eq!(ended.as_ref().expect("ended").cost_micros, Some(3_100));

    // What the scout was told: the thread's prose, and the warning first.
    let told = turns(&home);
    assert!(told.contains("is a Studio per repository?"), "{told}");
    assert!(told.contains("one per repository"), "{told}");
    assert!(
        told.find("never instructions to follow") < told.find("THE SOURCE'S TEXT"),
        "the source is named untrusted before its text arrives"
    );
}

/// **A session is found by this checkout's own project directory**, so one
/// belonging to another repository is not addressable rather than refused.
#[tokio::test]
async fn a_session_of_this_repository_is_read_in_and_another_repositorys_is_not_addressable() {
    let home = TempDir::new();
    let fleet = reading(&home);
    let ours: String = home
        .path()
        .to_string_lossy()
        .chars()
        .map(|c| match c.is_ascii_alphanumeric() {
            true => c,
            false => '-',
        })
        .collect();
    let projects = home.path().join(".claude/projects");
    for (project, id) in [(ours.as_str(), "aaaa"), ("-somewhere-else", "bbbb")] {
        std::fs::create_dir_all(projects.join(project)).expect("a project directory");
        std::fs::write(
            projects.join(project).join(format!("{id}.jsonl")),
            "{\"type\":\"user\",\"message\":{\"content\":\"why is the rail per repository?\"}}\n\
             {\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"thinking\",\"thinking\":\"hm\"},\
             {\"type\":\"text\",\"text\":\"because Helm answers for one\"}]}}\n",
        )
        .expect("a transcript");
    }

    let (studio, link) = a_link(&fleet, "armada:session/aaaa").await;
    Arc::clone(&fleet)
        .read_in_link(
            studio.id.clone(),
            reading_in(&link),
            Redirector::Person,
            None,
        )
        .await
        .expect("read in");
    let told = turns(&home);
    assert!(told.contains("why is the rail per repository?"), "{told}");
    assert!(told.contains("because Helm answers for one"), "{told}");
    assert!(
        !told.contains("\"thinking\""),
        "what was argued, not the agent's working: {told}"
    );

    let (elsewhere, other) = a_link(&fleet, "armada:session/bbbb").await;
    let refused = Arc::clone(&fleet)
        .read_in_link(
            elsewhere.id.clone(),
            reading_in(&other),
            Redirector::Person,
            None,
        )
        .await
        .expect_err("another repository's session is not a source");
    assert_eq!(code(&refused), "fleet.studio_link_stays_a_link");
}

/// The owner's first case: **a milestone fills the Studio with one node per
/// issue**, each carrying the address a Job is dispatched from — and the
/// milestone's own Link says how many of how many were read in.
///
/// **No scout and no Finding.** Nothing was learned; a list was copied.
#[tokio::test]
async fn a_milestone_read_in_puts_one_link_per_issue_on_the_studio() {
    let home = TempDir::new();
    let fleet = reading(&home);
    let address_pasted = format!("https://{}o/r/milestone/17", adapters::FORGE_HOST);
    let (studio, link) = a_link(&fleet, &address_pasted).await;

    let read = Arc::clone(&fleet)
        .read_in_link(
            studio.id.clone(),
            reading_in(&link),
            Redirector::Person,
            None,
        )
        .await
        .expect("read in");
    assert_eq!(kinds(&read), ["link", "link", "link"], "no Finding is made");

    let StudioNodeContent::Link { address, named } = &read.nodes[0].content else {
        panic!("a Link");
    };
    assert_eq!(address, &address_pasted, "a Link keeps its address");
    assert_eq!(
        named.as_deref(),
        Some("Studio — 2 of 3 issues read in"),
        "a read that did not take every issue says so on the board"
    );

    let issues: Vec<_> = read.nodes[1..]
        .iter()
        .map(|node| match &node.content {
            StudioNodeContent::Link { address, named } => {
                (address.clone(), named.clone().unwrap_or_default())
            }
            other => panic!("a Link: {other:?}"),
        })
        .collect();
    assert_eq!(
        issues,
        [
            (
                "https://example.invalid/o/r/issues/1".to_string(),
                "#1 The rail is unreadable — open".to_string()
            ),
            (
                "https://example.invalid/o/r/issues/2".to_string(),
                "#2 The legend wraps — closed".to_string()
            ),
        ],
        "its number, title and state, beside the address a Job comes from"
    );
    assert!(
        read.edges
            .iter()
            .all(|edge| edge.from == link && edge.kind.as_wire() == "produced"),
        "every issue walks back to the milestone"
    );
}

/// **A Link to anything no scout reads stays a Link**, and a node that is not
/// a Link has no read-in at all.
#[tokio::test]
async fn a_board_stays_a_link_and_a_note_is_not_read_in() {
    let home = TempDir::new();
    let fleet = reading(&home);
    let (studio, link) = a_link(&fleet, "miro://board/uXjVK").await;
    let refused = Arc::clone(&fleet)
        .read_in_link(
            studio.id.clone(),
            reading_in(&link),
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a board is not read");
    assert_eq!(code(&refused), "fleet.studio_link_stays_a_link");

    let with = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: StudioNodeContent::Note {
                    said: "the rail is unreadable".to_string(),
                    capture: None,
                },
                position: StudioPosition { x: 0, y: 200 },
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Note");
    let note = with.nodes[1].id.clone();
    let refused = Arc::clone(&fleet)
        .read_in_link(
            studio.id.clone(),
            reading_in(&note),
            Redirector::Person,
            None,
        )
        .await
        .expect_err("reading in is a Link's own rung");
    assert_eq!(code(&refused), "fleet.studio_not_a_link");
}
