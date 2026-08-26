//! What a Job record calls things: the ids that name records, and the string
//! newtypes that name a step, a criterion, a path, a model — and the Job
//! itself.
//!
//! Split from [`fields`](crate::job::fields) along the heading that module's
//! own doc comment already drew. What is here names something; what is left
//! there is the field vocabulary — closed enums and their wire mappings, which
//! stay beside the variants they map.
//!
//! # Ids are newtypes over the log envelope's `Ulid`
//!
//! Not aliases. `redispatched_from` takes a [`JobId`] and cannot be handed a
//! [`ManifestId`], which costs one wrapper and removes a whole class of call.
//! The inner [`Ulid`] is the envelope's, so an id put on a log line is the same
//! value the record holds rather than a parallel vocabulary.

use core::fmt;

use alloc::string::String;

use crate::envelope::Ulid;

/// Declare an id newtype over [`Ulid`]. Ten lines each, written once.
macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Ulid);

        impl $name {
            /// Carry an id something else minted. Fleet is the sole authority
            /// for the ids that name records; nothing here mints one.
            pub fn carried(id: Ulid) -> Self {
                $name(id)
            }

            pub fn as_ulid(&self) -> &Ulid {
                &self.0
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

id_newtype! {
    /// The `jobs` row's own key, and the `job_id` half of `job_steps`'
    /// composite key.
    ///
    /// **`job-fields.toml` has no row for it.** It is carried because
    /// `(job_id, step_id)` presupposes it and every other reference on the
    /// record points at one.
    JobId
}
id_newtype! {
    /// The Drone working a Job. Presence, not state.
    DroneId
}
id_newtype! {
    /// A Manifest — the project a Job belongs to, or one that gates it.
    ManifestId
}
id_newtype! {
    /// The WorkflowDef a Job follows. Not `task_type`: `task` is a banned
    /// synonym for Job, and this is a pointer to a row rather than a closed set.
    WorkflowId
}

/// A step's identifier, from the WorkflowDef and never generated.
///
/// The same value the log envelope carries as `step_id`, which is why it is a
/// string rather than an id: the WorkflowDef author writes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(String);

impl StepId {
    pub fn new(id: impl Into<String>) -> Self {
        StepId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An acceptance criterion's frozen identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CriterionId(String);

impl CriterionId {
    pub fn new(id: impl Into<String>) -> Self {
        CriterionId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A repo-relative path a Job intends to write.
///
/// A string rather than a `PathBuf` because this crate is `no_std` and because
/// the value is a declaration recorded on a record, not a handle to a file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(String);

impl RepoPath {
    pub fn new(path: impl Into<String>) -> Self {
        RepoPath(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which model the assigned Drone is using.
///
/// A string on purpose: the adapter passes one on every spawn, and naming a
/// closed set here would put a vendor's vocabulary under every crate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelName(String);

impl ModelName {
    pub fn new(name: impl Into<String>) -> Self {
        ModelName(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The name a person reads in a list row. **Never blank.**
///
/// [`Title::new`] is the only way to make one and it refuses a string that is
/// empty or only whitespace, so a Job with no readable name does not exist to
/// be rendered. The alternative — a `String` field plus a check somewhere on
/// the write path — is a check every new caller has to remember, and `""` is a
/// title to the type system and not one to a reader.
///
/// Not [`StepId`] or [`ModelName`]: those name something a machine matches on
/// and a blank one is merely wrong. This is the only string on the record whose
/// whole purpose is being read by a person, which is why it is the only one
/// that refuses.
///
/// The value is stored trimmed. `"  fix the parser  "` and `"fix the parser"`
/// are the same name, and a list cell that begins with three spaces is noise no
/// surface asked for.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Title(String);

impl Title {
    /// A title, or the refusal. Trims first, then refuses what is left of it.
    pub fn new(text: &str) -> Result<Title, BlankTitle> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(BlankTitle);
        }
        Ok(Title(String::from(trimmed)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A title that was blank, or was nothing but whitespace.
///
/// One variant, because there is one way to fail: the field is a name and the
/// only thing that is not a name is the absence of one. A length ceiling is not
/// here — nothing on the wire or in the schema bounds it, and inventing a
/// number would put a limit in the type that no document names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlankTitle;

impl fmt::Display for BlankTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a title is the name a person reads in a list, and blank is not one")
    }
}

impl core::error::Error for BlankTitle {}
