//! Helm: the conversation Fleet hosts for one repository, over the agent door.
//!
//! What a session is told and what it may do are here so the host and the door
//! (`#941`) read one answer. The conversation itself — one per repository,
//! resumed by id for each message — is `#939`'s, and its host sits behind one
//! interface, [`Hosting`], so a host elsewhere is a switch.

mod admitting;
mod asking;
mod brief;
mod conversation;
mod hosting;
mod permitting;
mod reach;
mod recording;
mod serving;
mod thread;
pub(crate) mod unanswered;

pub use asking::{Asks, HelmAskHold, NotAnswerable, Said};
pub use brief::{brief, Brief, Voice};
pub use conversation::{ConversationKey, Conversations};
#[cfg(test)]
pub(crate) use hosting::REPLY_BUDGET;
pub use hosting::{Carried, Carry, Carrying, Heard, Hosting, ProcessHost};
pub use permitting::SHIPPED_ASK_HOLD;
pub use reach::{may, Authority};
pub use thread::{what_was_said, THREAD};

#[cfg(test)]
mod tests;
