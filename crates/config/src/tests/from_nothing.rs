//! A whole `armada.yml` built from empty text, which is how Setup's Write uses the writer.

use std::path::Path;

use core_model::AutoMerge;

use crate::amending::{
    amend, CheckEdit, CommandEdit, Edit, NewCheck, NewCommand, NewPort, NotAmended, PortEdit,
};
use crate::error::Fault;

fn at() -> &'static Path {
    Path::new("/repos/storefront/armada.yml")
}

fn whole() -> Vec<Edit> {
    vec![
        Edit::Version(1),
        Edit::Id("storefront".to_string()),
        Edit::Port {
            name: "web".to_string(),
            edit: PortEdit::Add(NewPort {
                container: Some(3000),
                env: Some("PORT".to_string()),
            }),
        },
        Edit::Check {
            name: "e2e".to_string(),
            edit: CheckEdit::Add(NewCheck {
                run: "pnpm playwright test".to_string(),
                requires: vec!["migrate".to_string()],
                when: Vec::new(),
                narrow: None,
                runner: None,
            }),
        },
        Edit::Command {
            name: "migrate".to_string(),
            edit: CommandEdit::Add(NewCommand {
                run: Some("pnpm prisma migrate deploy".to_string()),
                destructive: false,
                serve: None,
                ready: None,
                links: Vec::new(),
            }),
        },
        // Destructive and required by nothing, which is the only way the parser takes one.
        Edit::Command {
            name: "reset".to_string(),
            edit: CommandEdit::Add(NewCommand {
                run: Some("pnpm prisma migrate reset --force".to_string()),
                destructive: true,
                serve: None,
                ready: None,
                links: Vec::new(),
            }),
        },
        Edit::SetupRequires(vec!["migrate".to_string()]),
        Edit::AutoMerge(Some(AutoMerge::ChecksPass)),
    ]
}

#[test]
fn every_band_built_from_empty_text_loads_in_the_order_it_was_written() {
    let done = amend(at(), "", &whole()).unwrap_or_else(|why| panic!("builds: {why}"));
    let manifest = done.manifest();
    assert_eq!(manifest.id().as_str(), "storefront");
    assert_eq!(manifest.checks_as_written(), ["e2e"]);
    assert!(manifest
        .command("reset")
        .expect("declared")
        .is_destructive());
    assert_eq!(manifest.port("web").expect("declared").env(), Some("PORT"));
    let setup: Vec<&str> = manifest
        .prepared_by()
        .iter()
        .map(|one| one.name())
        .collect();
    assert_eq!(setup, ["migrate"]);
    assert_eq!(manifest.auto_merge(), AutoMerge::ChecksPass);
    assert!(
        done.text().starts_with("version: 1\nid: storefront\n"),
        "{}",
        done.text()
    );
}

#[test]
fn without_version_and_id_empty_text_is_refused_at_both() {
    let refused = amend(at(), "", &whole()[2..]).expect_err("no version or id");
    let NotAmended::Refused(load) = refused else {
        panic!("refused by the parser: {refused}");
    };
    let mut missing: Vec<&str> = load
        .refusals()
        .iter()
        .filter(|one| one.fault == Fault::Missing)
        .map(|one| one.key.as_str())
        .collect();
    missing.sort_unstable();
    assert_eq!(missing, ["id", "version"]);
}

#[test]
fn clearing_setup_requires_removes_setup_and_an_id_with_a_colon_reads_back() {
    let mut edits = whole();
    edits.push(Edit::SetupRequires(Vec::new()));
    edits.push(Edit::Id("shop: web".to_string()));
    let done = amend(at(), "", &edits).unwrap_or_else(|why| panic!("builds: {why}"));
    assert!(done.manifest().prepared_by().is_empty());
    assert!(!done.text().contains("setup"), "{}", done.text());
    assert_eq!(done.manifest().id().as_str(), "shop: web");
}
