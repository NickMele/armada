//! A workspace Verify's `${port.NAME}`: its own file's ports, from a span
//! claimed for the Verify, laid over the root's.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use store::{PortClaim, PortClaimant};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::ports::PortRange;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A root declaring no ports.
const BARE_ROOT: &str = "version: 1\nid: 01FIXTUREMANIFEST\n";

/// A root declaring `docs` and, like the workspace, `web`.
const PORTED_ROOT: &str = "version: 1\nid: 01FIXTUREMANIFEST\nports:\n  docs: {}\n  web: {}\n";

/// `apps/web`'s own: `api` and `web`, and one Check writing down what it saw.
const WEB: &str = r#"version: 1
id: 01WEBMANIFEST
ports:
  api: {}
  web: {}
checks:
  seen:
    run: "/bin/sh -c 'echo $1 $2 $3 $4 $ARMADA_PORT_WEB > seen.txt' sh ${port.web} ${port.api} ${port.docs} ${port.nobody}"
"#;

/// A checkout with `root` at its root and `apps/web`'s own file, claiming from
/// `range` — each case its own, since a probe binds for an instant.
fn a_checkout(home: &TempDir, root: &str, range: PortRange) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), root)
        .unwrap_or_else(|why| panic!("the root manifest did not parse: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest = manifest;
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    fittings.port_range = range;
    let at = home.path();
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?} failed");
    };
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), root).expect("the root Manifest");
    std::fs::create_dir_all(at.join("apps/web")).expect("a workspace");
    std::fs::write(at.join("apps/web/armada.yml"), WEB).expect("its Manifest");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
    Arc::new(Fleet::assembled(fittings))
}

/// Verify `apps/web` to its end, and answer the words its Check wrote down:
/// `web api docs nobody ARMADA_PORT_WEB`.
async fn verified(home: &TempDir, fleet: &Arc<Fixture>) -> Vec<String> {
    Arc::clone(fleet)
        .begin_checkout_verify(fleet.first(), Some("apps/web"))
        .await
        .expect("underway");
    let verify = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let sheet = fleet
                .checkout_run_sheet(fleet.first())
                .await
                .expect("a sheet");
            if let Some(verify) = sheet.verify.filter(|one| one.ended_at.is_some()) {
                return verify;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the Verify ended");
    let [step] = verify.steps.as_slice() else {
        panic!("one Check: {:?}", verify.steps);
    };
    assert!(
        matches!(&step.state, ipc::VerifyStepState::Ran { record } if record.exit_code == Some(0)),
        "{step:?}"
    );
    let said = std::fs::read_to_string(home.path().join("apps/web/seen.txt")).expect("it wrote");
    said.split_whitespace().map(str::to_string).collect()
}

fn port(word: &str) -> u16 {
    word.parse()
        .unwrap_or_else(|_| panic!("`{word}` is not a number"))
}

fn workspace_claimant(fleet: &Fixture) -> PortClaimant {
    let dir = Path::new(fleet.first().root()).join("apps/web");
    PortClaimant::MainCheckout(dir.to_string_lossy().into_owned())
}

async fn claims(fleet: &Fixture) -> Vec<PortClaim> {
    fleet.store().lock().await.every_port_claim().expect("read")
}

/// **A port only the workspace declares resolves to a real number**, in the
/// command and as `ARMADA_PORT_<NAME>`, and its span is given back when the
/// Verify ends.
#[tokio::test]
async fn a_workspace_port_resolves_and_its_span_is_released_when_the_verify_ends() {
    let home = TempDir::new();
    let fleet = a_checkout(&home, BARE_ROOT, PortRange::of(44_600, 44_699, 8));

    let seen = verified(&home, &fleet).await;
    let (web, api) = (port(&seen[0]), port(&seen[1]));
    assert!((44_600..=44_699).contains(&web), "{seen:?}");
    assert_eq!(web, api + 1, "one span, in name order: {seen:?}");
    assert_eq!(seen[4], seen[0], "the variable carries the same number");
    assert_eq!(seen[2], "${port.docs}", "the root declares no `docs`");

    assert!(
        claims(&fleet).await.is_empty(),
        "given back, and the bare root claimed nothing"
    );
}

/// **A name both declare is the workspace's inside its run; a name only the
/// root declares is the root's; a name neither declares stays as written.**
/// The root's own span is kept.
#[tokio::test]
async fn the_workspace_wins_a_shared_name_and_an_undeclared_one_stays_literal() {
    let home = TempDir::new();
    let fleet = a_checkout(&home, PORTED_ROOT, PortRange::of(44_700, 44_799, 8));

    let seen = verified(&home, &fleet).await;
    let held = claims(&fleet).await;
    let [root] = held.as_slice() else {
        panic!("only the root's span is still held: {held:?}");
    };
    assert_eq!(
        root.claimant,
        PortClaimant::MainCheckout(fleet.first().root().to_string())
    );
    let (web, api, docs) = (port(&seen[0]), port(&seen[1]), port(&seen[2]));
    assert_eq!(docs, root.base, "`docs` is the root's: {seen:?}");
    assert_ne!(web, root.base + 1, "not the root's `web`: {seen:?}");
    assert_eq!(web, api + 1, "the workspace's own `web`: {seen:?}");
    assert_eq!(seen[4], seen[0], "the variable follows the workspace too");
    assert_eq!(
        seen[3], "${port.nobody}",
        "an undeclared name stays literal"
    );
}

/// **A span a crashed Verify left is found by its key and replaced**, sized
/// from the file as it reads now, then released like any other.
#[tokio::test]
async fn a_span_a_crashed_verify_left_is_reclaimed_by_its_key() {
    let home = TempDir::new();
    let fleet = a_checkout(&home, BARE_ROOT, PortRange::of(44_800, 44_899, 8));
    let left = PortClaim {
        claimant: workspace_claimant(&fleet),
        base: 44_890,
        width: 1,
        claimed_at: fleet.now(),
    };
    fleet
        .store()
        .lock()
        .await
        .claim_port_span(&left)
        .expect("a leftover row");

    let seen = verified(&home, &fleet).await;
    let (web, api) = (port(&seen[0]), port(&seen[1]));
    assert_eq!(
        web,
        api + 1,
        "both names resolved, not cut to width 1: {seen:?}"
    );
    assert!(
        claims(&fleet).await.is_empty(),
        "the leftover went with the Verify"
    );
}
