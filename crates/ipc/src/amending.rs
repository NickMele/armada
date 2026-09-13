//! A form's edits to `armada.yml`, on their way to Fleet, and the file they
//! left — Journey 9, *Editing*, and `#721`.
//!
//! **Edits, not text.** A form knows which key it changed, and Fleet splices
//! exactly that key, so every untouched line stays byte for byte. A removed
//! entry takes the comment block directly above it.
//! The vocabulary is closed — no dotted path — so a form reaches only the keys
//! somebody decided it may. Clearing a list removes its key.
//!
//! **What a form produces always loads**, unlike [`SaveManifestFile`], which
//! writes work in progress on purpose.
//!
//! [`SaveManifestFile`]: crate::SaveManifestFile

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// Edits to the Manifest, and the text they were made against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditManifest {
    /// The `ManifestFile.text` the form was drawn from, whole. **Required**,
    /// for [`crate::SaveManifestFile::read`]'s reason: Fleet refuses where the
    /// disk no longer holds it, so the edits never land on a file the form did
    /// not show.
    pub read: String,
    /// Applied in order, each to what the one before it left.
    pub edits: Vec<ManifestEdit>,
}

/// One edit a form makes. **`null` on a `set_` removes the key**, and the key
/// is required so that an edit forgetting the field is refused rather than
/// read as a removal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "edit", rename_all = "snake_case")]
pub enum ManifestEdit {
    /// Declared last among the Checks, which is where it starts in the gate.
    AddCheck {
        name: String,
        check: CheckDraft,
    },
    RemoveCheck {
        name: String,
    },
    SetCheckRun {
        name: String,
        run: String,
    },
    SetCheckRequires {
        name: String,
        requires: Vec<String>,
    },
    SetCheckWhen {
        name: String,
        when: Vec<String>,
    },
    SetCheckNarrow {
        name: String,
        #[serde(deserialize_with = "stated")]
        narrow: Option<NarrowingDraft>,
    },
    AddCommand {
        name: String,
        command: CommandDraft,
    },
    RemoveCommand {
        name: String,
    },
    SetCommandRun {
        name: String,
        #[serde(deserialize_with = "stated")]
        run: Option<String>,
    },
    /// `false` removes a written `true`; absent already means `false`.
    SetCommandDestructive {
        name: String,
        destructive: bool,
    },
    SetCommandServe {
        name: String,
        #[serde(deserialize_with = "stated")]
        serve: Option<String>,
    },
    SetCommandReady {
        name: String,
        #[serde(deserialize_with = "stated")]
        ready: Option<String>,
    },
    SetCommandLinks {
        name: String,
        links: Vec<LinkDraft>,
    },
    AddPort {
        name: String,
        port: PortDraft,
    },
    RemovePort {
        name: String,
    },
    SetPortContainer {
        name: String,
        #[serde(deserialize_with = "stated")]
        container: Option<u32>,
    },
    SetPortEnv {
        name: String,
        #[serde(deserialize_with = "stated")]
        env: Option<String>,
    },
    /// The word as `armada.yml` writes it — `never`, `checks-pass`, `always`.
    /// `null` removes the key, which means `never`.
    SetAutoMerge {
        #[serde(deserialize_with = "stated")]
        auto_merge: Option<String>,
    },
    /// `human_always` or `auto_if_judge_passes`. `null` removes the key, which
    /// means `human_always`.
    SetReviewGate {
        #[serde(deserialize_with = "stated")]
        review_gate: Option<String>,
    },
    /// `drone.cost_cap_micros_per_job`. `null` defers to what Fleet runs with.
    SetCostCapMicrosPerJob {
        #[serde(deserialize_with = "stated")]
        cost_cap_micros_per_job: Option<u32>,
    },
    /// `drone.turn_cap_per_job`. `null` defers to what Fleet runs with.
    SetTurnCapPerJob {
        #[serde(deserialize_with = "stated")]
        turn_cap_per_job: Option<u32>,
    },
}

/// A Check a form declares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckDraft {
    pub run: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrow: Option<NarrowingDraft>,
}

/// `checks.<name>.narrow`, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrowingDraft {
    pub run: String,
    pub each: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub under: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
}

/// A Command a form declares. `serve` makes it a server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDraft {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(default)]
    pub destructive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ready: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkDraft>,
}

/// One address a server offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkDraft {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A port a form declares. Neither field is a port Armada places.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortDraft {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
}

/// What the edits left on disk.
///
/// **The text comes back**, where [`crate::ManifestSaved`] carries none: a form
/// that edits again needs the text its next edit starts from, and reading the
/// file again would race whatever lands next.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEdited {
    /// Spelled as `ManifestFile.path` spells it.
    pub path: String,
    /// When the bytes landed.
    pub at: Instant,
    /// The whole file as written — the next edit's `read`.
    pub text: String,
}

/// An `Option` whose key must be present, for `commanding`'s reason: a
/// missing key is a mistake, and `null` is a person clearing the value.
fn stated<'de, D, T>(input: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(input)
}
