//! The JSON Schema for `armada.yml`, embedded.
//!
//! It is embedded rather than read from disk for two reasons: the core does no
//! I/O, and phase 5's `armada manifest config scan` hands this text to the agent that
//! authors the config — a schema the binary has to find on the filesystem is a
//! schema that is missing in the one situation it exists for.
//!
//! **This file is the authoritative statement of the contract** (PLAN.md
//! §4.1.1, decision 2). The structs in `model.rs` mirror it by hand; nothing
//! generates either from the other. What keeps them together is the fixture
//! suite, which parses every fixture with the structs *and* validates it
//! against this schema, plus a test asserting that every property named here
//! is used by at least one fixture — so a key that exists in only one of the
//! two, or in neither, fails the build.

/// The `armada.yml` JSON Schema, draft 2020-12.
pub const SCHEMA: &str = include_str!("../../schema/armada.schema.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_schema_is_json_and_declares_its_draft() {
        let value: serde_json::Value = serde_json::from_str(SCHEMA).expect("schema is valid JSON");
        assert_eq!(
            value["$schema"], "https://json-schema.org/draft/2020-12/schema",
            "the draft is part of the contract: the validator is chosen against it"
        );
    }
}
