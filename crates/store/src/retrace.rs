//! A step the loop's return went back past, entered again on the walk forward.
//!
//! Its own module, like `crate::limits` and `crate::shown_again`, so the one
//! migration it needs does not push `crate::attempt` past 500 lines. The
//! readings it keeps honest — the pass and the retry budget — stay there.

/// Version 58 — a step the loop's return went back past is entered again with
/// no emitter on its row.
///
/// `advanced -> running` had one walker, and [`V28`](crate::attempt::V28)
/// required `returned_by` on every row across it.
/// `core_model::StepTarget::Retraced` is the second: the walk forward over a
/// step that advanced on the pass before. The pass was charged to the emitter
/// at the return, so this row names nobody and `store::step_iteration` stays
/// one count per loop. **The column is now optional on that edge and still
/// refused on every other.** Nothing to backfill.
pub(crate) const V58: &str = r#"
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
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, or advances one that stopped, and the step that routed it only where it is a loop return, or a drone arriving on a step or leaving it beneath one');
END;
"#;
