//! The Manifest file itself: its text, and a corrected text on its way back.
//!
//! **The half a person acts with**, where [`ManifestReading`] is the half that
//! reports. Journey 9, *Editing*, is what both answer to.
//!
//! **Fleet resolves the path and nothing here carries one.** A Fleet serves one
//! repository and already holds the file it was started against, so a caller
//! names the act and Fleet names the file.
//!
//! **A write that does not parse is still a write.** Refusing invalid YAML
//! would mean work in progress cannot be saved, which is the editor this exists
//! to stop sending people to. The bytes go to disk, Fleet's watch re-reads
//! them, and [`ManifestReading`] reports what it could not adopt.
//!
//! [`ManifestReading`]: crate::ManifestReading

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// `armada.yml` as it is on disk, for the view that draws it.
///
/// **The bytes, not a document.** Nothing here is parsed, so a file that does
/// not parse reads back as well as one that does — which is the case a person
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
}

/// A corrected Manifest, on its way to disk.
///
/// **Text and nothing else.** No path, for this module's reason, and no flag
/// that would make this a staging or a commit — the act has one shape and a
/// caller cannot ask for a second.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveManifestFile {
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
