//! Who put each thing on a Studio, kept on the record: V78. `#1288`.
//!
//! Tested from the version before it, `allowing`'s way: a V77 file with a
//! Studio on it, written through raw SQL.

use core_model::{
    ManifestId, Studio, StudioAuthor, StudioEdge, StudioEdgeId, StudioId, StudioName, StudioNode,
    StudioNodeContent, StudioNodeId, StudioPosition, StudioRelation, Timestamp, Ulid,
};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{open, TempDir};

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-17T09:00:00.000Z")
}

fn node(id: &str) -> StudioNodeId {
    StudioNodeId::carried(Ulid::carried(id))
}

fn edge(id: &str) -> StudioEdgeId {
    StudioEdgeId::carried(Ulid::carried(id))
}

/// **Helm's acts read back as Helm's after a reopen**: a node it added, the
/// `produced` edge that node brought, an edge it proposed, and the name it gave.
#[test]
fn helms_node_edge_and_name_read_back_as_helms() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = StudioId::carried(Ulid::carried("01STUDIO"));
    store
        .create_studio(&Studio {
            id: studio.clone(),
            manifest_id: ManifestId::carried(Ulid::carried("armada")),
            name: None,
            named_by: None,
            created_at: at(),
            touched_at: at(),
        })
        .expect("kept");
    let note = StudioNode::added(
        node("01NOTE"),
        StudioNodeContent::Note {
            said: "the count is stale".to_string(),
            capture: None,
        },
        StudioPosition { x: 0, y: 0 },
        at(),
        StudioAuthor::Person,
    );
    store
        .add_studio_node(&studio, &note, None, &at())
        .expect("a person's Note");
    let finding = StudioNode::added(
        node("01FINDING"),
        StudioNodeContent::Finding(core_model::StudioFinding::asked("what reads it")),
        StudioPosition { x: 0, y: 200 },
        at(),
        StudioAuthor::Helm,
    );
    store
        .add_studio_node(
            &studio,
            &finding,
            Some((&node("01NOTE"), edge("01MADE"))),
            &at(),
        )
        .expect("Helm's Finding");
    let proposed = StudioEdge::proposed(
        edge("01SAME"),
        node("01FINDING"),
        node("01NOTE"),
        StudioRelation::Answers,
        at(),
        StudioAuthor::Helm,
    )
    .expect("two nodes");
    store
        .add_studio_edge(&studio, &proposed, &at())
        .expect("proposed");
    let name = StudioName::named("Stale counts").expect("a name");
    store
        .rename_studio(&studio, &name, StudioAuthor::Helm, &at())
        .expect("named");
    drop(store);

    let graph = open(&dir).studio(&studio).expect("reads");
    assert_eq!(graph.studio.named_by, Some(StudioAuthor::Helm));
    let nodes: Vec<_> = graph
        .nodes
        .iter()
        .map(|n| (n.id().as_str(), n.added_by()))
        .collect();
    assert_eq!(
        nodes,
        [
            ("01FINDING", Some(StudioAuthor::Helm)),
            ("01NOTE", Some(StudioAuthor::Person))
        ]
    );
    let edges: Vec<_> = graph
        .edges
        .iter()
        .map(|e| (e.id().as_str(), e.added_by()))
        .collect();
    assert_eq!(
        edges,
        [
            ("01MADE", Some(StudioAuthor::Helm)),
            ("01SAME", Some(StudioAuthor::Helm))
        ],
        "the edge a node brings is its adder's"
    );
}

/// **A row from before V78 reads back unrecorded**, never as a person's: Helm
/// could add one under V77 too.
#[test]
fn a_studio_from_before_the_authors_were_kept_reads_back_unrecorded() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..77] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '77');
         INSERT INTO studios VALUES ('01OLD', 'armada', 'Named then', '{at}', '{at}');
         INSERT INTO studio_nodes VALUES ('01N1', '01OLD', 'note', NULL, '{{\"said\":\"a\"}}', 0, 0, '{at}');
         INSERT INTO studio_nodes VALUES ('01N2', '01OLD', 'note', NULL, '{{\"said\":\"b\"}}', 9, 0, '{at}');
         INSERT INTO studio_edges VALUES ('01E', '01OLD', '01N1', '01N2', 'same_as', 'proposed', '{at}');",
        at = at().as_str()
    ))
    .expect("a Studio as V77 wrote it");
    drop(conn);

    let graph = open(&dir)
        .studio(&StudioId::carried(Ulid::carried("01OLD")))
        .expect("migrates and reads");
    assert_eq!(graph.studio.named_by, None);
    assert!(graph.nodes.iter().all(|node| node.added_by().is_none()));
    assert_eq!(graph.edges[0].added_by(), None);
}
