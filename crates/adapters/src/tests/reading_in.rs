//! What a Link's address is read as, and what a fetched source reduces to.
//! `#1293`.
//!
//! **No process starts here.** What a source needs run against it is a value
//! this crate renders, the way a Drone's confinement is, so the whole read-in
//! posture is asserted without a network or a credential.

use std::path::PathBuf;

use crate::reading_in::{
    bounded, fetching, milestone_read, source_of, text_of_a_page, text_of_a_session, Fetch, Source,
    FORGE_HOST, MOST_ISSUES,
};

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
    assert_eq!(read.issues.len(), MOST_ISSUES, "bounded");
    assert_eq!(read.issues[0].address, "https://x/issues/0");
    assert_eq!(read.issues[0].number, "0");
    assert_eq!(read.issues[0].title, "Something");
    assert_eq!(read.issues[0].state, Some(core_model::ForgeState::Open));
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
