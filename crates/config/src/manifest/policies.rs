//! `auto_merge:` and `review_gate:`, the two keys a `manifest_rule:<key>` gate
//! resolves against.
//!
//! **A file of its own for [`super::drone`]'s reason**, and one more: these are
//! dials rather than registries, and they are the only two top-level keys whose
//! value is a closed word list. `#525`.
//!
//! **Undotted and top level, because that is what the gate names.** A step
//! declares `manifest_rule:auto_merge` and this reads the key it names;
//! `crate::workflow::step` refuses any other key by name.

use core_model::{AutoMerge, ReviewGate};

use crate::error::Refusal;
use crate::yaml::{self, Table};

/// The values `auto_merge` takes. **Every one is carried**, so
/// [`yaml::word`]'s deferred-value arm is unreachable and both lists it is
/// given are the same set — the honest shape when nothing is deferred.
const AUTO_MERGE_WORDS: &[(&str, AutoMerge)] = &[
    ("never", AutoMerge::Never),
    ("tests-pass", AutoMerge::TestsPass),
    ("always", AutoMerge::Always),
];
/// `auto_merge`'s set, spelled for a refusal.
const AUTO_MERGE_LEGAL: &[&str] = &["never", "tests-pass", "always"];
/// The values `review_gate` takes, for [`AUTO_MERGE_WORDS`]' reason.
const REVIEW_GATE_WORDS: &[(&str, ReviewGate)] = &[
    ("human_always", ReviewGate::HumanAlways),
    ("auto_if_judge_passes", ReviewGate::AutoIfJudgePasses),
];
/// `review_gate`'s set, spelled for a refusal.
const REVIEW_GATE_LEGAL: &[&str] = &["human_always", "auto_if_judge_passes"];

/// What the two keys came to.
///
/// **Absent is the policy's own default and never a refusal.** Both are
/// `Manifest only` in `crates/config/settings.toml`, so a file that says
/// nothing has no tier above it to defer to — it means `never` and
/// `human_always`, which is what every `armada.yml` written before these keys
/// existed already meant.
///
/// A refused *value* reads as absent from here, which is safe for
/// [`super::drone`]'s reason: the refusal is already in `out`, and a file with
/// any refusal in it does not load at all.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> (AutoMerge, ReviewGate) {
    let auto_merge = top
        .optional("auto_merge")
        .and_then(|value| {
            yaml::word(
                "auto_merge",
                value,
                AUTO_MERGE_WORDS,
                AUTO_MERGE_LEGAL,
                AUTO_MERGE_LEGAL,
                out,
            )
        })
        .unwrap_or_default();
    let review_gate = top
        .optional("review_gate")
        .and_then(|value| {
            yaml::word(
                "review_gate",
                value,
                REVIEW_GATE_WORDS,
                REVIEW_GATE_LEGAL,
                REVIEW_GATE_LEGAL,
                out,
            )
        })
        .unwrap_or_default();
    (auto_merge, review_gate)
}
