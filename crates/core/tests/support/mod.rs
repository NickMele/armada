//! Shared setup for the integration suites in this crate.
//!
//! It holds one thing: compiling the embedded JSON Schema. Two suites validate
//! documents against it — the fixture set, and charkit's own `armada.yml` — and a
//! second copy of the compile step is how one of them ends up validating
//! against a schema the other does not.

/// Compile [`armada_core::config::SCHEMA`] into something documents can be
/// validated against.
pub fn compile_schema() -> (boon::Schemas, boon::SchemaIndex) {
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let value: serde_json::Value =
        serde_json::from_str(armada_core::config::SCHEMA).expect("schema is JSON");
    compiler
        .add_resource("armada.schema.json", value)
        .expect("schema is a usable resource");
    let index = compiler
        .compile("armada.schema.json", &mut schemas)
        .expect("schema compiles");
    (schemas, index)
}
