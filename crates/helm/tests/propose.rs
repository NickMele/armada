//! `armada manifest config scan`, from a real directory to a written
//! `armada.yml`.
//!
//! **The half of layer 1½ that unit tests cannot reach.** `armada_core::propose`
//! decides what is provable and is tested where it is decided; this drives the
//! whole exchange — a directory on disk, the scan that reads it, the tick list
//! that answers, and the file that ends up beside it — because a green test on a
//! pure function does not prove the verb ever calls it (`AGENTS.md`, *Testing*).
//!
//! **Nothing here starts a session or spends a token.** The hand-over is the
//! other answer to the same question and is covered by the render fixtures; what
//! runs here writes a YAML file into a `TempDir` and reads it back.

use armada_core::propose::Proposal;
use armada_helm::ask::Scripted;
use armada_helm::render::term::Terminal;
use armada_helm::verbs::{self, config::Wrote};
use armada_manifest::process::RealRun;
use std::path::Path;

/// A `package.json` whose scripts are half provable and half not — the shape
/// that raised this whole feature.
const PACKAGE: &str = r#"{
  "name": "shop",
  "packageManager": "pnpm@9.1.0",
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "test": "vitest run",
    "test:changed": "vitest",
    "lint": "eslint ."
  }
}"#;

/// A repository with one package at the root, on disk.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    write(dir.path(), "package.json", PACKAGE);
    write(dir.path(), "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    dir
}

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("a directory to write into");
    }
    std::fs::write(path, text).expect("a file");
}

/// What `config scan` proposes for a directory, through the verb rather than
/// through the pure function under it.
fn proposals(root: &Path) -> Vec<Proposal> {
    let output = verbs::config::scan(&RealRun, root, None, Terminal::piped(), true)
        .expect("a readable directory always answers");
    let armada_helm::verbs::Output::Scan(envelope) = output else {
        panic!("config scan answered with something that is not a scan");
    };
    envelope.data.proposals
}

/// **The whole exchange, and the file at the end of it.** Everything opens
/// ticked, the reader confirms, and what lands on disk is a document Armada's
/// own parser accepts — which is the difference between proposing lines and
/// proposing a config.
#[test]
fn ticking_everything_writes_a_config_that_parses() {
    let dir = repo();
    let proposals = proposals(dir.path());
    let mut ask = Scripted {
        picks: vec![vec![true; proposals.len()]],
        ..Default::default()
    };

    let wrote = verbs::config::confirm(&mut ask, &proposals, dir.path());
    assert!(
        matches!(wrote, Wrote::Config(lines) if lines == proposals.len()),
        "{wrote:?}"
    );

    let text = std::fs::read_to_string(dir.path().join("armada.yml")).expect("armada.yml");
    let config = armada_core::config::parse(&text, "armada.yml").expect("it parses");
    let resolved = armada_core::config::resolve(
        config,
        &armada_core::config::Defaults::built_in(),
        "armada.yml",
    )
    .expect("it resolves");

    let component = resolved.components.get("shop").expect("the component");
    assert_eq!(component.checks["test"].cmd, "pnpm run test");
    assert_eq!(component.checks["lint"].cmd, "pnpm run lint");
    assert_eq!(component.checks["build"].cmd, "pnpm run build");
    // The near-match is the one that must not be here, whatever else is.
    assert!(
        !component.checks.contains_key("test:changed"),
        "a near-match reached the file"
    );
}

/// **The question is put with every proposal in it**, in the words the report
/// used. A tick list that offered a subset would be a reader confirming
/// something other than what he read.
#[test]
fn every_proposal_is_offered() {
    let dir = repo();
    let proposals = proposals(dir.path());
    let mut ask = Scripted::default();
    verbs::config::confirm(&mut ask, &proposals, dir.path());

    let (question, offered) = ask.picked.first().expect("a tick list was put");
    assert_eq!(question, verbs::config::CONFIRM_QUESTION);
    assert_eq!(
        offered,
        &proposals
            .iter()
            .map(|proposal| proposal.at.clone())
            .collect::<Vec<_>>()
    );
}

/// **Unticking a component takes its checks with it**, because a check under a
/// component nobody accepted is a document that does not parse.
#[test]
fn unticking_the_component_drops_everything_under_it() {
    let dir = repo();
    let proposals = proposals(dir.path());
    let mut ticked = vec![true; proposals.len()];
    ticked[0] = false;
    let mut ask = Scripted {
        picks: vec![ticked],
        ..Default::default()
    };

    verbs::config::confirm(&mut ask, &proposals, dir.path());
    let text = std::fs::read_to_string(dir.path().join("armada.yml")).expect("armada.yml");
    assert!(!text.contains("checks:"), "{text}");
    armada_core::config::parse(&text, "armada.yml").expect("it still parses");
}

/// **Two ways to write nothing, and neither is a failure.** The reader was
/// asked and answered; the evidence he was answering about is still on screen.
#[test]
fn nothing_is_written_when_nothing_is_confirmed() {
    let dir = repo();
    let proposals = proposals(dir.path());
    // Esc, which answers nothing at all; and every row unticked, which is an
    // answer. Both write nothing, and neither is an error.
    for picks in [Vec::new(), vec![vec![false; proposals.len()]]] {
        let dir = repo();
        let mut ask = Scripted {
            picks,
            ..Default::default()
        };
        let wrote = verbs::config::confirm(&mut ask, &proposals, dir.path());
        assert!(matches!(wrote, Wrote::Nothing), "{wrote:?}");
        assert!(
            !dir.path().join("armada.yml").exists(),
            "a file was written by a reader who confirmed nothing"
        );
    }
}

/// **It refuses rather than overwrites.** A repository that gained an
/// `armada.yml` between the scan and the answer is one where the file in front
/// of Armada is not the one it was proposing against — and whether the two agree
/// is drift, which is not built.
#[test]
fn an_existing_config_is_never_overwritten() {
    let dir = repo();
    let proposals = proposals(dir.path());
    let mine = "manifest:\n  version: 1\n  components:\n    mine: {}\n";
    write(dir.path(), "armada.yml", mine);

    let mut ask = Scripted {
        picks: vec![vec![true; proposals.len()]],
        ..Default::default()
    };
    let wrote = verbs::config::confirm(&mut ask, &proposals, dir.path());
    let Wrote::Failed(error) = wrote else {
        panic!("it wrote over a config that was already there: {wrote:?}");
    };
    assert_eq!(error.class, armada_core::error::ErrClass::BadConfig);
    assert!(
        error.next_action.is_some(),
        "bad_config needs a next action"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("armada.yml")).expect("armada.yml"),
        mine,
        "the reader's own file was changed"
    );
}

/// **A repository whose evidence settles nothing proposes nothing**, and that is
/// the case the agent hand-over exists for. This is the reduction claim stated
/// as a test: proposals are the cheap path *and* they do not pretend to cover
/// what they cannot.
#[test]
fn a_repository_with_nothing_provable_proposes_nothing() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    write(
        dir.path(),
        "README.md",
        "# a repository nobody configured\n",
    );
    write(
        dir.path(),
        "docker-compose.yml",
        "services:\n  db:\n    image: postgres\n",
    );
    assert!(proposals(dir.path()).is_empty());
}

/// The three answers `scan` ends on, and the one that is offered only when there
/// is something to write.
///
/// **The numbers move and the meanings do not**, which is the whole reason the
/// answer is an enum: `1` means *write* in a repository with proposals and
/// *hand over* in one without, and a caller comparing against a literal would
/// silently mean the wrong one in half of them.
#[test]
fn the_answers_mean_the_same_thing_whichever_list_they_came_from() {
    use verbs::config::{next, onboarding_choices, stop, Next};

    assert_eq!(onboarding_choices(0).len(), 2);
    assert_eq!(onboarding_choices(6).len(), 3);

    assert_eq!(next(1, 6), Next::Write);
    assert_eq!(next(2, 6), Next::HandOver);
    assert_eq!(next(3, 6), Next::Stop);

    assert_eq!(next(1, 0), Next::HandOver);
    assert_eq!(next(2, 0), Next::Stop);

    // The default is always the last option, which is *stop*: doing nothing is
    // what an unanswered question gets.
    assert_eq!(next(stop(6), 6), Next::Stop);
    assert_eq!(next(stop(0), 0), Next::Stop);
}
