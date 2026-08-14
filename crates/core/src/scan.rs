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
//! reading is [`CANDIDATES`] plus the workflow directory, so the shell has a
//! list rather than a heuristic and nothing here opens a directory.

use crate::envelope::ResultRow;
use crate::error::Status;
use serde::Serialize;
use std::collections::BTreeSet;

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

/// The root-level files a scan reads, and nothing else.
///
/// **A list rather than a walk.** `armada manifest config scan` promises to
/// depend on nothing but a readable directory
/// (`docs/commands/manifest/config.md`), and a recursive walk of an unknown
/// repository is a promise about `node_modules` nobody made. Workflows are the
/// one exception and are read from `.github/workflows/` by name, because that
/// directory *is* the evidence of what a repo actually runs.
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
    "uv.lock",
    "poetry.lock",
    "Pipfile.lock",
    "Cargo.toml",
    "Cargo.lock",
    "go.mod",
    "go.sum",
    "go.work",
    "Gemfile.lock",
    "composer.lock",
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
    let find = |name: &str| files.iter().find(|f| f.path == name);

    evidence.config_present = find("armada.yml").is_some();

    // The package manager a `package.json` pins, so `pnpm-lock.yaml` can be
    // reported as `pnpm@9` rather than as `pnpm` plus a version nobody stated.
    let pinned = find("package.json").and_then(|f| package_manager(&f.text));

    for (name, manager) in LOCKFILES {
        if find(name).is_none() {
            continue;
        }
        let manager = match &pinned {
            Some(pinned) if pinned.split('@').next() == Some(manager) => pinned.clone(),
            _ => (*manager).to_string(),
        };
        evidence.lockfiles.push(Lockfile {
            file: (*name).to_string(),
            manager,
        });
    }

    if let Some(file) = find("package.json") {
        let scripts = json_scripts(&file.text);
        if !scripts.is_empty() {
            evidence.scripts.push(ScriptSource {
                file: file.path.clone(),
                scripts,
            });
        }
    }

    if let Some(file) = find("pyproject.toml") {
        evidence.pyproject.push(PyProject {
            file: file.path.clone(),
            tools: toml_tool_sections(&file.text),
        });
    }

    for name in ["Makefile", "makefile", "GNUmakefile"] {
        if let Some(file) = find(name) {
            evidence.makefiles.push(Makefile {
                file: file.path.clone(),
                targets: make_targets(&file.text),
            });
        }
    }

    for name in [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ] {
        if let Some(file) = find(name) {
            evidence.compose.push(ComposeFile {
                file: file.path.clone(),
                services: compose_services(&file.text),
            });
        }
    }

    // Workflows arrive from a directory listing, so they are sorted by path
    // rather than taken in the order the filesystem happened to hand them over.
    let mut workflows: Vec<&SourceFile> = files
        .iter()
        .filter(|f| f.path.starts_with(WORKFLOW_DIR))
        .collect();
    workflows.sort_by(|a, b| a.path.cmp(&b.path));
    for file in workflows {
        let (steps, runs) = workflow_steps(&file.text);
        evidence.ci.push(CiWorkflow {
            file: file.path.clone(),
            steps,
            runs,
        });
    }

    for (name, globs) in [
        (
            "pnpm-workspace.yaml",
            yaml_string_list(find("pnpm-workspace.yaml"), "packages"),
        ),
        ("package.json", json_workspaces(find("package.json"))),
        ("Cargo.toml", toml_workspace_members(find("Cargo.toml"))),
    ] {
        if !globs.is_empty() {
            evidence.workspace_globs.push(WorkspaceGlobs {
                file: name.to_string(),
                globs,
            });
        }
    }

    evidence
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
pub const KINDS: [&str; 6] = [
    "lockfile",
    "scripts",
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
            "scripts",
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

/// The published port of one compose `ports:` entry, in either syntax.
///
/// Short syntax is `[[IP:]HOST:]CONTAINER[/PROTO]`, so the published port is
/// the last numeric segment before the container port — or the only segment,
/// when a service publishes on the same number it listens on. Long syntax names
/// it outright.
fn published_port(entry: &serde_yaml_ng::Value) -> Option<String> {
    if let Some(mapping) = entry.as_mapping() {
        let published = mapping
            .get("published")
            .or_else(|| mapping.get("target"))?
            .clone();
        return scalar(&published);
    }
    let text = scalar(entry)?;
    let text = text.split('/').next().unwrap_or(&text).to_string();
    let segments: Vec<&str> = text.split(':').collect();
    let published = match segments.as_slice() {
        [single] => *single,
        // `IP:HOST:CONTAINER` and `HOST:CONTAINER` both publish the
        // second-to-last segment.
        [.., host, _container] => *host,
        [] => return None,
    };
    (!published.is_empty()).then(|| published.to_string())
}

fn scalar(value: &serde_yaml_ng::Value) -> Option<String> {
    match value {
        serde_yaml_ng::Value::String(text) => Some(text.clone()),
        serde_yaml_ng::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
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
            [
                (
                    "pnpm-workspace.yaml",
                    vec!["apps/*".to_string(), "packages/*".to_string()]
                ),
                ("package.json", vec!["web/*".to_string()]),
                (
                    "Cargo.toml",
                    vec!["crates/core".to_string(), "crates/cli".to_string()]
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
                ("scripts", "package.json", "1 in package.json"),
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
        assert_eq!(rows[0].id, "scripts");
    }

    /// The order the agreed layout draws, and it does not move when a
    /// repository gains or loses one kind.
    #[test]
    fn the_kinds_are_a_fixed_list_in_the_agreed_order() {
        assert_eq!(
            KINDS,
            [
                "lockfile",
                "scripts",
                "compose",
                "ci",
                "pyproject",
                "makefile"
            ]
        );
    }
}
