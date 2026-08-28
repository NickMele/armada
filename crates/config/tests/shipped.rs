//! Every workflow definition this repository ships parses.
//!
//! An integration test rather than a unit one because the subject is the files
//! in `.armada/workflows/`, not the parser: a definition that stops loading is
//! a Fleet that cannot dispatch, and nothing else in the workspace reads them.
//!
//! The four ways a definition has gone wrong so far were all silent until a Job
//! hit them — a key the parser defers, a gate disagreeing with its judge checks,
//! a `question` where the parser reads `criteria[]`, and a Judge asked something
//! on a step that produces nothing.

use std::path::Path;

#[test]
fn every_shipped_workflow_definition_parses() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".armada/workflows");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("the shipped definitions are there") {
        let path = entry.expect("a directory entry").path();
        let text = std::fs::read_to_string(&path).expect("a readable definition");
        if let Err(why) = config::WorkflowDef::parse(&path, &text) {
            panic!("{} is refused:\n{why}", path.display());
        }
        seen += 1;
    }
    assert!(seen >= 7, "seven workflows ship, and {seen} were read");
}
