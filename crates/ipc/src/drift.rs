//! Whether the repository still has what `armada.yml` names.
//!
//! **A read of the repository, where [`ManifestReading`](crate::ManifestReading)
//! is a read of the file.** A `run` line naming a script that was deleted
//! parses perfectly, so the reread that already crosses this seam says nothing
//! about it and cannot: it reports what Fleet could not adopt, and this file
//! was adopted. The first thing that notices is a Job failing on it.
//!
//! # Two verdicts, and there is no third
//!
//! `gone` and `current`, and nothing means *changed*. The file carries no
//! record of what it was written against, so nothing can call a script changed
//! without storing a scan of a previous one — and a stored scan is state this
//! read does not have and should not grow. `docs/journeys/run-and-edit-a-manifest.md`,
//! under *Verify*, settles it, and settles the direction too: drift never
//! reports a script the repository picked up that the file does not yet name.
//!
//! # It is free, and that is what makes it a read at all
//!
//! Nothing here runs a command to find out whether one exists. Drift runs on
//! opening the surface, beside a dry-run that runs a real test suite behind its
//! own button — and the only reason the two can sit together is that this half
//! costs a handful of `stat` calls.
//!
//! # What a clean answer does not say
//!
//! It does not say a listed command still does the right thing: a `test` script
//! narrowed to one directory reads as existing. It says nothing about policy,
//! permissions, budgets or ports, because none of those names anything runnable
//! — about a third of a mature Manifest carries no verdict here at all, and a
//! surface drawing this has to say so rather than let a clean list read as *all
//! of this is still right*. [`Drift::Current`] carries `checked` so that a row
//! judged on nothing is distinguishable from a row judged and found whole,
//! which is the same sentence one row down.

use serde::{Deserialize, Serialize};

/// What the repository still has of what one Manifest names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDrift {
    /// The file this was read against, as Fleet resolved it. A message that
    /// does not name the file names nothing a person can open.
    pub path: String,
    /// The checkout the paths were looked for in. **The tree on disk**, not a
    /// Job's worktree and not a copy: drift answers about the repository a
    /// person is about to run something in.
    pub checkout: String,
    /// One row per `run`, `serve` or `ready` line the file declares, in the
    /// order `armada.yml` writes them.
    pub declarations: Vec<Declaration>,
}

/// One line the Manifest declares, and whether what it names is still there.
///
/// **One row per line, not per declaration.** A Command that serves declares up
/// to three — what builds it, what starts it, and what says it is ready — and
/// each can go missing on its own, so one row carrying a verdict for all three
/// would be a verdict about none of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declaration {
    /// The section, spelled as `armada.yml` spells it — `checks`, `commands`.
    /// **Rendered, never matched on**, so a registry added later draws as
    /// itself; the same rule [`ManifestReading::at_restart`] carries.
    ///
    /// [`ManifestReading::at_restart`]: crate::ManifestReading::at_restart
    pub section: String,
    /// The name the file declares it under.
    pub name: String,
    /// The key inside it whose line this is — `run`, `serve`, `ready`. With
    /// the two above it this is the line's path in `armada.yml`, which is what
    /// a person searches the file for.
    pub key: String,
    /// The line, verbatim as the repository wrote it. `${port.NAME}` is
    /// unresolved, because that is what the file says and this is a read of
    /// what the file says.
    pub run: String,
    /// Whether the repository still has what it names.
    pub drift: Drift,
}

/// The verdict, carrying what it was reached on.
///
/// **The evidence is inside the variant it belongs to**, so `gone` with nothing
/// missing cannot be written down and `current` cannot be read as a promise it
/// does not make. A row's amber is exactly the non-empty list under it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Drift {
    /// Every repository path this line names is still in the checkout.
    ///
    /// **`checked` is zero for most lines, and that is the honest answer.**
    /// `cargo nextest run --workspace` names no path in this repository, so
    /// there was nothing to look for and nothing was found missing. A surface
    /// that draws a green `current` on a row checked against nothing is making
    /// a claim this read did not make.
    Current {
        /// How many repository paths this line named.
        checked: u32,
    },
    /// The line names repository paths that are not in the checkout.
    ///
    /// **Behind, not broken** — the journey renders this amber and never red,
    /// and nothing here offers to fix it. Acting on a row means editing the
    /// file, where the consequence is stated.
    Gone {
        /// The paths, relative to the checkout, in the order the line names
        /// them. Never empty.
        missing: Vec<String>,
    },
}
