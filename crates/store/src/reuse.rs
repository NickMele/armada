//! Whether a step's Check result came from a Drone's own dry run. `#1014`.

/// Version 69 — a Check's row says when it was answered from a dry run
/// instead of the gate's own run.
///
/// **Nullable, and no `DEFAULT`**: `NULL` is a Check the gate ran itself,
/// which is every row this database already holds.
pub(crate) const V70: &str = r#"
ALTER TABLE job_step_checks ADD COLUMN reused_from_dry_run TEXT;
"#;
