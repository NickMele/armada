//! SQLite in WAL mode, migrations, retention — and **the only crate that
//! deserializes**.
//!
//! That is a Cargo-graph fact rather than a review item: the SQLite dependency
//! is scoped to this crate alone, so nothing else *can* read a row. `ipc` is the
//! other place bytes enter the process, from the wire.
//!
//! # The rule that cost v1 twenty-one Jobs
//!
//! **Query functions never return pre-filtered results.** They return the parse
//! failures with them, in a shape the caller cannot silently ignore. v1 wrote
//! `.filter_map(Result::ok)` after a store call and dropped twenty-one real Jobs
//! without an error anywhere — the bug was not in the store, it was at the call
//! site, and a signature that hands back a bare `Vec` invites it.
//!
//! Schema versioning is auto-increment plus migration scripts, applied on Fleet
//! startup. `job_events`, `evidence` and `job_facts` are never deleted.
