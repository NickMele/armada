//! What a Studio on the wire keeps true: a node's kind is its content's tag,
//! spelled as `core_model` spells it; a graph round-trips; and a proposal
//! naming the edge the Studio draws itself does not decode.

use core_model::{
    ManifestId, StudioEdgeId, StudioGraph, StudioId, StudioName, StudioNodeId, StudioRelation,
    Timestamp, Ulid,
};

use crate::{
    decode, encode, Event, HelmStudioAct, Instant, ProposeStudioEdge, Studio, StudioHelmActed,
    StudioNodeContent,
};

fn content_of(kind: core_model::StudioNodeKind) -> core_model::StudioNodeContent {
    use core_model::StudioNodeContent as C;
    use core_model::StudioNodeKind as K;
    let text = || "said".to_string();
    match kind {
        K::Run => C::Run { run_id: text() },
        K::Note => C::Note { said: text() },
        K::Cluster => C::Cluster { title: text() },
        K::Finding => C::Finding { asked: text() },
        K::Contradiction => C::Contradiction {
            first: text(),
            second: text(),
        },
        K::Sketch => C::Sketch { body: text() },
        K::Link => C::Link { address: text() },
        K::Deferral => C::Deferral { what: text() },
        K::Outline => C::Outline { body: text() },
        K::IssueDraft => C::IssueDraft {
            title: text(),
            body: text(),
        },
        K::Job => C::Job {
            job_id: core_model::JobId::carried(Ulid::carried("01JOB")),
        },
    }
}

/// **One spelling per kind.** serde's tag here and `as_wire` there are two
/// spellings of one set, and this is what keeps them one.
#[test]
fn every_kinds_tag_is_the_spelling_core_model_gives_it() {
    for kind in core_model::StudioNodeKind::ALL {
        let content = content_of(*kind);
        assert_eq!(content.kind(), *kind);
        let wire = StudioNodeContent::from(&content);
        let json = encode(&wire).expect("plain data");
        assert!(
            json.starts_with(&format!(r#"{{"kind":"{}""#, kind.as_wire())),
            "{json}"
        );
        let back: StudioNodeContent = decode("content", json.as_bytes()).expect("round-trips");
        assert_eq!(back.to_domain(), content);
    }
}

fn a_graph() -> StudioGraph {
    let at = |minute: u32| Timestamp::from_rfc3339(format!("2026-09-17T09:{minute:02}:00.000Z"));
    let node = |id: &str, said: &str, x: i64| {
        core_model::StudioNode::added(
            StudioNodeId::carried(Ulid::carried(id)),
            core_model::StudioNodeContent::Note {
                said: said.to_string(),
            },
            core_model::StudioPosition { x, y: 0 },
            at(1),
            core_model::StudioAuthor::Person,
        )
    };
    let first = node("01NOTE1", "The chip keeps its count", 0);
    let second = node("01NOTE2", "Overview says three", 200);
    let edge = core_model::StudioEdge::proposed(
        StudioEdgeId::carried(Ulid::carried("01EDGE")),
        first.id().clone(),
        second.id().clone(),
        StudioRelation::SameAs,
        at(2),
        core_model::StudioAuthor::Helm,
    )
    .expect("two nodes");
    StudioGraph {
        studio: core_model::Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried("armada")),
            name: None,
            named_by: None,
            created_at: at(0),
            touched_at: at(2),
        },
        nodes: vec![first, second],
        edges: vec![edge],
    }
}

#[test]
fn a_studio_round_trips_flat_and_an_untitled_one_sends_no_name() {
    let studio = Studio::of(&a_graph());
    let json = encode(&studio).expect("plain data");
    assert!(!json.contains("\"name\""), "left out, not null: {json}");
    assert!(
        json.contains(r#""kind":"note","said":"The chip keeps its count""#),
        "{json}"
    );
    assert!(json.contains(r#""position":{"x":200,"y":0}"#), "{json}");
    assert!(
        json.contains(r#""kind":"same_as","standing":"proposed""#),
        "{json}"
    );
    assert!(!json.contains("\"state\""), "a Note has none: {json}");
    assert!(json.contains(r#""added_by":"person""#), "{json}");
    assert!(
        json.contains(
            r#""standing":"proposed","created_at":"2026-09-17T09:02:00.000Z","added_by":"helm""#
        ),
        "{json}"
    );
    assert!(
        !json.contains("named_by"),
        "untitled, so nobody named it: {json}"
    );
    assert_eq!(
        decode::<Studio>("a Studio", json.as_bytes()).expect("round-trips"),
        studio
    );

    let named = StudioGraph {
        studio: core_model::Studio {
            name: StudioName::named("Stale counts"),
            ..a_graph().studio
        },
        ..a_graph()
    };
    assert_eq!(Studio::of(&named).name.as_deref(), Some("Stale counts"));
}

/// **`produced` is the Studio's to draw.** A proposal naming it is refused by
/// the decoder, so no route or daemon is asked.
#[test]
fn a_proposal_naming_the_produced_edge_does_not_decode() {
    let body = br#"{"from":"01A","to":"01B","kind":"produced"}"#;
    let refused = decode::<ProposeStudioEdge>("a proposed edge", body)
        .expect_err("not a relation that may be proposed");
    assert!(refused.to_string().contains("produced"), "{refused}");
    let body = br#"{"from":"01A","to":"01B","kind":"blocks"}"#;
    decode::<ProposeStudioEdge>("a proposed edge", body).expect("a relation");
}

#[test]
fn a_node_of_a_kind_the_studio_has_no_name_for_does_not_decode() {
    let body = br#"{"kind":"observation","said":"not a node"}"#;
    decode::<StudioNodeContent>("content", body).expect_err("not a kind");
}

/// **Helm's act is its own kind**, flat: which act beside the ids, and the
/// repository at the top level where a poll's tally reads it.
#[test]
fn helms_act_on_a_studio_is_its_own_kind_and_names_its_repository() {
    let acted = Event::StudioHelmActed(StudioHelmActed {
        studio_id: crate::StudioId::carried("01STUDIO"),
        manifest_id: crate::ManifestId::carried("armada"),
        act: HelmStudioAct::AddedNode {
            node_id: crate::StudioNodeId::carried("01NODE"),
        },
        at: Instant::from(&Timestamp::from_rfc3339("2026-09-17T09:00:00.000Z")),
    });
    let json = encode(&acted).expect("plain data");
    assert!(
        json.starts_with(r#"{"kind":"studio.helm_acted","studio_id":"01STUDIO""#),
        "{json}"
    );
    assert!(
        json.contains(r#""act":"added_node","node_id":"01NODE""#),
        "{json}"
    );
    assert_eq!(acted.kind(), "studio.helm_acted");
    assert_eq!(acted.about(), (None, Some("armada".to_string())));
    assert_eq!(
        decode::<Event>("an event", json.as_bytes()).expect("round-trips"),
        acted
    );
}
