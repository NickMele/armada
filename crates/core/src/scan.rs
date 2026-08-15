//! Layer 1 of the bootstrap sandwich: **evidence, and never a config**
//! (PLAN.md §5).
//!
//! > *"Do not write a stack-detection engine. Do not infer intent."*
//!
//! That is the whole design, and it is what makes this module safe to trust.
//! "These fourteen scripts exist in `package.json`" cannot be wrong; "your test
//! command is `pnpm test`" can. So every value below is copied out of a file
//! the repository already had, and nothing here decides which of four test
//! scripts is the one that counts — that is layer 2's, and layer 2 is an agent
//! or a person.
//!
//! **No truncation, anywhere.** All fourteen scripts are carried, not the first
//! five and a count: the author reading this evidence is the one who has to
//! find the one script that mattered, and evidence with a `…9 more` on it is
//! evidence somebody has to go and fetch separately.
//!
//! **Pure, like everything else in this crate.** The caller reads the files and
//! hands them over as [`SourceFile`]s; [`scan`] parses. Which files are worth
//! reading is [`CANDIDATES`], and how far down to look is [`MAX_DEPTH`] and
//! [`SKIP_DIRS`] — stated here because they are policy, and applied by the
//! shell because only the shell opens a directory.
//!
//! # Where a scan looks, and why it is not just the root
//!
//! **The first version read the root and nothing else, and on a monorepo that
//! made it blind to the thing it most needed to see.** Against a real polyglot
//! repository it reported `absent lockfile` and `absent scripts` while
//! `web/pnpm-lock.yaml`, `backend/uv.lock` and `backend/pyproject.toml` sat one
//! level down; the two things it did find — a compose file and a workflow —
//! were the only two at the root. The parsing was right and the search was
//! wrong.
//!
//! So it descends, and the bound is the whole design:
//!
//! | Bound | Why |
//! |---|---|
//! | `.gitignore`, via git | a build output is not a package, and git already knows which is which |
//! | [`SKIP_DIRS`] | the same answer for a repository that is not a git checkout, and for one that commits `node_modules` |
//! | [`MAX_DEPTH`] | a lockfile eight levels down is a vendored copy, not a package |
//!
//! # Location is evidence, and merging it away loses the report
//!
//! `backend/` holding `uv.lock` and `pyproject.toml` while `web/` holds
//! `pnpm-lock.yaml` is the single most important fact about such a repository —
//! it is three products in one checkout — and a flat "found: lockfile" list
//! destroys it. So every path here is workspace-relative and complete, and
//! [`Evidence::packages`] states the grouping outright: which directories carry
//! a manifest of their own, and which of those resolve their own dependencies.
//!
//! **That last distinction is reported, never decided.** A directory with its
//! own lockfile resolves its dependencies for itself, which is what tells a
//! separate product from a member of a workspace — and separate products
//! sharing one repository is exactly what `workspaces:` is for (PLAN.md §4.6).
//! Whether to declare one is the author's call. Layer 1 says which qualify.

use crate::envelope::ResultRow;
use crate::error::Status;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// A file the caller read, and the workspace-relative path it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Workspace-relative, with `/` separators.
    pub path: String,
    /// The contents, verbatim.
    pub text: String,
}

impl SourceFile {
    /// A file, from its path and contents.
    pub fn new(path: impl Into<String>, text: impl Into<String>) -> SourceFile {
        SourceFile {
            path: path.into(),
            text: text.into(),
        }
    }
}

/// The file **names** a scan reads, wherever in the tree they turn up.
///
/// **Names rather than paths, because location is what the first version got
/// wrong.** A `pyproject.toml` is a `pyproject.toml` whether it sits at the
/// root or under `backend/`, and a list of root-relative paths could only ever
/// find the first kind.
pub const CANDIDATES: &[&str] = &[
    "armada.yml",
    "package.json",
    "pnpm-workspace.yaml",
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "pyproject.toml",
    "setup.py",
    "uv.lock",
    "poetry.lock",
    "Pipfile.lock",
    "Cargo.toml",
    "Cargo.lock",
    "go.mod",
    "go.sum",
    "go.work",
    "Gemfile",
    "Gemfile.lock",
    "composer.json",
    "composer.lock",
    "mix.exs",
    "mix.lock",
    "Makefile",
    "makefile",
    "GNUmakefile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
];

/// The directory whose every `.yml`/`.yaml` file is a CI workflow.
pub const WORKFLOW_DIR: &str = ".github/workflows";

/// How many directories below the root a scan will descend.
///
/// **Three, and the number is an argument rather than a taste.** `web/` is one,
/// `apps/web/` and `crates/core/` are two, and a repository that nests a
/// package three deep is unusual but real. At four the things a scan finds stop
/// being packages: a vendored copy, a fixture, a test resource, an example app
/// inside a library. Under-reporting a genuinely four-deep package costs one
/// line of evidence; over-reporting turns the report into a list of other
/// people's lockfiles, which is the state this whole layer exists to avoid.
pub const MAX_DEPTH: usize = 3;

/// Directories a scan never descends into.
///
/// **The second line of defence, not the first.** Git already answers
/// "is this ignored" exactly, including a repository's own local rules, and
/// that is what a scan uses when it has a checkout to ask. This list is what
/// answers the same question for a directory that is not one — and for the
/// repository that commits its `vendor/` tree, where git would say yes and the
/// evidence would still be somebody else's.
pub const SKIP_DIRS: &[&str] = &[
    ".git",
    ".armada",
    ".cache",
    ".gradle",
    ".mypy_cache",
    ".next",
    ".nuxt",
    ".pytest_cache",
    ".ruff_cache",
    ".svelte-kit",
    ".terraform",
    ".tox",
    ".venv",
    "_build",
    "bower_components",
    "build",
    "deps",
    "dist",
    "node_modules",
    "site-packages",
    "target",
    "vendor",
    "venv",
    "__pycache__",
];

/// The manifests that make a directory a package of its own.
///
/// A lockfile is **not** one: a lockfile says how dependencies resolved, and a
/// manifest says there is something here to resolve them for. The pair is what
/// [`Package`] reports, and keeping them apart is what lets it tell a workspace
/// member from a separate product.
const PACKAGE_MANIFESTS: &[&str] = &[
    "package.json",
    "pyproject.toml",
    "setup.py",
    "Cargo.toml",
    "go.mod",
    "Gemfile",
    "composer.json",
    "mix.exs",
];

/// Every lockfile Armada recognises, and the package manager it implies.
///
/// **A lockfile is a fact and the manager is the file's own name**, which is
/// why this is a table rather than an inference: `pnpm-lock.yaml` means pnpm
/// because pnpm writes it, not because Armada guessed from the shape of the
/// repository.
const LOCKFILES: &[(&str, &str)] = &[
    ("pnpm-lock.yaml", "pnpm"),
    ("package-lock.json", "npm"),
    ("yarn.lock", "yarn"),
    ("bun.lock", "bun"),
    ("bun.lockb", "bun"),
    ("uv.lock", "uv"),
    ("poetry.lock", "poetry"),
    ("Pipfile.lock", "pipenv"),
    ("Cargo.lock", "cargo"),
    ("go.sum", "go"),
    ("Gemfile.lock", "bundler"),
    ("composer.lock", "composer"),
    ("mix.lock", "mix"),
];

/// Everything one scan found, in the order a reader meets it.
///
/// Serialized whole into `data.evidence`, so the agent authoring the config
/// reads exactly what the table and the sections below it were drawn from.
///
/// **An empty list is emitted rather than omitted, which is the opposite of
/// what the rest of the envelope does** — and for the same underlying reason.
/// Elsewhere an absent key means "this verb had nothing of that kind to say";
/// here the *kinds are the report*, and `"makefiles": []` is the payload's way
/// of saying Armada looked and there was no Makefile. That is exactly the
/// distinction the human render draws with its `absent` row, and a consumer
/// that could not tell "looked, found none" from "did not look" would have to
/// re-scan the directory to find out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Evidence {
    /// Whether this directory already has an `armada.yml`.
    ///
    /// Reported rather than refused: `scan` is the one verb that runs in a repo
    /// with no config (PLAN.md §2.1), and a repo that has one is still allowed
    /// to ask what is here.
    pub config_present: bool,
    /// Lockfiles, in [`CANDIDATES`] order.
    pub lockfiles: Vec<Lockfile>,
    /// `package.json` scripts, verbatim and in declaration order.
    pub scripts: Vec<ScriptSource>,
    /// `pyproject.toml` tool sections.
    pub pyproject: Vec<PyProject>,
    /// Makefile targets.
    pub makefiles: Vec<Makefile>,
    /// Compose files and the services they declare.
    pub compose: Vec<ComposeFile>,
    /// CI workflows and the commands their steps run.
    pub ci: Vec<CiWorkflow>,
    /// Monorepo layout: the globs a workspace manifest declares.
    pub workspace_globs: Vec<WorkspaceGlobs>,
    /// Monorepo layout: every directory with a package manifest of its own.
    ///
    /// **The grouping the flat lists above cannot carry.** `backend/` holding
    /// `uv.lock` and `pyproject.toml` while `web/` holds `pnpm-lock.yaml` is
    /// the single most important fact about a polyglot repository, and reading
    /// it off seven separate lists is work the author should not have to do.
    pub packages: Vec<Package>,
}

/// What `config scan` does once it has printed the evidence.
///
/// It has produced evidence, and evidence is not a config, so the last thing it
/// does is hand over. **`ARCHITECTURE.md` §1.9 permits that**: the rule governs
/// what Manifest may *accept* — a Job id, a model name, a transcript — not
/// whether it may hand a repository to an agent, which is the same shape as
/// `fleet board` handing you `claude --resume`.
///
/// # The three-audiences rule, applied to input
///
/// PLAN.md §3.1.1 splits three audiences by what gets *written* — colour,
/// progress, the envelope. The same split decides what may be **read**, and the
/// section does not say so because until now nothing here read anything:
///
/// | Audience | Handover |
/// |---|---|
/// | a person at a terminal | [`Handover::Ask`] — draw the choice, read the answer |
/// | an agent reading stdout | [`Handover::Tell`] — print the command it would have run |
/// | a parser (`--json`) | [`Handover::Silent`] — the envelope and nothing else |
///
/// **An agent running `config scan` inside a Job must never block on stdin that
/// will never arrive.** That is the failure mode "always interactive" causes,
/// and it is the whole reason the terminal decides this rather than a flag: a
/// flag can be forgotten, and a Job that hangs on a prompt nobody can answer
/// burns its ceiling and reports nothing.
///
/// **Both streams have to be a terminal, not either.** stdin decides whether an
/// answer can arrive and stdout decides whether the question was seen, and a
/// menu written to a pipe while stdin happens to be a tty is a prompt the
/// reader never read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Handover {
    /// Draw the choice and read an answer.
    Ask,
    /// Print the command that would have been run, and read nothing.
    Tell(TellWhy),
    /// Offer nothing: the caller asked for the envelope alone.
    Silent,
}

/// Why `config scan` printed a command instead of asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TellWhy {
    /// Nobody is at a terminal to answer.
    NotATerminal,
    /// There is no onboarding skill to hand over to.
    NoSkill,
}

impl Default for Handover {
    /// The safe answer, and the one a caller that never asked gets: say what
    /// would have happened and read nothing.
    fn default() -> Self {
        Handover::Tell(TellWhy::NotATerminal)
    }
}

/// Which handover this invocation gets.
///
/// `skill` is whether there is an onboarding skill to hand over to. **A missing
/// one is reported rather than exec'd**: offering to launch something that is
/// not there produces a failure at the moment the reader was expecting help.
pub fn handover(json: bool, stdin_is_tty: bool, stdout_is_tty: bool, skill: bool) -> Handover {
    if json {
        return Handover::Silent;
    }
    if !skill {
        return Handover::Tell(TellWhy::NoSkill);
    }
    if stdin_is_tty && stdout_is_tty {
        Handover::Ask
    } else {
        Handover::Tell(TellWhy::NotATerminal)
    }
}

/// A directory that is a package in its own right.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Package {
    /// Workspace-relative directory. Empty for the root.
    pub dir: String,
    /// The manifests that make it one, by name — `pyproject.toml`,
    /// `package.json`.
    pub manifests: Vec<String>,
    /// The lockfiles it carries, by name.
    pub lockfiles: Vec<String>,
    /// Whether a workspace glob declared elsewhere in the repository covers
    /// this directory — which is what makes it a *member* rather than a
    /// neighbour.
    pub member: bool,
    /// Whether this directory looks like a **separate product sharing the
    /// repository**, which is what `workspaces:` is for (PLAN.md §4.6).
    ///
    /// **Reported, never decided.** The fact underneath it is that the
    /// directory resolves its own dependencies: it has a manifest *and* a
    /// lockfile of its own, and no workspace glob claims it. A member of a pnpm
    /// workspace has the manifest and no lockfile, because the root resolved
    /// for it — so the two are told apart by what is on disk rather than by a
    /// guess. Whether to declare one is the author's call; layer 1 says which
    /// qualify.
    pub independent: bool,
}

/// A lockfile, and the package manager that writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lockfile {
    /// The file.
    pub file: String,
    /// `pnpm`, or `pnpm@9` when `package.json` pins one.
    pub manager: String,
}

/// One file's scripts, verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScriptSource {
    /// The file they came from.
    pub file: String,
    /// **Declaration order, never sorted.** A `package.json` is written in the
    /// order its author thought about the work, and re-alphabetising it throws
    /// that away for no gain.
    pub scripts: Vec<Script>,
}

/// One script: a name and the command it runs, uninterpreted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Script {
    /// The key.
    pub name: String,
    /// The value, exactly as written.
    pub cmd: String,
}

/// A `pyproject.toml` and the tool sections it configures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PyProject {
    /// The file.
    pub file: String,
    /// `[tool.ruff]` is reported as `ruff`.
    pub tools: Vec<String>,
}

/// A makefile and its targets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Makefile {
    /// The file.
    pub file: String,
    /// Target names, in file order.
    pub targets: Vec<String>,
}

/// A compose file and its services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComposeFile {
    /// The file.
    pub file: String,
    /// Services, in declaration order.
    pub services: Vec<ComposeService>,
}

/// One compose service and the ports it declares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComposeService {
    /// The service key.
    pub name: String,
    /// The published ports, as strings, because a compose `ports:` entry is not
    /// always a number and Armada is reporting what is written.
    pub ports: Vec<String>,
}

/// A CI workflow: how many steps, and which of them run a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CiWorkflow {
    /// The file.
    pub file: String,
    /// Every step in every job, including the `uses:` ones.
    pub steps: usize,
    /// The `run:` lines, first line each, in file order. **The best existing
    /// evidence of what a repository actually runs** (PLAN.md §5).
    pub runs: Vec<String>,
}

/// A monorepo's package globs, and the manifest that declares them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceGlobs {
    /// The file.
    pub file: String,
    /// The globs, verbatim.
    pub globs: Vec<String>,
}

/// Read the evidence out of a set of files.
///
/// Total: a file that does not parse contributes nothing rather than failing
/// the scan. `scan` exits 0 whenever the directory was readable
/// (`docs/commands/manifest/config.md`), and a repository with a half-written
/// `docker-compose.yml` still has thirteen other pieces of evidence worth
/// printing.
pub fn scan(files: &[SourceFile]) -> Evidence {
    let mut evidence = Evidence::default();

    // **Sorted by directory and then by name, so the root's evidence comes
    // first and everything below it is grouped.** A directory listing has no
    // order a golden snapshot can hold still, and a reader scanning for
    // `backend/` wants its three files together rather than spread by
    // alphabet across the whole report.
    let mut files: Vec<&SourceFile> = files.iter().collect();
    files.sort_by(|a, b| {
        (directory(&a.path), base(&a.path)).cmp(&(directory(&b.path), base(&b.path)))
    });

    evidence.config_present = files.iter().any(|f| f.path == "armada.yml");

    for file in &files {
        let name = base(&file.path);
        let dir = directory(&file.path);

        if let Some((_, manager)) = LOCKFILES.iter().find(|(n, _)| *n == name) {
            // **The pin is read from the `package.json` beside it**, not from
            // the root's. In a polyglot repo the root may have no manifest at
            // all, and `web/package.json` is the only thing that knows which
            // pnpm `web/pnpm-lock.yaml` was written by.
            let pinned = beside(&files, dir, "package.json").and_then(|f| package_manager(&f.text));
            let manager = match pinned {
                Some(pinned) if pinned.split('@').next() == Some(manager) => pinned,
                _ => (*manager).to_string(),
            };
            evidence.lockfiles.push(Lockfile {
                file: file.path.clone(),
                manager,
            });
        }

        match name {
            "package.json" => {
                let scripts = json_scripts(&file.text);
                if !scripts.is_empty() {
                    evidence.scripts.push(ScriptSource {
                        file: file.path.clone(),
                        scripts,
                    });
                }
                let globs = json_workspaces(Some(file));
                if !globs.is_empty() {
                    evidence.workspace_globs.push(WorkspaceGlobs {
                        file: file.path.clone(),
                        globs,
                    });
                }
            }
            "pyproject.toml" => evidence.pyproject.push(PyProject {
                file: file.path.clone(),
                tools: toml_tool_sections(&file.text),
            }),
            "Makefile" | "makefile" | "GNUmakefile" => evidence.makefiles.push(Makefile {
                file: file.path.clone(),
                targets: make_targets(&file.text),
            }),
            "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
                evidence.compose.push(ComposeFile {
                    file: file.path.clone(),
                    services: compose_services(&file.text),
                })
            }
            "pnpm-workspace.yaml" => {
                let globs = yaml_string_list(Some(file), "packages");
                if !globs.is_empty() {
                    evidence.workspace_globs.push(WorkspaceGlobs {
                        file: file.path.clone(),
                        globs,
                    });
                }
            }
            "Cargo.toml" => {
                let globs = toml_workspace_members(Some(file));
                if !globs.is_empty() {
                    evidence.workspace_globs.push(WorkspaceGlobs {
                        file: file.path.clone(),
                        globs,
                    });
                }
            }
            _ => {}
        }

        if file.path.starts_with(WORKFLOW_DIR) {
            let (steps, runs) = workflow_steps(&file.text);
            evidence.ci.push(CiWorkflow {
                file: file.path.clone(),
                steps,
                runs,
            });
        }
    }

    evidence.packages = packages(&files, &evidence.workspace_globs);
    evidence
}

/// Every directory that carries a package manifest, and what makes it one.
fn packages(files: &[&SourceFile], globs: &[WorkspaceGlobs]) -> Vec<Package> {
    let mut dirs: BTreeMap<&str, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for file in files {
        let name = base(&file.path);
        let entry = if PACKAGE_MANIFESTS.contains(&name) {
            0
        } else if LOCKFILES.iter().any(|(n, _)| *n == name) {
            1
        } else {
            continue;
        };
        let slot = dirs.entry(directory(&file.path)).or_default();
        if entry == 0 {
            slot.0.push(name.to_string());
        } else {
            slot.1.push(name.to_string());
        }
    }

    dirs.into_iter()
        // A directory with a lockfile and no manifest is not a package — it is
        // a lockfile somebody left there. Reported in `lockfiles`, and not
        // dignified with a package of its own.
        .filter(|(_, (manifests, _))| !manifests.is_empty())
        .map(|(dir, (manifests, lockfiles))| {
            let member = globs.iter().any(|declared| {
                declared
                    .globs
                    .iter()
                    .any(|glob| crate::glob::matches(&rooted(&declared.file, glob), dir))
            });
            Package {
                dir: dir.to_string(),
                // **A directory that locks its own dependencies is a separate
                // product**; one whose manifest is claimed by somebody else's
                // glob is a member of theirs. The root is neither — it is the
                // repository.
                independent: !dir.is_empty() && !lockfiles.is_empty() && !member,
                manifests,
                lockfiles,
                member,
            }
        })
        .collect()
}

/// A workspace glob, made relative to the repository rather than to the
/// manifest that declared it.
///
/// `packages/*` in a root `pnpm-workspace.yaml` means `packages/*`; the same
/// line in `web/pnpm-workspace.yaml` means `web/packages/*`.
fn rooted(declared_in: &str, glob: &str) -> String {
    match directory(declared_in) {
        "" => glob.to_string(),
        dir => format!("{dir}/{glob}"),
    }
}

/// The directory part of a workspace-relative path; `""` for the root.
pub fn directory(path: &str) -> &str {
    match path.rfind('/') {
        Some(at) => &path[..at],
        None => "",
    }
}

/// The file name part of a workspace-relative path.
fn base(path: &str) -> &str {
    match path.rfind('/') {
        Some(at) => &path[at + 1..],
        None => path,
    }
}

/// A file of this name in this directory, if the scan was handed one.
fn beside<'a>(files: &[&'a SourceFile], dir: &str, name: &str) -> Option<&'a SourceFile> {
    files
        .iter()
        .copied()
        .find(|f| directory(&f.path) == dir && base(&f.path) == name)
}

/// The kinds of evidence a scan reports, in the order a reader meets them.
///
/// **A fixed list, and the human render draws every one of them** — including
/// the ones that turned nothing up, because `absent  makefile  —` is a
/// statement that Armada looked, and a missing row is a statement about
/// nothing. The order is the agreed layout's
/// (`docs/reference-output/command-output.html`) and is not sorted by outcome:
/// a table whose rows move when a repository gains a `Makefile` is a table
/// nobody can diff.
///
/// **`package scripts` and not `scripts`**, because the first spelling was
/// read as a path. A repository with a `scripts/` directory that is a Python
/// package had its `absent scripts —` row understood as a statement about that
/// directory, when the row is about the `scripts` block of a `package.json` and
/// nothing else. The space is deliberate: a kind with a space in it cannot be
/// mistaken for a directory, whatever a reader is expecting to see.
pub const KINDS: [&str; 6] = [
    "lockfile",
    "package scripts",
    "compose",
    "ci",
    "pyproject",
    "makefile",
];

/// One row per finding, in [`KINDS`] order.
///
/// **Findings only.** A kind with nothing under it contributes no row, because
/// an absence is not something the scan found — `docs/commands/manifest/config.md`
/// asks for "one result per finding with its source file", and that is what
/// `path` carries.
pub fn findings(evidence: &Evidence) -> Vec<ResultRow> {
    let mut out = Vec::new();

    for lockfile in &evidence.lockfiles {
        out.push(finding(
            "lockfile",
            &lockfile.file,
            format!("{}, {}", lockfile.file, lockfile.manager),
        ));
    }
    for source in &evidence.scripts {
        out.push(finding(
            "package scripts",
            &source.file,
            format!("{} in {}", source.scripts.len(), source.file),
        ));
    }
    for compose in &evidence.compose {
        out.push(finding(
            "compose",
            &compose.file,
            format!(
                "{}, {}",
                compose.file,
                plural(compose.services.len(), "service")
            ),
        ));
    }
    for workflow in &evidence.ci {
        out.push(finding(
            "ci",
            &workflow.file,
            format!("{}, {}", workflow.file, plural(workflow.steps, "step")),
        ));
    }
    for pyproject in &evidence.pyproject {
        out.push(finding(
            "pyproject",
            &pyproject.file,
            format!(
                "{}, {}",
                pyproject.file,
                plural(pyproject.tools.len(), "tool section")
            ),
        ));
    }
    for makefile in &evidence.makefiles {
        out.push(finding(
            "makefile",
            &makefile.file,
            format!(
                "{}, {}",
                makefile.file,
                plural(makefile.targets.len(), "target")
            ),
        ));
    }

    out
}

fn finding(kind: &str, file: &str, detail: String) -> ResultRow {
    // **`OK` and never a verdict.** A scan judges nothing: the row says this
    // evidence was read, not that the repository is configured correctly.
    let mut row = ResultRow::new(kind, Status::Ok);
    row.path = Some(file.to_string());
    row.reason = Some(detail);
    row
}

/// `n` of something, pluralised by the only rule English is reliable about.
///
/// A second copy of `render::format::count`, one altitude down, because the
/// detail string belongs to the envelope: `--json` carries it in
/// `results[].reason` and the human render reads it back rather than deriving
/// a second wording that could disagree.
fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

// -------------------------------------------------------------------- parsers
// Each is total and each reports only what its file already said.

/// `packageManager: "pnpm@9.1.0"` → `pnpm@9`.
///
/// The major alone, because that is the number a repository means when it says
/// which package manager it uses — and a full semver in an evidence table is
/// three digits of noise in the column a reader is scanning.
fn package_manager(text: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        #[serde(rename = "packageManager")]
        package_manager: Option<String>,
    }
    let manifest: Manifest = serde_json::from_str(text).ok()?;
    let pinned = manifest.package_manager?;
    let (name, version) = pinned.split_once('@')?;
    let major = version.split('.').next().unwrap_or(version);
    Some(format!("{name}@{major}"))
}

/// `package.json`'s `scripts`, in declaration order.
///
/// **Streamed through `MapAccess` rather than through a `serde_json::Value`.**
/// Measured, and the same trap the envelope records: a `Value`'s map is a
/// `BTreeMap`, so anything routed through one comes out alphabetised. The order
/// a `package.json` is written in is part of the evidence, so it may not be
/// thrown away on the way through the parser (`docs/traps.md`).
fn json_scripts(text: &str) -> Vec<Script> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        #[serde(default, deserialize_with = "ordered_pairs")]
        scripts: Vec<Script>,
    }
    serde_json::from_str::<Manifest>(text)
        .map(|manifest| manifest.scripts)
        .unwrap_or_default()
}

fn ordered_pairs<'de, D>(deserializer: D) -> Result<Vec<Script>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct InOrder;
    impl<'de> serde::de::Visitor<'de> for InOrder {
        type Value = Vec<Script>;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map of script names to commands")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Vec<Script>, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some((name, value)) = map.next_entry::<String, serde_json::Value>()? {
                // A non-string value is not a script. Reported by omission
                // rather than by failing the file, on the same rule every
                // parser here follows.
                if let serde_json::Value::String(cmd) = value {
                    out.push(Script { name, cmd });
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(InOrder)
}

/// `workspaces:` in a `package.json`, in either of the two spellings npm
/// accepts — a bare list, or `{ "packages": [...] }`.
fn json_workspaces(file: Option<&SourceFile>) -> Vec<String> {
    let Some(file) = file else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&file.text) else {
        return Vec::new();
    };
    let workspaces = value.get("workspaces");
    let list = match workspaces {
        Some(serde_json::Value::Array(items)) => items.clone(),
        Some(serde_json::Value::Object(map)) => match map.get("packages") {
            Some(serde_json::Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    list.into_iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect()
}

/// A top-level `key:` whose value is a list of strings, read out of YAML.
fn yaml_string_list(file: Option<&SourceFile>, key: &str) -> Vec<String> {
    let Some(file) = file else {
        return Vec::new();
    };
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&file.text) else {
        return Vec::new();
    };
    value
        .get(key)
        .and_then(serde_yaml_ng::Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// `[workspace] members = [...]` out of a `Cargo.toml`.
///
/// **Read line by line rather than with a TOML parser.** The core takes no TOML
/// dependency, and the two things a scan wants out of a `Cargo.toml` — the
/// members list and `pyproject`'s tool section headers — are both single-line
/// shapes. A repository whose manifest writes `members` across several lines
/// contributes no globs, which is a scan under-reporting rather than
/// mis-reporting, and under-reporting is the failure this whole layer prefers.
fn toml_workspace_members(file: Option<&SourceFile>) -> Vec<String> {
    let Some(file) = file else {
        return Vec::new();
    };
    let mut section = "";
    for line in file.text.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = header.trim();
            continue;
        }
        if section != "workspace" {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "members" {
            continue;
        }
        return value
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|item| item.trim().trim_matches(['"', '\'']).to_string())
            .filter(|item| !item.is_empty())
            .collect();
    }
    Vec::new()
}

/// The `[tool.X]` sections a `pyproject.toml` carries, deduplicated and in file
/// order.
fn toml_tool_sections(text: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
            continue;
        };
        let header = header.trim().trim_start_matches('[').trim_end_matches(']');
        let Some(rest) = header.strip_prefix("tool.") else {
            continue;
        };
        // `[tool.ruff.lint]` is still the `ruff` tool.
        let name = rest.split('.').next().unwrap_or(rest).to_string();
        if seen.insert(name.clone()) {
            out.push(name);
        }
    }
    out
}

/// A makefile's targets, in file order.
///
/// A target is a line starting in column zero with a name, a colon, and no `=`
/// before it — which excludes `VAR := value` and every recipe line, both of
/// which are indented or assign.
fn make_targets(text: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t', '#']) || line.is_empty() {
            continue;
        }
        let Some((head, _)) = line.split_once(':') else {
            continue;
        };
        // `VAR := x` and `VAR ::= x` split before the `=`, so the assignment
        // forms are excluded by looking at what follows rather than at `head`.
        if line[head.len()..].starts_with(":=") || line[head.len()..].starts_with("::=") {
            continue;
        }
        let name = head.trim();
        if name.is_empty()
            || head.contains('=')
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-/$(){}%".contains(c))
        {
            continue;
        }
        for target in name.split_whitespace() {
            if seen.insert(target.to_string()) {
                out.push(target.to_string());
            }
        }
    }
    out
}

/// A compose file's services and their published ports.
fn compose_services(text: &str) -> Vec<ComposeService> {
    let Ok(document) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return Vec::new();
    };
    let Some(services) = document.get("services").and_then(|s| s.as_mapping()) else {
        return Vec::new();
    };
    services
        .iter()
        .filter_map(|(name, body)| {
            let name = name.as_str()?.to_string();
            let ports = body
                .get("ports")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .map(|entries| entries.iter().filter_map(published_port).collect())
                .unwrap_or_default();
            Some(ComposeService { name, ports })
        })
        .collect()
}

/// The host side of one compose `ports:` entry, as the file wrote it.
///
/// **The grammar is [`crate::compose::parse_port`]'s and not this module's.**
/// There were two parsers for one small grammar, and each was wrong in a way
/// the other was not: this one split on every `:` and cut
/// `${POSTGRES_PORT:-5432}` in half at the colon inside the variable's default,
/// reporting a host port of `-5432}`. Scan reports what it finds and refuses
/// nothing; the transform rewrites or refuses. Same parse, different consumers.
///
/// A bare `"6379"` reports `6379`, which is what the file says. Compose will
/// publish it on an ephemeral port and `armada manifest up` will rewrite it
/// into the claimed block — but neither of those has happened yet, and a scan
/// reports the repository as it is.
fn published_port(entry: &serde_yaml_ng::Value) -> Option<String> {
    let parsed = crate::compose::parse_port(entry)?;
    Some(parsed.published.unwrap_or(parsed.target).text())
}

/// How many steps a workflow declares, and what each `run:` step runs.
fn workflow_steps(text: &str) -> (usize, Vec<String>) {
    let Ok(document) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return (0, Vec::new());
    };
    let Some(jobs) = document.get("jobs").and_then(|j| j.as_mapping()) else {
        return (0, Vec::new());
    };
    let mut steps = 0;
    let mut runs = Vec::new();
    for (_, job) in jobs {
        let Some(list) = job.get("steps").and_then(serde_yaml_ng::Value::as_sequence) else {
            continue;
        };
        steps += list.len();
        for step in list {
            let Some(run) = step.get("run").and_then(serde_yaml_ng::Value::as_str) else {
                continue;
            };
            // A `run:` block may be a whole shell script. Its first non-empty
            // line is the evidence; the rest is the repository's business.
            if let Some(first) = run.lines().map(str::trim).find(|l| !l.is_empty()) {
                runs.push(first.to_string());
            }
        }
    }
    (steps, runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(entries: &[(&str, &str)]) -> Vec<SourceFile> {
        entries
            .iter()
            .map(|(path, text)| SourceFile::new(*path, *text))
            .collect()
    }

    /// **The rule the whole module exists for.** A `package.json` is written in
    /// the order its author thought about the work, and a scan that
    /// alphabetises it has already lost part of the evidence.
    #[test]
    fn scripts_keep_the_order_the_file_wrote_them_in() {
        let evidence = scan(&files(&[(
            "package.json",
            r#"{"scripts":{"dev":"next dev","build":"next build","test":"vitest run"}}"#,
        )]));
        let scripts = &evidence.scripts[0].scripts;
        assert_eq!(
            scripts.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["dev", "build", "test"],
            "sorted, which loses the author's order"
        );
        assert_eq!(scripts[0].cmd, "next dev");
    }

    /// **No truncation.** Fourteen scripts are fourteen scripts; the author
    /// reading this is the one who has to find the one that mattered.
    #[test]
    fn every_script_is_carried_however_many_there_are() {
        let entries: Vec<String> = (0..40).map(|n| format!("\"s{n}\":\"cmd {n}\"")).collect();
        let text = format!("{{\"scripts\":{{{}}}}}", entries.join(","));
        let evidence = scan(&files(&[("package.json", &text)]));
        assert_eq!(evidence.scripts[0].scripts.len(), 40);
    }

    #[test]
    fn a_lockfile_names_its_manager_and_the_pin_when_the_manifest_states_one() {
        let evidence = scan(&files(&[
            ("pnpm-lock.yaml", ""),
            ("package.json", r#"{"packageManager":"pnpm@9.1.0"}"#),
        ]));
        assert_eq!(evidence.lockfiles[0].file, "pnpm-lock.yaml");
        assert_eq!(evidence.lockfiles[0].manager, "pnpm@9");

        // No pin: the manager is the lockfile's own name and nothing more.
        let evidence = scan(&files(&[("Cargo.lock", "")]));
        assert_eq!(evidence.lockfiles[0].manager, "cargo");
    }

    /// A pin for a different manager is not evidence about this lockfile.
    #[test]
    fn a_pin_naming_another_manager_does_not_relabel_this_lockfile() {
        let evidence = scan(&files(&[
            ("yarn.lock", ""),
            ("package.json", r#"{"packageManager":"pnpm@9.1.0"}"#),
        ]));
        assert_eq!(evidence.lockfiles[0].manager, "yarn");
    }

    #[test]
    fn compose_services_carry_their_declared_ports_in_both_syntaxes() {
        let evidence = scan(&files(&[(
            "docker-compose.yml",
            "services:\n  postgres:\n    ports: [\"5432:5432\"]\n  \
             redis:\n    ports: [6379]\n  \
             mailhog:\n    ports:\n      - \"1025:1025\"\n      - \"8025:8025\"\n  \
             worker: {}\n",
        )]));
        let services = &evidence.compose[0].services;
        assert_eq!(services.len(), 4, "a service with no ports still counts");
        assert_eq!(services[0].name, "postgres");
        assert_eq!(services[0].ports, ["5432"]);
        assert_eq!(services[1].ports, ["6379"]);
        assert_eq!(services[2].ports, ["1025", "8025"]);
        assert!(services[3].ports.is_empty());
    }

    /// **The reported bug, on the verb that reported it.** A compose file with
    /// `ports: ["${POSTGRES_PORT:-5432}:5432"]` rendered as `db  -5432}`: the
    /// scanner split on every `:` and cut inside the variable's default, then
    /// kept the tail. `${VAR:-default}` is how a compose file supports both a
    /// fixed port and a per-worktree override, so it is not an exotic spelling.
    #[test]
    fn a_variable_port_is_reported_whole_rather_than_cut_at_its_default() {
        let evidence = scan(&files(&[(
            "docker-compose.yml",
            "services:\n  db:\n    ports: [\"${POSTGRES_PORT:-5432}:5432\"]\n  \
             api:\n    ports: [\"${API_PORT:-8000}:8000\"]\n",
        )]));
        let services = &evidence.compose[0].services;
        assert_eq!(services[0].name, "db");
        assert_eq!(services[0].ports, ["${POSTGRES_PORT:-5432}"]);
        assert_eq!(services[1].ports, ["${API_PORT:-8000}"]);
    }

    /// The grammar is `compose::parse_port`'s, so every spelling it reads
    /// reaches the report — including the ones the old scan-side parser had
    /// never met.
    #[test]
    fn the_evidence_carries_every_spelling_the_grammar_reads() {
        let evidence = scan(&files(&[(
            "docker-compose.yml",
            "services:\n  a:\n    ports: [\"6379-6380:6379-6380\"]\n  \
             b:\n    ports: [\"[::1]:6379:6379\"]\n  \
             c:\n    ports: [\"53:53/udp\"]\n  \
             d:\n    ports:\n      - target: 6379\n        published: \"5432\"\n",
        )]));
        let ports = |n: usize| evidence.compose[0].services[n].ports.clone();
        assert_eq!(ports(0), ["6379-6380"]);
        assert_eq!(ports(1), ["6379"], "the bind address is not the host port");
        assert_eq!(ports(2), ["53"], "the protocol is not part of the port");
        assert_eq!(ports(3), ["5432"]);
    }

    #[test]
    fn a_published_port_is_the_host_side_of_every_short_form() {
        for (written, expected) in [
            ("8080", "8080"),
            ("8080:80", "8080"),
            ("127.0.0.1:8080:80", "8080"),
            ("8080:80/tcp", "8080"),
        ] {
            let value = serde_yaml_ng::Value::String(written.to_string());
            assert_eq!(
                published_port(&value).as_deref(),
                Some(expected),
                "{written}"
            );
        }
    }

    /// Every step counts, but only the `run:` ones are commands. A workflow of
    /// six steps that runs four is exactly the shape the agreed layout draws.
    #[test]
    fn a_workflow_counts_every_step_and_lists_only_the_commands() {
        let evidence = scan(&files(&[(
            ".github/workflows/ci.yml",
            "jobs:\n  build:\n    steps:\n      \
             - uses: actions/checkout@v4\n      \
             - uses: pnpm/action-setup@v4\n      \
             - run: pnpm install --frozen-lockfile\n      \
             - run: pnpm lint\n      \
             - run: pnpm test\n      \
             - run: pnpm build\n",
        )]));
        assert_eq!(evidence.ci[0].steps, 6);
        assert_eq!(
            evidence.ci[0].runs,
            [
                "pnpm install --frozen-lockfile",
                "pnpm lint",
                "pnpm test",
                "pnpm build"
            ]
        );
    }

    /// A multi-line `run:` contributes its first line. The rest is a shell
    /// script, and a scan reports evidence rather than transcribing programs.
    #[test]
    fn a_block_run_step_contributes_its_first_line() {
        let evidence = scan(&files(&[(
            ".github/workflows/ci.yml",
            "jobs:\n  b:\n    steps:\n      - run: |\n          make build\n          make test\n",
        )]));
        assert_eq!(evidence.ci[0].runs, ["make build"]);
    }

    #[test]
    fn pyproject_reports_each_tool_once_however_many_subsections_it_has() {
        let evidence = scan(&files(&[(
            "pyproject.toml",
            "[project]\nname = \"x\"\n\n[tool.ruff]\n\n[tool.ruff.lint]\n\n[tool.mypy]\n",
        )]));
        assert_eq!(evidence.pyproject[0].tools, ["ruff", "mypy"]);
    }

    #[test]
    fn make_targets_exclude_assignments_and_recipe_lines() {
        let evidence = scan(&files(&[(
            "Makefile",
            "SHELL := /bin/bash\nCC = gcc\n\n.PHONY: test lint\n\ntest:\n\tpytest\n\nlint:\n\truff check\n",
        )]));
        assert_eq!(evidence.makefiles[0].targets, [".PHONY", "test", "lint"]);
    }

    #[test]
    fn workspace_globs_are_read_from_whichever_manifest_declares_them() {
        let evidence = scan(&files(&[
            (
                "pnpm-workspace.yaml",
                "packages:\n  - 'apps/*'\n  - 'packages/*'\n",
            ),
            ("package.json", r#"{"workspaces":["web/*"]}"#),
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/core\", \"crates/cli\"]\n",
            ),
        ]));
        assert_eq!(
            evidence
                .workspace_globs
                .iter()
                .map(|w| (w.file.as_str(), w.globs.clone()))
                .collect::<Vec<_>>(),
            // Sorted by directory and then by name, which is the order every
            // list here comes back in and the only one a golden snapshot can
            // hold still.
            [
                (
                    "Cargo.toml",
                    vec!["crates/core".to_string(), "crates/cli".to_string()]
                ),
                ("package.json", vec!["web/*".to_string()]),
                (
                    "pnpm-workspace.yaml",
                    vec!["apps/*".to_string(), "packages/*".to_string()]
                ),
            ]
        );
    }

    /// **Total, on purpose.** A half-written compose file is not a reason to
    /// refuse to print thirteen other facts, and `scan` exits 0 whenever the
    /// directory was readable.
    #[test]
    fn a_file_that_does_not_parse_contributes_nothing_rather_than_failing() {
        let evidence = scan(&files(&[
            ("docker-compose.yml", "services: [this is not a mapping\n"),
            ("package.json", "{ not json"),
            ("package-lock.json", ""),
        ]));
        assert!(evidence.compose[0].services.is_empty());
        assert!(evidence.scripts.is_empty());
        assert_eq!(evidence.lockfiles[0].manager, "npm");
    }

    #[test]
    fn an_existing_config_is_reported_rather_than_refused() {
        assert!(!scan(&files(&[("package.json", "{}")])).config_present);
        assert!(scan(&files(&[("armada.yml", "manifest:\n  version: 1\n")])).config_present);
    }

    /// An empty directory is an answer, not a failure: nothing found is what a
    /// repository with no manifests of any kind actually has.
    #[test]
    fn nothing_at_all_is_an_empty_evidence_report() {
        assert_eq!(scan(&[]), Evidence::default());
        assert!(findings(&Evidence::default()).is_empty());
    }

    /// Each finding names the file it came from, which is the question
    /// `--json`'s consumer asks second.
    #[test]
    fn every_finding_carries_its_source_file_and_a_one_line_detail() {
        let evidence = scan(&files(&[
            ("pnpm-lock.yaml", ""),
            (
                "package.json",
                r#"{"packageManager":"pnpm@9.1.0","scripts":{"a":"x"}}"#,
            ),
            ("docker-compose.yml", "services:\n  db: {}\n"),
            (
                ".github/workflows/ci.yml",
                "jobs:\n  b:\n    steps:\n      - run: make\n",
            ),
        ]));
        let rows = findings(&evidence);
        let seen: Vec<(&str, &str, &str)> = rows
            .iter()
            .map(|row| {
                (
                    row.id.as_str(),
                    row.path.as_deref().unwrap_or(""),
                    row.reason.as_deref().unwrap_or(""),
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                ("lockfile", "pnpm-lock.yaml", "pnpm-lock.yaml, pnpm@9"),
                ("package scripts", "package.json", "1 in package.json"),
                (
                    "compose",
                    "docker-compose.yml",
                    "docker-compose.yml, 1 service"
                ),
                (
                    "ci",
                    ".github/workflows/ci.yml",
                    ".github/workflows/ci.yml, 1 step"
                ),
            ]
        );
        assert!(rows.iter().all(|row| row.status == Status::Ok));
    }

    /// **A scan judges nothing**, so a kind that turned nothing up produces no
    /// row at all — the `absent` line a reader sees is the human render
    /// stating that it looked, not a result the scan reported.
    #[test]
    fn an_absent_kind_produces_no_finding() {
        let rows = findings(&scan(&files(&[(
            "package.json",
            r#"{"scripts":{"a":"x"}}"#,
        )])));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "package scripts");
    }

    // ---------------------------------------------------------- monorepos
    // The shape the first version was blind to, and the reason it was: the
    // scanner was built against the simple repositories that were in front of
    // it, and every one of them kept its evidence at the root.

    /// The polyglot shape, reduced to its bones: nothing at the root but a
    /// compose file and a workflow, and three products one level down.
    fn polyglot() -> Vec<SourceFile> {
        files(&[
            ("docker-compose.yml", "services:\n  db: {}\n"),
            (
                ".github/workflows/ci.yml",
                "jobs:\n  b:\n    steps:\n      - run: make check\n",
            ),
            (
                "web/package.json",
                r#"{"packageManager":"pnpm@9.1.0","scripts":{"dev":"next dev"}}"#,
            ),
            ("web/pnpm-lock.yaml", ""),
            (
                "backend/pyproject.toml",
                "[project]\n[tool.ruff]\n[tool.mypy]\n",
            ),
            ("backend/uv.lock", ""),
            ("scripts/pyproject.toml", "[project]\n[tool.ruff]\n"),
            ("scripts/uv.lock", ""),
        ])
    }

    /// **The bug, as a test.** Every one of these was reported `absent` while
    /// the file sat one level down.
    #[test]
    fn evidence_one_level_down_is_found_and_named_by_its_full_path() {
        let evidence = scan(&polyglot());

        assert_eq!(
            evidence
                .lockfiles
                .iter()
                .map(|l| (l.file.as_str(), l.manager.as_str()))
                .collect::<Vec<_>>(),
            [
                ("backend/uv.lock", "uv"),
                ("scripts/uv.lock", "uv"),
                ("web/pnpm-lock.yaml", "pnpm@9"),
            ],
            "a lockfile below the root was invisible"
        );
        assert_eq!(evidence.scripts[0].file, "web/package.json");
        assert_eq!(
            evidence
                .pyproject
                .iter()
                .map(|p| p.file.as_str())
                .collect::<Vec<_>>(),
            ["backend/pyproject.toml", "scripts/pyproject.toml"]
        );
        // And the two the first version *did* find, which were the only two at
        // the root — the parsing was always right.
        assert_eq!(evidence.compose[0].file, "docker-compose.yml");
        assert_eq!(evidence.ci[0].file, ".github/workflows/ci.yml");
    }

    /// **The pin is read from the manifest beside the lockfile.** In a polyglot
    /// repo the root may have no `package.json` at all, so a root-only lookup
    /// would report `pnpm` and lose the version the repository stated.
    #[test]
    fn a_nested_lockfile_takes_its_pin_from_the_manifest_in_its_own_directory() {
        let evidence = scan(&files(&[
            ("package.json", r#"{"packageManager":"pnpm@8.0.0"}"#),
            ("web/package.json", r#"{"packageManager":"pnpm@9.1.0"}"#),
            ("web/pnpm-lock.yaml", ""),
        ]));
        assert_eq!(
            evidence.lockfiles[0].manager, "pnpm@9",
            "the root's pin won"
        );
    }

    /// **Three products in one checkout, stated once.** Reading that off seven
    /// separate lists is work the author should not have to do, and it is the
    /// fact `workspaces:` exists for (PLAN.md §4.6).
    #[test]
    fn a_directory_with_its_own_manifest_and_lockfile_is_an_independent_package() {
        let evidence = scan(&polyglot());
        assert_eq!(
            evidence
                .packages
                .iter()
                .map(|p| (p.dir.as_str(), p.independent))
                .collect::<Vec<_>>(),
            [("backend", true), ("scripts", true), ("web", true)]
        );
        let backend = &evidence.packages[0];
        assert_eq!(backend.manifests, ["pyproject.toml"]);
        assert_eq!(backend.lockfiles, ["uv.lock"]);
        assert!(!backend.member);
    }

    /// **A workspace member is not an independent package**, and the fact that
    /// tells them apart is on disk: the member's dependencies were resolved by
    /// the root, so it has a manifest and no lockfile of its own.
    #[test]
    fn a_member_of_a_declared_workspace_is_not_a_candidate() {
        let evidence = scan(&files(&[
            ("package.json", r#"{"packageManager":"pnpm@9.0.0"}"#),
            ("pnpm-workspace.yaml", "packages:\n  - 'apps/*'\n"),
            ("pnpm-lock.yaml", ""),
            ("apps/web/package.json", r#"{"scripts":{"dev":"next dev"}}"#),
        ]));
        let web = evidence
            .packages
            .iter()
            .find(|p| p.dir == "apps/web")
            .expect("the member is still reported as a package");
        assert!(web.member, "a declared glob claims it");
        assert!(!web.independent, "it does not resolve its own dependencies");

        // The root is a package and never a candidate: it is the repository.
        let root = evidence.packages.iter().find(|p| p.dir.is_empty()).unwrap();
        assert!(!root.independent);
    }

    /// A glob is relative to the manifest that declared it, not to the
    /// repository — so a nested workspace claims its own neighbours and not the
    /// root's.
    #[test]
    fn a_nested_workspace_glob_claims_only_what_is_under_it() {
        let evidence = scan(&files(&[
            ("web/pnpm-workspace.yaml", "packages:\n  - 'apps/*'\n"),
            ("web/package.json", "{}"),
            ("web/apps/site/package.json", "{}"),
            ("apps/other/package.json", "{}"),
        ]));
        let claimed = |dir: &str| {
            evidence
                .packages
                .iter()
                .find(|p| p.dir == dir)
                .map(|p| p.member)
        };
        assert_eq!(claimed("web/apps/site"), Some(true));
        assert_eq!(
            claimed("apps/other"),
            Some(false),
            "`apps/*` in web/ must not reach the root's apps/"
        );
    }

    /// A lockfile with no manifest beside it is a lockfile somebody left there,
    /// not a package. It is still reported as evidence.
    #[test]
    fn a_lockfile_with_no_manifest_is_not_a_package() {
        let evidence = scan(&files(&[("tools/uv.lock", "")]));
        assert_eq!(evidence.lockfiles[0].file, "tools/uv.lock");
        assert!(evidence.packages.is_empty());
    }

    // ------------------------------------------------------------ handover

    /// **The failure mode that decided this.** An agent running `config scan`
    /// inside a Job reads stdout and has no stdin to type into; a menu there is
    /// a prompt that blocks until the Job's ceiling expires and reports
    /// nothing.
    #[test]
    fn nothing_is_asked_of_an_audience_that_cannot_answer() {
        assert_eq!(
            handover(false, false, false, true),
            Handover::Tell(TellWhy::NotATerminal)
        );
        // Both streams, not either: stdin decides whether an answer can
        // arrive and stdout decides whether the question was seen.
        assert_eq!(
            handover(false, true, false, true),
            Handover::Tell(TellWhy::NotATerminal),
            "a menu written to a pipe is a question nobody read"
        );
        assert_eq!(
            handover(false, false, true, true),
            Handover::Tell(TellWhy::NotATerminal),
            "a question nobody can answer"
        );
    }

    #[test]
    fn a_person_at_a_terminal_is_asked() {
        assert_eq!(handover(false, true, true, true), Handover::Ask);
    }

    /// `--json` is a parser waiting for one payload, so it gets no menu and no
    /// command — whatever the terminal happens to be.
    #[test]
    fn a_parser_is_offered_nothing_at_all() {
        for (stdin, stdout) in [(true, true), (false, false)] {
            assert_eq!(handover(true, stdin, stdout, true), Handover::Silent);
        }
    }

    /// **Offering to launch something that is not there produces a failure at
    /// the moment the reader was expecting help.** No skill means the command
    /// is printed and its absence is said out loud, at a terminal as much as
    /// through a pipe.
    #[test]
    fn a_missing_skill_is_reported_rather_than_launched() {
        assert_eq!(
            handover(false, true, true, false),
            Handover::Tell(TellWhy::NoSkill)
        );
        assert_eq!(
            handover(false, false, false, false),
            Handover::Tell(TellWhy::NoSkill)
        );
    }

    /// The default is the one that reads nothing, so a caller that never asked
    /// cannot accidentally block.
    #[test]
    fn the_default_handover_reads_nothing() {
        assert_eq!(Handover::default(), Handover::Tell(TellWhy::NotATerminal));
    }

    /// The order the agreed layout draws, and it does not move when a
    /// repository gains or loses one kind.
    #[test]
    fn the_kinds_are_a_fixed_list_in_the_agreed_order() {
        assert_eq!(
            KINDS,
            [
                "lockfile",
                "package scripts",
                "compose",
                "ci",
                "pyproject",
                "makefile"
            ]
        );
    }
}
