//! The seams: the traits an adapter implements, and nothing that implements
//! them.
//!
//! Five boundaries are named in the Adapters spec — agent harness, version
//! control, secrets, model client, and container runtime. Each is a trait here
//! and an implementation in `adapters`, which is the only crate permitted to
//! know whose API it is talking to. That separation is a gate rule: a vendor's
//! name outside `adapters` is the boundary having leaked, and it leaks in
//! comments first.
//!
//! # What may not enter this crate
//!
//! No runtime, no I/O, no vendor — checked by `cargo tree`, same as
//! `core-model`. A trait that requires `tokio` to *declare* has already chosen
//! the implementation's runtime for it.
//!
//! # Why it is empty
//!
//! The trait signatures are design work this step does not own. M0 step 7
//! creates the crate and its dependency rule; what each seam exposes is the
//! Adapters spec's, and spawning a real Drone through the harness seam is M1
//! step 8. Declaring five empty traits now would fix five shapes by accident.
