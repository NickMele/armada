//! The one place in the workspace that mints an id.
//!
//! # Because every other crate refuses to
//!
//! `core_model::Ulid` has no constructor but `carried`, `ipc`'s ids have the
//! same one, and both say why in the same words: **Fleet is the sole authority
//! for the ids that name records**, and an id invented by a peer joins to
//! nothing. That rule leaves exactly one gap, and this fills it.
//!
//! # ULID, because a sort is the join
//!
//! `Ulid`'s own comment gives the reason it is not a UUIDv4: lexicographic
//! order is chronological, so merging Fleet's, Bridge's and a Drone's lines
//! into one ordered view costs a string sort. That property is a property of
//! the *encoding*, so it has to be produced here rather than assumed — 48 bits
//! of milliseconds first, Crockford base32, most significant character first.
//!
//! # Injected, like the clock, and for the same reason
//!
//! [`Mint`] is a trait so a test can plant ids it can write down. A test that
//! had to discover the id it just created could not assert on the store
//! afterwards without reading the answer out of the thing under test.

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_model::Ulid;
use rand::RngCore;

/// A fresh id, asked for rather than invented at the call site.
///
/// **One method.** There is no `job_id()`/`drone_id()` pair, because a ULID
/// does not know what it names — the newtype it is wrapped in does, and that
/// wrapping happens where the record is made.
pub trait Mint: Send + Sync {
    fn ulid(&self) -> Ulid;
}

impl<M: Mint + ?Sized> Mint for Arc<M> {
    fn ulid(&self) -> Ulid {
        (**self).ulid()
    }
}

/// Crockford's base32 alphabet: no `I`, `L`, `O` or `U`, so a transcribed id
/// cannot be misread as a different one.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The machine's mint.
///
/// **The only type in this workspace that produces an id nothing handed it.**
///
/// The randomness is real, and the counter beside it is what makes two ids
/// minted in the same millisecond ordered rather than merely distinct: a Job
/// and the Drone created for it are minted microseconds apart, and a Board that
/// sorted them by id would otherwise interleave two Jobs' records arbitrarily.
#[derive(Debug, Default)]
pub struct UlidMint {
    within_the_millisecond: AtomicU16,
}

impl UlidMint {
    pub fn new() -> UlidMint {
        UlidMint::default()
    }
}

impl Mint for UlidMint {
    fn ulid(&self) -> Ulid {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or(0);
        let ordinal = self.within_the_millisecond.fetch_add(1, Ordering::Relaxed);
        let mut entropy = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut entropy);
        Ulid::carried(encode(millis, ordinal, u64::from_be_bytes(entropy)))
    }
}

/// Twenty-six characters: ten of milliseconds, sixteen of the rest.
///
/// The ordinal takes the two most significant characters of the random half, so
/// that two ids from the same millisecond order by the sequence they were
/// minted in and only then by chance.
fn encode(millis: u64, ordinal: u16, entropy: u64) -> String {
    let mut out = String::with_capacity(26);
    // 48 bits of time, five bits at a time, most significant first. The first
    // character carries only the top three bits, which is why the shift starts
    // at 45 rather than at a multiple of five from the top of the word.
    for shift in (0..50).step_by(5).rev() {
        out.push(char_at((millis >> shift) as u32));
    }
    for shift in (0..15).step_by(5).rev() {
        out.push(char_at(u32::from(ordinal) >> shift));
    }
    for shift in (0..65).step_by(5).rev() {
        out.push(char_at((entropy >> shift) as u32));
    }
    out
}

fn char_at(bits: u32) -> char {
    ALPHABET[(bits & 0b1_1111) as usize] as char
}
