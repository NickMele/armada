//! The two spellings a stored setting and an allow are read back from.

use crate::job::{Reach, WhenBlocked};

#[test]
fn every_when_blocked_reads_back_from_its_own_spelling() {
    assert_eq!(WhenBlocked::RefuseAndHold.as_wire(), "refuse_and_hold");
    assert_eq!(WhenBlocked::AskMe.as_wire(), "ask_me");
    for setting in WhenBlocked::ALL {
        assert_eq!(WhenBlocked::from_wire(setting.as_wire()), Some(*setting));
    }
    assert_eq!(
        WhenBlocked::from_wire("ask"),
        None,
        "not a near miss either"
    );
}

/// A Job nobody set this on refuses, so nothing ungranted runs by omission.
#[test]
fn a_job_that_chose_nothing_refuses_and_holds() {
    assert_eq!(WhenBlocked::default(), WhenBlocked::RefuseAndHold);
}

#[test]
fn every_reach_reads_back_from_its_own_spelling() {
    assert_eq!(Reach::Job.as_wire(), "job");
    assert_eq!(Reach::Repository.as_wire(), "repository");
    for reach in Reach::ALL {
        assert_eq!(Reach::from_wire(reach.as_wire()), Some(*reach));
    }
    assert_eq!(Reach::from_wire("Job"), None, "the spelling is exact");
}
