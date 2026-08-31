//! The Drone pointer, where it now lives: one column per step.
//!
//! `jobs.assigned_drone` was one nullable pointer per Job, and it went null
//! when the last Drone exited — taking with it the only record of which Drone
//! had worked which step. A finished Job could not name the transcripts of the
//! Drones that ran it. The pointer is a `job_steps` column now, and the drone
//! rows in `job_events` carry the step they are about, so every Drone that
//! ever worked a Job is reachable from the step it worked.
//!
//! **The column is still a cache of the fold**, in exactly the sense
//! `job_steps.state` is: `crate::read` rebuilds every step row from those log
//! rows and does not read this column back.

/// Version 19 — a Drone belongs to a step.
///
/// Beside the change it makes, like [`V17`](crate::report::V17) and
/// [`V18`](crate::plan::V18): `schema.rs` is at the 900 the gate refuses at.
///
/// | What moves | Why |
/// |---|---|
/// | `job_steps.assigned_drone` | The pointer, one per step instead of one per Job |
/// | `job_events.step_id` on drone rows | The fold has to know which row's pointer a move is about; without it a Job that ran four Drones collapses them into one |
/// | `jobs.assigned_drone` | Dropped. A second statement of one fact is a pair that can disagree, and the Job-level reading is derived from these rows |
///
/// The shape trigger is rewritten whole because `SQLite` cannot alter one, and
/// the drone arm is what changes: `step_id` was required to be null and is now
/// required present. `job_events_are_never_edited` comes off for one statement
/// and goes straight back on — a column added to rows already written cannot be
/// filled without lifting it, and lifting it inside the migration list is that
/// edit being versioned rather than hidden in a query.
///
/// **The step is derived, not invented.** `fleet::spawning` moves a step to
/// `running` before a process exists, so the step in force at a `drone_spawned`
/// is the last `step_transition` to `running` before it, and an exit follows
/// its own spawn — `V5`'s rule, backfill only what is observed. A row with no
/// earlier one keeps a null `step_id`, which the fold refuses as it refuses a
/// null `workflow`. Nothing backfills `job_steps.assigned_drone`: it is a cache
/// the fold fills.
pub(crate) const V19: &str = r#"
ALTER TABLE job_steps ADD COLUMN assigned_drone TEXT;

DROP TRIGGER job_events_are_never_edited;

UPDATE job_events SET step_id = (
    SELECT earlier.step_id FROM job_events AS earlier
    WHERE earlier.job_id = job_events.job_id
      AND earlier.kind = 'step_transition'
      AND earlier.state_to = 'running'
      AND earlier.seq < job_events.seq
    ORDER BY earlier.seq DESC
    LIMIT 1
)
WHERE kind IN ('drone_spawned', 'drone_exited');

CREATE TRIGGER job_events_are_never_edited
BEFORE UPDATE ON job_events
BEGIN
    SELECT RAISE(ABORT, 'job_events is append-only: a recorded transition is never edited');
END;

DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND (CASE WHEN NEW.state_to IN ('stopped', 'retrying')
                    OR (NEW.state_to = 'advanced' AND NEW.state_from = 'stopped')
                  THEN NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL
                  ELSE NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL
             END))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NOT NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, or advances one that stopped, or a drone arriving on a step or leaving it beneath one');
END;

ALTER TABLE jobs DROP COLUMN assigned_drone;
"#;
