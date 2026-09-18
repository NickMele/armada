//! What a Link's address is read as, and what a fetched source reduces to.
//! `#1293`.
//!
//! **No process starts here.** What a source needs run against it is a value
//! this crate renders, the way a Drone's confinement is, so the whole read-in
//! posture is asserted without a network or a credential.

use std::path::PathBuf;

use core_model::{ForgeState, StudioNodeContent, StudioNodeKind};

use crate::reading_in::{
    bounded, fetching, forge_facts, forge_node, milestone_read, source_of, text_of_a_page,
    text_of_a_session, Fetch, Source, FORGE_HOST, MOST_ISSUES,
};

/// **The adapter decides the kind, and nothing else may.** `#1394`: a pasted
/// address is an Issue, a Pull request or an Epic where this crate recognises
/// it, and stays a Link where it does not — which is what keeps a board, a
/// page and a session Links without a rule about them anywhere.
#[test]
fn an_address_this_crate_recognises_is_the_node_kind_it_names() {
    let kind = |address: &str| forge_node(address, None).map(|node| node.kind());
    assert_eq!(
        kind(&at("NickMele/armada/issues/1394")),
        Some(StudioNodeKind::Issue)
    );
    assert_eq!(
        kind(&at("NickMele/armada/pull/1391")),
        Some(StudioNodeKind::PullRequest)
    );
    assert_eq!(
        kind(&at("NickMele/armada/milestone/17")),
        Some(StudioNodeKind::Epic)
    );
    assert_eq!(
        kind(&at("NickMele/armada/pulls/1391")),
        Some(StudioNodeKind::PullRequest),
        "two spellings of one pull request"
    );
    for stays_a_link in [
        "https://example.invalid/a-board",
        "https://react.dev/reference/react/useId",
        "armada:session/b1c9d559",
        "armada:thread",
        &at("NickMele/armada/blob/main/README.md"),
        &at("NickMele/armada/issues/not-a-number"),
    ] {
        assert_eq!(
            kind(stays_a_link),
            None,
            "`{stays_a_link}` names nothing on the forge, so it stays a Link"
        );
    }

    // The number comes off the address and nothing is fetched; the line a
    // person typed beside it is carried over whatever the address turned out
    // to be, because they typed it about this address.
    let pasted = forge_node(
        &at("NickMele/armada/issues/1394#issuecomment-4"),
        Some(String::from("  where the kinds landed  ")),
    )
    .expect("an issue");
    let StudioNodeContent::Issue {
        number,
        said,
        title,
        state,
        ..
    } = &pasted
    else {
        panic!("an Issue: {pasted:?}");
    };
    assert_eq!(number, "1394", "a comment anchor is still the issue");
    assert_eq!(said.as_deref(), Some("where the kinds landed"), "trimmed");
    assert_eq!((title, state), (&None, &None), "nothing was fetched");
}

/// **A read-in fills in what the forge already printed.** The `--jq` filters
/// reduce an issue and a pull request to `#<number> <title> (<state>)` on the
/// first line, so resolving a node's title and state costs no second call —
/// `#1394`. A line this does not recognise leaves both absent.
#[test]
fn what_a_fetch_printed_says_a_nodes_title_and_where_it_stands() {
    let read = forge_facts(
        "#1394 An issue is a Link with rules bolted on (OPEN)

body",
    );
    assert_eq!(
        read.title.as_deref(),
        Some("An issue is a Link with rules bolted on")
    );
    assert_eq!(read.state, Some(ForgeState::Open));
    assert_eq!(read.read_in, None, "an Epic's count is a milestone's read");

    // The *last* bracket, so a title carrying one of its own survives whole.
    let bracketed = forge_facts("#1391 Dispatch (from a node) (MERGED)");
    assert_eq!(bracketed.title.as_deref(), Some("Dispatch (from a node)"));
    assert_eq!(bracketed.state, Some(ForgeState::Merged));

    // A forge's case is the forge's and the word is ours.
    assert_eq!(
        forge_facts("#1 A title (closed)").state,
        Some(ForgeState::Closed)
    );
    let unknown = forge_facts("#1 A title (draft)");
    assert_eq!(unknown.title.as_deref(), Some("A title"));
    assert_eq!(unknown.state, None, "a word this build has no name for");
    assert_eq!(forge_facts("").title, None);
}

fn at(path: &str) -> String {
    format!("https://{FORGE_HOST}{path}")
}

/// A checkout and a home no fixture writes to: the cases below that matter
/// resolve without touching either.
const ROOT: &str = "/repos/armada";
const HOME: &str = "/home/user";

/// **The forge's shapes, and everything else on the web as a page.** A link
/// this does not recognise is still text somebody wrote, so it is read rather
/// than refused; what is refused is an address that names no source at all.
#[test]
fn a_forge_link_is_read_as_what_it_names_and_anything_else_on_the_web_as_a_page() {
    let kind = |address: &str| source_of(address, ROOT, HOME).map(|source| source.kind());
    assert_eq!(kind(&at("NickMele/armada/issues/1293")), Some("issue"));
    assert_eq!(kind(&at("NickMele/armada/pull/1377")), Some("pull_request"));
    assert_eq!(kind(&at("NickMele/armada/milestone/17")), Some("milestone"));
    assert_eq!(
        kind(&at("NickMele/armada/issues/1293#issuecomment-4")),
        Some("issue"),
        "a comment anchor is still the issue"
    );
    assert_eq!(
        kind(&at("NickMele/armada/blob/main/README.md")),
        Some("page"),
        "a forge link this does not know is a page, not a refusal"
    );
    assert_eq!(kind("https://react.dev/reference"), Some("page"));
    assert_eq!(kind("armada:thread"), Some("thread"));

    // A board, a wiki, a bare word: not a source, so the Link stays a Link.
    for stays in ["miro://board/uXjVK", "notion.so/a-page", "", "the rail"] {
        assert_eq!(kind(stays), None, "`{stays}` is not a source");
    }
}

/// **A session is found by where this checkout keys its project directory**,
/// so another repository's is not addressable rather than addressable and
/// refused. A worktree lives under the checkout, so its directory is ours too.
#[test]
fn a_session_resolves_only_under_this_checkouts_own_project_directory() {
    // A directory to hang project directories off, removed with the repo.
    let home = crate::tests::repo::TempRepo::empty();
    let root = "/Users/user/Development/armada";
    let ours = "-Users-user-Development-armada";
    let projects = home.root().join(".claude/projects");
    for (project, id) in [
        (ours, "aaaa"),
        // A worktree of this repository: its key begins with ours.
        (
            "-Users-user-Development-armada--claude-worktrees-one",
            "bbbb",
        ),
        // Another repository entirely.
        ("-Users-user-Development-other", "cccc"),
        // A repository whose path merely begins with the same letters.
        ("-Users-user-Development-armadillo", "dddd"),
    ] {
        std::fs::create_dir_all(projects.join(project)).expect("a project directory");
        std::fs::write(projects.join(project).join(format!("{id}.jsonl")), "{}\n")
            .expect("a transcript");
    }
    let home = home.root_str();
    let found = |id: &str| -> Option<PathBuf> {
        match source_of(&format!("armada:session/{id}"), root, &home) {
            Some(Source::Session { file }) => Some(file),
            _ => None,
        }
    };
    assert!(found("aaaa").is_some(), "this repository's own");
    assert!(found("bbbb").is_some(), "a worktree of it");
    assert!(found("cccc").is_none(), "another repository's");
    assert!(
        found("dddd").is_none(),
        "a path that merely starts the same"
    );
    assert!(found("../../etc/passwd").is_none(), "an id is not a path");
    assert!(found("eeee").is_none(), "a session that is not there");
}

/// **A milestone takes two calls and one of them counts.** The issue list is
/// bounded, and the count is what lets the Link say how many of how many.
#[test]
fn a_milestone_is_two_calls_and_reads_back_bounded_with_its_total() {
    let source = source_of(&at("NickMele/armada/milestone/17"), ROOT, HOME).expect("a milestone");
    assert!(source.read_by_fleet_alone(), "a list needs no model");
    let Fetch::Calls(calls) = fetching(&source) else {
        panic!("a milestone is fetched by calls");
    };
    assert_eq!(calls.len(), 2);
    assert!(calls[0]
        .args()
        .iter()
        .any(|arg| arg == "repos/NickMele/armada/milestones/17"));
    assert!(calls[1].args().iter().any(|arg| arg == "milestone=17"));
    assert!(
        calls[1].args().iter().any(|arg| arg == "state=all"),
        "a milestone's closed issues are on it too"
    );

    let mut printed = String::from("Studio\t137\n");
    for n in 0..MOST_ISSUES + 20 {
        printed.push_str(&format!("https://x/issues/{n}\t{n}\tSomething\topen\n"));
    }
    let read = milestone_read(&printed, "17");
    assert_eq!(read.title, "Studio");
    assert_eq!(read.total, 137);
    assert_eq!(
        read.issues.len(),
        MOST_ISSUES + 20,
        "every line the fetch printed: the bound lands on what the answer took"
    );
    assert_eq!(read.issues[0].address, "https://x/issues/0");
    assert_eq!(read.issues[0].number, "0");
    assert_eq!(read.issues[0].title, "Something");
    assert_eq!(read.issues[0].state, Some(core_model::ForgeState::Open));
    assert_eq!(
        read.taking(core_model::EpicTake::Everything).issues.len(),
        MOST_ISSUES,
        "bounded"
    );
}

/// **The bound lands after the answer, never before it** — `#1405`. A milestone
/// read front-first and then filtered would take fifty issues and show
/// whichever of them happened to be open, so "only what is open" would be a
/// bound on the wrong set.
#[test]
fn only_what_is_open_is_bounded_on_the_open_issues_and_says_what_it_left_out() {
    let mut printed = String::from(
        "Overview	200
",
    );
    // Every closed issue first, which is the order that used to lose them all.
    for n in 0..80 {
        printed.push_str(&format!(
            "https://x/issues/{n}	{n}	Shipped	closed
"
        ));
    }
    for n in 80..80 + MOST_ISSUES + 10 {
        printed.push_str(&format!(
            "https://x/issues/{n}	{n}	To do	open
"
        ));
    }
    let read = milestone_read(&printed, "17");
    let taken = read.taking(core_model::EpicTake::Open);
    assert_eq!(taken.issues.len(), MOST_ISSUES, "fifty open ones");
    assert!(taken
        .issues
        .iter()
        .all(|issue| issue.state == Some(core_model::ForgeState::Open)));
    assert_eq!(taken.left_out, 80, "the closed ones, said and not silent");

    let every = read.taking(core_model::EpicTake::Everything);
    assert_eq!(every.issues.len(), MOST_ISSUES);
    assert_eq!(every.left_out, 0);
    assert_eq!(
        every.issues[0].state,
        Some(core_model::ForgeState::Closed),
        "taking everything takes the forge's own order"
    );
}

/// **Headings and text, and no markup.** A script's body is dropped whole
/// rather than unwrapped, since its source reads as prose once the tags go, and
/// a block's own break is kept so two paragraphs do not read as one sentence.
#[test]
fn a_page_reduces_to_its_headings_and_text() {
    let html = "<html><head><title>T</title><style>p { color: red }</style>\
                <script>var a = \"hello\";</script></head>\
                <body><h1>The rail</h1><p>It is per repository.</p>\
                <p>A &amp; B &lt;here&gt;</p></body></html>";
    assert_eq!(
        text_of_a_page(html),
        "T\nThe rail\n\nIt is per repository.\n\nA & B <here>"
    );
}

/// **What was argued, not the agent's working.** A transcript carries thinking,
/// tool calls, every result they returned and the harness's own reminders, and
/// a scout handed those would be reading a record of reads.
#[test]
fn a_session_reduces_to_what_the_person_and_the_agent_said() {
    let transcript = concat!(
        "{\"type\":\"custom-title\",\"customTitle\":\"Canvases\"}\n",
        "{\"type\":\"user\",\"message\":{\"content\":\"<system-reminder>ignore me</system-reminder>",
        "why is the rail per repository?\"}}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[",
        "{\"type\":\"thinking\",\"thinking\":\"hm\"},",
        "{\"type\":\"text\",\"text\":\"Because Helm answers for one.\"},",
        "{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"Read\",\"input\":{}}]}}\n",
        "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",",
        "\"tool_use_id\":\"t1\",\"content\":\"a file\"}]}}\n",
        "not json at all\n",
    );
    assert_eq!(
        text_of_a_session(transcript),
        "The person: why is the rail per repository?\n\nThe agent: Because Helm answers for one."
    );
}

/// **What was cut is counted**, because a Finding that did not say so would
/// claim a scout read a source whole when it read the front of one.
#[test]
fn a_source_is_cut_at_a_character_and_says_how_much_went() {
    assert_eq!(bounded("short", 10), ("short".to_string(), 0));
    let (kept, cut) = bounded("a… long enough", 4);
    assert_eq!(kept, "a… l", "cut by characters, never mid-codepoint");
    assert_eq!(cut, 10);
}
