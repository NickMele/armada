//! A scout under Fleet, against a stand-in agent: asked from a Studio, it
//! reads, its Finding freezes with every file it read and the commit; a second
//! is stopped from its node and ends with its cost; a restart settles one left
//! gathering. `#1292`.
//!
//! **The stand-in runs through the real renderer, the real reader and the real
//! process group.** Its stop is the CLI's measured answer to an interrupt —
//! spike 017 — so a stand-in that ignored the signal would be the timeout case,
//! not this one. The checkout is a real repository, since the commit and the
//! uncommitted changes are git's to say.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use adapters::HeadlessAgent;
use api::{Redirector, Refusal, Studios};
use ipc::{
    AddStudioNode, AskScout, CreateStudio, ScoutOutcome, StartScout, StopScout, Studio, StudioNode,
    StudioNodeContent, StudioPosition,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::drone::HostPaths;
use crate::scout::ScoutHost;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Scouted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The agent CLI as a scout meets it. An ask naming `slowly` reads one file and
/// then waits to be interrupted, answering the interrupt with the turn's end
/// and its cost; any other reads two files, has a third refused as outside the
/// checkout, searches the contents of one file and lists names, and answers.
const STAND_IN: &str = r##"#!/bin/sh
state='@STATE@'
root='@ROOT@'
printf '%s\n' "$*" >> "$state/argv.log"
IFS= read -r turn
printf '%s\n' "$turn" >> "$state/turns.log"
call() {
  printf '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"%s","name":"%s","input":{%s}}]}}\n' "$1" "$2" "$3"
  printf '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"%s","is_error":%s,"content":"%s"}]}}\n' "$1" "$4" "$5"
}
said() {
  printf '{"type":"assistant","message":{"content":[{"type":"text","text":"%s"}]}}\n' "$1"
}
ended() {
  printf '{"type":"result","subtype":"success","is_error":false,"num_turns":%s,"total_cost_usd":%s,"permission_denials":[]}\n' "$1" "$2"
}
printf '{"type":"system","subtype":"init","session_id":"scout-1","model":"stand-in","mcp_servers":[]}\n'
case "$turn" in
  *slowly*)
    call toolu_1 Read "\"file_path\":\"$root/src/routing.rs\"" false
    trap 'kill $! 2>/dev/null; said "[Request interrupted by user]"; ended 1 0.0042; exit 0' INT
    sleep 30 >/dev/null 2>&1 &
    wait
    ;;
  *)
    call toolu_1 Read "\"file_path\":\"$root/src/routing.rs\"" false
    call toolu_2 Grep "\"pattern\":\"weight\",\"path\":\"$root/src\",\"output_mode\":\"content\"" false "src/lib.rs:3:// weight\\nsrc/lib.rs-4-fn main() {}"
    call toolu_6 Glob "\"pattern\":\"**/*.toml\"" false "Cargo.toml"
    call toolu_3 Read "\"file_path\":\"/elsewhere/notes.txt\"" true
    call toolu_4 Read "\"file_path\":\"$root/src/weights.rs\"" false
    call toolu_5 Read "\"file_path\":\"$root/src/routing.rs\"" false
    said "Routing is decided by weight, in src/routing.rs."
    ended 3 0.0123
    ;;
esac
"##;

fn git(root: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .args([
            "-c",
            "user.email=scout@example.com",
            "-c",
            "user.name=scout",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?}: {ran:?}");
}

/// A Fleet whose repository is a real checkout with one commit, and whose
/// scouts are the stand-in. The store and the stand-in's files are ignored, so
/// the checkout reads clean until a case writes to it.
fn scouted(home: &TempDir) -> Arc<Scouted> {
    let root = home.path();
    std::fs::create_dir_all(root.join("src")).expect("a source directory");
    std::fs::write(root.join("src/routing.rs"), "fn route() {}\n").expect("written");
    std::fs::write(root.join("src/weights.rs"), "const WEIGHT: u8 = 1;\n").expect("written");
    std::fs::write(root.join(".gitignore"), "armada.db*\nstand-in*\n").expect("written");
    git(root, &["init", "-q"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "routing"]);
    hosted(home)
}

/// The Fleet over `home` again, as a restart assembles it.
fn hosted(home: &TempDir) -> Arc<Scouted> {
    let root = home.path();
    let state = root.join("stand-in");
    std::fs::create_dir_all(&state).expect("a state directory");
    let script = root.join("stand-in.sh");
    let written = STAND_IN
        .replace("@STATE@", &state.to_string_lossy())
        .replace("@ROOT@", &root.to_string_lossy());
    std::fs::write(&script, written).expect("the stand-in is written");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
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
    Arc::new(Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[]))).scouting_on(host))
}

async fn a_studio(fleet: &Scouted) -> Studio {
    fleet
        .create_studio(CreateStudio { name: None }, None)
        .await
        .expect("created")
}

fn asking(asked: &str) -> AskScout {
    AskScout {
        asked: asked.to_string(),
        position: StudioPosition { x: 0, y: 0 },
        produced_by: None,
    }
}

fn state(node: &StudioNode) -> &'static str {
    node.state.map(|state| state.as_wire()).unwrap_or("none")
}

/// The node once it is Frozen, read the way Bridge reads a Studio.
async fn frozen(fleet: &Scouted, studio: &Studio, node: usize) -> StudioNode {
    for _ in 0..1000 {
        let read = fleet
            .get_studio(studio.id.clone(), None)
            .await
            .expect("reads");
        if let Some(node) = read.nodes.get(node).filter(|node| state(node) == "frozen") {
            return node.clone();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the Finding never froze");
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

/// `#1292`'s definition of done, first half: **a person asks, a Finding
/// appears Gathering, then Frozen with the files it read and the commit.** A
/// read the agent refused is not listed, a file read twice is listed once, and
/// what the scout said last is kept.
#[tokio::test]
async fn an_ask_gathers_then_freezes_with_every_file_read_and_the_commit() {
    let home = TempDir::new();
    let fleet = scouted(&home);
    let studio = a_studio(&fleet).await;
    std::fs::write(home.path().join("src/draft.rs"), "// not committed\n").expect("written");

    let asked = Arc::clone(&fleet)
        .ask_scout(studio.id.clone(), asking("how is routing decided"), None)
        .await
        .expect("asked");
    let started = &asked.nodes[0];
    assert!(
        matches!(state(started), "gathering" | "frozen"),
        "answered once started: {started:?}"
    );

    let node = frozen(&fleet, &studio, 0).await;
    let StudioNodeContent::Finding {
        asked,
        checkout,
        read,
        searched,
        learned,
        ended,
    } = node.content
    else {
        panic!("a Finding: {:?}", node.content);
    };
    assert_eq!(asked, "how is routing decided");
    assert_eq!(
        read,
        ["src/routing.rs", "src/lib.rs", "src/weights.rs"],
        "a content search's files are read; a listing's names are not"
    );
    assert_eq!(searched, ["weight in src", "**/*.toml"]);
    let checkout = checkout.expect("the checkout is recorded");
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(home.path())
        .output()
        .expect("git runs");
    assert_eq!(
        checkout.commit,
        String::from_utf8_lossy(&head.stdout).trim()
    );
    assert!(checkout.uncommitted, "src/draft.rs is not committed");
    assert_eq!(
        learned.as_deref(),
        Some("Routing is decided by weight, in src/routing.rs.")
    );
    let ended = ended.expect("it ended");
    assert_eq!(ended.outcome, ScoutOutcome::Answered);
    assert_eq!(ended.cost_micros, Some(12_300), "its cost is shown");

    let turns = std::fs::read_to_string(home.path().join("stand-in/turns.log")).expect("told");
    assert!(turns.contains("You are a scout, in Armada."), "{turns}");
    assert!(turns.contains("how is routing decided"), "{turns}");
    let argv = std::fs::read_to_string(home.path().join("stand-in/argv.log")).expect("ran");
    assert!(argv.contains("--tools Read,Grep,Glob"), "{argv}");
    assert!(argv.contains("--restricted"), "{argv}");
}

/// Second half: **a scout stopped from its node ends Stopped, with its cost
/// shown**, and a stop with no scout reading is refused.
#[tokio::test]
async fn a_scout_stopped_from_its_node_ends_stopped_with_its_cost() {
    let home = TempDir::new();
    let fleet = scouted(&home);
    let studio = a_studio(&fleet).await;
    let asked = Arc::clone(&fleet)
        .ask_scout(studio.id.clone(), asking("read slowly"), None)
        .await
        .expect("asked");
    let node_id = asked.nodes[0].id.clone();

    // Stopped once it has read something, so the interrupt lands mid-turn.
    for _ in 0..1000 {
        let read = fleet
            .get_studio(studio.id.clone(), None)
            .await
            .expect("reads");
        if matches!(&read.nodes[0].content, StudioNodeContent::Finding { read, .. } if !read.is_empty())
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    fleet
        .stop_scout(
            studio.id.clone(),
            StopScout {
                node_id: node_id.clone(),
            },
            None,
        )
        .await
        .expect("stopped");

    let node = frozen(&fleet, &studio, 0).await;
    let StudioNodeContent::Finding { read, ended, .. } = node.content else {
        panic!("a Finding");
    };
    assert_eq!(
        read,
        ["src/routing.rs"],
        "what it read before the stop is kept"
    );
    let ended = ended.expect("it ended");
    assert_eq!(ended.outcome, ScoutOutcome::Stopped);
    assert_eq!(ended.cost_micros, Some(4_200), "its cost is shown");

    let again = fleet
        .stop_scout(studio.id.clone(), StopScout { node_id }, None)
        .await
        .expect_err("nothing is reading for it now");
    assert_eq!(code(&again), "fleet.scout_not_running");
}

/// **Helm's proposed Finding starts once, and only a Proposed one starts.** A
/// Finding added claiming what its scout would record is refused.
#[tokio::test]
async fn a_proposed_finding_starts_once_and_a_claimed_one_is_never_added() {
    let home = TempDir::new();
    let fleet = scouted(&home);
    let studio = a_studio(&fleet).await;
    let proposed = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: StudioNodeContent::finding_asked("what reads the weights"),
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
            },
            Redirector::Helm,
            None,
        )
        .await
        .expect("Helm proposes a Finding");
    let node_id = proposed.nodes[0].id.clone();
    Arc::clone(&fleet)
        .start_scout(
            studio.id.clone(),
            StartScout {
                node_id: node_id.clone(),
            },
            None,
        )
        .await
        .expect("started on a person's ask");
    let twice = Arc::clone(&fleet)
        .start_scout(studio.id.clone(), StartScout { node_id }, None)
        .await
        .expect_err("already started");
    assert_eq!(code(&twice), "fleet.scout_not_proposed");
    assert_eq!(state(&frozen(&fleet, &studio, 0).await), "frozen");

    let StudioNodeContent::Finding { ended, .. } = frozen(&fleet, &studio, 0).await.content else {
        panic!("a Finding");
    };
    let claimed = StudioNodeContent::Finding {
        asked: "what reads the weights".to_string(),
        checkout: None,
        read: vec!["src/weights.rs".to_string()],
        searched: Vec::new(),
        learned: None,
        ended,
    };
    let refused = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: claimed,
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a scout's record is the scout's");
    assert_eq!(code(&refused), "fleet.studio_finding_is_the_scouts");
}

/// **A restart settles a Finding nobody is reading for**: frozen as failed,
/// keeping what it read, rather than pulsing Gathering forever. Planted in the
/// store, as the Fleet before this one left it.
#[tokio::test]
async fn a_finding_left_gathering_by_a_restart_freezes_as_failed() {
    let home = TempDir::new();
    let fleet = scouted(&home);
    let studio = a_studio(&fleet).await;
    {
        let id = studio.id.to_domain();
        let mut store = fleet.store().lock().await;
        let node = core_model::StudioNode::added(
            core_model::StudioNodeId::carried(core_model::Ulid::carried("01LEFT")),
            core_model::StudioNodeContent::Finding(core_model::StudioFinding::asked(
                "how is routing decided",
            )),
            core_model::StudioPosition { x: 0, y: 0 },
            fleet.now(),
            core_model::StudioAuthor::Person,
        );
        store
            .add_studio_node(&id, &node, None, &fleet.now())
            .expect("added");
        let mut gathering = node
            .scouting(core_model::ScoutCheckout {
                commit: "4bdb169c".to_string(),
                uncommitted: false,
            })
            .expect("proposed");
        gathering.looked(core_model::ScoutLook::File("src/routing.rs".to_string()));
        store
            .keep_scouted(&id, &gathering, &fleet.now())
            .expect("kept");
    }

    fleet.reconcile().await.expect("reconciled");
    let node = frozen(&fleet, &studio, 0).await;
    let StudioNodeContent::Finding { read, ended, .. } = node.content else {
        panic!("a Finding");
    };
    assert_eq!(read, ["src/routing.rs"], "what it read is kept");
    let ended = ended.expect("it ended");
    assert!(
        matches!(&ended.outcome, ScoutOutcome::Failed { why } if why.contains("Fleet stopped")),
        "{ended:?}"
    );
    assert_eq!(ended.cost_micros, None, "no cost was reported to anyone");
}
