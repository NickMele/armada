//! A scout: the read-only agent a person starts from a Studio, whose Finding
//! lists everything it read. `#1292`, `docs/concepts/scout.md`.
//!
//! What it is told is [`brief`]; the process it runs as, and the stop that
//! reaches it, are [`hosting`]. What starting and stopping one does to the
//! Studio is `crate::scouting`.

mod brief;
mod hosting;

pub(crate) use brief::{told, told_a_read_in};
pub use hosting::{ScoutHost, Scouts};
