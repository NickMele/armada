//! Running a Manifest's Checks and Commands.
//!
//! Called with explicitly injected data — a lease id, a worktree path — and
//! **never a live scheduler handle**. That is the whole reason this is a crate.
//!
//! v1's equivalent reached 1,816 lines inside its core, coupled to the
//! scheduler, the lease table and process-group plumbing, and could not be
//! lifted out. The seam is here so the runner can be driven by a test that has
//! no scheduler at all.
