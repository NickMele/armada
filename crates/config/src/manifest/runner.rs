//! `checks.<name>.runner`, and nothing else about how the Check is run.
//!
//! **A file of its own because `manifest.rs` is already over 500 lines.** A
//! rule already broken is not a licence to break it further, and this is the
//! one parser here that reads a block belonging to another document — the
//! runner's own description, `docs/concepts/runner-adapter.md`.

use core_model::Runner;
use serde_yaml_ng::Value;

use crate::error::Refusal;
use crate::manifest::Table;
use crate::yaml;

/// The keys read inside `checks.<name>.runner`.
const RUNNER_KEYS: &[&str] = &["name", "pkg"];

/// `checks.<name>.runner`: which runner drives this Check, and the package it
/// runs in.
///
/// **A name and nothing else about how to run it.** Every command a runner can
/// make is written once in that runner's own description, never here — see
/// `docs/concepts/runner-adapter.md`. `pkg` is what `{pkg}` resolves to in
/// those templates, and a Check whose runner is rooted at the repository omits
/// it.
pub(super) fn runner(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Runner> {
    let mut table = Table::open(at, value, out)?;
    let name_key = table.at("name");
    let name = table
        .required("name", out)
        .and_then(|value| yaml::text(&name_key, value, out));
    let pkg_key = table.at("pkg");
    let pkg = table
        .optional("pkg")
        .and_then(|value| yaml::text(&pkg_key, value, out));
    table.close(RUNNER_KEYS, out);
    Some(Runner::declared(name?, pkg))
}
