//! What a Job's Drones have cost it: one row per Drone, summed per Job.
//!
//! A Drone belongs to a step, so a four-step Job is four Drones that never
//! meet, and nothing added up what they spent. The Job is what a cap is set
//! against — `docs/concepts/machine.md`, Budget — so the sum has to outlive
//! every Drone of it, which means the record.
//!
//! # The key is the Drone, and that is the whole of the idempotence
//!
//! A Drone's exit is folded at two sites in `fleet`, and a per-Job counter
//! incremented at both would bill one Drone twice.
//! [`Store::record_drone_spend`] is an upsert on `(job_id, drone_id)`, so
//! recording the same Drone twice writes the same row twice. There is no
//! arrangement of calls that inflates the figure, and no convention a future
//! caller has to know.
//!
//! # Cost and turns fold differently from one event, and it is measured
//!
//! `docs/spikes/004-transcript-idle-session.ndjson` is one session with two
//! terminating lines: `num_turns` reads 3 then 2, while the second line's
//! `modelUsage` holds the sum of both and its `total_cost_usd` reconstructs
//! from that sum exactly. So the cost is the session's running total and the
//! turns are per invocation. `fleet::allowance::spent` does the fold: last
//! cost, summed turns.

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// Version 23 — what each Drone of a Job spent.
///
/// Beside the change it makes, like [`V22`](crate::judged::V22):
/// `schema.rs` is at the 900 the gate refuses at.
///
/// **A table and not columns on `jobs`**, which is the opposite call to
/// [`crate::delivery`]'s and for the reason that one gives: delivery is at most
/// one row per Job, written once. This is one row per Drone, and a Job has as
/// many Drones as its workflow has steps.
///
/// **No event, and nothing folded.** A spend is not a move — no status
/// changes, no step changes — so there is nothing for `job_events` to carry
/// and `crate::read` has nothing to rebuild. The table is the authority for its
/// own figures, as [`crate::note`]'s column is for the note.
///
/// Nothing is backfilled. A Job that ran before this spent what it spent and
/// the figure was thrown away; an empty set of rows and a Job that has not
/// started yet are the same answer, which is a total of nothing.
pub(crate) const V23: &str = r#"
CREATE TABLE job_drone_spend (
    job_id      TEXT    NOT NULL REFERENCES jobs(job_id),
    drone_id    TEXT    NOT NULL,
    cost_micros INTEGER NOT NULL,
    turns       INTEGER NOT NULL,
    ran_ms      INTEGER NOT NULL,
    PRIMARY KEY (job_id, drone_id)
) STRICT;
"#;

/// What one Drone's run came to.
///
/// **Three counts and no verdict.** Whether the run went well is the gate's,
/// decided from evidence — this is what it cost to find out, and it is the
/// same three numbers whether the Drone succeeded, was refused everything, or
/// vanished.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DroneSpend {
    /// Millionths of a dollar, as `DroneEvent::Ended` carries it. An integer
    /// because a cap compared against a float is a cap that answers
    /// differently on two machines.
    pub cost_micros: u64,
    /// How many turns the Drone took, summed across every terminating line of
    /// its stream. See this module's note on why cost is not summed with it.
    pub turns: u64,
    /// How long the Drone was held, in milliseconds. Fleet's own clock rather
    /// than anything on the stream: the harness reports a duration per
    /// terminating line and Armada does not carry it, and what a person means
    /// by how long a step took is the wall clock either way.
    pub ran_ms: u64,
}

/// What a Job's Drones have come to together.
///
/// **`drones` is carried rather than derived by a caller counting rows**,
/// because there is no other way to tell a Job that has spent nothing from a
/// Job that has not run: both have a cost of zero, and only one of them has a
/// Drone behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spend {
    pub cost_micros: u64,
    pub turns: u64,
    pub ran_ms: u64,
    /// How many Drones this is the sum of.
    pub drones: u64,
}

impl Store {
    /// Write what one Drone of a Job spent.
    ///
    /// **An upsert on the Drone, so calling it twice is calling it once.**
    /// That is the property the two exit paths rest on — see this module's
    /// note. A caller that records the same Drone again with a larger figure
    /// replaces the smaller one, which is the right answer for a stream that
    /// was read further the second time.
    pub fn record_drone_spend(
        &mut self,
        job_id: &core_model::JobId,
        drone_id: &core_model::DroneId,
        spend: &DroneSpend,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_drone_spend \
                 (job_id, drone_id, cost_micros, turns, ran_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT (job_id, drone_id) DO UPDATE SET \
                 cost_micros = excluded.cost_micros, \
                 turns = excluded.turns, \
                 ran_ms = excluded.ran_ms",
                (
                    job_id.as_str(),
                    drone_id.as_str(),
                    spend.cost_micros as i64,
                    spend.turns as i64,
                    spend.ran_ms as i64,
                ),
            )
            // **Read off the error's own code and never off its message**,
            // which is `fleet::refusing`'s rule applied one layer down: a
            // string match would make this mapping depend on SQLite's wording.
            // The foreign key is on and `open.rs` turns it on, so a Job that
            // has been forgotten refuses the write here rather than leaving an
            // orphan row.
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("recording what a Drone spent")(other)),
            })?;
        Ok(())
    }

    /// What every Drone of this Job has spent, added up.
    ///
    /// **A Job with no rows answers zero of everything**, which is every Job
    /// that has not started and every Job that finished before version 23. A
    /// caller that needs to tell those apart reads [`Spend::drones`].
    pub fn spend_for(&self, job_id: &core_model::JobId) -> Result<Spend, LoadJobError> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(cost_micros), 0), COALESCE(SUM(turns), 0), \
                 COALESCE(SUM(ran_ms), 0), COUNT(*) \
                 FROM job_drone_spend WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok(Spend {
                        cost_micros: row.get::<_, i64>(0)? as u64,
                        turns: row.get::<_, i64>(1)? as u64,
                        ran_ms: row.get::<_, i64>(2)? as u64,
                        drones: row.get::<_, i64>(3)? as u64,
                    })
                },
            )
            .map_err(|why| {
                LoadJobError::Unreadable(RowError::Database(fault("reading what a Job spent")(why)))
            })
    }
}
