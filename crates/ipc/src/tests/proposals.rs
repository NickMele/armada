//! What a proposal must and must not decode into.
//!
//! **The one DTO on this seam a peer *writes*.** Everything else here is read
//! by Bridge and written by Fleet, and the cases in `mod` hold what survives a
//! round trip. These hold the opposite property, which is why they are their
//! own file: `ProposeJob` arrives from outside, so what matters is what it
//! refuses — an untitled Job, a spelling the registry does not have, an origin
//! only Fleet may mint — and what it tolerates, which is every field a newer
//! peer might add.
//!
//! Split out of `mod` when that file reached 900 lines, along the seam its own
//! header already names.

use crate::{decode, AttachmentRef, ProposeJob};

/// The field is required on the wire, not merely expected: a proposal that
/// omits it does not become a `ProposeJob` at all, so nothing downstream has to
/// decide what an untitled Job is called.
#[test]
fn a_proposal_with_no_title_does_not_decode() {
    let body = br#"{"workflow_id":"01WF","owner_manifest_id":"01MF","origin":"manual",
        "urgency":"normal","atomic":false,"model":"a-model"}"#;
    let refused = decode::<ProposeJob>("proposal", body).expect_err("a Job has a title");
    assert!(refused.to_string().contains("title"), "{refused}");
}

#[test]
fn a_spelling_the_registry_does_not_have_is_refused() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF","origin":"manual",
        "urgency":"whenever","atomic":false,"model":"a-model"}"#;
    let refused = decode::<ProposeJob>("proposal", body).expect_err("`whenever` is not an urgency");
    assert!(refused.to_string().contains("whenever"));
}

#[test]
fn a_proposal_cannot_claim_to_be_sub_dispatched() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF","origin":"sub_dispatched",
        "urgency":"normal","atomic":false,"model":"a-model"}"#;
    let refused = decode::<ProposeJob>("proposal", body)
        .expect_err("a peer does not create a sub-dispatched Job");
    assert!(refused.to_string().contains("sub_dispatched"));
}

#[test]
fn an_unknown_field_parses_and_is_ignored() {
    // The minor-skew row in one assertion: a newer peer adds a field, an older
    // peer reads the message anyway. `deny_unknown_fields` would fail here.
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF","origin":"manual",
        "urgency":"normal","atomic":false,"model":"a-model","dispatch_budget":12}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("unknown fields are ignored");
    assert_eq!(proposal.model.as_deref(), Some("a-model"));
    assert!(proposal.acceptance_criteria.is_empty());
}

/// **Absent is the ordinary case.** A caller with no opinion about the model
/// sends nothing, and Fleet fills the value in from configuration — which is
/// why the field is optional rather than required-and-emptyable. The empty
/// string still decodes, because a DTO is deserialised rather than constructed;
/// it is refused where text becomes a Job.
#[test]
fn a_proposal_may_name_no_model_at_all() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF",
        "origin":"manual","urgency":"normal","atomic":false}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("a model is optional");
    assert_eq!(proposal.model, None);
}

/// **Additive, like `model`.** A proposal that predates this field carries no
/// `attachments` key at all, and `#[serde(default)]` is what lets it still
/// decode — the minor bump this field cost rests on exactly this.
#[test]
fn a_proposal_with_no_attachments_key_still_decodes() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF",
        "origin":"manual","urgency":"normal","atomic":false}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("attachments default to none");
    assert!(proposal.attachments.is_empty());
}

/// A staged file crosses as a path, never as bytes — the same same-machine
/// assumption `write_targets` already rests on.
#[test]
fn a_proposal_carries_the_staged_files_a_person_attached() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF",
        "origin":"manual","urgency":"normal","atomic":false,
        "attachments":[{"staged_path":"/tmp/armada-attachments/01/before.png",
        "filename":"before.png","mime_type":"image/png"}]}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("attachments decode");
    assert_eq!(
        proposal.attachments,
        vec![AttachmentRef {
            staged_path: "/tmp/armada-attachments/01/before.png".to_string(),
            filename: "before.png".to_string(),
            mime_type: "image/png".to_string(),
        }]
    );
}
