//! The two verification tiers, and the gate between them.
//!
//! Mechanical Checks are pure functions over facts that already exist — an exit
//! code, a file's presence, whether a diff is empty. **Armada does not parse**:
//! deciding which lines of a test run were the failure is a Judge's question,
//! answered by reading the diff, never a runner's output.
//!
//! The Judge is a veto and not a grant. It fires on mechanical triggers, is
//! blind to the Drone, and judges whether evidence satisfies the step's intent.
//! It cannot vouch for something an exit code already contradicted.
//!
//! # Why the diff computation lives behind this crate
//!
//! The raw diff-computation adapter method is exposed **only** here, so exactly
//! one place decides whether files changed outside their declared scope. Two
//! places deciding that is two answers.
