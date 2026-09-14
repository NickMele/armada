//! What this crate proves about itself.
//!
//! **Every module below carries its own header, saying what it proves and why
//! its cases are shaped that way. There is deliberately no index of them
//! here.** The one this replaces described about half of them and numbered
//! them, and the numbering had drifted far enough that two modules were both
//! "the eleventh" — a copy of every header, checked by nothing, is the thing
//! that can be wrong about a suite while every test in it passes.
//!
//! What no single module can say is which of them are one subject:
//!
//! - `process`, `runtime` and `detach` are one. A Fleet that outlives the app
//!   must be findable, and its runtime file must let a reader tell a live Fleet
//!   from a pid that used to be one.
//! - `dry_run` is `gate` asked from the other side, before a step is spent.
//! - `peer` is the primitive `concurrency` rests on: a call attributed by the
//!   connection it arrived on rather than by which Job was admitted first.

mod admitted;
mod adopting;
mod allowance;
mod always_allow;
mod amending;
mod asked;
mod asking;
mod attachments;
mod attribution;
mod auto_merging;
mod basing;
mod boundary;
mod bounding;
mod briefing;
mod capacity;
mod checking;
mod checkout_runs;
mod checkouts_apart;
mod checks;
mod checks_at_once;
mod cloning;
mod code_review;
mod concurrency;
mod confidence;
mod conflict_resolution;
mod converging;
mod coupling;
mod covering;
mod crossing;
mod daemon;
mod declaring;
mod delete_branch;
mod delivering;
mod delivery;
mod detach;
mod detail;
mod dismissing;
mod door_per_repository;
mod drifting;
mod drone;
mod dry_run;
mod editing;
mod epic;
mod evidence;
mod explaining;
mod fixing;
mod following;
mod following_up;
mod footprint;
mod forget;
mod freezing;
mod frozen;
mod gaming;
mod gate;
mod group;
mod headings;
mod headroom;
mod helm_conversation;
mod helm_door;
mod history;
mod holding;
mod host;
mod http;
mod journal;
mod judging;
mod keeping;
mod landing;
mod landing_committed;
mod left_behind;
mod limits;
mod linking;
mod listener;
mod looping;
mod manifest_proposals;
mod mending;
mod merging;
mod migrating;
mod modelling;
mod noticing;
mod out_of_bounds;
mod overlap;
mod overruling;
mod paying;
mod peer;
mod peers;
mod permitting;
mod places;
mod plan_person;
mod plan_person_told;
mod plan_tools;
mod planning;
mod planted;
mod policy_gate;
mod ports;
mod ports_dispatch;
mod precedent;
mod preferences;
mod preparing;
mod prerequisites;
mod process;
mod proposing;
mod proving;
mod questioning;
mod queued;
mod raising;
mod read_only_git;
mod reading_a_gate;
mod reclaim;
mod records;
mod redirect;
mod redispatch;
mod refused;
mod regating;
mod rehearsing;
mod rejecting;
mod remarks;
mod reporting;
pub(crate) mod repositories;
mod rerunning;
mod resources;
mod restarting;
mod resting;
mod resuming;
mod retrying;
mod reuse;
mod review_model;
mod reviewing;
mod reviewing_brief;
mod runtime;
mod scope;
pub(crate) mod seeding;
mod sending_back;
pub(crate) mod servers;
mod serving;
mod session;
mod settling;
mod showing;
mod showing_again;
mod silence;
mod snapshotting;
mod starting;
mod starting_empty;
mod stuck;
mod sub_dispatch;
mod superseding;
mod terms;
mod work_plan;
// `pub(crate)`, not `mod`: `crate::records::migrating`'s own tests are not a
// descendant of this module and need the same temporary directory every
// fixture here already uses, rather than a second one invented beside it.
pub(crate) mod tmp;
mod tools;
mod transcript;
mod unattended;
mod under_review;
mod underway;
mod verify_runs;
mod watching;
mod widening;
mod workspace_ports;
mod workspace_runs;
mod workspace_verify;
