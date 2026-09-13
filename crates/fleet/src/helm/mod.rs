//! Helm: the conversation Fleet hosts for one repository, over the agent door.
//!
//! Nothing hosts a session yet (`#939`). This holds what a session is told and
//! what it may do, so the host and the door (`#941`) read one answer.

mod brief;
mod reach;

pub use brief::{brief, Brief, Voice};
pub use reach::{may, Authority};

#[cfg(test)]
mod tests;
