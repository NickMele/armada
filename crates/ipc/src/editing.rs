//! The Manifest file itself: its text, and a corrected text on its way back.
//!
//! **The half a person acts with**, where [`ManifestReading`] is the half that
//! reports. Journey 9, *Editing*, is what both answer to.
//!
//! **Fleet resolves the path and nothing here carries one.** A Fleet serves one
//! repository and already holds the file it was started against, so a caller
//! names the act and Fleet names the file.
//!
//! **A write that does not parse is still a write**, and [`ManifestReading`]
//! reports what Fleet could not adopt. **A write over a file that moved is
//! refused** — `armada.yml` is tracked, so a blind save clobbers a committed
//! edit. [`SaveManifestFile::read`] is the guard.
//!
//! [`ManifestReading`]: crate::ManifestReading

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// `armada.yml` as it is on disk, for the view that draws it.
///
/// **The bytes first.** A file that does not parse reads back as well as one
/// that does — which is the case a person
/// opening this is most likely to be in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    /// The file, as Fleet resolved it — **the same string
    /// [`ManifestReading::path`](crate::ManifestReading::path) carries**, so a
    /// surface holding both can see they are about one file. It is also what
    /// names the toggle that reveals this view, the lexicon having banned
    /// naming a Manifest by its format.
    pub path: String,
    /// The whole file. **Not windowed**, where `get_check_output` serves a tail
    /// and says so: a partial text handed to an editor would be saved back over
    /// the rest.
    pub text: String,
    /// What `text` loads as, for the forms. **Absent where it does not load**,
    /// which the file view is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<crate::ManifestDeclared>,
}

/// A corrected Manifest, on its way to disk. **Both fields are required**, so
/// omitting [`SaveManifestFile::read`] fails to decode rather than overwriting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveManifestFile {
    /// What the edit started from — [`ManifestFile::text`], unchanged, and
    /// whole rather than a digest, which answers *probably the same*. Fleet
    /// refuses where the disk no longer holds it.
    pub read: String,
    /// The bytes to put on disk, exactly as they arrive.
    pub text: String,
}

/// What a save left on disk, and when.
///
/// **Not a reading, and deliberately not one.** Fleet's watch settles before it
/// re-reads, so what a save moved is not known when the write returns — and an
/// answer that guessed would restate a judgement
/// [`ManifestReading::worth_saying`](crate::ManifestReading::worth_saying)
/// already makes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSaved {
    /// The file written, spelled as [`ManifestFile::path`] spells it.
    pub path: String,
    /// When the bytes landed — **the write's instant, not a read's**. The
    /// reading that follows carries its own `at`, later by up to the settle
    /// window, which is what tells the answer to this save from a reading that
    /// was already on screen.
    pub at: Instant,
}
