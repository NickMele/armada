//! A Command with `serve` — `docs/concepts/manifest.md`, *Commands that keep
//! running*. `run`, `serve`, `ready` and `links`.

use super::parse;
use crate::error::Fault;
use crate::tests::{fault_at, refusals};

const STORYBOOK: &str = "version: 1\nid: armada\nports:\n  storybook: {}\ncommands:\n  \
     storybook:\n    run: pnpm build\n    serve: storybook dev -p ${port.storybook}\n    \
     ready: curl -sf http://localhost:${port.storybook}\n    links:\n      - url: \
     http://localhost:${port.storybook}\n        name: Storybook\n      - url: \
     http://localhost:${port.storybook}/docs\n  fmt:\n    run: cargo fmt\n";

#[test]
fn a_command_with_serve_is_a_server_and_never_a_command() {
    let manifest = parse(STORYBOOK).expect("a server parses");
    assert_eq!(manifest.server_names(), ["storybook"]);
    assert_eq!(
        manifest.command_names(),
        ["fmt"],
        "a server is not a Command"
    );
    assert!(manifest.command("storybook").is_none());
    let server = manifest.server("storybook").expect("storybook");
    assert_eq!(server.run(), Some("pnpm build"));
    assert_eq!(server.serve(), "storybook dev -p ${port.storybook}");
    assert_eq!(
        server.ready(),
        Some("curl -sf http://localhost:${port.storybook}")
    );
    let links: Vec<(&str, Option<&str>)> =
        server.links().iter().map(|l| (l.url(), l.name())).collect();
    assert_eq!(
        links,
        [
            ("http://localhost:${port.storybook}", Some("Storybook")),
            ("http://localhost:${port.storybook}/docs", None),
        ]
    );
}

#[test]
fn serve_alone_is_a_server_serving_once_started_with_no_links() {
    let manifest = parse("version: 1\nid: armada\ncommands:\n  web:\n    serve: node server.js\n")
        .expect("serve alone parses");
    let server = manifest.server("web").expect("web");
    assert_eq!((server.run(), server.ready()), (None, None));
    assert!(server.links().is_empty());
}

#[test]
fn ready_or_links_without_serve_is_refused() {
    let refused = refusals(parse(
        "version: 1\nid: armada\ncommands:\n  fmt:\n    run: cargo fmt\n    ready: \
         /usr/bin/true\n    links:\n      - url: http://localhost:1\n",
    ));
    assert_eq!(
        fault_at(&refused, "commands.fmt.ready"),
        &Fault::OnlyAServerReads
    );
    assert_eq!(
        fault_at(&refused, "commands.fmt.links"),
        &Fault::OnlyAServerReads
    );
}

#[test]
fn a_command_with_neither_run_nor_serve_is_missing_run() {
    let refused = refusals(parse(
        "version: 1\nid: armada\ncommands:\n  nothing:\n    destructive: false\n",
    ));
    assert_eq!(fault_at(&refused, "commands.nothing.run"), &Fault::Missing);
}

#[test]
fn a_link_with_no_url_or_an_unknown_key_is_refused() {
    let refused = refusals(parse(
        "version: 1\nid: armada\ncommands:\n  web:\n    serve: node s.js\n    links:\n      \
         - name: Web\n        target: _blank\n",
    ));
    assert_eq!(
        fault_at(&refused, "commands.web.links[0].url"),
        &Fault::Missing
    );
    assert!(matches!(
        fault_at(&refused, "commands.web.links[0].target"),
        Fault::Unknown { .. }
    ));
}

#[test]
fn a_requires_naming_a_server_is_refused_because_it_never_exits() {
    let refused = refusals(parse(
        "version: 1\nid: armada\ncommands:\n  web:\n    serve: node s.js\nchecks:\n  e2e:\n    \
         run: pnpm e2e\n    requires: [web]\nsetup:\n  requires: [web]\n",
    ));
    let expected = Fault::RequiresAServer {
        value: String::from("web"),
    };
    assert_eq!(fault_at(&refused, "checks.e2e.requires[0]"), &expected);
    assert_eq!(fault_at(&refused, "setup.requires[0]"), &expected);
}

#[test]
fn a_server_named_like_a_check_is_in_both_registries() {
    let refused = refusals(parse(
        "version: 1\nid: armada\nchecks:\n  web:\n    run: pnpm test\ncommands:\n  web:\n    \
         serve: node s.js\n",
    ));
    assert_eq!(
        fault_at(&refused, "commands.web"),
        &Fault::DeclaredInBothRegistries
    );
}
