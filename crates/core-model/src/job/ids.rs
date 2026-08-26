//! What a Job record calls things: the ids that name records, and the string
//! newtypes that name a step, a criterion, a path or a model.
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
