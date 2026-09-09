//! Turning what a person said into the Job they meant.
//!
//! **A second way in, never a second key.** The ULID stays the `jobs` row's own
//! key and every join is still written in terms of it; this is three indexed
//! lookups that all end at one, so nothing downstream learns that a handle
//! exists. `#564` minted the handle and Bridge shows it as the id to copy, and
//! until this there was nothing anywhere that turned one back into a Job.
//!
//! **The number counts within a Manifest, so resolving one needs a Manifest.**
//! `jobs_number_within_a_manifest` is what makes the answer single, and a
//! resolver handed no Manifest refuses rather than picking a *job 1* out of the
//! several a store may hold. That refusal is [`ResolveJobError::NoManifest`]
//! and it is not a gap to close later: a resolver that guesses which repository
//! a number belongs to is worse than one that will not.
//!
//! **The slug is checked and never searched on.** A handle's number is what
//! finds the row; the title on that row is put back through
//! [`handle_of`](core_model::handle_of) and compared, so a handle carrying some
//! other repository's words names nothing rather than the Job that happens to
//! share its number.

use core_model::{handle_of, JobId, JobNumber, JobReference, ManifestId, Title, Ulid};

use crate::error::{fault, DatabaseFault};
use crate::open::Store;

/// The Job a reference named, and what it is called.
///
/// **Both, because the caller needs both.** The id is what every query joins
/// on; the handle is what names this Job's worktree, its branch and every path
/// under `.armada/`, and a caller that had to load the Job again to derive it
/// would read the row twice for facts one read already had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedJob {
    pub job_id: JobId,
    pub handle: String,
}

/// Why a reference named no Job.
///
/// **Three answers and not one.** "No such Job" sends a caller to check what it
/// typed; "no Manifest" tells it the reference was fine and the scope was
/// missing; a database fault is neither and is worth retrying. A single
/// not-found would send two of the three to the wrong place.
#[derive(Debug)]
pub enum ResolveJobError {
    Database(DatabaseFault),
    /// Nothing in the store answers to this. The text is carried as it was
    /// given, because a refusal that does not quote it is one a person cannot
    /// act on.
    NoSuchJob {
        named: String,
    },
    /// A number or a handle, with no Manifest to count it within. See the
    /// module comment: this is refused on purpose and stays refused.
    NoManifest {
        named: String,
    },
}

impl core::fmt::Display for ResolveJobError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ResolveJobError::Database(fault) => write!(out, "{fault}"),
            ResolveJobError::NoSuchJob { named } => write!(out, "no Job is `{named}`"),
            ResolveJobError::NoManifest { named } => write!(
                out,
                "`{named}` counts within a Manifest and none was given, so it names \
                 no one Job"
            ),
        }
    }
}

impl std::error::Error for ResolveJobError {}

impl Store {
    /// The Job a ULID, a whole handle or a bare number names.
    ///
    /// **`within` is the Manifest a number counts inside**, and `None` is a
    /// caller that holds none — which resolves an id and refuses the other two.
    /// Fleet serves one repository and passes its own; nothing here guesses.
    pub fn resolve_job(
        &self,
        reference: &JobReference,
        within: Option<&ManifestId>,
    ) -> Result<NamedJob, ResolveJobError> {
        match reference {
            JobReference::Id(job_id) => self.named_by_id(job_id),
            JobReference::Number(number) => self.named_by_number(*number, within, reference),
            JobReference::Handle { number, said } => {
                let found = self.named_by_number(*number, within, reference)?;
                // The number found the row; this is what says the row is the
                // one the words named. See the module comment.
                match &found.handle == said {
                    true => Ok(found),
                    false => Err(ResolveJobError::NoSuchJob {
                        named: reference.to_string(),
                    }),
                }
            }
        }
    }

    /// The row's own key, confirmed to be a row.
    ///
    /// **Confirmed rather than trusted**, because every other form comes back
    /// having proved the Job exists and a caller cannot be given one answer
    /// that means "it is there" and another that means "it is spelled like an
    /// id".
    fn named_by_id(&self, job_id: &JobId) -> Result<NamedJob, ResolveJobError> {
        self.named_by(
            "SELECT job_id, number, title FROM jobs WHERE job_id = ?1",
            rusqlite::params![job_id.as_str()],
            job_id.as_str(),
        )
    }

    fn named_by_number(
        &self,
        number: JobNumber,
        within: Option<&ManifestId>,
        reference: &JobReference,
    ) -> Result<NamedJob, ResolveJobError> {
        let Some(manifest) = within else {
            return Err(ResolveJobError::NoManifest {
                named: reference.to_string(),
            });
        };
        self.named_by(
            "SELECT job_id, number, title FROM jobs \
             WHERE owner_manifest_id = ?1 AND number = ?2",
            rusqlite::params![manifest.as_str(), number.get()],
            &reference.to_string(),
        )
    }

    /// One row, read as an id and the handle derived from the two frozen
    /// columns beside it.
    ///
    /// **A row that will not read is not a Job**, and is answered as one that
    /// is not there. `number` and `title` have been `NOT NULL` since the
    /// migration that added them, so the only way here is a store somebody has
    /// written into by hand — and the alternative, a fourth error variant
    /// meaning "it is there and will not read", is one no caller of a resolver
    /// can do anything different about.
    fn named_by(
        &self,
        query: &str,
        binding: &[&dyn rusqlite::ToSql],
        named: &str,
    ) -> Result<NamedJob, ResolveJobError> {
        let found = self
            .conn
            .query_row(query, binding, |row| {
                Ok((
                    row.get::<_, String>("job_id")?,
                    row.get::<_, i64>("number")?,
                    row.get::<_, String>("title")?,
                ))
            })
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(ResolveJobError::Database(fault("resolving a job")(other))),
            })?;
        let missing = || ResolveJobError::NoSuchJob {
            named: named.to_string(),
        };
        let (job_id, number, title) = found.ok_or_else(missing)?;
        let title = Title::new(&title).map_err(|_| missing())?;
        Ok(NamedJob {
            job_id: JobId::carried(Ulid::carried(job_id)),
            handle: handle_of(JobNumber::carried(number as u32), &title),
        })
    }
}
