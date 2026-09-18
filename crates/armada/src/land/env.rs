//! Every environment variable `armada land` reads, read once — but *which*
//! variables are read, and what each defaults to, matches `scripts/land`
//! exactly, because `scripts/test_land.py`'s `self.env` dict exercises this
//! surface directly, against whichever binary `ARMADA_LAND_ARMADA` names.

use std::path::Path;
use std::time::Duration;

/// Every swappable binary and setting a turn reads. Read once, at the verb's
/// entry point, rather than a `std::env::var` scattered through every
/// module that needs one of these.
#[derive(Clone, Debug)]
pub struct Env {
    pub gh: String,
    pub remote: String,
    pub base: String,
    pub armada: String,
    pub foundations: Vec<String>,
    pub setup: Vec<String>,
    pub seed: Vec<String>,
    /// [`Env::seed`]'s paths, plus whatever `ARMADA_LAND_KEEP` adds — what
    /// survives `git clean -xdff` in a reused worktree.
    pub keep: Vec<String>,
    pub head_wait: Duration,
}

/// A copy of `armada.yml`'s `setup.requires`, named in the capability doc as
/// what does not port — kept honest by
/// `crate::tests::land::the_setup_default_says_what_the_manifest_requires`.
const DEFAULT_SETUP: &str = "bootstrap browsers";

/// Rounds of "the base moved again while this was gated" before a turn gives
/// up.
pub const ROUNDS: u32 = 5;

impl Env {
    pub fn read() -> Env {
        let words = |name: &str, default: &str| {
            var(name, default)
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let seed = words("ARMADA_LAND_SEED", "target");
        let mut keep = seed.clone();
        keep.extend(words("ARMADA_LAND_KEEP", "node_modules"));
        Env {
            gh: var("ARMADA_LAND_GH", "gh"),
            remote: var("ARMADA_LAND_REMOTE", "origin"),
            base: var("ARMADA_LAND_BASE", "main"),
            armada: var("ARMADA_LAND_ARMADA", "armada"),
            foundations: words("ARMADA_LAND_FOUNDATIONS", "cargo xtask verify-foundations"),
            setup: words("ARMADA_LAND_SETUP", DEFAULT_SETUP),
            seed,
            keep,
            head_wait: Duration::from_secs_f64(
                var("ARMADA_LAND_HEAD_WAIT", "120").parse().unwrap_or(120.0),
            ),
        }
    }

    /// [`Env::keep`], as the borrowed slice [`super::worktree::reused_keeping`]
    /// takes — built at the call site rather than stored this way, since
    /// `git clean`'s argv needs `&str`s and `Env` itself needs to own them.
    pub fn keep_refs(&self) -> Vec<&str> {
        self.keep.iter().map(String::as_str).collect()
    }

    /// Every swappable binary that must be named by an absolute path once it
    /// holds one — a relative path resolves against the runner's own working
    /// directory, not the branch's.
    pub fn relative_binaries(&self) -> Option<(&'static str, &str)> {
        let candidates = [
            ("ARMADA_LAND_ARMADA", self.armada.as_str()),
            ("ARMADA_LAND_GH", self.gh.as_str()),
            (
                "ARMADA_LAND_FOUNDATIONS",
                self.foundations.first().map(String::as_str).unwrap_or(""),
            ),
        ];
        candidates
            .into_iter()
            .find(|(_, value)| value.contains('/') && !Path::new(value).is_absolute())
    }
}

fn var(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_SETUP;

    /// The Rust port's own copy of
    /// `test_the_setup_default_says_what_the_manifest_requires`: the default
    /// this crate hardcodes must match `armada.yml`'s `setup.requires`,
    /// read the same way the Python test reads it — by slicing the text
    /// between markers, not by parsing YAML, so this test fails the same way
    /// the original did if either copy drifts.
    #[test]
    fn the_setup_default_says_what_the_manifest_requires() {
        let root = crate::tests::repository();
        let manifest = std::fs::read_to_string(root.join("armada.yml")).expect("armada.yml");
        let requires = manifest
            .split("setup:")
            .nth(1)
            .expect("a setup: block")
            .split("requires:")
            .nth(1)
            .expect("a requires: list")
            .split("seed:")
            .next()
            .expect("something before seed:");
        let wanted: Vec<String> = requires
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('-'))
            .map(|line| line.trim_start_matches('-').trim().to_string())
            .collect();
        let default: Vec<String> = DEFAULT_SETUP
            .split_whitespace()
            .map(str::to_string)
            .collect();
        assert_eq!(
            default, wanted,
            "the copy of setup.requires in crates/armada/src/land/env.rs has drifted"
        );
    }
}
