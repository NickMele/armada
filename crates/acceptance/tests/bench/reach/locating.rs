//! Locate's apparatus for Reach: repositories one Fleet serves, each read from
//! text. Beside `reach.rs` so neither file carries both halves.

use std::path::Path;

use config::{Manifest, ResolvedWorkflow};
use fleet::repositories::{Located, Repositories, Served, SetUp};

use super::{definition, written, CARRYABLE, CHECKOUT};

/// A second repository, nothing like the storefront.
pub const MAILER_AT: &str = "/repos/mailer";

/// Its `armada.yml`: one Check the storefront does not declare.
pub const MAILER: &str = "version: 1\nid: mailer\nchecks:\n  deliver:\n    run: go test ./...\n";

/// A Fleet started in the storefront.
pub fn started_in_the_storefront() -> Repositories {
    let records = String::from("/records/storefront");
    Repositories::starting_in(
        CHECKOUT.to_string(),
        records,
        SetUp::of(written(), Default::default()),
    )
}

/// A folder as the composition root reads it: its root, and its Manifest where
/// it has an `armada.yml`.
pub fn folder(root: &str, manifest: Option<&str>) -> Located {
    let at = Path::new(root).join("armada.yml");
    let loaded = |text| Manifest::parse(&at, text).unwrap_or_else(|why| panic!("{root}: {why}"));
    Located {
        root: root.to_string(),
        records_root: format!("/records{root}"),
        set_up: manifest.map(|text| SetUp::of(loaded(text), Default::default())),
    }
}

/// Every Manifest served, by id, first first.
pub fn manifest_ids(served: &Repositories) -> Vec<String> {
    let every = served.served();
    every
        .iter()
        .map(|one| one.manifest().id().as_str().to_string())
        .collect()
}

/// The named Checks the carried `bug` holds a Job to, resolved in one repository.
pub fn held_to(there: &Served) -> Vec<String> {
    let bug = ResolvedWorkflow::resolve(&definition(CARRYABLE), there.manifest())
        .unwrap_or_else(|why| panic!("the carried bug resolves there: {why}"));
    let steps = bug.frozen().steps().iter();
    steps
        .flat_map(|step| {
            step.checks()
                .iter()
                .filter_map(|check| check.name().map(str::to_string))
        })
        .collect()
}
