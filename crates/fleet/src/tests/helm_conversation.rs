//! A Helm conversation under Fleet, against a stand-in agent: two messages in
//! one session with the second remembering the first, the same across a Fleet
//! restart, a lost session started fresh and said so, and Start fresh
//! forgetting the session and the thread. `#939`.
//!
//! **The stand-in runs through the real renderer, the real reader and the real
//! process host.** It keeps each session's messages in a file named by the id
//! it reported, which is the whole of what resume needs from the agent CLI — so
//! a reply naming an earlier message proves Fleet asked for the session that
//! heard it.

use std::future::Future;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use adapters::HeadlessAgent;
use api::{Conversations, HelmSeen, HelmWatch, Next, Refusal, Subscription};
use ipc::{
    AskHelm, Freshness, HelmContext, HelmMessage, HelmScreen, HelmSilence, HelmText, JobId, Saw,
    StudioId, StudioNodeId,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::drone::HostPaths;
use crate::helm::{Carry, Carrying, Heard, Hosting, ProcessHost};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The agent CLI's resume, as far as Fleet can see it: `init` names a session,
/// `--resume` of an id it holds continues that session, and one it does not
/// hold exits 1 with one `result` line — spike 016's measured shape.
const STAND_IN: &str = r##"#!/bin/sh
state='@STATE@'
printf '%s\n' "$*" >> "$state/argv.log"
resume=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--resume" ]; then resume="$2"; shift; fi
  shift
done
IFS= read -r turn
printf '%s\n' "$turn" >> "$state/turns.log"
if [ -n "$resume" ]; then
  if [ ! -f "$state/sessions/$resume" ]; then
    echo "No conversation found with session ID: $resume" >&2
    printf '%s\n' '{"type":"result","subtype":"error_during_execution","is_error":true,"num_turns":0,"total_cost_usd":0,"permission_denials":[]}'
    exit 1
  fi
  id="$resume"
else
  minted=$(cat "$state/minted" 2>/dev/null || echo 0)
  minted=$((minted + 1))
  echo "$minted" > "$state/minted"
  id="session-$minted"
fi
said=$(printf '%s' "$turn" | sed -e 's/"}}$//' -e 's/.*"content":"//' -e 's/.*\\n\\n//')
printf '%s\n' "$said" >> "$state/sessions/$id"
remembered=$(paste -s -d ',' "$state/sessions/$id" | sed 's/,/, /g')
printf '{"type":"system","subtype":"init","session_id":"%s","model":"stand-in","mcp_servers":[]}\n' "$id"
case "$said" in
  *edit*)
    printf '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Edit","input":{"file_path":"@ROOT@/crates/api/src/lib.rs","old_string":"a","new_string":"b"}}]}}\n'
    printf '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_2","name":"Bash","input":{"command":"sed -i s/a/b/ elsewhere.rs"}}]}}\n'
    ;;
esac
printf '{"type":"assistant","message":{"content":[{"type":"text","text":"remembered: %s"}]}}\n' "$remembered"
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"num_turns":1,"total_cost_usd":0.01,"permission_denials":[]}'
"##;

fn state(home: &TempDir) -> PathBuf {
    home.path().join("stand-in")
}

/// A Fleet over `home` whose Helm is the stand-in. **Called again on the same
/// `home` is a restart**: the store, the thread and the stand-in's sessions
/// are all files, and the Fleet and its host are new.
fn hosted(home: &TempDir) -> Arc<Hosted> {
    std::fs::create_dir_all(state(home).join("sessions")).expect("a state directory");
    let script = home.path().join("stand-in.sh");
    let written = STAND_IN
        .replace("@STATE@", &state(home).to_string_lossy())
        .replace("@ROOT@", &home.path().to_string_lossy());
    std::fs::write(&script, written).expect("the stand-in is written");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
    let home_dir = home.path().to_string_lossy().to_string();
    let host = ProcessHost::new(
        HeadlessAgent::at(script.to_string_lossy()),
        "stand-in",
        home.path().join("helm-mcp.json"),
        HostPaths {
            path: "/usr/bin:/bin",
            home: &home_dir,
            user: "someone",
        },
    );
    Arc::new(
        Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
            .hosting_helm_on(Arc::new(host)),
    )
}

fn asking(text: &str) -> AskHelm {
    AskHelm {
        text: HelmText::said(text).expect("not blank"),
        context: None,
    }
}

/// Every message one reply brings, through the row that ends it.
async fn reply(live: &mut HelmWatch) -> Vec<HelmMessage> {
    let mut seen = Vec::new();
    loop {
        let next = tokio::time::timeout(Duration::from_secs(20), live.next())
            .await
            .expect("a reply within the bound")
            .expect("the conversation is open");
        let HelmSeen::Message(message) = next else {
            panic!("nothing was missed");
        };
        let last = match &message {
            HelmMessage::Unanswered(_) => true,
            HelmMessage::Row(row) => matches!(row.row().saw, Saw::Ended { .. }),
            _ => false,
        };
        seen.push(message);
        if last {
            return seen;
        }
    }
}

/// What the session said, in order.
fn said(messages: &[HelmMessage]) -> Vec<String> {
    messages
        .iter()
        .filter_map(|message| match message {
            HelmMessage::Row(row) => match &row.row().saw {
                Saw::Said { text } => Some(text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Until no reply is outstanding, so the session the last one ran in is kept.
async fn settled(fleet: &Hosted) {
    for _ in 0..500 {
        if !fleet.observe_helm(None).await.expect("observed").replying {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the reply never settled");
}

/// One line per process the stand-in was started as.
fn started_as(home: &TempDir) -> Vec<String> {
    std::fs::read_to_string(state(home).join("argv.log"))
        .expect("the stand-in ran")
        .lines()
        .map(String::from)
        .collect()
}

async fn asked(fleet: &Arc<Hosted>, text: &str) {
    Arc::clone(fleet)
        .ask_helm(asking(text), None)
        .await
        .expect("the message is taken");
}

#[tokio::test]
async fn two_messages_in_one_conversation_and_the_second_remembers_the_first() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;

    asked(&fleet, "grackle").await;
    let first = reply(&mut live).await;
    assert!(
        matches!(&first[0], HelmMessage::Asked(asked) if asked.text == "grackle"),
        "the thread says what was asked first: {first:?}"
    );
    assert_eq!(said(&first), ["remembered: grackle"]);

    asked(&fleet, "thistle").await;
    assert_eq!(
        said(&reply(&mut live).await),
        ["remembered: grackle, thistle"]
    );

    let started = started_as(&home);
    assert!(!started[0].contains("--resume"), "{}", started[0]);
    assert!(started[1].contains("--resume session-1"), "{}", started[1]);
}

#[tokio::test]
async fn the_conversation_resumes_after_fleet_restarts() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;
    asked(&fleet, "grackle").await;
    reply(&mut live).await;
    settled(&fleet).await;
    drop(live);
    drop(fleet);

    let fleet = hosted(&home);
    let observed = fleet.observe_helm(None).await.expect("a conversation");
    assert!(
        matches!(&observed.history[0], HelmMessage::Asked(asked) if asked.text == "grackle"),
        "the thread is read back: {:?}",
        observed.history
    );
    assert_eq!(said(&observed.history), ["remembered: grackle"]);
    let mut live = observed.live;

    asked(&fleet, "thistle").await;
    assert_eq!(
        said(&reply(&mut live).await),
        ["remembered: grackle, thistle"]
    );
    assert!(started_as(&home)[1].contains("--resume session-1"));
}

#[tokio::test]
async fn a_new_session_is_told_helms_brief_before_the_message_and_only_then() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;
    asked(&fleet, "grackle").await;
    reply(&mut live).await;
    asked(&fleet, "thistle").await;
    reply(&mut live).await;

    let turns = std::fs::read_to_string(state(&home).join("turns.log")).expect("turns");
    let turns: Vec<&str> = turns.lines().collect();
    assert!(
        turns[0].contains("You are Helm, in Armada."),
        "{}",
        turns[0]
    );
    assert!(turns[0].contains("\\n\\ngrackle"), "{}", turns[0]);
    assert!(!turns[1].contains("You are Helm"), "{}", turns[1]);
}

/// `#1075`: the context Bridge sends with an ask reaches the session ahead of
/// what was typed, and the thread's `asked` row never carries it.
#[tokio::test]
async fn the_context_reaches_the_session_and_not_the_thread() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;

    let context = HelmContext {
        screen: HelmScreen::JobDetail,
        picked: None,
        chip: Some(JobId::carried("01JOB0000000000000000000A")),
        cursor: None,
        studio: None,
        node: None,
    };
    Arc::clone(&fleet)
        .ask_helm(
            AskHelm {
                text: HelmText::said("what is this stuck on?").expect("not blank"),
                context: Some(context),
            },
            None,
        )
        .await
        .expect("the message is taken");
    let first = reply(&mut live).await;

    let turns = std::fs::read_to_string(state(&home).join("turns.log")).expect("turns");
    let turn = turns.lines().next().expect("one turn logged");
    assert!(
        turn.contains("Job 01JOB0000000000000000000A is chipped"),
        "the session is told where the person is: {turn}"
    );

    assert!(
        matches!(&first[0], HelmMessage::Asked(asked) if asked.text == "what is this stuck on?"),
        "the thread keeps only what was typed: {first:?}"
    );
}

/// `#1287`: a person on a Studio is placed there — the screen, the Studio and
/// the node they selected — so Helm reads that Studio rather than guessing one.
#[tokio::test]
async fn a_studio_and_its_selected_node_reach_the_session() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;

    let context = HelmContext {
        screen: HelmScreen::Studio,
        picked: None,
        chip: None,
        cursor: None,
        studio: Some(StudioId::carried("01STUDIO00000000000000000A")),
        node: Some(StudioNodeId::carried("01NODE0000000000000000000B")),
    };
    Arc::clone(&fleet)
        .ask_helm(
            AskHelm {
                text: HelmText::said("what does this note connect to?").expect("not blank"),
                context: Some(context),
            },
            None,
        )
        .await
        .expect("the message is taken");
    reply(&mut live).await;

    let turns = std::fs::read_to_string(state(&home).join("turns.log")).expect("turns");
    let turn = turns.lines().next().expect("one turn logged");
    assert!(turn.contains("The person is on Studios"), "{turn}");
    assert!(
        turn.contains("Studio 01STUDIO00000000000000000A is open, with node 01NODE0000000000000000000B selected"),
        "{turn}"
    );
}

#[tokio::test]
async fn a_session_the_agent_no_longer_has_starts_fresh_and_says_so() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;
    asked(&fleet, "grackle").await;
    reply(&mut live).await;
    settled(&fleet).await;
    std::fs::remove_file(state(&home).join("sessions").join("session-1"))
        .expect("the agent's transcript is swept");

    asked(&fleet, "thistle").await;
    let second = reply(&mut live).await;
    assert!(
        second.iter().any(|message| matches!(
            message,
            HelmMessage::Fresh(fresh) if fresh.because == Freshness::SessionNotFound
        )),
        "the thread says it started over: {second:?}"
    );
    let ended = second
        .iter()
        .filter(|message| {
            matches!(message, HelmMessage::Row(row) if matches!(row.row().saw, Saw::Ended { .. }))
        })
        .count();
    assert_eq!(ended, 1, "the refused resume leaves no row: {second:?}");
    assert_eq!(said(&second), ["remembered: thistle"]);

    asked(&fleet, "wren").await;
    assert_eq!(said(&reply(&mut live).await), ["remembered: thistle, wren"]);
    assert!(started_as(&home)
        .last()
        .expect("started")
        .contains("--resume session-2"));
}

#[tokio::test]
async fn starting_fresh_forgets_the_session_and_the_thread() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;
    asked(&fleet, "grackle").await;
    reply(&mut live).await;
    settled(&fleet).await;

    let fresh = fleet
        .start_helm_fresh(None)
        .await
        .expect("nothing is replying");
    assert!(!fresh.resumes);
    let Some(HelmSeen::Message(HelmMessage::Closed(closed))) = live.next().await else {
        panic!("a viewer is told the thread is gone");
    };
    assert_eq!(closed.because, HelmSilence::StartedFresh);

    let observed = fleet.observe_helm(None).await.expect("a conversation");
    assert!(observed.history.is_empty(), "{:?}", observed.history);
    let mut live = observed.live;
    asked(&fleet, "thistle").await;
    assert_eq!(said(&reply(&mut live).await), ["remembered: thistle"]);
    assert!(!started_as(&home)
        .last()
        .expect("started")
        .contains("--resume"));
}

/// A host that never answers, standing in for a reply still being written.
struct Silent;

impl Hosting for Silent {
    fn carry<'a>(&'a self, _carry: Carry, _heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(std::future::pending()) as Pin<Box<dyn Future<Output = _> + Send>>
    }

    fn running(&self) -> Vec<u32> {
        Vec::new()
    }
}

#[tokio::test]
async fn starting_fresh_while_a_reply_is_written_is_refused() {
    let home = TempDir::new();
    let fleet = Arc::new(
        Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])))
            .hosting_helm_on(Arc::new(Silent)),
    );
    asked(&fleet, "grackle").await;

    match fleet.start_helm_fresh(None).await {
        Err(Refusal::IllegalMove(error)) => assert_eq!(error.code, "fleet.helm_still_replying"),
        other => panic!("refused while replying, not {other:?}"),
    }
}

/// Every event offered within a short window, so a test says what was
/// published rather than only what was not.
async fn published(watching: &mut Subscription) -> Vec<ipc::Event> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(50), watching.next()).await
    {
        seen.push(delivered.event);
    }
    seen
}

/// `#1373`: Helm edits the repository's own checkout when asked, so the write
/// is an event of its own — `docs/concepts/helm.md`, *Audit trail*. A person
/// who finds the file changed reads this to see that Helm changed it.
#[tokio::test]
async fn a_write_to_the_checkout_is_helms_own_event_and_a_shell_line_is_not() {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let mut watching = fleet.events().subscribe();
    let mut live = fleet.observe_helm(None).await.expect("a conversation").live;

    asked(&fleet, "edit the api crate").await;
    let messages = reply(&mut live).await;

    let events = published(&mut watching).await;
    let kinds: Vec<String> = events.iter().map(|event| event.kind()).collect();
    assert_eq!(
        kinds,
        ["helm.changed_checkout"],
        "the Edit is Helm's own event and the Bash line is not: {kinds:?}"
    );
    let [ipc::Event::HelmChangedCheckout(wrote)] = &events[..] else {
        panic!("one write event: {events:?}");
    };
    assert_eq!(wrote.tool, "Edit");
    assert_eq!(
        wrote.path, "crates/api/src/lib.rs",
        "named relative to the checkout, without how much moved"
    );

    let called: Vec<String> = messages
        .iter()
        .filter_map(|message| match message {
            HelmMessage::Row(row) => match &row.row().saw {
                Saw::Called { tool, .. } => Some(tool.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        called,
        ["Edit", "Bash"],
        "both calls are on the thread, whatever the stream published: {called:?}"
    );
}
