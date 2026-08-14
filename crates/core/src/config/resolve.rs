//! Parsing and resolution: text in, a typed and fully defaulted value out.
//!
//! Three rules decide what belongs here, and they are the same three the
//! schema and `config verify` are split along:
//!
//! - **The schema** decides anything readable from one value — a name's
//!   grammar, a port's range, `shell: true` next to `${files}`.
//! - **Resolution** decides anything needed to produce a typed value at all: a
//!   `driver:` with the wrong keys beside it cannot become a [`ResolvedRun`],
//!   so it is rejected here, with a key path saying where.
//! - **`config verify`** (phase 5) decides everything needing a second part of
//!   the document or the filesystem: `needs:` targets, duplicate port names,
//!   `argv[0]` on `PATH`, globs that match nothing.
//!
//! Resolution deliberately does not re-check what the schema already rejects.
//! Duplicating a rule is how two implementations of it drift apart, and the
//! test suite runs both over every fixture, so a gap between them is visible.

use super::model::{
    Check, CommandEntry, Component, Config, Document, OneOrMany, OwnsCommand, OwnsComponent,
    OwnsRun, Ready, Run, SetupStep,
};
use super::resolved::{
    Need, ReadyKind, ResolvedCheck, ResolvedCommand, ResolvedComponent, ResolvedConfig,
    ResolvedReady, ResolvedRun, ResolvedService, ResolvedSetupStep, Scope, Stdio,
};
use crate::error::{CharError, ConfigWhere};
use std::collections::BTreeMap;

/// Machine-level defaults, from `~/.armada/machine.yml` (PLAN.md §4.3.1).
///
/// Passed in rather than read, because they are not the repo's to state and
/// nothing below the entrypoint may reach for ambient state
/// (`ARCHITECTURE.md` §1.4). The golden snapshots resolve against
/// [`Defaults::built_in`], so a snapshot diff means a *default* changed rather
/// than someone's laptop did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defaults {
    /// Seconds a check gets when it declares no `timeout:`.
    pub check_timeout: u32,
    /// Seconds a ready-check gets when it declares no `timeout:`.
    pub ready_timeout: u32,
}

impl Defaults {
    /// The documented defaults: `check_timeout` 900, `ready.timeout` 60.
    pub const fn built_in() -> Self {
        Defaults {
            check_timeout: 900,
            ready_timeout: 60,
        }
    }
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults::built_in()
    }
}

/// The schema's advice, attached to every parse failure. Pointing at the one
/// authoritative statement of the contract beats restating a fragment of it.
const SEE_SCHEMA: &str = "every key Armada accepts is in the armada.yml JSON Schema";

/// The one section of `armada.yml` that exists, and the prefix every key path
/// into it carries.
///
/// Stated once so the locator and the document cannot drift: a `where` of
/// `armada.yml:components.api.checks.lint.cmd` would name a key that is not
/// there, which is worse than no locator (PLAN.md §4.1.1 decision 4).
pub const SECTION: &str = "manifest";

/// Parse `armada.yml`.
///
/// `file` is the workspace-relative path the error should cite, which is
/// `armada.yml` for an ordinary workspace and `<path>/armada.yml` for a nested one.
pub fn parse(text: &str, file: &str) -> Result<Config, CharError> {
    let config: Config = serde_yaml_ng::from_str::<Document>(text)
        .map(|document| document.manifest)
        .map_err(|e| {
            // Decision 4 (PLAN.md §4.1.1): `where` keeps the `bad_config`
            // grammar — `armada.yml:` and then a locator. A parser cannot give
            // a key path, so the locator is `line:column`; serde's own message
            // still carries the path when it has one, so nothing is lost.
            let location = match e.location() {
                Some(loc) => ConfigWhere::Location {
                    file: file.to_string(),
                    line: loc.line(),
                    column: loc.column(),
                },
                None => ConfigWhere::File {
                    file: file.to_string(),
                },
            };
            CharError::bad_config(location, e.to_string(), SEE_SCHEMA)
        })?;

    if config.version != 1 {
        return Err(CharError::bad_config(
            ConfigWhere::Path {
                file: file.to_string(),
                path: format!("{SECTION}.version"),
            },
            format!(
                "unsupported config version {} — this Armada understands version 1",
                config.version
            ),
            "set `version: 1`, or upgrade Armada",
        ));
    }

    Ok(config)
}

/// Read only the `workspaces:` list, tolerating everything else.
///
/// Workspace resolution has to ask each `armada.yml` on the way up whether it
/// declares the one below it (PLAN.md §2.1), and it has to ask **before** it
/// knows which config is this workspace's. A full parse there would make an
/// unrelated error in an outer config — a key phase 5 adds, a typo in a
/// component Armada is not going to run — fail the inner workspace it has no
/// authority over. So this reads one key and forms no opinion about the rest.
///
/// A document that does not parse at all yields an empty list, which makes the
/// nesting undeclared and the situation `bad_config` — reported against the
/// nesting rather than against a parse Armada did not need.
pub fn declared_workspaces(text: &str) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct JustWorkspaces {
        #[serde(default)]
        workspaces: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct JustManifest {
        #[serde(default)]
        manifest: Option<JustWorkspaces>,
    }
    serde_yaml_ng::from_str::<JustManifest>(text)
        .ok()
        .and_then(|parsed| parsed.manifest)
        .map(|section| section.workspaces)
        .unwrap_or_default()
}

/// Apply defaults, derive check ids, and normalise every key that has two
/// spellings into one.
pub fn resolve(
    config: Config,
    defaults: &Defaults,
    file: &str,
) -> Result<ResolvedConfig, CharError> {
    let mut components = BTreeMap::new();
    for (name, component) in config.components {
        let resolved = resolve_component(&name, component, defaults, file)?;
        components.insert(name, resolved);
    }

    let mut commands = BTreeMap::new();
    for (name, entry) in config.commands {
        let resolved = resolve_command(&name, entry, file)?;
        commands.insert(name, resolved);
    }

    let secret_providers = config
        .secret_providers
        .into_iter()
        .map(|(scheme, provider)| (scheme, provider.cmd))
        .collect();

    Ok(ResolvedConfig {
        version: config.version,
        components,
        commands,
        workspaces: config.workspaces,
        secrets: config.secrets,
        secret_providers,
    })
}

fn resolve_component(
    name: &str,
    component: Component,
    defaults: &Defaults,
    file: &str,
) -> Result<ResolvedComponent, CharError> {
    let match_globs = match component.match_globs {
        Some(globs) => globs,
        None => default_match_globs(component.root.as_deref(), component.checks.is_empty()),
    };

    let setup = component
        .setup
        .map(OneOrMany::into_vec)
        .unwrap_or_default()
        .into_iter()
        .map(|step| match step {
            SetupStep::Plain(cmd) => ResolvedSetupStep { cmd, shell: false },
            SetupStep::Object(object) => ResolvedSetupStep {
                cmd: object.cmd,
                shell: object.shell.unwrap_or(false),
            },
        })
        .collect();

    let OwnsComponent {
        files: owns_files,
        release: owns_release,
    } = component.owns.unwrap_or_default();

    let run = match component.run {
        Some(run) => Some(resolve_run(name, run, defaults, file)?),
        None => None,
    };

    let mut checks = BTreeMap::new();
    for (check_name, check) in component.checks {
        let resolved = resolve_check(name, &check_name, check, defaults, file)?;
        checks.insert(check_name, resolved);
    }

    Ok(ResolvedComponent {
        root: component.root,
        match_globs,
        setup,
        owns_files,
        owns_release: release_list(owns_release),
        run,
        checks,
    })
}

/// The `match:` default: a pure function of `(root, checks)`.
///
/// Leaving it unstated would make it a per-implementer decision — the thing
/// PLAN.md §4.1 opens by rejecting. A component with a `root:` matches its own
/// subtree; one without matches the workspace, which is what makes a
/// single-component repo need no scoping keys at all (PLAN.md §4.1.1, and the
/// `go-service` fixture).
///
/// A component with **no checks** gets no globs at all. `match:` exists to
/// decide which checks a changed file selects, so a run-only component has
/// nothing to scope.
///
/// A declared `workspaces:` entry is not an input here, because it cannot be
/// one: a nested workspace is not part of this workspace's file set at all
/// (PLAN.md §4.6), so `**` means everything in *this* workspace and reaches
/// into nothing. The overlap `config verify` rejects is a `root:` or a `match:`
/// glob that *names* a path inside a declared workspace — which is the author's
/// to write and never char's to infer.
fn default_match_globs(root: Option<&str>, no_checks: bool) -> Vec<String> {
    if no_checks {
        return Vec::new();
    }
    vec![match root {
        Some(root) => format!("{root}/**"),
        None => "**".to_string(),
    }]
}

fn resolve_check(
    component: &str,
    name: &str,
    check: Check,
    defaults: &Defaults,
    file: &str,
) -> Result<ResolvedCheck, CharError> {
    let scope = match check.scope.as_deref() {
        None | Some("file") => Scope::File,
        Some("component") => Scope::Component,
        Some(other) => {
            return Err(CharError::bad_config(
                ConfigWhere::Path {
                    file: file.to_string(),
                    path: format!("{SECTION}.components.{component}.checks.{name}.scope"),
                },
                format!("unknown scope `{other}`"),
                "use `file` (the default) or `component`",
            ))
        }
    };

    // Sorted here rather than wherever they are acquired: exclusives are taken
    // in sorted name order, and that ordering is what makes a cross-workspace
    // deadlock impossible for every interleaving (PLAN.md §4.3). Deciding it
    // in the pure core means no acquisition site can forget.
    let mut exclusive = check.exclusive;
    exclusive.sort();
    exclusive.dedup();

    Ok(ResolvedCheck {
        id: format!("{component}:{name}"),
        cmd: check.cmd,
        fix: check.fix,
        shell: check.shell.unwrap_or(false),
        timeout: check.timeout.unwrap_or(defaults.check_timeout),
        cost: check.cost.unwrap_or(1),
        scope,
        exclusive,
        needs: check.needs.into_iter().map(classify_need).collect(),
        env: check.env,
        in_service: check.in_service,
        secrets: check.secrets,
    })
}

fn resolve_run(
    component: &str,
    run: Run,
    defaults: &Defaults,
    file: &str,
) -> Result<ResolvedRun, CharError> {
    let at = |suffix: &str| ConfigWhere::Path {
        file: file.to_string(),
        path: format!("{SECTION}.components.{component}.run{suffix}"),
    };

    let OwnsRun {
        containers,
        networks,
        images,
        ports: owns_ports,
        files: owns_files,
        release,
    } = run.owns.unwrap_or_default();

    let common = ResolvedService {
        ports: run.ports,
        ready: resolve_ready(component, run.ready, defaults, file)?,
        needs: run.needs.into_iter().map(classify_need).collect(),
        env: run.env,
        owns_containers: containers,
        owns_networks: networks,
        owns_images: images,
        owns_ports,
        owns_files,
        owns_release: release_list(release),
        secrets: run.secrets,
    };

    match run.driver.as_str() {
        "compose" => {
            let file_list = run.file.ok_or_else(|| {
                CharError::bad_config(
                    at(""),
                    "`driver: compose` needs a `file:` list",
                    "add `file: [docker-compose.yml]`, or switch to `driver: command`",
                )
            })?;
            if file_list.is_empty() {
                return Err(CharError::bad_config(
                    at(".file"),
                    "`file:` is empty",
                    "list at least the base compose file",
                ));
            }
            for (key, present) in [
                ("cmd", run.cmd.is_some()),
                ("stop", run.stop.is_some()),
                ("shell", run.shell.is_some()),
            ] {
                if present {
                    return Err(CharError::bad_config(
                        at(&format!(".{key}")),
                        format!("`{key}:` belongs to `driver: command`, not `driver: compose`"),
                        format!("remove `{key}:`, or switch this service to `driver: command`"),
                    ));
                }
            }
            Ok(ResolvedRun::Compose {
                file: file_list,
                common,
            })
        }
        "command" => {
            if run.file.is_some() {
                return Err(CharError::bad_config(
                    at(".file"),
                    "`file:` belongs to `driver: compose`, not `driver: command`",
                    "remove `file:`, or switch this service to `driver: compose`",
                ));
            }
            let cmd = run.cmd.ok_or_else(|| {
                CharError::bad_config(
                    at(""),
                    "`driver: command` needs a `cmd:`",
                    "add the command that starts this service",
                )
            })?;
            Ok(ResolvedRun::Command {
                cmd,
                stop: run.stop,
                shell: run.shell.unwrap_or(false),
                common,
            })
        }
        other => Err(CharError::bad_config(
            at(".driver"),
            format!("unknown driver `{other}`"),
            "use `compose` or `command` — there are deliberately no vendor-named drivers",
        )),
    }
}

fn resolve_ready(
    component: &str,
    ready: Option<Ready>,
    defaults: &Defaults,
    file: &str,
) -> Result<ResolvedReady, CharError> {
    // An omitted `ready:` is `{none: true}`, and `char up` then reports UP on
    // spawn — which is why a service with a real health endpoint declares one.
    let Some(ready) = ready else {
        return Ok(ResolvedReady {
            kind: ReadyKind::None,
            timeout: defaults.ready_timeout,
        });
    };

    let mut kinds: Vec<ReadyKind> = Vec::new();
    if let Some(url) = ready.http {
        kinds.push(ReadyKind::Http(url));
    }
    if let Some(port) = ready.tcp {
        kinds.push(ReadyKind::Tcp(port));
    }
    if let Some(pattern) = ready.log {
        kinds.push(ReadyKind::Log(pattern));
    }
    if let Some(cmd) = ready.exec {
        kinds.push(ReadyKind::Exec(cmd));
    }
    if ready.none == Some(true) {
        kinds.push(ReadyKind::None);
    }

    let at = ConfigWhere::Path {
        file: file.to_string(),
        path: format!("{SECTION}.components.{component}.run.ready"),
    };

    // Exactly one kind key. Two is bad_config rather than a precedence rule
    // nobody would remember (PLAN.md §6.0).
    let mut kinds = kinds.into_iter();
    let Some(kind) = kinds.next() else {
        return Err(CharError::bad_config(
            at,
            "`ready:` names no kind",
            "give exactly one of `http:`, `tcp:`, `log:`, `exec:` or `none: true`",
        ));
    };
    if kinds.next().is_some() {
        return Err(CharError::bad_config(
            at,
            "`ready:` names more than one kind",
            "give exactly one of `http:`, `tcp:`, `log:`, `exec:` or `none: true`",
        ));
    }

    Ok(ResolvedReady {
        kind,
        timeout: ready.timeout.unwrap_or(defaults.ready_timeout),
    })
}

fn resolve_command(
    name: &str,
    entry: CommandEntry,
    file: &str,
) -> Result<ResolvedCommand, CharError> {
    // The default is inferred — `pipe` when the entry grants secrets, so char
    // can scrub what it writes; `inherit` otherwise, so colours and prompts
    // work. Inference is wrong in both directions, which is why the key exists
    // and why an explicit value always wins (PLAN.md §4.5).
    let stdio = match entry.stdio.as_deref() {
        Some("inherit") => Stdio::Inherit,
        Some("pipe") => Stdio::Pipe,
        None if entry.secrets.is_empty() => Stdio::Inherit,
        None => Stdio::Pipe,
        Some(other) => {
            return Err(CharError::bad_config(
                ConfigWhere::Path {
                    file: file.to_string(),
                    path: format!("{SECTION}.commands.{name}.stdio"),
                },
                format!("unknown stdio `{other}`"),
                "use `inherit` or `pipe`",
            ))
        }
    };

    let OwnsCommand {
        containers,
        networks,
        images,
        files: owns_files,
        release,
    } = entry.owns.unwrap_or_default();

    Ok(ResolvedCommand {
        cmd: entry.cmd,
        help: entry.help,
        shell: entry.shell.unwrap_or(false),
        stdio,
        env: entry.env,
        owns_containers: containers,
        owns_networks: networks,
        owns_images: images,
        owns_files,
        owns_release: release_list(release),
        secrets: entry.secrets,
    })
}

/// A component or a check id, told apart by the colon — which is the whole
/// reason `:` is reserved in the name grammar.
fn classify_need(entry: String) -> Need {
    if entry.contains(':') {
        Need::Check(entry)
    } else {
        Need::Component(entry)
    }
}

fn release_list(release: Option<OneOrMany<String>>) -> Vec<String> {
    release.map(OneOrMany::into_vec).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(yaml: &str) -> ResolvedConfig {
        let config = parse(yaml, "armada.yml").expect("parses");
        resolve(config, &Defaults::built_in(), "armada.yml").expect("resolves")
    }

    #[test]
    fn check_ids_are_derived_not_written() {
        let cfg = resolved("manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: ruff check ${files}\n");
        assert_eq!(cfg.components["api"].checks["lint"].id, "api:lint");
    }

    #[test]
    fn defaults_are_materialised_from_the_machine_values_passed_in() {
        let cfg = resolved("manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: ruff check ${files}\n");
        let check = &cfg.components["api"].checks["lint"];
        assert_eq!(check.timeout, 900);
        assert_eq!(check.cost, 1);
        assert_eq!(check.scope, Scope::File);
        assert!(!check.shell);
    }

    #[test]
    fn a_machine_with_a_different_check_timeout_resolves_differently() {
        let config = parse(
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: ruff check ${files}\n",
            "armada.yml",
        )
        .unwrap();
        let defaults = Defaults {
            check_timeout: 60,
            ready_timeout: 5,
        };
        let cfg = resolve(config, &defaults, "armada.yml").unwrap();
        assert_eq!(cfg.components["api"].checks["lint"].timeout, 60);
    }

    #[test]
    fn match_defaults_to_the_component_root_or_the_whole_workspace() {
        let cfg = resolved(
            "manifest:\n  version: 1\n  components:\n    api:\n      root: services/api\n      checks:\n        lint: { cmd: lint }\n    whole:\n      checks:\n        lint: { cmd: lint }\n    service:\n      run: { driver: command, cmd: serve }\n",
        );
        assert_eq!(cfg.components["api"].match_globs, ["services/api/**"]);
        assert_eq!(cfg.components["whole"].match_globs, ["**"]);
        // Nothing to scope, so no glob — and in particular no `**` reaching
        // into a declared nested workspace.
        assert!(cfg.components["service"].match_globs.is_empty());
    }

    #[test]
    fn a_declared_workspace_does_not_change_the_default() {
        // `**` is everything in *this* workspace, and a declared workspace is
        // not in it (PLAN.md §4.6) — so the default is the same glob whether or
        // not the config declares one, and a checked component still needs no
        // scoping keys.
        let cfg = resolved(
            "manifest:\n  version: 1\n  workspaces: [apps/site]\n  components:\n    app:\n      checks:\n        lint: { cmd: eslint }\n    core:\n      root: packages/core\n      checks:\n        lint: { cmd: eslint }\n    svc:\n      run: { driver: command, cmd: serve }\n    docs:\n      match: [\"*.md\"]\n      checks:\n        lint: { cmd: markdownlint }\n",
        );
        assert_eq!(cfg.components["app"].match_globs, ["**"]);
        assert_eq!(cfg.components["core"].match_globs, ["packages/core/**"]);
        assert!(cfg.components["svc"].match_globs.is_empty());
        assert_eq!(cfg.components["docs"].match_globs, ["*.md"]);
    }

    #[test]
    fn setup_normalises_both_spellings_to_one_list() {
        let one =
            resolved("manifest:\n  version: 1\n  components:\n    api:\n      setup: uv sync\n");
        assert_eq!(one.components["api"].setup.len(), 1);
        assert!(!one.components["api"].setup[0].shell);

        let many = resolved(
            "manifest:\n  version: 1\n  components:\n    api:\n      setup:\n        - uv sync\n        - { cmd: \"createdb x || true\", shell: true }\n",
        );
        let steps = &many.components["api"].setup;
        assert_eq!(steps.len(), 2);
        assert!(!steps[0].shell);
        assert!(steps[1].shell);
    }

    #[test]
    fn needs_are_classified_by_the_colon() {
        let cfg = resolved(
            "manifest:\n  version: 1\n  components:\n    ui:\n      checks:\n        types:\n          cmd: tsc\n          needs: [postgres, core:build]\n",
        );
        assert_eq!(
            cfg.components["ui"].checks["types"].needs,
            [
                Need::Component("postgres".into()),
                Need::Check("core:build".into())
            ]
        );
    }

    #[test]
    fn exclusives_are_sorted_because_acquisition_order_is_the_deadlock_proof() {
        let cfg = resolved(
            "manifest:\n  version: 1\n  components:\n    web:\n      checks:\n        e2e:\n          cmd: e2e\n          scope: component\n          exclusive: [gpu, browser]\n",
        );
        assert_eq!(
            cfg.components["web"].checks["e2e"].exclusive,
            ["browser", "gpu"]
        );
    }

    #[test]
    fn an_omitted_ready_is_none_with_the_default_timeout() {
        let cfg = resolved(
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n",
        );
        let ResolvedRun::Command { common, .. } = &cfg.components["api"].run.as_ref().unwrap()
        else {
            panic!("expected a command driver");
        };
        assert_eq!(common.ready.kind, ReadyKind::None);
        assert_eq!(common.ready.timeout, 60);
    }

    #[test]
    fn stdio_is_inferred_from_the_grant_and_overridden_when_stated() {
        let cfg = resolved(
            "manifest:\n  version: 1\n  secrets:\n    T: op://a/b\n  secret_providers:\n    op: { cmd: \"op read ${ref}\" }\n  commands:\n    a: { cmd: ./a }\n    b: { cmd: ./b, secrets: [T] }\n    c: { cmd: ./c, secrets: [T], stdio: inherit }\n",
        );
        assert_eq!(cfg.commands["a"].stdio, Stdio::Inherit);
        assert_eq!(cfg.commands["b"].stdio, Stdio::Pipe);
        assert_eq!(cfg.commands["c"].stdio, Stdio::Inherit);
    }

    #[test]
    fn a_version_other_than_one_is_bad_config_at_the_version_key() {
        let err = parse("manifest:\n  version: 2\n", "armada.yml").unwrap_err();
        assert_eq!(err.r#where, "armada.yml:manifest.version");
        assert!(err.next_action.is_some());
    }

    #[test]
    fn a_syntax_error_reports_line_and_column_in_the_same_grammar() {
        let err = parse("manifest:\n  version: 1\n  components: {\n", "armada.yml").unwrap_err();
        assert!(
            err.r#where.starts_with("armada.yml:"),
            "where was {}",
            err.r#where
        );
        // `armada.yml:<line>:<column>` — two trailing numeric segments.
        let locator = err.r#where.strip_prefix("armada.yml:").unwrap();
        let parts: Vec<&str> = locator.split(':').collect();
        assert_eq!(parts.len(), 2, "locator was {locator}");
        assert!(parts.iter().all(|p| p.parse::<usize>().is_ok()));
    }

    #[test]
    fn a_driver_with_the_wrong_keys_beside_it_names_the_key() {
        let err = parse(
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: compose\n        file: [docker-compose.yml]\n        cmd: serve\n",
            "armada.yml",
        )
        .and_then(|c| resolve(c, &Defaults::built_in(), "armada.yml"))
        .unwrap_err();
        assert_eq!(err.r#where, "armada.yml:manifest.components.api.run.cmd");
    }

    #[test]
    fn two_ready_kinds_is_bad_config_rather_than_a_precedence_rule() {
        let err = parse(
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        ready: { http: \"http://127.0.0.1/health\", none: true }\n",
            "armada.yml",
        )
        .and_then(|c| resolve(c, &Defaults::built_in(), "armada.yml"))
        .unwrap_err();
        assert_eq!(err.r#where, "armada.yml:manifest.components.api.run.ready");
    }

    #[test]
    fn an_unknown_key_is_a_parse_error_not_a_silently_ignored_line() {
        let err = parse("manifest:\n  version: 1\n  componentz: {}\n", "armada.yml").unwrap_err();
        assert!(err.message.contains("unknown field"), "{}", err.message);
    }
}
