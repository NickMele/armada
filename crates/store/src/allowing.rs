//! How a Job meets a command its Drone was not granted, and the commands a
//! person allowed it.

/// Version 44 — how a Job meets a blocked command, and what a person allowed.
///
/// Beside the change it makes, like [`V43`](crate::shown_again::V43):
/// `schema.rs` is at the 900 lines the gate refuses at.
///
/// **The `DEFAULT` is the backfill**: a Job written before this reads as
/// `refuse_and_hold`, which is what a Job that chose nothing is. The table
/// starts empty, which is what a Job nobody allowed anything reads as.
///
/// `by` is quoted because it is an SQL keyword, and the key is the pair so a
/// second allow of one command is a conflict rather than a second row.
pub(crate) const V44: &str = r#"
ALTER TABLE jobs ADD COLUMN when_blocked TEXT NOT NULL DEFAULT 'refuse_and_hold';

CREATE TABLE job_allowed_commands (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    run        TEXT NOT NULL,
    reach      TEXT NOT NULL,
    allowed_at TEXT NOT NULL,
    "by"       TEXT NOT NULL,
    PRIMARY KEY (job_id, run)
) STRICT;
"#;
