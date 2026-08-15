//! What a scan can write down without guessing — layer 1½ of the bootstrap
//! sandwich (PLAN.md §5).
//!
//! # The line this does not cross
//!
//! **`PLAN.md` §5 still says do not write a stack-detection engine, and this is
//! not one.** The distinction that keeps that rule standing, recorded in
//! `docs/reserved/007-scanner-should-propose.md`: the scanner may **propose what
//! it can prove**, and must never guess. Every proposal below is an exact match
//! on a name that is literally written in a file the repository already had, and
//! every one of them is rejectable before anything is written.
//!
//! | Proposal | The proof |
//! |---|---|
//! | `workspaces: [dir]` | `dir/armada.yml` exists — the file `verify` requires is already there |
//! | `components.<name>` | `dir` carries a package manifest, and something in it is a check |
//! | `setup: <pm> install` | the lockfile in that directory names `<pm>`, and `install` is what all four spell it |
//! | `checks.<name>` | a script or target whose name is **exactly** `<name>` |
//!
//! # The judgement calls are still the reader's, and that is the whole feature
//!
//! Run against a real polyglot repository the scan found three directories that
//! resolve their own dependencies, and only two of them were products —
//! `scripts/` was a toolset. **No amount of evidence would have settled that.**
//! So nothing here decides: [`propose`] produces a list, the reader ticks what is
//! right, and [`write`] emits only what survived.
//!
//! # Why `test` and not `test:changed`
//!
//! A near-match is a guess wearing a fact's clothes. `test:changed` and
//! `test:coverage` are *plausibly* the test check and neither is provably it, so
//! neither is offered — and the schema agrees by accident of grammar, because a
//! check id must match `^[a-z0-9][a-z0-9_-]*$` and a colon is not in it.

use crate::scan::{Evidence, Package};
use serde::Serialize;

/// Script and target names that are provably a check.
///
/// **A closed list, and every exclusion from it is an argument rather than an
/// oversight**:
///
/// - **`fmt` / `format` are out.** A formatter's bare name rewrites the tree.
///   `armada manifest check` must never mutate a working copy, and the check is
///   `fmt --check` — which is a flag nobody wrote down, so it cannot be proposed.
/// - **`e2e` is out.** It is a real check id in this repository's own fixtures
///   and it always arrives with `cost:` and often `exclusive:`; proposing it
///   without either is proposing a scheduling decision, not a fact.
/// - **`check` is out.** It is Armada's own verb, and a repository's `check`
///   script usually means *run everything*, which would nest a suite inside a
///   suite for a reader who could not see it happening.
///
/// What is left are the five names whose meaning is the same in every repository
/// that has them: run this, read the exit code, change nothing.
pub const CHECKS: &[&str] = &["build", "lint", "test", "typecheck", "types"];

/// The package managers whose script runner is spelled `<pm> run <name>` and
/// whose installer is spelled `<pm> install`.
///
/// **Four, and the same four `config verify` already restricts its `exec`
/// suggestion to.** `uv`, `poetry`, `cargo` and `go` each resolve dependencies
/// from a lockfile and none of them runs a named script out of a manifest, so a
/// proposal phrased for them would be invented rather than read. A repository
/// with one of those lockfiles still gets its `Makefile` targets, which need no
/// runner at all.
pub const RUNNERS: &[&str] = &["pnpm", "npm", "yarn", "bun"];

/// The name a root component gets when its manifest declares none Armada can
/// use.
///
/// **`app`, because both single-component fixtures already call it that** —
/// `go-service` and `rails-monolith` — and a repository that *is* one component
/// needs some name for it. It is a proposal like every other line here, so a
/// reader who calls it something else unticks it and writes his own.
pub const ROOT: &str = "app";

/// Which part of `armada.yml` a proposal would write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A nested workspace, because it already has an `armada.yml`.
    Workspace,
    /// A component, because a directory holds a package with work in it.
    Component,
    /// A component's `setup:`, from the lockfile in its directory.
    Setup,
    /// One check, from a name that matched exactly.
    Check,
}

impl Kind {
    /// The word the report and the tick list print.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Workspace => "workspace",
            Kind::Component => "component",
            Kind::Setup => "setup",
            Kind::Check => "check",
        }
    }
}

/// One line Armada is prepared to write, and the file that proves it.
///
/// **`because` is not decoration.** Everything here is derived from a file the
/// repository already had, and a reader deciding whether a proposal is right is
/// deciding about that file — so the report names it beside every row rather
/// than asking him to work out which of fifteen `package.json`s a check came
/// from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Proposal {
    /// Which part of the document this touches.
    pub kind: Kind,
    /// The key path it would write: `workspaces`, `components.web`,
    /// `components.web.setup`, `components.web.checks.test`.
    pub at: String,
    /// The value it would write, or the words for one that has no value of its
    /// own.
    pub writes: String,
    /// The file that proves it. Workspace-relative, like every other path in the
    /// report.
    pub because: String,
    /// Which component it belongs to; empty for a workspace.
    ///
    /// **Carried so that rejecting a component takes its checks with it.** A
    /// reader who unticks `components.scripts` is saying that directory is not a
    /// component, and writing three checks into a component that does not exist
    /// would be an `armada.yml` that cannot parse.
    pub component: String,
    /// The component's `root:`, empty when the component is the repository.
    pub root: String,
}

/// Everything provable in one scan's evidence, in the order a reader meets it.
///
/// Workspaces first, because they are a statement about the shape of the whole
/// repository; then each component with its own lines beneath it, so a reader
/// ticking through the list never has to hold a component in his head while the
/// checks that belong to it go past.
pub fn propose(evidence: &Evidence) -> Vec<Proposal> {
    let mut out = Vec::new();

    for dir in &evidence.nested {
        out.push(Proposal {
            kind: Kind::Workspace,
            at: "workspaces".to_string(),
            writes: dir.clone(),
            because: format!("{dir}/armada.yml"),
            component: String::new(),
            root: String::new(),
        });
    }

    let mut taken: Vec<String> = Vec::new();
    for package in &evidence.packages {
        // **A package inside a nested workspace belongs to that workspace, not
        // to this one.** Its `armada.yml` is what says what it runs, and a
        // component here whose `root:` reached into it is the overlap PLAN.md
        // §4.6 makes illegal — `verify` would reject the file this wrote.
        if evidence
            .nested
            .iter()
            .any(|nested| package.dir == *nested || package.dir.starts_with(&format!("{nested}/")))
        {
            continue;
        }

        let Some(name) = component_name(package) else {
            continue;
        };
        // Two directories cannot both be one component. The first wins, which is
        // the shallower one: `packages` arrives sorted by directory.
        if taken.contains(&name) {
            continue;
        }

        let manager = manager_in(evidence, &package.dir);
        let checks = checks_in(evidence, package, manager.as_deref());
        // **A component with nothing to run is a name, not a fact.** Proposing
        // one would be proposing that this directory is a unit of work, which is
        // exactly the judgement `scripts/` cost the reader in the first place.
        if checks.is_empty() {
            continue;
        }

        taken.push(name.clone());
        let at = format!("components.{name}");
        out.push(Proposal {
            kind: Kind::Component,
            at: at.clone(),
            writes: if package.dir.is_empty() {
                "the repository itself".to_string()
            } else {
                format!("root: {}", package.dir)
            },
            because: under(&package.dir, &package.manifests),
            component: name.clone(),
            root: package.dir.clone(),
        });

        // `setup:` goes on the component that **holds** the lockfile, because
        // that is the directory the install resolves for. A pnpm workspace
        // member has no lockfile of its own — the root resolved for it — so the
        // root's component carries the one install and the members carry none.
        if let Some(manager) = manager.as_deref().filter(|_| !package.lockfiles.is_empty()) {
            if RUNNERS.contains(&manager) {
                out.push(Proposal {
                    kind: Kind::Setup,
                    at: format!("{at}.setup"),
                    writes: format!("{manager} install"),
                    because: under(&package.dir, &package.lockfiles),
                    component: name.clone(),
                    root: package.dir.clone(),
                });
            }
        }

        for (check, cmd, from) in checks {
            out.push(Proposal {
                kind: Kind::Check,
                at: format!("{at}.checks.{check}"),
                writes: cmd,
                because: from,
                component: name.clone(),
                root: package.dir.clone(),
            });
        }
    }

    out
}

/// The component a package would be called.
///
/// **The directory's own last segment**, which is the name the repository
/// already uses for it in every path a reader has ever typed. The root has no
/// segment, so it falls back to the name its `package.json` declares — with any
/// `@scope/` dropped, because a scope is a registry's namespace and not part of
/// what the package is called — and then to [`ROOT`].
///
/// `None` when nothing legal comes out of that. A component id must match
/// `^[a-z0-9][a-z0-9_-]*$`, and inventing a spelling for `Web UI` is the kind of
/// small helpful guess that makes a whole report untrustworthy.
fn component_name(package: &Package) -> Option<String> {
    let candidate = match package.dir.rsplit('/').next().filter(|s| !s.is_empty()) {
        Some(segment) => segment.to_string(),
        None => package
            .name
            .as_deref()
            .and_then(|name| name.rsplit('/').next())
            .unwrap_or(ROOT)
            .to_string(),
    };
    legal(&candidate).then_some(candidate)
}

/// Whether a name is one the schema's `^[a-z0-9][a-z0-9_-]*$` accepts.
///
/// **Written out rather than reached for through the regex crate**, because it
/// is four conditions and a dependency on a pattern spelled in a second place is
/// how the two drift.
fn legal(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// The package manager proved for a directory: its own lockfile, else the
/// nearest one above it.
///
/// **Nearest and not first.** A repository with `uv.lock` at the root and
/// `web/pnpm-lock.yaml` one level down runs `web`'s scripts with pnpm, and a
/// scan that answered "the first lockfile I met" would phrase every check in the
/// repository for whichever directory sorted earliest.
fn manager_in(evidence: &Evidence, dir: &str) -> Option<String> {
    evidence
        .lockfiles
        .iter()
        .filter(|lockfile| covers(crate::scan::directory(&lockfile.file), dir))
        .max_by_key(|lockfile| crate::scan::directory(&lockfile.file).len())
        // `pnpm@9` is the manager and the version it is pinned to; the command
        // is the manager alone.
        .map(|lockfile| {
            lockfile
                .manager
                .split('@')
                .next()
                .unwrap_or_default()
                .to_string()
        })
}

/// Whether `above` is `dir` or an ancestor of it.
fn covers(above: &str, dir: &str) -> bool {
    above.is_empty() || above == dir || dir.starts_with(&format!("{above}/"))
}

/// Every check this package proves, as `(name, cmd, the file that proves it)`.
///
/// **Declaration order, never sorted** — the same rule the evidence report
/// follows, and for the same reason: a `package.json` is written in the order
/// its author thought about the work.
fn checks_in(
    evidence: &Evidence,
    package: &Package,
    manager: Option<&str>,
) -> Vec<(String, String, String)> {
    let mut out = Vec::new();

    if let Some(manager) = manager.filter(|manager| RUNNERS.contains(manager)) {
        for source in evidence
            .scripts
            .iter()
            .filter(|source| crate::scan::directory(&source.file) == package.dir)
        {
            for script in &source.scripts {
                if CHECKS.contains(&script.name.as_str()) {
                    out.push((
                        script.name.clone(),
                        format!("{manager} run {}", script.name),
                        source.file.clone(),
                    ));
                }
            }
        }
    }

    // **A `Makefile` needs no package manager**, which is what makes it the only
    // evidence a Python or Go repository has that Armada can phrase a command
    // from. A target and a script of the same name are the same check twice, so
    // the script wins: it is the one the package manager already knows how to
    // run.
    for makefile in evidence
        .makefiles
        .iter()
        .filter(|makefile| crate::scan::directory(&makefile.file) == package.dir)
    {
        for target in &makefile.targets {
            if CHECKS.contains(&target.as_str()) && !out.iter().any(|(name, _, _)| name == target) {
                out.push((
                    target.clone(),
                    format!("make {target}"),
                    makefile.file.clone(),
                ));
            }
        }
    }

    out
}

/// Files in a directory, named the way every other path in the report is named:
/// **workspace-relative and complete**.
///
/// `package.json in web` and `web/package.json` are the same fact, and the
/// second is the one a reader can paste. Location is evidence
/// (`docs/commands/manifest/config.md`), and a proposal's whole claim on the
/// reader's trust is that he can open the file it names.
fn under(dir: &str, files: &[String]) -> String {
    files
        .iter()
        .map(|file| match dir.is_empty() {
            true => file.clone(),
            false => format!("{dir}/{file}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The proposals a reader left ticked.
///
/// **A flag per proposal, in the order they were offered**, which is the shape
/// the tick list answers in. A short answer keeps what it did not reach rather
/// than dropping it: the flags and the proposals are two halves of one list, and
/// a mismatch is a bug in the caller, not a licence to write a different file
/// from the one the reader confirmed.
pub fn accepted(proposals: &[Proposal], ticked: &[bool]) -> Vec<Proposal> {
    proposals
        .iter()
        .enumerate()
        .filter(|(index, _)| ticked.get(*index).copied().unwrap_or(true))
        .map(|(_, proposal)| proposal.clone())
        .collect()
}

/// The `armada.yml` that the accepted proposals add up to.
///
/// **Only what survived.** A check whose component was rejected is dropped
/// rather than orphaned — writing `components.scripts.checks.test` under a
/// component nobody accepted is a document that does not parse, which is a worse
/// outcome than the proposal the reader was refusing.
///
/// **Every line is attributed in a comment.** The file this writes will be read
/// by whoever inherits the repository, and a `cmd:` with no note of where it came
/// from is indistinguishable from one an agent invented — which is the whole
/// distrust this layer exists to remove.
pub fn write(accepted: &[Proposal]) -> String {
    let mut out = String::from(
        "# armada.yml — proposed by `armada manifest config scan` and confirmed by hand.\n\
         #\n\
         # Every line below was read out of a file in this repository, named in the\n\
         # comment above it. Nothing here was inferred. What a scan cannot prove — which\n\
         # suite is slow enough to deserve `cost:`, which two cannot share a browser,\n\
         # what genuinely needs Postgres — is still yours to add.\n\
         \n\
         manifest:\n  version: 1\n",
    );

    let workspaces: Vec<&Proposal> = accepted
        .iter()
        .filter(|proposal| proposal.kind == Kind::Workspace)
        .collect();
    if !workspaces.is_empty() {
        out.push_str("\n  # each of these has an armada.yml of its own\n");
        out.push_str(&format!(
            "  workspaces: [{}]\n",
            workspaces
                .iter()
                .map(|proposal| proposal.writes.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let components: Vec<&Proposal> = accepted
        .iter()
        .filter(|proposal| proposal.kind == Kind::Component)
        .collect();
    if components.is_empty() {
        return out;
    }

    out.push_str("\n  components:\n");
    for component in components {
        let mine = |kind: Kind| {
            accepted.iter().filter(move |proposal| {
                proposal.kind == kind && proposal.component == component.component
            })
        };
        out.push_str(&format!("\n    # {}\n", component.because));
        out.push_str(&format!("    {}:\n", component.component));
        if !component.root.is_empty() {
            out.push_str(&format!("      root: {}\n", component.root));
        }
        for setup in mine(Kind::Setup) {
            out.push_str(&format!("      # {}\n", setup.because));
            out.push_str(&format!("      setup: {}\n", setup.writes));
        }

        let mut checks = mine(Kind::Check).peekable();
        if checks.peek().is_none() {
            continue;
        }
        out.push_str("      checks:\n");
        for check in checks {
            let name = check.at.rsplit('.').next().unwrap_or_default();
            out.push_str(&format!("        # {}\n", check.because));
            out.push_str(&format!("        {name}:\n"));
            out.push_str(&format!("          cmd: {}\n", check.writes));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{scan, SourceFile};

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile::new(path, text)
    }

    fn of(files: &[SourceFile]) -> Vec<Proposal> {
        propose(&scan(files))
    }

    fn ats(proposals: &[Proposal]) -> Vec<&str> {
        proposals.iter().map(|p| p.at.as_str()).collect()
    }

    const SCRIPTS: &str = r#"{
      "name": "@acme/web",
      "scripts": {
        "dev": "next dev",
        "test": "vitest run",
        "test:changed": "vitest",
        "lint": "eslint .",
        "lint:fix": "eslint . --fix"
      }
    }"#;

    /// **The rule the whole feature stands on.** `test` is proposed because a
    /// script is literally called that; `test:changed` is not, because a
    /// near-match is a guess wearing a fact's clothes.
    #[test]
    fn only_an_exact_name_is_a_check() {
        let proposals = of(&[
            file("web/package.json", SCRIPTS),
            file("web/pnpm-lock.yaml", ""),
        ]);
        assert_eq!(
            ats(&proposals),
            [
                "components.web",
                "components.web.setup",
                "components.web.checks.test",
                "components.web.checks.lint",
            ],
            "a near-match was proposed, or declaration order was lost"
        );
    }

    /// **A lockfile is proof of a package manager, not evidence of one**, and it
    /// is the pinned version's manager rather than the pin.
    #[test]
    fn the_lockfile_names_the_runner() {
        let proposals = of(&[
            file(
                "package.json",
                r#"{"name":"x","packageManager":"pnpm@9.1.0","scripts":{"test":"vitest"}}"#,
            ),
            file("pnpm-lock.yaml", ""),
        ]);
        let check = proposals.last().expect("a check");
        assert_eq!(check.writes, "pnpm run test");
        let setup = &proposals[1];
        assert_eq!(setup.writes, "pnpm install");
    }

    /// **The nearest lockfile above, not the first one found.** A repository
    /// with `uv.lock` at the root and pnpm one level down runs `web`'s scripts
    /// with pnpm.
    #[test]
    fn the_nearest_lockfile_wins() {
        let proposals = of(&[
            file("pyproject.toml", ""),
            file("uv.lock", ""),
            file("web/package.json", SCRIPTS),
            file("web/pnpm-lock.yaml", ""),
        ]);
        assert!(proposals
            .iter()
            .all(|p| p.kind != Kind::Check || p.writes.starts_with("pnpm run")));
    }

    /// A manager with no script runner phrases nothing. `uv` resolves
    /// dependencies and does not run a named script out of a manifest, so a
    /// `uv run test` would be invented rather than read.
    #[test]
    fn a_manager_without_a_runner_proposes_no_check() {
        let proposals = of(&[
            file("backend/pyproject.toml", ""),
            file("backend/uv.lock", ""),
        ]);
        assert!(proposals.is_empty(), "{proposals:#?}");
    }

    /// **A Makefile needs no package manager**, which is what gives a Python or
    /// Go repository a check at all.
    #[test]
    fn a_makefile_target_is_a_check_on_its_own() {
        let proposals = of(&[
            file("backend/pyproject.toml", ""),
            file("backend/uv.lock", ""),
            file(
                "backend/Makefile",
                "test:\n\tpytest\nrelease:\n\ttwine upload\n",
            ),
        ]);
        assert_eq!(
            ats(&proposals),
            ["components.backend", "components.backend.checks.test"],
            "a target that is not a check name was proposed, or uv grew a setup line"
        );
    }

    /// A directory with a manifest and nothing to run is a name, not a fact —
    /// which is exactly what `scripts/` was in the repository that raised this.
    #[test]
    fn a_package_with_no_check_proposes_no_component() {
        let proposals = of(&[
            file("scripts/pyproject.toml", ""),
            file("scripts/uv.lock", ""),
            file("web/package.json", SCRIPTS),
            file("web/pnpm-lock.yaml", ""),
        ]);
        assert!(
            proposals.iter().all(|p| p.component != "scripts"),
            "{proposals:#?}"
        );
    }

    /// **`workspaces:` is proposed from the file `verify` will look for.** A
    /// directory that merely resolves its own dependencies is a candidate and
    /// not a workspace: declaring one that has no `armada.yml` writes a config
    /// that cannot verify.
    #[test]
    fn a_nested_config_is_the_only_proof_of_a_workspace() {
        let proposals = of(&[
            file(
                "package.json",
                r#"{"name":"root","scripts":{"test":"vitest"}}"#,
            ),
            file("pnpm-lock.yaml", ""),
            file("apps/site/armada.yml", "manifest:\n  version: 1\n"),
            file("apps/site/package.json", SCRIPTS),
            file("apps/site/pnpm-lock.yaml", ""),
        ]);
        assert_eq!(proposals[0].kind, Kind::Workspace);
        assert_eq!(proposals[0].writes, "apps/site");
        // And nothing below it: what `apps/site` runs is its own file's to say,
        // and a `root:` reaching into a declared workspace is illegal anyway.
        assert!(
            proposals.iter().all(|p| p.component != "site"),
            "{proposals:#?}"
        );
    }

    /// The root component is named by its manifest, with the registry scope
    /// dropped — and by [`ROOT`] when there is no legal name to be had.
    #[test]
    fn the_root_component_is_named_by_its_manifest() {
        let named = of(&[file("package.json", SCRIPTS), file("pnpm-lock.yaml", "")]);
        assert_eq!(named[0].at, "components.web", "the @acme/ scope survived");

        let unnamed = of(&[
            file("package.json", r#"{"scripts":{"test":"vitest"}}"#),
            file("pnpm-lock.yaml", ""),
        ]);
        assert_eq!(unnamed[0].at, "components.app");
    }

    /// A name the schema would reject is not spelled differently to make it fit.
    #[test]
    fn an_illegal_name_proposes_nothing() {
        assert!(legal("web-ui") && legal("core2"));
        assert!(!legal("Web") && !legal("web ui") && !legal("-web") && !legal(""));
    }

    /// **Rejecting a component takes its checks with it.** The alternative is a
    /// document that does not parse, which is worse than the proposal the reader
    /// was refusing.
    #[test]
    fn a_check_whose_component_was_rejected_is_not_written() {
        let all = of(&[
            file("web/package.json", SCRIPTS),
            file("web/pnpm-lock.yaml", ""),
        ]);
        let kept: Vec<Proposal> = all
            .iter()
            .filter(|p| p.kind != Kind::Component)
            .cloned()
            .collect();
        let yaml = write(&kept);
        assert!(!yaml.contains("checks:"), "{yaml}");
        assert!(!yaml.contains("components:"), "{yaml}");
    }

    /// The written document is the one `armada.yml` parser's, and it carries the
    /// provenance of every line it wrote.
    #[test]
    fn what_it_writes_parses_and_says_where_it_came_from() {
        let yaml = write(&of(&[
            file("web/package.json", SCRIPTS),
            file("web/pnpm-lock.yaml", ""),
            file("apps/site/armada.yml", "manifest:\n  version: 1\n"),
        ]));
        assert!(yaml.contains("workspaces: [apps/site]"), "{yaml}");
        assert!(yaml.contains("# web/package.json"), "{yaml}");
        let config = crate::config::parse(&yaml, "armada.yml").expect("it parses");
        let resolved =
            crate::config::resolve(config, &crate::config::Defaults::built_in(), "armada.yml")
                .expect("it resolves");
        let component = resolved
            .components
            .get("web")
            .expect("the component it proposed");
        assert_eq!(component.checks["test"].cmd, "pnpm run test");
    }

    /// A repository with nothing provable in it proposes nothing, and the
    /// document that would write is not a document.
    #[test]
    fn nothing_provable_proposes_nothing() {
        assert!(of(&[file("README.md", "# hi")]).is_empty());
    }
}
