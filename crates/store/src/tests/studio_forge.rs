//! An Issue, a Pull request and an Epic on a Studio: V79. `#1394`.
//!
//! Tested from the version before it, `studio_authors`' way: a V78 file with a
//! Studio on it, written through raw SQL, so what is measured is a file the
//! owner already has rather than one this build made.

use core_model::{
    EpicRead, ForgeFacts, ForgeState, StudioId, StudioNodeContent, StudioNodeKind, Ulid,
};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{open, TempDir};

const AT: &str = "2026-09-17T09:00:00.000Z";

/// A V78 file holding one Studio, two Link nodes and a `produced` edge joining
/// them — the shape V79 rebuilds both tables under.
fn a_v78_file(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..78] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '78');
         INSERT INTO studios VALUES ('01OLD', 'armada', 'Named then', '{AT}', '{AT}', 'person');
         INSERT INTO studio_nodes VALUES ('01ISSUE', '01OLD', 'link', NULL,
             '{{\"address\":\"https://github.com/NickMele/armada/issues/1379\",\"said\":\"where dispatch landed\"}}',
             40, 80, '{AT}', 'person');
         INSERT INTO studio_nodes VALUES ('01BOARD', '01OLD', 'link', NULL,
             '{{\"address\":\"https://example.invalid/a-board\"}}', 9, 0, '{AT}', NULL);
         INSERT INTO studio_edges VALUES ('01E', '01OLD', '01ISSUE', '01BOARD', 'produced',
             'accepted', '{AT}', 'person');"
    ))
    .expect("a Studio as V78 wrote it");
}

fn old() -> StudioId {
    StudioId::carried(Ulid::carried("01OLD"))
}

/// **V79 rebuilds both tables and loses nothing.** The `CHECK` cannot be
/// widened in place and `ALTER TABLE ... RENAME` rewrites `studio_edges`'
/// `REFERENCES studio_nodes` to follow the rename, so the nodes are dropped
/// under their edges — and the edges cascade. This is the test that the
/// cascade takes nothing with it.
#[test]
fn a_studio_written_before_the_forge_kinds_keeps_every_node_and_edge() {
    let dir = TempDir::new();
    a_v78_file(&dir);

    let mut store = open(&dir);
    let graph = store.studio(&old()).expect("migrates and reads");
    assert_eq!(graph.nodes.len(), 2, "both nodes survive the rebuild");
    assert_eq!(graph.edges.len(), 1, "and the edge between them does");
    assert_eq!(graph.edges[0].id().as_str(), "01E");
    assert_eq!(
        graph.studio.touched_at.as_str(),
        AT,
        "a migration is nobody's write, so the list does not reorder"
    );

    // And a node of a kind V78's `CHECK` refused now writes.
    let content = StudioNodeContent::on_the_forge(
        StudioNodeKind::Epic,
        String::from("https://github.com/NickMele/armada/milestone/17"),
        String::from("17"),
        None,
    )
    .expect("an Epic")
    .resolved(&ForgeFacts {
        title: Some(String::from("Studio")),
        state: None,
        read_in: Some(EpicRead {
            issues: 12,
            total: 30,
        }),
    })
    .expect("an Epic takes a count");
    let node = core_model::StudioNode::added(
        core_model::StudioNodeId::carried(Ulid::carried("01EPIC")),
        content.clone(),
        core_model::StudioPosition { x: 0, y: 0 },
        core_model::Timestamp::from_rfc3339(AT),
        core_model::StudioAuthor::Person,
    );
    store
        .add_studio_node(
            &old(),
            &node,
            None,
            &core_model::Timestamp::from_rfc3339(AT),
        )
        .expect("an Epic is a kind V79 admits");
    let read = store.studio(&old()).expect("reads");
    assert_eq!(
        read.nodes
            .iter()
            .find(|one| one.id().as_str() == "01EPIC")
            .map(|one| one.content()),
        Some(&content),
        "every field reads back, the count included"
    );
}

/// **A Link becomes the kind its address names, keeping everything else.**
/// The store cannot decide which addresses are an issue — the vendor-literal
/// gate refuses a host in this crate — so what is held here is the write:
/// given the node `adapters` answered with, the row moves kind and content and
/// nothing else, and a Link nothing recognised is untouched.
#[test]
fn a_link_is_recognised_in_place_and_one_nothing_names_is_left_alone() {
    let dir = TempDir::new();
    a_v78_file(&dir);
    let mut store = open(&dir);

    let links = store.every_link().expect("reads every Link");
    assert_eq!(links.len(), 2, "both Links, and no other kind");

    for (studio, node) in &links {
        // The caller's own reading, standing in for the adapter's: this crate
        // may not spell a host, and what it is handed is a node either way.
        let Some(number) = node
            .content()
            .address()
            .and_then(|address| address.rsplit_once("/issues/"))
            .map(|(_, number)| String::from(number))
        else {
            continue;
        };
        let content = StudioNodeContent::on_the_forge(
            StudioNodeKind::Issue,
            String::from(node.content().address().expect("a Link keeps one")),
            number,
            node.content().said().map(String::from),
        )
        .expect("an Issue");
        let recognised = node
            .recognised(content)
            .expect("a Link becomes one of three");
        store
            .recognise_studio_node(studio, &recognised)
            .expect("written");
    }

    let graph = store.studio(&old()).expect("reads");
    let kinds: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| (node.id().as_str(), node.kind()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("01BOARD", StudioNodeKind::Link),
            ("01ISSUE", StudioNodeKind::Issue)
        ],
        "the address that names an issue moved; the board did not"
    );
    let issue = &graph.nodes[1];
    assert_eq!(
        issue.position(),
        core_model::StudioPosition { x: 40, y: 80 },
        "where a person left it"
    );
    assert_eq!(issue.added_by(), Some(core_model::StudioAuthor::Person));
    let StudioNodeContent::Issue {
        number,
        said,
        title,
        state,
        ..
    } = issue.content()
    else {
        panic!("an Issue");
    };
    assert_eq!(number, "1379");
    assert_eq!(
        said.as_deref(),
        Some("where dispatch landed"),
        "the line the person typed is theirs and survives"
    );
    assert_eq!((title, state), (&None, &None), "nothing was fetched");
    assert_eq!(graph.edges.len(), 1, "the edge on it stands");

    // A second pass moves nothing: there is no Link left that names an issue.
    assert_eq!(
        store
            .every_link()
            .expect("reads")
            .iter()
            .map(|(_, node)| node.id().as_str().to_string())
            .collect::<Vec<_>>(),
        ["01BOARD"],
        "a conversion converges, which is why it runs on every boot"
    );
}

/// A state a build after this one writes does not read back as a state this
/// one knows. **Named rather than guessed** — `ForgeState` is a closed set.
#[test]
fn a_forge_state_this_build_does_not_know_refuses_the_row() {
    assert_eq!(ForgeState::from_wire("draft"), None);
    assert_eq!(ForgeState::from_wire("merged"), Some(ForgeState::Merged));
}
