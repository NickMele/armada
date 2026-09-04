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

mod adopting;
mod allowance;
mod asked;
mod attachments;
mod attribution;
mod boundary;
mod bounding;
mod briefing;
mod capacity;
mod checking;
mod checks;
mod concurrency;
mod converging;
mod coupling;
mod covering;
mod crossing;
mod daemon;
mod delivering;
mod delivery;
mod detach;
mod detail;
mod drone;
mod dry_run;
mod evidence;
mod footprint;
mod forget;
mod frozen;
mod gaming;
mod gate;
mod group;
mod headings;
mod headroom;
mod history;
mod holding;
mod host;
mod http;
mod judging;
mod keeping;
mod landing;
mod linking;
mod looping;
mod modelling;
mod noticing;
mod overlap;
mod overruling;
mod peer;
mod planning;
mod planted;
mod preparing;
mod prerequisites;
mod process;
mod proposing;
mod questioning;
mod queued;
mod reclaim;
mod redirect;
mod redispatch;
mod regating;
mod reporting;
mod restarting;
mod resting;
mod resuming;
mod retrying;
mod reviewing;
mod runtime;
mod scope;
mod sending_back;
mod serving;
mod session;
mod settling;
mod silence;
mod starting;
mod stuck;
mod sub_dispatch;
mod terms;
mod tmp;
mod tools;
mod transcript;
mod watching;
mod widening;
