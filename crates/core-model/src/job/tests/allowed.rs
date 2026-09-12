//! The two spellings a stored setting and an allow are read back from.

use crate::job::{Reach, WhenBlocked};

#[test]
fn every_when_blocked_reads_back_from_its_own_spelling() {
    assert_eq!(WhenBlocked::RefuseAndHold.as_wire(), "refuse_and_hold");
    assert_eq!(WhenBlocked::AskMe.as_wire(), "ask_me");
    assert_eq!(WhenBlocked::AllowAll.as_wire(), "allow_all");
    assert_eq!(WhenBlocked::ALL.len(), 3, "every setting is in the list");
    for setting in WhenBlocked::ALL {
        assert_eq!(WhenBlocked::from_wire(setting.as_wire()), Some(*setting));
    }
    assert_eq!(
        WhenBlocked::from_wire("ask"),
        None,
        "not a near miss either"
    );
}

/// The type's own default is the safe answer to a store that will not read,
/// not what a new Job starts at — `crates/store` binds that explicitly.
#[test]
fn the_type_defaults_to_refuse_and_hold_for_a_store_that_will_not_say() {
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
