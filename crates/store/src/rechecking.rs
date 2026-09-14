//! A person running a stopped step's Checks again, as the log records it.
//!
//! Its own module for `crate::retrace`'s reason. The readings it keeps honest —
//! a step's runs and its retry budget — stay in `crate::attempt`.

/// Version 74 — a stopped step re-entered for a re-run of its Checks carries
/// the trigger it stopped on.
///
/// `core_model::StepTarget::Rechecking` crosses `stopped -> running` as a
/// restart does, and no Drone works it. The trigger is what tells the two rows
/// apart, so `attempt_now` and `spent_now` leave this one out. **A trigger is
/// now optional on that edge and still refused on every other arrival at
/// `running`.** Nothing to backfill.
pub(crate) const V74: &str = r#"
DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND (CASE WHEN NEW.state_to IN ('stopped', 'retrying')
                    OR (NEW.state_to = 'advanced' AND NEW.state_from = 'stopped')
                  THEN NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL
                  WHEN NEW.state_to = 'running' AND NEW.state_from = 'stopped'
                  THEN (NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
                    OR (NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL)
                  ELSE NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL
             END)
        AND (NEW.returned_by IS NULL
             OR (NEW.state_from = 'advanced' AND NEW.state_to = 'running')))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NOT NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, advances one that stopped, or re-enters one that stopped to run its Checks again, and the step that routed it only where it is a loop return, or a drone arriving on a step or leaving it beneath one');
END;
"#;
