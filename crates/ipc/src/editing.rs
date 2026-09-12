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
//! **A write over a file that moved is refused.** Nothing here destroys work
//! without saying so, and `armada.yml` is tracked — what a blind save clobbers
//! is somebody's committed edit. [`SaveManifestFile::read`] is the guard.
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
/// **No path, and no flag that would make this a staging or a commit** — the
/// act has one shape and a caller cannot ask for a second.
///
/// **Both fields are required, so there is no unguarded save to reach for.** A
/// caller that omits [`SaveManifestFile::read`] does not get an overwrite; it
/// gets a body that will not decode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveManifestFile {
    /// What the edit started from: [`ManifestFile::text`], unchanged.
    ///
    /// **The whole text and not a digest.** A digest needs an algorithm both
    /// sides agree on and a dependency neither has, to compare a few kilobytes
    /// — and it answers *probably the same* where this answers *the same*.
    ///
    /// Fleet compares it against the disk at the moment of the save and
    /// refuses where the two differ. A `git checkout` landing while the view is
    /// open is somebody's committed edit, and this is what stops the next Save
    /// taking it silently.
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
