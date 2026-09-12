//! How much of the tree a Check reads when a Drone asks about its own change.
//!
//! **A second question, not a second `when`.** `when` answers whether a Check
//! applies at all; this answers what it reads given that it does — independent
//! keys, since `format` applies to every step and still has a narrower run.
//!
//! **The path-to-argument derivation is the repository's to declare, not a
//! runner's.** `-p ipc` from `crates/ipc/src/mcp/tools.rs` is a fact about
//! Cargo's layout that a runner would get wrong for `pnpm`. The Manifest
//! writes it down instead: [`Narrowing::under`] is a directory whose child
//! names the value, [`Narrowing::each`] spells it as an argument.
//!
//! Command-line assembly lives in `checks_runner::narrowed`, beside the
//! splitter it must agree with about quoting.

use alloc::string::String;
use alloc::vec::Vec;

use super::covers::Covers;

/// How a Check is run against a subset of the tree, as the Manifest declared it.
///
/// **Absent is the answer for most Checks and always will be.** A Check that
/// declares no narrowing runs whole, which is what every Check did before this
/// existed. There is no default narrowing and there cannot be one: what a
/// command does with a file list is the command's business.
///
/// Frozen onto the Job beside `run`, `when` and `requires`, and for their
/// reason — see [`ResolvedCheck::ManifestCheck`](super::workflow::ResolvedCheck).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Narrowing {
    run: String,
    each: String,
    from: Option<Covers>,
    under: Option<String>,
    except: Vec<String>,
}

impl Narrowing {
    /// Build one from keys already read off a Manifest, or off a row that was
    /// written from one. **`config` and `store` are the only callers**, which
    /// is [`Prerequisite::resolved`](super::Prerequisite::resolved)'s shape and
    /// its reason.
    pub fn declared(
        run: String,
        each: String,
        from: Option<Covers>,
        under: Option<String>,
        except: Vec<String>,
    ) -> Narrowing {
        Narrowing {
            run,
            each,
            from,
            under,
            except,
        }
    }

    /// The command a narrowed run starts from, before any value is appended.
    ///
    /// **A whole second command line and not an edit of the first.** The Check
    /// this repository narrows first is `cargo nextest run --workspace`, and
    /// what has to go for `-p` to mean anything is `--workspace` — which no
    /// rule about appending arguments could ever have removed.
    pub fn run(&self) -> &str {
        &self.run
    }

    /// How one value is spelled as an argument. `{}` is where the value goes,
    /// and a template without it is refused where the Manifest is read.
    pub fn each(&self) -> &str {
        &self.each
    }

    /// Which changed paths feed the values. **`None` means all of them**, never
    /// none — the same reading `when` has, spelled the same way for the same
    /// reason.
    ///
    /// It is not `when` and cannot be folded into it: `format` covers every
    /// path and narrows only to the Rust ones, so the list a Check is *run
    /// against* is not the list that decides whether it runs.
    pub fn from(&self) -> Option<&Covers> {
        self.from.as_ref()
    }

    /// The directory whose child names the value. **`None` means the value is
    /// the changed path itself**, which is the verbatim case.
    ///
    /// `crates` turns `crates/ipc/src/mcp/tools.rs` into `ipc`. That is the
    /// whole of the derivation, and it is deliberately not a pattern: a capture
    /// group would be a second glob dialect, and `docs/contracts/configuration.md`
    /// has one.
    pub fn under(&self) -> Option<&str> {
        self.under.as_deref()
    }

    /// Values this never produces, however the change is derived. **Empty on a
    /// Check that excludes nothing**, which is most of them.
    ///
    /// It is not `from` narrowed further: `from` filters paths and this filters
    /// what they derived to, and there is no pattern for *every crate but this
    /// one* — the dialect refuses a leading `!` by name. What it is for is a
    /// whole run that already excludes something the narrowed run would
    /// otherwise pull back in, which is a bar the gate deliberately does not
    /// apply.
    pub fn except(&self) -> &[String] {
        &self.except
    }
}
