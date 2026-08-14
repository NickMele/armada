//! Layer 3 of the bootstrap sandwich, pass 1: **static, seconds, and nothing
//! is executed** (PLAN.md §5).
//!
//! ```text
//! pass 1  STATIC     schema, references, argv[0] resolvability, glob coverage
//!                    failures short-circuit here
//! pass 2  FOR REAL   run the check suite properly, exactly as `check` would
//! ```
//!
//! **Layer 3 is load-bearing because agents hallucinate config** — a plausible
//! script name that does not exist, a flag from a different version. Pass 1
//! catches that in seconds rather than in a fresh worktree at the worst moment,
//! which is the whole reason the layer exists.
//!
//! # What is here and what is the schema's
//!
//! One rule decides, and it is worth stating in general form because it settles
//! every future check too: **if it needs a second part of the document, or the
//! filesystem, it is verify; if it can be decided from the value in front of
//! you, it is the schema.** A schema rejection is a parse error at every entry
//! point, including the ones nobody remembered to route through this module.
//!
//! # Pure, with the filesystem answered in between
//!
//! Deciding needs facts only the outside world has — is `vitest` on `PATH`,
//! does `docs/skills/add.md` exist, does `web/**` match anything. So this is
//! two functions and not one: [`probe`] says what to go and find out, the shell
//! finds it out, and [`pass_one`] decides from the [`Answers`]. Nothing here
//! opens anything, which is what keeps the hard part — the deciding — inside a
//! unit test.
//!
//! # `shell: true` has no `argv[0]`, and that is reported rather than assumed
//!
//! Under `shell: true` the string is a program in a language Armada does not
//! parse, and `VAR=x exec "$TOOL"` has no first word that is a command. Those
//! entries are counted as **unchecked**, never guessed at and never silently
//! passed. That count is the honest cost of the key and is worth seeing.

use crate::config::{Need, ResolvedConfig, ResolvedRun, ResolvedSkill};
use crate::error::{ArmadaError, ConfigWhere};
use crate::glob;
use crate::ports::{assign_ports, PortBlock};
use crate::template::split_argv;
use std::collections::{BTreeMap, BTreeSet};

/// The three static checks, in the order they are drawn.
///
/// **Named rather than numbered**, because these ids reach `results[]` and an
/// agent reads them: `argv[0]` says what failed, `check 3` says nothing.
pub const CHECKS: [&str; 3] = ["schema", "references", "argv[0]"];

/// What the `references` row covers, for the reader who is wondering what it
/// just passed.
///
/// **A constant label, not a derived list.** It names the family of checks, and
/// a detail that grew a phrase whenever a repository happened to declare one
/// more key would make two correct runs of one config read differently.
pub const REFERENCES_DETAIL: &str = "needs:, match:, ports fit block";

/// Validate a document against the embedded schema.
///
/// **This is the `schema` row, and it is the real validator rather than "it
/// parsed".** The structs and the schema are the same contract at two entry
/// points, but they are not the same *strength*: `deny_unknown_fields` catches
/// a key nobody declared, and every value-level rule — the name grammar, a
/// port's range, `shell: true` beside `${files}` — belongs to the schema alone
/// (PLAN.md §4.1.1). Those are exactly the shapes a hallucinated config has, so
/// a `schema` row that only re-reported the parse would pass the documents this
/// layer exists to catch.
///
/// `text` is the whole `armada.yml`, **not** its `manifest:` section: the schema
/// describes the file. It is passed as text rather than as a value so that the
/// YAML parser stays on this side of the seam — one crate reads `armada.yml`,
/// as PLAN.md §4.1.1 decision 5 requires, and a caller cannot hand the
/// validator a document a second parser produced.
pub fn schema_findings(text: &str, file: &str) -> Vec<Finding> {
    let document: serde_json::Value = match serde_yaml_ng::from_str(text) {
        Ok(document) => document,
        Err(error) => {
            return vec![Finding {
                check: "schema",
                error: ArmadaError::bad_config(
                    ConfigWhere::File {
                        file: file.to_string(),
                    },
                    error.to_string(),
                    "every key Armada accepts is in the armada.yml JSON Schema",
                ),
            }]
        }
    };
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let value: serde_json::Value = match serde_json::from_str(crate::config::SCHEMA) {
        Ok(value) => value,
        // Unreachable: the schema is embedded at compile time and a test parses
        // it. Reported rather than panicked on, because a panic in the verb
        // whose job is to report problems is the worst possible shape.
        Err(error) => {
            return vec![internal(
                file,
                format!("the embedded schema is not JSON: {error}"),
            )]
        }
    };
    if compiler.add_resource("armada.schema.json", value).is_err() {
        return vec![internal(
            file,
            "the embedded schema is not a usable resource".to_string(),
        )];
    }
    let Ok(index) = compiler.compile("armada.schema.json", &mut schemas) else {
        return vec![internal(
            file,
            "the embedded schema does not compile".to_string(),
        )];
    };

    match schemas.validate(&document, index) {
        Ok(()) => Vec::new(),
        Err(error) => vec![Finding {
            check: "schema",
            error: ArmadaError::bad_config(
                ConfigWhere::File {
                    file: file.to_string(),
                },
                // The validator's own message, which names the instance
                // location. Rewriting it would be a second vocabulary for the
                // one authoritative statement of the contract.
                error
                    .to_string()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                "every key Armada accepts is in the armada.yml JSON Schema",
            ),
        }],
    }
}

fn internal(file: &str, message: String) -> Finding {
    Finding {
        check: "schema",
        error: ArmadaError {
            class: crate::error::ErrClass::ArmadaBug,
            r#where: file.to_string(),
            message,
            next_action: None,
        },
    }
}

/// One thing pass 1 needs the outside world to answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The key path this came from — also the key into [`Answers::resolvable`].
    pub at: String,
    /// The check or command id, for the message.
    pub id: String,
    /// `argv[0]`, split from the command.
    pub argv0: String,
    /// The command in full, for the suggestion.
    pub cmd: String,
    /// The component root it would resolve under, when the entry has one.
    pub root: Option<String>,
}

/// Everything pass 1 must be told before it can decide.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Probe {
    /// Every argv-split command, and the `argv[0]` to resolve.
    pub programs: Vec<Program>,
    /// Workspace-relative paths whose existence decides a finding.
    pub paths: Vec<String>,
    /// `match:` globs that must hit at least one file.
    pub globs: Vec<String>,
}

/// The outside world's answers to a [`Probe`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    /// [`Program::at`] → whether `argv[0]` is on `PATH` or is an executable
    /// file under the component root.
    pub resolvable: BTreeMap<String, bool>,
    /// The probed paths that exist **and** resolve inside the workspace root.
    /// A symlink whose target escapes is absent from this set, which is what
    /// the schema cannot express and only a `realpath` can answer.
    pub inside: BTreeSet<String>,
    /// The probed globs that matched at least one tracked file.
    pub matched: BTreeSet<String>,
    /// The package manager this repository's lockfile names, when it has one.
    ///
    /// **A fact, not an inference.** `pnpm-lock.yaml` means pnpm because pnpm
    /// writes it, and the only use made of it is to phrase a *question* —
    /// "did you mean `pnpm exec vitest run`?" — rather than to decide anything.
    pub package_manager: Option<String>,
}

/// One problem, attributed to the row that reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Which of [`CHECKS`] this belongs under.
    pub check: &'static str,
    /// The failure, with the key path to edit and what would fix it.
    pub error: ArmadaError,
}

/// What pass 1 decided.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Empty means pass 1 passed and pass 2 may be attempted.
    pub findings: Vec<Finding>,
    /// Entries under `shell: true`, which have no `argv[0]` to resolve.
    pub unchecked: usize,
}

impl Report {
    /// Whether pass 1 passed, and pass 2 is therefore worth attempting.
    ///
    /// **`unchecked` is not a failure.** It is a count of what could not be
    /// established either way, and refusing to run the suite over it would make
    /// `shell: true` unusable rather than honest.
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    /// The findings under one of [`CHECKS`].
    pub fn under(&self, check: &str) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.check == check).collect()
    }
}

/// Everything pass 1 needs to be told, read off the resolved config.
pub fn probe(config: &ResolvedConfig) -> Probe {
    let mut probe = Probe::default();

    for (name, component) in &config.components {
        let root = component.root.clone();
        for (check_name, check) in &component.checks {
            // **Under `shell: true` there is nothing to probe**, which is what
            // makes those entries `unchecked` rather than passed.
            if check.shell {
                continue;
            }
            for (key, cmd) in [("cmd", Some(&check.cmd)), ("fix", check.fix.as_ref())] {
                let Some(cmd) = cmd else { continue };
                push_program(
                    &mut probe,
                    &format!("components.{name}.checks.{check_name}.{key}"),
                    &check.id,
                    cmd,
                    root.clone(),
                );
            }
        }
        for (index, step) in component.setup.iter().enumerate() {
            if step.shell {
                continue;
            }
            push_program(
                &mut probe,
                &format!("components.{name}.setup[{index}]"),
                name,
                &step.cmd,
                root.clone(),
            );
        }
        if let Some(ResolvedRun::Command { cmd, shell, .. }) = &component.run {
            if !shell {
                push_program(
                    &mut probe,
                    &format!("components.{name}.run.cmd"),
                    name,
                    cmd,
                    root.clone(),
                );
            }
        }
        probe.globs.extend(component.match_globs.iter().cloned());
        probe.paths.extend(component.owns_files.iter().cloned());
        if let Some(run) = &component.run {
            probe.paths.extend(run.common().owns_files.iter().cloned());
        }
        if let Some(ResolvedRun::Compose { file, .. }) = &component.run {
            probe.paths.extend(file.iter().cloned());
        }
    }

    for (name, command) in &config.commands {
        if command.shell {
            continue;
        }
        push_program(
            &mut probe,
            &format!("commands.{name}.cmd"),
            name,
            &command.cmd,
            None,
        );
    }
    for command in config.commands.values() {
        probe.paths.extend(command.owns_files.iter().cloned());
    }

    for skill in config.skills.values() {
        probe.paths.push(skill.doc.clone());
    }
    for path in &config.workspaces {
        probe.paths.push(format!("{path}/armada.yml"));
    }

    probe.paths.sort();
    probe.paths.dedup();
    probe.globs.sort();
    probe.globs.dedup();
    probe
}

/// Split `argv[0]` off a command, ignoring one that templating owns.
///
/// A command whose first word is a substitution — `${port.api}` never is, but a
/// future one might be — has no literal `argv[0]`, and neither does an empty
/// string. Both are left alone rather than reported as a missing program.
fn push_program(probe: &mut Probe, at: &str, id: &str, cmd: &str, root: Option<String>) {
    let locator = ConfigWhere::Path {
        file: "armada.yml".to_string(),
        path: at.to_string(),
    };
    let Ok(argv) = split_argv(cmd, &locator) else {
        return;
    };
    let Some(argv0) = argv.first() else { return };
    if argv0.contains("${") {
        return;
    }
    probe.programs.push(Program {
        at: at.to_string(),
        id: id.to_string(),
        argv0: argv0.clone(),
        cmd: cmd.to_string(),
        root,
    });
}

/// Pass 1: schema, references, `argv[0]`, and the count of what has none.
///
/// `reserved` is the built-in verb list, passed in because the verb surface is
/// the CLI's and the core may not reach up for it.
pub fn pass_one(
    config: &ResolvedConfig,
    answers: &Answers,
    block: PortBlock,
    reserved: &[&str],
    file: &str,
) -> Report {
    let mut findings: Vec<Finding> = Vec::new();
    let at = |path: String| ConfigWhere::Path {
        file: file.to_string(),
        path: format!("{}.{path}", crate::config::SECTION),
    };
    let mut fail = |check: &'static str, where_: ConfigWhere, message: String, fix: String| {
        findings.push(Finding {
            check,
            error: ArmadaError::bad_config(where_, message, fix),
        });
    };

    // ---------------------------------------------------------- references
    let check_ids: BTreeSet<&str> = config
        .components
        .values()
        .flat_map(|component| component.checks.values())
        .map(|check| check.id.as_str())
        .collect();

    for (name, component) in &config.components {
        let needs = component
            .checks
            .iter()
            .flat_map(|(check_name, check)| {
                check
                    .needs
                    .iter()
                    .map(move |need| (format!("components.{name}.checks.{check_name}.needs"), need))
            })
            .chain(component.run.iter().flat_map(|run| {
                run.common()
                    .needs
                    .iter()
                    .map(|need| (format!("components.{name}.run.needs"), need))
            }));
        for (path, need) in needs {
            let missing = match need {
                Need::Component(target) => !config.components.contains_key(target),
                Need::Check(target) => !check_ids.contains(target.as_str()),
            };
            if missing {
                let (kind, target) = match need {
                    Need::Component(target) => ("component", target),
                    Need::Check(target) => ("check", target),
                };
                fail(
                    "references",
                    at(path),
                    format!("`needs: [{target}]` names no declared {kind}"),
                    format!("declare `{target}`, or point `needs:` at one that exists"),
                );
            }
        }

        // `in:` runs a check inside a *compose service*, so the component it
        // names has to have one — a `driver: command` service has no container
        // for Armada to build an exec against.
        for (check_name, check) in &component.checks {
            let Some(service) = &check.in_service else {
                continue;
            };
            let compose = config
                .components
                .get(service)
                .and_then(|target| target.run.as_ref())
                .is_some_and(|run| matches!(run, ResolvedRun::Compose { .. }));
            if !compose {
                fail(
                    "references",
                    at(format!("components.{name}.checks.{check_name}.in")),
                    format!("`in: {service}` names no component with `run.driver: compose`"),
                    "run the check outside a container, or give that component a compose driver"
                        .to_string(),
                );
            }
        }

        // A `root:` that normalises outside the workspace breaks the identity
        // derivation §2.2 depends on, and the schema cannot catch `a/../../b`
        // because only normalisation can.
        if let Some(root) = &component.root {
            if escapes(root) {
                fail(
                    "references",
                    at(format!("components.{name}.root")),
                    format!("`root: {root}` normalises to a path outside the workspace"),
                    "point `root:` inside the workspace; multi-repo is reserved (PLAN.md §7)"
                        .to_string(),
                );
            }
            for nested in &config.workspaces {
                if inside(root, nested) {
                    fail(
                        "references",
                        at(format!("components.{name}.root")),
                        format!("`root: {root}` reaches into the declared workspace `{nested}`"),
                        format!("a nested workspace runs its own checks; drop `{nested}` from `workspaces:`, or move this component's root"),
                    );
                }
            }
        }

        for pattern in &component.match_globs {
            if !answers.matched.contains(pattern) {
                fail(
                    "references",
                    at(format!("components.{name}.match")),
                    format!("`match: [{pattern}]` matches no tracked file"),
                    "correct the glob, or remove the component".to_string(),
                );
            }
            for nested in &config.workspaces {
                if glob::matches(pattern, nested) {
                    fail(
                        "references",
                        at(format!("components.{name}.match")),
                        format!(
                            "`match: [{pattern}]` reaches into the declared workspace `{nested}`"
                        ),
                        format!("scope the glob outside `{nested}`, which is its own workspace"),
                    );
                }
            }
        }
    }

    for (a, b) in cycles(&config.components) {
        fail(
            "references",
            at("components.*.checks.*.needs".to_string()),
            format!("`needs:` is cyclic: {a} and {b} wait for each other"),
            "break the cycle; a check graph that waits on itself never starts".to_string(),
        );
    }

    // Two components declaring one port name are declaring one port — the
    // namespace is workspace-global, which is what makes a cross-component
    // `${port.NAME}` have exactly one answer. Saying so is what tells a typo
    // from an intention.
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, component) in &config.components {
        let Some(run) = &component.run else { continue };
        for port in run.common().ports.keys() {
            if let Some(first) = seen.insert(port, name) {
                fail(
                    "references",
                    at(format!("components.{name}.run.ports.{port}")),
                    format!("`{port}` is already declared by `{first}`; port names are workspace-global"),
                    format!("rename one of them — `${{port.{port}}}` has to have one answer"),
                );
            }
        }
    }

    // The one finding that arrives already typed: `assign_ports` is the single
    // implementation of "the block holds them", and re-deriving it here would
    // be a second answer to a question that already has one.
    let ports_over_block = assign_ports(config, block, file).err();

    for (name, granted) in granted_secrets(config) {
        if !config.secrets.contains_key(&granted) {
            fail(
                "references",
                at(format!("{name}.secrets")),
                format!("`secrets: [{granted}]` names nothing declared under `secrets:`"),
                format!("declare `{granted}:` with the reference that fetches it"),
            );
        }
    }
    for (name, reference) in &config.secrets {
        let scheme = reference.split("://").next().unwrap_or_default();
        if !config.secret_providers.contains_key(scheme) {
            fail(
                "references",
                at(format!("secrets.{name}")),
                format!("`{reference}` uses the `{scheme}` scheme, which no `secret_providers:` entry declares"),
                format!("add `secret_providers: {{ {scheme}: {{ cmd: … }} }}` — the value is never fetched here"),
            );
        }
    }

    for path in config
        .components
        .values()
        .flat_map(|component| {
            component.owns_files.iter().chain(
                component
                    .run
                    .iter()
                    .flat_map(|run| run.common().owns_files.iter()),
            )
        })
        .chain(
            config
                .commands
                .values()
                .flat_map(|command| command.owns_files.iter()),
        )
    {
        // **Only a path that exists and escapes is a finding.** A declared
        // `owns.files` that is not there yet is the ordinary state before
        // `setup:` has run; one that is a symlink out of the tree is what
        // `clean --artifacts` would follow, and the schema cannot see it.
        if escapes(path) {
            fail(
                "references",
                at("owns.files".to_string()),
                format!("`owns.files: [{path}]` normalises outside the workspace"),
                "declare a path inside the workspace — `clean --artifacts` deletes what these name"
                    .to_string(),
            );
        }
    }

    for (name, skill) in &config.skills {
        skill_findings(name, skill, config, answers, reserved, &at, &mut fail);
    }

    for nested in &config.workspaces {
        let marker = format!("{nested}/armada.yml");
        if !answers.inside.contains(&marker) {
            fail(
                "references",
                at(format!("workspaces.{nested}")),
                format!("`{nested}` has no armada.yml, so it is not a workspace"),
                format!("write one at {marker}, or drop the entry"),
            );
        }
    }

    // ------------------------------------------------------------ argv[0]
    for program in probe(config).programs {
        if answers
            .resolvable
            .get(&program.at)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        let suggestion = match &answers.package_manager {
            Some(manager) => format!(
                "{} declares `{}`; did you mean `{manager} exec {}`?",
                program.id, program.cmd, program.cmd
            ),
            None => format!(
                "{} declares `{}`; install it, or point it at a path inside the component root",
                program.id, program.cmd
            ),
        };
        fail(
            "argv[0]",
            at(program.at.clone()),
            format!("`{}` not on PATH or in root", program.argv0),
            suggestion,
        );
    }

    findings.extend(ports_over_block.map(|error| Finding {
        check: "references",
        error,
    }));
    // Reported in the order the checks are drawn, so a reader's eye goes down
    // the table and then down the fix lines in the same order.
    findings.sort_by_key(|finding| CHECKS.iter().position(|c| *c == finding.check));

    Report {
        findings,
        unchecked: unchecked_entries(config),
    }
}

/// The four cross-references a skill adds (PLAN.md §4.8).
///
/// **That set is the whole argument for a schema block rather than a loose
/// directory of markdown**: a skill naming a command or a check that does not
/// exist fails in seconds at authoring time, rather than in a fresh worktree at
/// the worst moment.
fn skill_findings(
    name: &str,
    skill: &ResolvedSkill,
    config: &ResolvedConfig,
    answers: &Answers,
    reserved: &[&str],
    at: &impl Fn(String) -> ConfigWhere,
    fail: &mut impl FnMut(&'static str, ConfigWhere, String, String),
) {
    if reserved.contains(&name) {
        fail(
            "references",
            at(format!("skills.{name}")),
            format!("`{name}` is a built-in verb, so a skill may not take the name"),
            "rename the skill; the verbs mean the same thing in every repository".to_string(),
        );
    }

    if !answers.inside.contains(&skill.doc) {
        fail(
            "references",
            at(format!("skills.{name}.doc")),
            format!("`doc: {}` is not a file inside the workspace", skill.doc),
            "write the prose there — Armada holds the path and never reads it".to_string(),
        );
    }

    // **`uses:` grants nothing**, and this is the check that makes that true: a
    // skill can only ever name capability the repository already declared in a
    // file a human reviewed.
    for used in &skill.uses {
        if !config.commands.contains_key(used) {
            fail(
                "references",
                at(format!("skills.{name}.uses")),
                format!("`uses: [{used}]` names no declared `commands:` entry"),
                format!("declare `commands: {{ {used}: … }}` — a skill grants nothing new"),
            );
        }
    }

    let check_ids: BTreeSet<&str> = config
        .components
        .values()
        .flat_map(|component| component.checks.values())
        .map(|check| check.id.as_str())
        .collect();
    let check_names: BTreeSet<&str> = config
        .components
        .values()
        .flat_map(|component| component.checks.keys())
        .map(String::as_str)
        .collect();
    for scope in &skill.verify {
        // The same grammar `check --scope` accepts: a full `component:check`
        // id, or a bare check name meaning every component's copy of it.
        let known = check_ids.contains(scope.as_str())
            || (!scope.contains(':') && check_names.contains(scope.as_str()));
        if !known {
            fail(
                "references",
                at(format!("skills.{name}.verify.check")),
                format!("`verify.check: [{scope}]` names no check"),
                "name a check id, or a check name every component shares".to_string(),
            );
        }
    }
}

/// Entries whose `shell: true` leaves no `argv[0]` to resolve.
fn unchecked_entries(config: &ResolvedConfig) -> usize {
    let checks = config.components.values().flat_map(|component| {
        component.checks.values().map(|check| {
            // `cmd:` and `fix:` are covered by one `shell:`, so an entry with
            // both is two strings Armada cannot read rather than one.
            usize::from(check.shell) * (1 + usize::from(check.fix.is_some()))
        })
    });
    let setup = config
        .components
        .values()
        .flat_map(|component| component.setup.iter().map(|step| usize::from(step.shell)));
    let services = config.components.values().map(|component| {
        usize::from(matches!(
            &component.run,
            Some(ResolvedRun::Command { shell: true, .. })
        ))
    });
    let commands = config.commands.values().map(|c| usize::from(c.shell));
    checks.chain(setup).chain(services).chain(commands).sum()
}

/// Every `secrets:` grant, and the entry that made it.
fn granted_secrets(config: &ResolvedConfig) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, component) in &config.components {
        for (check_name, check) in &component.checks {
            for granted in &check.secrets {
                out.push((
                    format!("components.{name}.checks.{check_name}"),
                    granted.clone(),
                ));
            }
        }
        if let Some(run) = &component.run {
            for granted in &run.common().secrets {
                out.push((format!("components.{name}.run"), granted.clone()));
            }
        }
    }
    for (name, command) in &config.commands {
        for granted in &command.secrets {
            out.push((format!("commands.{name}"), granted.clone()));
        }
    }
    out
}

/// Whether a workspace-relative path normalises to somewhere outside the
/// workspace.
///
/// **Verify owns this rule because only verify can normalise `a/../../b`.** The
/// schema rejects a leading `/` and a leading `..`, which is everything
/// decidable from the string in front of you; the rest needs arithmetic over
/// the whole path.
fn escapes(path: &str) -> bool {
    if path.starts_with('/') {
        return true;
    }
    let mut depth: i32 = 0;
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => depth -= 1,
            _ => depth += 1,
        }
        if depth < 0 {
            return true;
        }
    }
    false
}

/// Whether `path` is inside `directory`, both workspace-relative.
fn inside(path: &str, directory: &str) -> bool {
    let directory = directory.trim_end_matches('/');
    path == directory || path.starts_with(&format!("{directory}/"))
}

/// The check-id pairs that wait for each other, directly or through others.
///
/// Reported as pairs rather than as a cycle path, because the fix is the same
/// either way — break one edge — and a path is a longer sentence saying it.
fn cycles(
    components: &BTreeMap<String, crate::config::ResolvedComponent>,
) -> Vec<(String, String)> {
    let edges: BTreeMap<&str, Vec<&str>> = components
        .values()
        .flat_map(|component| component.checks.values())
        .map(|check| {
            let targets = check
                .needs
                .iter()
                .filter_map(|need| match need {
                    Need::Check(id) => Some(id.as_str()),
                    Need::Component(_) => None,
                })
                .collect();
            (check.id.as_str(), targets)
        })
        .collect();

    let mut found = Vec::new();
    for start in edges.keys() {
        // Depth-first from each node, reporting the first edge that closes back
        // onto the start. A check graph is a handful of nodes, so the repeated
        // walk costs nothing and reads as what it is.
        let mut stack = vec![*start];
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        while let Some(node) = stack.pop() {
            for next in edges.get(node).into_iter().flatten() {
                if next == start {
                    let pair = (start.to_string(), node.to_string());
                    if !found.contains(&pair) {
                        found.push(pair);
                    }
                } else if seen.insert(next) {
                    stack.push(next);
                }
            }
        }
    }
    // A → B and B → A are one cycle reported twice; keep the first spelling.
    let all = found.clone();
    found.retain(|(a, b)| !all.iter().any(|(x, y)| x == b && y == a && x < a));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{parse, resolve, Defaults};

    const RESERVED: [&str; 3] = ["init", "check", "clean"];

    fn block() -> PortBlock {
        PortBlock {
            from: 5460,
            to: 5469,
        }
    }

    fn config(yaml: &str) -> ResolvedConfig {
        let parsed = parse(yaml, "armada.yml").expect("the fixture parses");
        resolve(parsed, &Defaults::built_in(), "armada.yml").expect("the fixture resolves")
    }

    /// Everything the probe asked for, answered "yes" — so a test states only
    /// the fact it is about.
    fn all_good(config: &ResolvedConfig) -> Answers {
        let probe = probe(config);
        Answers {
            resolvable: probe
                .programs
                .iter()
                .map(|program| (program.at.clone(), true))
                .collect(),
            inside: probe.paths.iter().cloned().collect(),
            matched: probe.globs.iter().cloned().collect(),
            package_manager: None,
        }
    }

    fn run(yaml: &str) -> Report {
        let config = config(yaml);
        let answers = all_good(&config);
        pass_one(&config, &answers, block(), &RESERVED, "armada.yml")
    }

    fn messages(report: &Report) -> Vec<String> {
        report
            .findings
            .iter()
            .map(|f| f.error.message.clone())
            .collect()
    }

    const ONE_CHECK: &str = "manifest:\n  version: 1\n  components:\n    web:\n      \
                             match: [\"web/**\"]\n      checks:\n        test:\n          \
                             cmd: vitest run\n";

    #[test]
    fn a_config_whose_references_all_resolve_passes_pass_one() {
        let report = run(ONE_CHECK);
        assert!(report.passed(), "{:?}", messages(&report));
        assert_eq!(report.unchecked, 0);
    }

    /// **The hallucinated script name that motivated layer 3.** A plausible
    /// command that is not there, caught in seconds instead of in a fresh
    /// worktree at the worst moment.
    #[test]
    fn an_unresolvable_argv0_fails_and_names_the_entry_that_declared_it() {
        let config = config(ONE_CHECK);
        let mut answers = all_good(&config);
        for value in answers.resolvable.values_mut() {
            *value = false;
        }
        answers.package_manager = Some("pnpm".to_string());

        let report = pass_one(&config, &answers, block(), &RESERVED, "armada.yml");
        assert!(!report.passed());
        let finding = report.under("argv[0]")[0];
        assert_eq!(finding.error.message, "`vitest` not on PATH or in root");
        assert_eq!(
            finding.error.next_action.as_deref(),
            Some("web:test declares `vitest run`; did you mean `pnpm exec vitest run`?")
        );
        assert_eq!(
            finding.error.r#where,
            "armada.yml:manifest.components.web.checks.test.cmd"
        );
    }

    /// The suggestion names the manager the **lockfile** does, and says nothing
    /// when there is no lockfile: `verify` may phrase a question, never a guess.
    #[test]
    fn the_suggestion_is_dropped_when_no_lockfile_names_a_manager() {
        let config = config(ONE_CHECK);
        let mut answers = all_good(&config);
        for value in answers.resolvable.values_mut() {
            *value = false;
        }
        let report = pass_one(&config, &answers, block(), &RESERVED, "armada.yml");
        let fix = report.under("argv[0]")[0]
            .error
            .next_action
            .clone()
            .unwrap();
        assert!(fix.contains("install it"), "{fix}");
        assert!(!fix.contains("exec"), "a manager was invented: {fix}");
    }

    /// **`shell: true` has no `argv[0]`**, so it is never probed and never
    /// guessed at — it is counted.
    #[test]
    fn a_shell_entry_is_counted_as_unchecked_rather_than_resolved() {
        let report = run("manifest:\n  version: 1\n  components:\n    api:\n      \
                          match: [\"api/**\"]\n      checks:\n        lint:\n          \
                          cmd: \"VAR=x exec $TOOL\"\n          fix: \"VAR=x exec $TOOL --fix\"\n          \
                          shell: true\n");
        assert_eq!(report.unchecked, 2, "cmd: and fix: are two strings");
        assert!(report.passed(), "unchecked is not a failure");
        assert!(probe(&config(
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          \
             cmd: \"VAR=x exec $TOOL\"\n          shell: true\n"
        ))
        .programs
        .is_empty());
    }

    #[test]
    fn a_needs_naming_nothing_is_a_reference_failure() {
        let report = run("manifest:\n  version: 1\n  components:\n    ui:\n      \
                          match: [\"ui/**\"]\n      checks:\n        types:\n          \
                          cmd: tsc\n          needs: [postgres, core:build]\n");
        let said = messages(&report);
        assert!(
            said.iter()
                .any(|m| m.contains("names no declared component")),
            "{said:?}"
        );
        assert!(
            said.iter().any(|m| m.contains("names no declared check")),
            "{said:?}"
        );
    }

    #[test]
    fn a_cyclic_check_graph_is_reported_rather_than_deadlocked() {
        let report = run("manifest:\n  version: 1\n  components:\n    a:\n      \
                          match: [\"a/**\"]\n      checks:\n        one:\n          cmd: x\n          \
                          needs: [a:two]\n        two:\n          cmd: y\n          needs: [a:one]\n");
        assert!(
            messages(&report).iter().any(|m| m.contains("cyclic")),
            "{:?}",
            messages(&report)
        );
    }

    #[test]
    fn a_glob_that_matches_nothing_is_reported() {
        let config = config(ONE_CHECK);
        let mut answers = all_good(&config);
        answers.matched.clear();
        let report = pass_one(&config, &answers, block(), &RESERVED, "armada.yml");
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("matches no tracked file")),
            "{:?}",
            messages(&report)
        );
    }

    /// `${port.NAME}` is workspace-global, so one name has to have one answer.
    #[test]
    fn two_components_declaring_one_port_name_is_reported() {
        let report = run("manifest:\n  version: 1\n  components:\n    a:\n      \
                          run: { driver: command, cmd: ./a, ports: { web: 3000 } }\n    b:\n      \
                          run: { driver: command, cmd: ./b, ports: { web: 3000 } }\n");
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("workspace-global")),
            "{:?}",
            messages(&report)
        );
    }

    #[test]
    fn more_ports_than_the_block_holds_is_reported() {
        let report = {
            let config = config(
                "manifest:\n  version: 1\n  components:\n    a:\n      run:\n        \
                 driver: command\n        cmd: ./a\n        ports: { one: 1, two: 2, three: 3 }\n",
            );
            let answers = all_good(&config);
            pass_one(
                &config,
                &answers,
                PortBlock {
                    from: 5460,
                    to: 5461,
                },
                &RESERVED,
                "armada.yml",
            )
        };
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("its block holds")),
            "{:?}",
            messages(&report)
        );
    }

    /// **Verify owns this rule because only verify can normalise `a/../../b`.**
    #[test]
    fn a_root_that_normalises_outside_the_workspace_is_reported() {
        assert!(escapes("a/../../b"));
        assert!(escapes("/etc"));
        assert!(!escapes("a/../b"));
        assert!(!escapes("services/api"));

        let report = run("manifest:\n  version: 1\n  components:\n    api:\n      \
                          root: a/../../b\n      match: [\"a/**\"]\n      checks:\n        \
                          lint: { cmd: ruff }\n");
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("outside the workspace")),
            "{:?}",
            messages(&report)
        );
    }

    #[test]
    fn a_component_reaching_into_a_declared_workspace_is_reported() {
        let config = config(
            "manifest:\n  version: 1\n  workspaces: [apps/site]\n  components:\n    site:\n      \
             root: apps/site\n      match: [\"apps/site/**\"]\n      checks:\n        \
             lint: { cmd: eslint }\n",
        );
        let mut answers = all_good(&config);
        answers.inside.insert("apps/site/armada.yml".to_string());
        let report = pass_one(&config, &answers, block(), &RESERVED, "armada.yml");
        let said = messages(&report);
        assert_eq!(
            said.iter().filter(|m| m.contains("apps/site")).count(),
            2,
            "the root and the glob are two statements: {said:?}"
        );
    }

    #[test]
    fn an_in_naming_a_component_with_no_compose_driver_is_reported() {
        let report = run("manifest:\n  version: 1\n  components:\n    api:\n      \
                          run: { driver: command, cmd: ./api }\n    web:\n      \
                          match: [\"web/**\"]\n      checks:\n        test:\n          \
                          in: api\n          cmd: pytest\n");
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("run.driver: compose")),
            "{:?}",
            messages(&report)
        );
    }

    #[test]
    fn a_secret_grant_or_scheme_with_nothing_behind_it_is_reported() {
        let report = run(
            "manifest:\n  version: 1\n  secrets:\n    A: vault://a/b\n  \
                          commands:\n    x: { cmd: ./x, secrets: [B] }\n",
        );
        let said = messages(&report);
        assert!(
            said.iter().any(|m| m.contains("`secrets: [B]`")),
            "{said:?}"
        );
        assert!(
            said.iter().any(|m| m.contains("`vault` scheme")),
            "{said:?}"
        );
    }

    // ------------------------------------------------------------- schema

    /// **The row is the real validator, not "it parsed".** A name the grammar
    /// forbids is a value-level rule the schema owns and the structs do not, so
    /// a `schema` row that only re-reported the parse would pass it.
    #[test]
    fn the_schema_row_catches_what_the_structs_alone_would_not() {
        let bad = "manifest:\n  version: 1\n  components:\n    NotALegalName:\n      \
                   checks:\n        lint: { cmd: ruff }\n";
        let findings = schema_findings(bad, "armada.yml");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].check, "schema");
        assert!(
            findings[0].error.next_action.is_some(),
            "bad_config requires one"
        );

        // And the same document goes through the structs without complaint,
        // which is the whole reason the row cannot be derived from them.
        assert!(parse(
            "manifest:\n  version: 1\n  components:\n    NotALegalName:\n      \
             checks:\n        lint: { cmd: ruff }\n",
            "armada.yml"
        )
        .is_ok());
    }

    #[test]
    fn a_document_the_schema_accepts_produces_no_finding() {
        assert!(schema_findings(ONE_CHECK, "armada.yml").is_empty());
        assert!(schema_findings(SKILLED, "armada.yml").is_empty());
    }

    /// A document that is not YAML at all fails the same row, with the parser's
    /// own message: from the reader's side "this is not a config" and "this is
    /// not the config Armada accepts" are one answer.
    #[test]
    fn a_document_that_is_not_yaml_fails_the_schema_row() {
        let findings = schema_findings("manifest:\n  version: 1\n  components: {\n", "armada.yml");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check, "schema");
    }

    // ------------------------------------------------------------- skills

    const SKILLED: &str = "manifest:\n  version: 1\n  commands:\n    migrate: { cmd: ./m }\n  \
                           components:\n    api:\n      match: [\"api/**\"]\n      checks:\n        \
                           test: { cmd: pytest }\n  skills:\n    add-migration:\n      \
                           summary: Add one\n      doc: docs/skills/add.md\n      \
                           uses: [migrate]\n      verify:\n        check: [api:test]\n";

    #[test]
    fn a_skill_whose_four_cross_references_all_resolve_passes() {
        let report = run(SKILLED);
        assert!(report.passed(), "{:?}", messages(&report));
    }

    /// **`uses:` grants nothing**, and this is the check that makes it true: a
    /// skill can only name capability the repository already declared in a file
    /// a human reviewed.
    #[test]
    fn a_skill_may_not_name_a_command_the_repository_never_declared() {
        let report = run(&SKILLED.replace("uses: [migrate]", "uses: [migrate, deploy]"));
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("`uses: [deploy]`")),
            "{:?}",
            messages(&report)
        );
    }

    #[test]
    fn a_skill_doc_that_is_not_in_the_workspace_is_reported() {
        let config = config(SKILLED);
        let mut answers = all_good(&config);
        answers.inside.remove("docs/skills/add.md");
        let report = pass_one(&config, &answers, block(), &RESERVED, "armada.yml");
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("not a file inside the workspace")),
            "{:?}",
            messages(&report)
        );
    }

    /// The scope a skill verifies with is the same string `check --scope`
    /// takes, so a bare check name is legal and a name nothing declares is not.
    #[test]
    fn a_verify_scope_naming_no_check_is_reported_and_a_bare_name_is_not() {
        let report = run(&SKILLED.replace("check: [api:test]", "check: [test, api:lint]"));
        let said = messages(&report);
        assert!(said.iter().any(|m| m.contains("[api:lint]")), "{said:?}");
        assert!(
            !said.iter().any(|m| m.contains("[test]")),
            "a bare check name is the documented spelling: {said:?}"
        );
    }

    /// Same rule and same reason as `commands:`: the verbs mean the same thing
    /// in every repository, and a skill that took one of their names would be
    /// the first exception.
    #[test]
    fn a_skill_may_not_shadow_a_built_in_verb() {
        let report = run(&SKILLED.replace("add-migration:", "check:"));
        assert!(
            messages(&report)
                .iter()
                .any(|m| m.contains("is a built-in verb")),
            "{:?}",
            messages(&report)
        );
    }
}
