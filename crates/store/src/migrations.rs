//! The migration list, and the two constants that name where a file stands
//! against it.
//!
//! **Moved out of `schema.rs`, faithfully.** That file holds `V1`..`V16` and
//! sits at the 900 lines the gate refuses at — see its own header, and
//! [`crate::ports::V44`]'s. `MIGRATIONS`, [`KNOWN_SCHEMA_VERSION`],
//! [`SCHEMA_VERSION_KEY`] and [`tables_pointing_at_a_job`] behave exactly as
//! they did there; nothing about what they do changed, only where they live.
//! `V1`..`V16` stay in `schema.rs`, `pub(crate)` so this module can name them
//! the same way it already names every later migration's own module.

use rusqlite::Connection;

use crate::schema::{V1, V10, V11, V12, V13, V14, V15, V16, V2, V3, V4, V5, V6, V7, V8, V9};

/// The key under which [`MIGRATIONS`]' applied count is recorded.
pub const SCHEMA_VERSION_KEY: &str = "schema_version";

/// How many migrations this build knows. A file recording more than this was
/// written by a newer Armada, and is refused rather than read with the older
/// crate's assumptions.
pub const KNOWN_SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// Applied in order. Index `n` takes a file from version `n` to `n + 1`.
///
/// **Nothing is ever edited here.** Changing entry zero changes what an already
/// migrated file is assumed to contain, which is the one thing the version
/// number exists to stop.
pub const MIGRATIONS: &[&str] = &[
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    V16,
    // Beside the table each creates, because this file is at the 900 lines the
    // gate refuses at. The order lives here, and may not be anywhere else.
    crate::report::V17,
    crate::plan::V18,
    crate::drone::V19,
    crate::note::V20,
    crate::delivery::V21,
    crate::judged::V22,
    crate::spend::V23,
    crate::gaming::V24,
    crate::footprint::V25,
    crate::delivery::V26,
    crate::process::V27,
    crate::attempt::V28,
    crate::proving::V29,
    crate::judged::V30,
    crate::remarks::V31,
    crate::spend::V32,
    crate::spend::V33,
    crate::proposing::V34,
    crate::judged::V35,
    crate::numbering::V36,
    crate::showing::V37,
    crate::retain::V38,
    crate::gaming::V39,
    crate::spend::V40,
    crate::showing::V41,
    crate::showing::V42,
    crate::shown_again::V43,
    crate::allowing::V44,
];

/// Every table whose rows belong to one Job, asked of the file rather than
/// listed here.
///
/// [`forget_job`](crate::Store::forget_job) deletes from each. It asks because
/// a list is a thing somebody has to remember to extend, and three times in a
/// row nobody did: `job_step_judgments`, `job_step_gaming_flags` and
/// `job_step_evidence` arrived in [`V8`], [`V12`] and [`V10`] and none of them
/// reached the delete. Nothing failed while no shipped workflow declared a
/// `judge_check` and the tables stayed empty; the day nine of them went live,
/// every Job that reached a gate became one `armada clean` could not forget,
/// because the `jobs` row it deletes first is the parent those rows point at.
/// A catalog cannot fall behind the schema it is the schema of.
///
/// **The file's catalog, not the constants above it.** [`V13`] builds four
/// tables under `_wide` names and renames them over the originals, so the
/// `CREATE TABLE` text in that module names four tables no migrated file has
/// and misses the four every one of them does.
///
/// `job_events` is in this set and belongs there — it is append-only by
/// trigger only while its Job row still exists, which by then it does not. See
/// [`V4`], and the ordering it forces on `forget_job`.
pub(crate) fn tables_pointing_at_a_job(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    conn.prepare(
        r#"SELECT m.name
           FROM sqlite_master AS m
           JOIN pragma_foreign_key_list(m.name) AS fk
           WHERE m.type = 'table' AND fk."table" = 'jobs'
           GROUP BY m.name
           ORDER BY m.name"#,
    )?
    .query_map([], |row| row.get(0))?
    .collect()
}
