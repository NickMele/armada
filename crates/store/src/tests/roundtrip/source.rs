//! Where a Job's frozen workflow came from, read back off the column. #425.
//!
//! Straight at the column, as `workflow.rs` beside this reads its older rows:
//! the key is inside the frozen document, so no table and no migration moved.

use core_model::WorkflowSource;

use crate::columns::{read_workflow, write_workflow};
use crate::tests::workflow;

#[test]
fn each_source_survives_the_column() {
    for source in WorkflowSource::ALL.iter().copied() {
        let stored = write_workflow(&workflow().from_source(source));
        let read = read_workflow(&stored).expect("a row this store wrote");
        assert_eq!(read.source(), source);
    }
}

/// **Every Job frozen before #425 came from its repository**, the one place a
/// workflow could come from then, so a row with no key says exactly that.
#[test]
fn a_row_frozen_before_the_key_existed_reads_back_as_the_repositorys() {
    let written = write_workflow(&workflow().from_source(WorkflowSource::Kit));
    let older = written.replace(r#""source":"kit","#, "");
    assert_ne!(older, written, "the key was there to take out");
    let read = read_workflow(&older).expect("a pre-#425 row");
    assert_eq!(read.source(), WorkflowSource::Repository);
}

#[test]
fn a_source_outside_the_three_is_refused_rather_than_read_as_one() {
    let written = write_workflow(&workflow());
    let stranger = written.replace(r#""source":"repository""#, r#""source":"elsewhere""#);
    assert_ne!(stranger, written, "the key was there to change");
    let refused = read_workflow(&stranger).expect_err("a word outside the set");
    assert!(refused.contains("elsewhere"), "{refused}");
}
