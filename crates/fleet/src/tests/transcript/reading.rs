//! The one way a test reads a Drone's transcript.
//!
//! A row is handed to [`Recording`](crate::transcript::Recording) on a bounded
//! queue and written by a task nothing awaits, so the call that produced a turn
//! returns before the file carries it. A test that reads once reads whatever
//! the scheduler happened to have done — always enough idle, not enough on the
//! merge line, where `#1436`'s panic was `[]` and said nothing else.
//!
//! **`restarting::until_spoken` already did this correctly before the next two
//! sites were written**, and the knowledge did not reach their authors: each
//! wrote the loop again, one of them only after it failed. So the answer is not
//! a paragraph but a type with no bare read in it. A [`Transcript`] hands out
//! neither its text nor its path unwaited, and `xtask`'s
//! `no_bare_transcript_read_in_a_test` refuses the one way round it — naming
//! `transcript_of` again in a test module.

use std::path::PathBuf;
use std::time::Duration;

use core_model::DroneId;

use crate::tests::tmp::TempDir;
use crate::transcript::transcript_of;

/// How often the file is looked at.
const A_PASS: Duration = Duration::from_millis(5);

/// What a row already handed to `Recording` gets before a case calls itself
/// broken.
///
/// Generous on purpose: fifty milliseconds was enough on an idle machine and
/// not enough at load 147, where a case read two of three rows and called the
/// record wrong. Every wait below breaks the moment it has what it asked for,
/// so a passing run spends a pass and only a broken one spends this.
pub(crate) const A_WRITER_HAS_LONG_ENOUGH: Duration = Duration::from_secs(6);

/// How many identical passes make a file settled, for [`Transcript::settled`].
const STEADY: usize = 10;

/// One Drone's transcript, as a test reads it.
///
/// **No `read`, and no accessor for the path**, so every reader in the suite is
/// one of the waiting ones below and the read that was the bug cannot be
/// spelled at a call site.
pub(crate) struct Transcript(PathBuf);

impl Transcript {
    /// This Drone's rows, under the records root a case is running in.
    pub(crate) fn of(under: &TempDir, handle: &str, drone: &DroneId) -> Self {
        Transcript(transcript_of(
            &under.path().to_string_lossy(),
            handle,
            drone,
        ))
    }

    /// The whole transcript, once `enough` is true of it.
    ///
    /// `Err` carries the text as it stood when the patience ran out, so the
    /// case can say what did arrive rather than that nothing did.
    pub(crate) async fn until(
        &self,
        patience: Duration,
        enough: impl Fn(&str) -> bool,
    ) -> Result<String, String> {
        let mut said = self.now();
        for _ in 0..passes(patience) {
            if enough(&said) {
                return Ok(said);
            }
            tokio::time::sleep(A_PASS).await;
            said = self.now();
        }
        if enough(&said) {
            Ok(said)
        } else {
            Err(said)
        }
    }

    /// The transcript once it has stopped changing.
    ///
    /// **For a case asserting a Drone was *not* told**, and for a baseline
    /// taken before the act under test. There is no row to wait on, so this
    /// waits the writer out instead — [`STEADY`] passes with nothing appended.
    /// A heuristic, and said to be one, which a single read was not.
    pub(crate) async fn settled(&self, patience: Duration) -> String {
        let mut last = self.now();
        let mut steady = 0usize;
        for _ in 0..passes(patience) {
            tokio::time::sleep(A_PASS).await;
            let said = self.now();
            steady = if said == last { steady + 1 } else { 0 };
            last = said;
            if steady == STEADY {
                break;
            }
        }
        last
    }

    /// What is on disk this instant. A transcript that is not open yet reads as
    /// empty, which is what it is.
    ///
    /// **Private, and that is the whole point.**
    fn now(&self) -> String {
        std::fs::read_to_string(&self.0).unwrap_or_default()
    }
}

/// How many [`A_PASS`] polls fit in `patience`, at least one.
fn passes(patience: Duration) -> usize {
    (patience.as_millis() / A_PASS.as_millis()).max(1) as usize
}
