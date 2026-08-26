//! Bytes in, bytes out. **The only place on the Fleet side that parses JSON
//! from the wire.**
//!
//! Gate rule five refuses the untyped-JSON entry points everywhere but `store`
//! and here, and this module is why `ipc` is on that list: reading bytes is a
//! crate boundary rather than a call site in the middle of something. A
//! transport crate that parsed its own bodies would be another.
//!
//! **Three callers now, and the third is not the wire.** `fleet` reads its own
//! runtime file through [`decode`], and `adapters` reads a Drone's transcript
//! and writes its MCP configuration through this pair. Neither is Bridge, and
//! both are a place bytes genuinely arrive from outside — so what this module
//! is, precisely, is *the* reader and writer of untyped JSON on the Fleet side,
//! rather than the reader of one particular wire. The DTOs above it are the
//! wire; this is not.
//!
//! # Unknown fields are ignored, on purpose
//!
//! No DTO in this crate sets `deny_unknown_fields`. That is not laxity — it is
//! the entire basis of the minor-skew row: an older peer parses the fields it
//! knows and ignores the rest, so an additive change is invisible to it. Adding
//! `deny_unknown_fields` anywhere in this crate would turn every additive minor
//! bump into a breaking one, silently.

use std::error::Error;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// A message that did not parse.
///
/// The failure is carried as one line rather than as a `serde_json::Error`,
/// because the caller is a route handler that has to answer a peer, and the
/// peer cannot act on a type from this crate's dependency graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Undecodable {
    /// What the bytes were being read as. A fixed string chosen at the call
    /// site, never anything a peer sent.
    pub expected: &'static str,
    pub why: String,
}

impl fmt::Display for Undecodable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "not a readable {}: {}", self.expected, self.why)
    }
}

impl Error for Undecodable {}

/// A value that would not serialise. Unreachable for every DTO here — each one
/// is plain data — and returned rather than panicked because a transport crate
/// that can panic on a response is a transport crate that can drop a
/// connection while a Job runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unencodable {
    pub why: String,
}

impl fmt::Display for Unencodable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "could not be written to the wire: {}", self.why)
    }
}

impl Error for Unencodable {}

/// Read a DTO off the wire.
pub fn decode<T: DeserializeOwned>(expected: &'static str, bytes: &[u8]) -> Result<T, Undecodable> {
    serde_json::from_slice(bytes).map_err(|why| Undecodable {
        expected,
        why: why.to_string(),
    })
}

/// Write a DTO to the wire.
pub fn encode<T: Serialize>(value: &T) -> Result<String, Unencodable> {
    serde_json::to_string(value).map_err(|why| Unencodable {
        why: why.to_string(),
    })
}
