//! Helm on a Studio through a stand-in agent: an ask goes through `ask_helm`
//! to a host that answers it by calling the agent door from a connection Fleet
//! places as Helm's. `#1288`.
//!
//! **The stand-in chooses the calls**, so what is proved is what the door and
//! Fleet do with them. A Run node is `#1289`'s, and writing up and dispatching
//! from an Issue draft `#1291`'s; the brief's rule for both is pinned in
//! `helm::tests`.

use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use api::{Conversations, Next, Queries, Studios, Subscription};
use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::{AskHelm, CreateStudio, HelmText, RunId};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::helm::{Authority, Carried, Carry, Carrying, Heard, Hosting};
use crate::tests::daemon::fittings;
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The pid the stand-in's session runs as, and the port its relay calls from.
const SESSION: u32 = 4343;
const RELAY: u16 = 52100;

/// `web` is what a person means by "the web app": it prints and exits, so its
/// end is read back without a Stop.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
commands:
  web:
    run: "/bin/sh -c 'echo serving on 5173'"
"#;

/// A session that answers each message by calling the door, and hands back
/// every answer it was given.
struct StandIn {
    door: OnceLock<Router>,
    turns: Mutex<Vec<String>>,
    answered: tokio::sync::mpsc::UnboundedSender<Vec<String>>,
}

impl Hosting for StandIn {
    fn carry<'a>(&'a self, carry: Carry, _heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(async move {
            self.turns
                .lock()
                .expect("not poisoned")
                .push(carry.turn.clone());
            let door = self.door.get().expect("the door is mounted before an ask");
            let answers = acted_on(door, &carry.turn).await;
            let _ = self.answered.send(answers);
            Carried::Answered {
                session: "stand-in".to_string(),
            }
        }) as Pin<Box<dyn Future<Output = _> + Send>>
    }

    fn model(&self) -> String {
        String::from("a-model")
    }

    fn running(&self) -> Vec<u32> {
        vec![SESSION]
    }
}

/// What the stand-in calls for each ask. **The first line of an answer is the
/// call it answers**, so a failure names which.
async fn acted_on(door: &Router, turn: &str) -> Vec<String> {
    let mut answers = Vec::new();
    let mut called = |tool: &'static str, answer: String| {
        answers.push(format!("{tool}\n{answer}"));
        answer
    };
    let studio = between(turn, "On Studio ", ",").unwrap_or_default();
    if turn.contains("what could we try here") {
        called(
            "get_studio",
            tool(door, "get_studio", &on(studio, "")).await,
        );
        let finding = |asked: &str, x: i64| {
            format!(r#""kind":"finding","asked":"{asked}","position":{{"x":{x},"y":0}}"#)
        };
        let first = on(studio, &finding("what reads the count", 0));
        called(
            "add_studio_node",
            tool(door, "add_studio_node", &first).await,
        );
        let second = on(studio, &finding("who resets it", 240));
        let added = called(
            "add_studio_node",
            tool(door, "add_studio_node", &second).await,
        );
        let ids = every_id(&added);
        if let [_, from, to, ..] = ids.as_slice() {
            let edge = on(
                studio,
                &format!(r#""from":"{from}","to":"{to}","kind":"same_as""#),
            );
            called(
                "propose_studio_edge",
                tool(door, "propose_studio_edge", &edge).await,
            );
        }
        let name = on(studio, r#""name":"Stale counts""#);
        called("rename_studio", tool(door, "rename_studio", &name).await);
        let note = on(
            studio,
            r#""kind":"note","said":"Helm saw this","position":{"x":0,"y":400}"#,
        );
        called(
            "add_studio_node",
            tool(door, "add_studio_node", &note).await,
        );
        called(
            "list_checkout_runs",
            tool(door, "list_checkout_runs", "{}").await,
        );
    }
    if turn.contains("start the web app") {
        let body = r#"{"body":{"name":"web"}}"#;
        let started = called(
            "start_checkout_run",
            tool(door, "start_checkout_run", body).await,
        );
        let run = every_id(&started).first().cloned().unwrap_or_default();
        for _ in 0..500 {
            let listed = tool(door, "list_checkout_runs", "{}").await;
            if listed.contains(r#"\"exit_code\":0"#) {
                called("list_checkout_runs", listed);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let output = format!(r#"{{"run_id":"{run}"}}"#);
        called(
            "get_checkout_run_output",
            tool(door, "get_checkout_run_output", &output).await,
        );
    }
    if turn.contains("accept that and delete the rest") {
        let accept = on(studio, r#""edge_id":"any","accepted":true"#);
        called(
            "decide_studio_edge",
            tool(door, "decide_studio_edge", &accept).await,
        );
        called(
            "delete_studio",
            tool(door, "delete_studio", &on(studio, "")).await,
        );
    }
    answers
}

/// A call's arguments naming `studio`, with `body` where there is one.
fn on(studio: &str, body: &str) -> String {
    match body.is_empty() {
        true => format!(r#"{{"studio_id":"{studio}"}}"#),
        false => format!(r#"{{"studio_id":"{studio}","body":{{{body}}}}}"#),
    }
}

/// One tool call on the door, from the relay Fleet places as Helm's.
async fn tool(door: &Router, name: &str, arguments: &str) -> String {
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{name}","arguments":{arguments}}}}}"#
    );
    let peer: SocketAddr = format!("127.0.0.1:{RELAY}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(api::DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body))
        .expect("a well-formed request");
    let response = door.clone().oneshot(request).await.expect("an answer");
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

/// Every `id` in a tool answer, in the order written, **read as text** — the
/// answer is JSON inside a JSON-RPC string, and a model reads it that way too.
fn every_id(answer: &str) -> Vec<String> {
    answer
        .split(r#"\"id\":\""#)
        .skip(1)
        .filter_map(|rest| rest.split_once(r#"\""#).map(|(id, _)| id.to_string()))
        .collect()
}

fn between<'a>(text: &'a str, from: &str, to: &str) -> Option<&'a str> {
    let (_, rest) = text.split_once(from)?;
    Some(rest.split_once(to).map_or(rest, |(said, _)| said))
}

/// The checkout as a repository with one commit, which a run's snapshot needs.
fn a_repository_at(at: &Path) {
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?}");
    };
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), MANIFEST).expect("the Manifest");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
}

struct Held {
    fleet: Arc<Hosted>,
    host: Arc<StandIn>,
    answered: tokio::sync::mpsc::UnboundedReceiver<Vec<String>>,
}

/// A Fleet over a repository, hosting the stand-in as Helm under `authority`.
fn held(home: &TempDir, authority: Authority) -> Held {
    a_repository_at(home.path());
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest =
        config::Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("the fixture parses");
    fittings.helm_authority = authority;
    let peers = Placing::nothing();
    peers.holding(SESSION + 1, RELAY, fittings.host.port);
    peers.started(SESSION, SESSION + 1);
    fittings.peers = peers;
    let (sender, answered) = tokio::sync::mpsc::unbounded_channel();
    let host = Arc::new(StandIn {
        door: OnceLock::new(),
        turns: Mutex::new(Vec::new()),
        answered: sender,
    });
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_helm_on(host.clone()));
    let events = fleet.events();
    let door = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    assert!(host.door.set(door).is_ok(), "mounted once");
    Held {
        fleet,
        host,
        answered,
    }
}

impl Held {
    /// Ask, and wait for every answer the stand-in was given.
    async fn asked(&mut self, text: &str) -> Vec<String> {
        let ask = AskHelm {
            text: HelmText::said(text).expect("not blank"),
            context: None,
        };
        Arc::clone(&self.fleet)
            .ask_helm(ask, None)
            .await
            .expect("the message is taken");
        tokio::time::timeout(Duration::from_secs(30), self.answered.recv())
            .await
            .expect("the stand-in answered")
            .expect("the host is held")
    }

    fn first_turn(&self) -> String {
        self.host.turns.lock().expect("not poisoned")[0].clone()
    }
}

/// The answer to the `at`th call, checked to be the call named.
fn answer(answers: &[String], at: usize, tool: &str) -> String {
    let answer = answers
        .get(at)
        .unwrap_or_else(|| panic!("no call {at}: {answers:?}"));
    assert!(answer.starts_with(tool), "call {at} is `{tool}`: {answer}");
    answer.clone()
}

async fn kinds(watching: &mut Subscription) -> Vec<String> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(30), watching.next()).await
    {
        seen.push(delivered.event.kind());
    }
    seen
}

/// **"What could we try here" adds proposed nodes and nothing runs.** Two
/// Findings, an edge and a name land, each published as Helm's own act; a Note
/// is refused as a person's; no run starts and no Job is made. A person's acts,
/// asked for next, are no call at all.
#[tokio::test]
async fn what_helm_does_unasked_on_a_studio_is_proposed_and_starts_nothing() {
    let home = TempDir::new();
    let mut held = held(&home, Authority::Acting);
    let studio = held
        .fleet
        .create_studio(CreateStudio { name: None }, None)
        .await
        .expect("a person opens a Studio");
    let mut watching = held.fleet.events().subscribe();

    let ask = format!("On Studio {}, what could we try here?", studio.id.as_str());
    let answers = held.asked(&ask).await;
    let brief = held.first_turn();
    assert!(
        brief.contains("ON A STUDIO"),
        "the session is told: {brief}"
    );
    let unasked = [
        "get_studio",
        "add_studio_node",
        "add_studio_node",
        "propose_studio_edge",
        "rename_studio",
    ];
    for (at, tool) in unasked.into_iter().enumerate() {
        let said = answer(&answers, at, tool);
        assert!(said.contains(r#""isError":false"#), "{said}");
    }
    let note = answer(&answers, 5, "add_studio_node");
    assert!(note.contains("fleet.studio_node_not_helms"), "{note}");
    let runs = answer(&answers, 6, "list_checkout_runs");
    assert!(runs.contains(r#"\"runs\":[]"#), "nothing ran: {runs}");

    let now = held
        .fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("kept");
    assert_eq!(now.name.as_deref(), Some("Stale counts"));
    assert_eq!(now.nodes.len(), 2);
    for node in &now.nodes {
        assert_eq!(node.state.map(|state| state.as_wire()), Some("proposed"));
    }
    assert_eq!(now.edges[0].standing.as_wire(), "proposed");
    let helms = Some("helm");
    assert_eq!(now.named_by.map(|by| by.as_wire()), helms);
    assert_eq!(now.edges[0].added_by.map(|by| by.as_wire()), helms);
    for node in &now.nodes {
        assert_eq!(
            node.added_by.map(|by| by.as_wire()),
            helms,
            "kept on the record"
        );
    }
    let jobs = held.fleet.list_jobs(None).await.expect("listed");
    assert!(jobs.jobs.is_empty(), "nothing unasked made a Job");

    let each = ["studio.changed", "studio.helm_acted"];
    assert_eq!(
        kinds(&mut watching).await,
        [each, each, each, each].concat(),
        "each of Helm's acts is its own event, and nothing ran"
    );

    let persons = format!(
        "On Studio {}, accept that and delete the rest",
        studio.id.as_str()
    );
    let refused = held.asked(&persons).await;
    for (at, tool) in ["decide_studio_edge", "delete_studio"]
        .into_iter()
        .enumerate()
    {
        let said = answer(&refused, at, tool);
        assert!(said.contains("is not a tool this Fleet offers"), "{said}");
    }
    let after = held
        .fleet
        .get_studio(studio.id, None)
        .await
        .expect("not deleted");
    assert_eq!(after, now, "a person's acts moved nothing");
}

/// **"Start the web app" starts a run Helm reads back** through the door, since
/// its end is on a stream no session receives. The Run node is `#1289`'s.
#[tokio::test]
async fn a_run_helm_is_asked_to_start_is_one_it_reads_back() {
    let home = TempDir::new();
    let mut held = held(&home, Authority::Acting);

    let answers = held.asked("start the web app").await;
    let started = answer(&answers, 0, "start_checkout_run");
    assert!(started.contains(r#""isError":false"#), "{started}");
    let listed = answer(&answers, 1, "list_checkout_runs");
    assert!(listed.contains(r#"\"name\":\"web\""#), "{listed}");
    let output = answer(&answers, 2, "get_checkout_run_output");
    assert!(output.contains("serving on 5173"), "{output}");
}

/// **Read-only reaches a Studio too**: the Studio and the runs are read, every
/// unasked act is refused by the setting, and the brief said so first.
#[tokio::test]
async fn a_read_only_helm_proposes_nothing_on_a_studio() {
    let home = TempDir::new();
    let mut held = held(&home, Authority::ReadOnly);
    let studio = held
        .fleet
        .create_studio(CreateStudio { name: None }, None)
        .await
        .expect("a Studio");

    let ask = format!("On Studio {}, what could we try here?", studio.id.as_str());
    let answers = held.asked(&ask).await;
    let brief = held.first_turn();
    assert!(brief.contains("On a Studio you only read"), "{brief}");
    let read = answer(&answers, 0, "get_studio");
    assert!(read.contains(r#""isError":false"#), "{read}");
    let refused = answer(&answers, 1, "add_studio_node");
    let why = "this machine is set so that Helm only reads";
    assert!(refused.contains(why), "{refused}");
    let runs = answers.last().expect("a last call");
    assert!(runs.starts_with("list_checkout_runs"), "{runs}");
    assert!(runs.contains(r#""isError":false"#), "{runs}");
    let now = held.fleet.get_studio(studio.id, None).await.expect("kept");
    assert!(now.nodes.is_empty() && now.edges.is_empty() && now.name.is_none());
}
