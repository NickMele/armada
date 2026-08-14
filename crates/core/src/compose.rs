//! The compose driver's pure half: **resolve → transform** (PLAN.md §6.0).
//!
//! ```text
//! 1. RESOLVE   docker compose -f <base…> -p armada-<id> \
//!                  --project-directory <workspace-root> config
//! 2. TRANSFORM ports[].published      → the claimed block
//!              labels.armada.*        → every service
//!              build.labels.armada.*  → services that build
//!              networks.<n>.labels    TOP-LEVEL, not inherited
//!              volumes.<n>.labels     TOP-LEVEL, not inherited
//! 3. HOLD      in memory — never written to disk
//! 4. RUN       <document on stdin> | docker compose -f - -p armada-<id> …
//! ```
//!
//! Step 2 is here. Steps 1 and 4 are argv, which is also here, because argv is
//! the thing a test can assert and the thing that is wrong when ownership
//! breaks (`ARCHITECTURE.md` §1.1).
//!
//! **Armada never parses compose semantics.** Step 1 hands that entire problem
//! to compose: `extends:`, YAML anchors and `${VAR}` interpolation are resolved
//! before Armada sees the document, which is why this works on any version and
//! why none of them appear below.
//!
//! **Why a whole document rather than an override.** Measured: an override
//! *appends* to `ports:` rather than replacing, so the base port stays published
//! and every workspace still binds it — the exact collision this project exists
//! to prevent. The `!override` tag fixes that on Compose ≥ 2.24.4 and is
//! **silently ignored below it**, and a merge feature that fails silently in the
//! older direction is not something to build on when a repo's developers are, as
//! usual, on different versions (`docs/traps.md`).
//!
//! **Stamping services is not stamping the stack.** Measured: compose does not
//! propagate a service's labels to the network or the volumes it creates, so a
//! `clean` that finds resources by label finds the containers and leaves the
//! network and the volumes behind — with no verb that can ever locate them
//! again, which is the founding bug of this project reintroduced. The top-level
//! blocks are stamped separately, and [`transform`] creates the `default`
//! network entry when the document has none.
//!
//! **The document is never written to disk**, and nothing here returns a path.
//! Measured: `docker compose config` resolves `env_file:` and `${VAR}` inline,
//! so persisting it manufactures a cleartext credentials file for every repo —
//! including repos that never adopt Armada's secrets mechanism, whose values
//! have therefore never passed through the scrubber (`ARCHITECTURE.md` §1.8).

use crate::config::{ResolvedConfig, ResolvedRun};
use crate::error::{ArmadaError, ConfigWhere, ErrClass};
use serde_yaml_ng::{Mapping, Value};
use std::collections::BTreeMap;

/// The compose project name Armada uses, and the container-name prefix that
/// falls out of it: `armada-<id>`.
///
/// Compose applies `com.docker.compose.project=<name>` to containers and
/// networks automatically, which is ownership Armada gets for free — but
/// `clean` filters on the `armada.*` labels rather than on this, so it stays
/// driver-agnostic.
pub fn project(workspace_id: &str) -> String {
    format!("armada-{workspace_id}")
}

/// Step 1: resolve the repo's own files into one canonical document.
///
/// **`-p` is passed here and not only on the run step.** Measured: `config`
/// bakes the project name into `networks.default.name`, deriving it from the
/// *directory* when none is given — so a document resolved without it names its
/// networks after wherever the resolve happened to run.
///
/// **`COMPOSE_FILE` and `COMPOSE_PROJECT_NAME` are ignored**, because `-f` and
/// `-p` are passed explicitly every time: the result must not depend on the
/// caller's environment.
pub fn resolve_argv(files: &[String], project: &str, project_directory: &str) -> Vec<String> {
    let mut argv = vec!["docker".to_string(), "compose".to_string()];
    for file in files {
        argv.push("-f".to_string());
        argv.push(file.clone());
    }
    argv.push("-p".to_string());
    argv.push(project.to_string());
    argv.push("--project-directory".to_string());
    argv.push(project_directory.to_string());
    argv.push("config".to_string());
    argv
}

/// Step 4: start the stack from the transformed document, on stdin.
///
/// `-f -` is verified to accept a document on stdin and produce identical
/// resolved output, which is what makes step 3's "never written to disk" free
/// rather than a compromise.
pub fn up_argv(project: &str, project_directory: &str) -> Vec<String> {
    stdin_argv(project, project_directory, &["up", "-d"])
}

/// `down`, from the same document. **The volumes stay**: `down` is pause, and a
/// named volume is the workspace's data — `clean` releases it, `down` does not.
pub fn down_argv(project: &str, project_directory: &str) -> Vec<String> {
    stdin_argv(project, project_directory, &["down"])
}

fn stdin_argv(project: &str, project_directory: &str, verb: &[&str]) -> Vec<String> {
    let mut argv = vec![
        "docker".to_string(),
        "compose".to_string(),
        "-f".to_string(),
        "-".to_string(),
        "-p".to_string(),
        project.to_string(),
        "--project-directory".to_string(),
        project_directory.to_string(),
    ];
    argv.extend(verb.iter().map(|word| (*word).to_string()));
    argv
}

/// Container port → the declared port *name*, over the compose services only.
///
/// **Compose services only, and that is the whole reason this is a function.**
/// A `command` component may legitimately declare `{ app: 3000 }` while a
/// compose service targets 3000 for something unrelated; a map built over every
/// component would rewrite the compose one to the `app` port. Port *names* are
/// workspace-global (PLAN.md §4.4) and the assignment is by name, so the only
/// thing that has to be unambiguous here is the container port.
pub fn port_names(
    config: &ResolvedConfig,
    config_label: &str,
) -> Result<BTreeMap<u16, String>, ArmadaError> {
    let mut map: BTreeMap<u16, String> = BTreeMap::new();
    for (component, resolved) in &config.components {
        let Some(ResolvedRun::Compose { common, .. }) = &resolved.run else {
            continue;
        };
        for (name, container_port) in &common.ports {
            if let Some(existing) = map.get(container_port) {
                if existing != name {
                    return Err(ArmadaError::bad_config(
                        ConfigWhere::Path {
                            file: config_label.to_string(),
                            path: format!("components.{component}.run.ports.{name}"),
                        },
                        format!(
                            "two compose services both listen on {container_port} \
                             (`{existing}` and `{name}`), so Armada cannot tell which \
                             claimed port each published entry should get"
                        ),
                        "give the two services different container ports, \
                         or declare them in one component",
                    ));
                }
            }
            map.insert(*container_port, name.clone());
        }
    }
    Ok(map)
}

/// Step 2, over the document compose has already normalised.
///
/// `names` maps a container port to a declared port name ([`port_names`]);
/// `assigned` maps that name to the port claimed for this workspace.
///
/// **A published port Armada was never told about is `bad_config`.** Leaving it
/// alone is the one thing this function may not do: an unrewritten `published`
/// is the base port, every workspace binds it, and concurrent workspaces
/// collide — which is the exact failure §6.0 exists to prevent, arriving
/// silently. Declaring it under `ports:` is one line, and the message says so.
pub fn transform(
    document: &str,
    names: &BTreeMap<u16, String>,
    assigned: &BTreeMap<String, u16>,
    labels: &BTreeMap<String, String>,
    config_label: &str,
) -> Result<String, ArmadaError> {
    let mut root: Value = serde_yaml_ng::from_str(document).map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "docker compose config".to_string(),
        message: format!("compose resolved a document Armada cannot read: {e}"),
        next_action: None,
    })?;
    let root = root.as_mapping_mut().ok_or_else(|| ArmadaError {
        class: ErrClass::Environment,
        r#where: "docker compose config".to_string(),
        message: "compose resolved something that is not a mapping".to_string(),
        next_action: None,
    })?;

    if let Some(services) = root.get_mut("services").and_then(Value::as_mapping_mut) {
        for (name, service) in services.iter_mut() {
            let service_name = name.as_str().unwrap_or("?").to_string();
            let Some(service) = service.as_mapping_mut() else {
                continue;
            };
            republish(service, &service_name, names, assigned, config_label)?;
            stamp(service, labels);
            // **`build.labels`, and only for services that build.** A pulled
            // image such as `postgres:16` is shared with the rest of the
            // machine and was never Armada's to remove; an image Armada caused
            // to be built is ~2.1 GB per stale workspace and is the single
            // biggest thing one holds.
            if let Some(build) = service.get_mut("build").and_then(Value::as_mapping_mut) {
                stamp(build, labels);
            }
        }
    }

    // **Top-level, not inherited — this is the founding bug.** A `clean` that
    // finds resources by label finds the containers and leaves the network and
    // the volumes behind, with no verb that can ever locate them again.
    for block in ["networks", "volumes"] {
        let Some(entries) = root.get_mut(block).and_then(Value::as_mapping_mut) else {
            continue;
        };
        for (_, entry) in entries.iter_mut() {
            // `volumes: { pgdata: }` is legal and resolves to null, which has
            // nowhere to hang a label until it is a mapping.
            if entry.is_null() {
                *entry = Value::Mapping(Mapping::new());
            }
            if let Some(entry) = entry.as_mapping_mut() {
                stamp(entry, labels);
            }
        }
    }

    // The default network exists whether or not the document mentions it, and
    // an unmentioned one is exactly the network that was leaking.
    if !root.contains_key("networks") {
        let mut default = Mapping::new();
        let mut inner = Mapping::new();
        stamp(&mut inner, labels);
        default.insert(Value::from("default"), Value::Mapping(inner));
        root.insert(Value::from("networks"), Value::Mapping(default));
    }

    serde_yaml_ng::to_string(&Value::Mapping(root.clone())).map_err(|e| ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: "compose".to_string(),
        message: format!("the transformed document would not serialize: {e}"),
        next_action: None,
    })
}

/// Add Armada's three labels to whatever holds a `labels:` block.
///
/// Merged rather than replaced: a repo's own labels are its own, and measured,
/// override merging works for `labels:` — it was `ports:` that did not.
fn stamp(holder: &mut Mapping, labels: &BTreeMap<String, String>) {
    let existing = holder
        .remove("labels")
        .and_then(|value| as_labels(&value))
        .unwrap_or_default();

    let mut merged = Mapping::new();
    for (key, value) in existing {
        merged.insert(Value::from(key), Value::from(value));
    }
    for (key, value) in labels {
        merged.insert(Value::from(key.clone()), Value::from(value.clone()));
    }
    holder.insert(Value::from("labels"), Value::Mapping(merged));
}

/// A `labels:` block in either of the two spellings compose accepts.
fn as_labels(value: &Value) -> Option<Vec<(String, String)>> {
    match value {
        Value::Mapping(map) => Some(
            map.iter()
                .filter_map(|(key, value)| Some((key.as_str()?.to_string(), scalar_text(value)?)))
                .collect(),
        ),
        // The list spelling: `labels: ["a=b"]`.
        Value::Sequence(items) => Some(
            items
                .iter()
                .filter_map(|item| {
                    let text = item.as_str()?;
                    let (key, value) = text.split_once('=')?;
                    Some((key.to_string(), value.to_string()))
                })
                .collect(),
        ),
        _ => None,
    }
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// Rewrite one service's published ports into the claimed block.
fn republish(
    service: &mut Mapping,
    service_name: &str,
    names: &BTreeMap<u16, String>,
    assigned: &BTreeMap<String, u16>,
    config_label: &str,
) -> Result<(), ArmadaError> {
    let Some(ports) = service.get_mut("ports").and_then(Value::as_sequence_mut) else {
        return Ok(());
    };

    for entry in ports.iter_mut() {
        // **Every entry under `ports:` publishes, so every entry is rewritten
        // or refused.** An entry Armada cannot read is refused rather than
        // skipped, for the same reason an undeclared one is: skipping it leaves
        // compose to publish it, and a port Armada did not place is a port
        // outside the claimed block.
        let target = target_of(entry).ok_or_else(|| {
            ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: config_label.to_string(),
                    path: format!("components.*.run.ports (service `{service_name}`)"),
                },
                format!(
                    "Armada cannot read a `ports:` entry on the compose service \
                     `{service_name}`, so it cannot move it into the claimed block"
                ),
                "write the entry as `\"<container-port>\"` or `\"<host>:<container>\"`",
            )
        })?;
        let name = names.get(&target).ok_or_else(|| {
            ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: config_label.to_string(),
                    path: format!("components.*.run.ports (service `{service_name}`)"),
                },
                format!(
                    "the compose service `{service_name}` publishes container port \
                     {target}, which no component declares under `ports:` — so every \
                     workspace would bind the same host port"
                ),
                format!("add `ports: {{ <name>: {target} }}` to the component that runs `{service_name}`"),
            )
        })?;
        let port = assigned.get(name).ok_or_else(|| ArmadaError {
            class: ErrClass::ArmadaBug,
            r#where: "ports".to_string(),
            message: format!("`{name}` is declared and was never assigned a port"),
            next_action: None,
        })?;
        set_published(entry, *port);
    }
    Ok(())
}

/// The container port an entry publishes, or `None` when Armada cannot read it.
///
/// **Every entry under `ports:` publishes something — `published` being absent
/// means *ephemeral*, not *none*.** Measured on darwin against Docker 29.6.2 and
/// Compose v5.3.1: `ports: ["6379"]` resolves to `{mode: ingress, target: 6379,
/// protocol: tcp}` with no `published` key, and the container comes up on a
/// random host port (`docs/traps.md`). The key that exposes without publishing
/// is `expose:`, which is a different key and never reaches here.
///
/// An earlier version required `published` to be present and skipped the entry
/// when it was not, on the reasoning that a bare port "exposes a container port
/// without binding a host one". That was wrong, and it was wrong in the
/// direction this whole module exists to prevent: the claimed block was
/// silently bypassed, the service came up on an ephemeral port, and a `tcp:`
/// ready-check waited on the claimed one until it timed out.
fn target_of(entry: &Value) -> Option<u16> {
    match entry {
        // The long form `docker compose config` emits. `target` is the
        // container port whether or not `published` is there.
        Value::Mapping(map) => map.get("target").and_then(port_of),
        // `"5460:5432"`, `"127.0.0.1:5460:5432"`, or a bare `"5432"` — the
        // container port is last in every one of them.
        Value::String(text) => {
            let container = text.rsplit(':').next()?;
            container.split('/').next()?.parse().ok()
        }
        // `ports: [5432]`, which YAML reads as an integer.
        Value::Number(number) => number.as_u64().and_then(|n| u16::try_from(n).ok()),
        _ => None,
    }
}

fn port_of(value: &Value) -> Option<u16> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|n| u16::try_from(n).ok()),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

/// Write the claimed port back, keeping whatever shape the entry arrived in.
///
/// **Inserting `published` where there was none is the fix for the ephemeral
/// case**, and it is the same operation as replacing one that was there: the
/// entry either names its host port or gets one at random, and Armada's job is
/// to make sure it is the claimed one either way.
fn set_published(entry: &mut Value, port: u16) {
    match entry {
        Value::Mapping(map) => {
            // A string, because that is what `docker compose config` emits and
            // the document goes straight back to compose.
            map.insert(Value::from("published"), Value::from(port.to_string()));
        }
        Value::String(text) => {
            let container = text.rsplit(':').next().unwrap_or_default().to_string();
            *text = format!("{port}:{container}");
        }
        // An integer entry, `ports: [5432]`, becomes the string form: there is
        // nowhere in a scalar to put a second number.
        Value::Number(number) => {
            let container = number.as_u64().unwrap_or_default();
            *entry = Value::from(format!("{port}:{container}"));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, Defaults};

    fn labels() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("armada.workspace".to_string(), "a3f91c02".to_string()),
            ("armada.workspace_path".to_string(), "/srv/repo".to_string()),
            ("armada.namespace".to_string(), "ns-1".to_string()),
        ])
    }

    fn names() -> BTreeMap<u16, String> {
        BTreeMap::from([(5432, "pg".to_string()), (6379, "redis".to_string())])
    }

    fn assigned() -> BTreeMap<String, u16> {
        BTreeMap::from([("pg".to_string(), 5460), ("redis".to_string(), 5461)])
    }

    fn run(document: &str) -> Value {
        let text = transform(document, &names(), &assigned(), &labels(), "armada.yml")
            .expect("the document transforms");
        serde_yaml_ng::from_str(&text).expect("it is still YAML")
    }

    /// The document `docker compose config` actually emits.
    const RESOLVED: &str = "\
name: armada-a3f91c02
services:
  postgres:
    image: postgres:16
    ports:
    - mode: ingress
      target: 5432
      published: \"5432\"
      protocol: tcp
networks:
  default:
    name: armada-a3f91c02_default
volumes:
  pgdata:
    name: armada-a3f91c02_pgdata
";

    /// **The whole reason a document is generated rather than an override.** An
    /// override appends to `ports:` and the base port stays published, so every
    /// workspace still binds it.
    #[test]
    fn a_published_port_is_rewritten_into_the_claimed_block() {
        let doc = run(RESOLVED);
        let port = &doc["services"]["postgres"]["ports"][0];
        assert_eq!(port["published"].as_str(), Some("5460"));
        assert_eq!(
            port["target"].as_u64(),
            Some(5432),
            "the container port is untouched"
        );
    }

    /// Measured: compose does not propagate a service's labels to the network or
    /// the volumes, so a `clean` that finds by label finds the containers and
    /// leaves the rest behind — the founding bug of this project.
    #[test]
    fn the_network_and_the_volume_are_stamped_separately_from_the_service() {
        let doc = run(RESOLVED);
        for path in [
            &doc["services"]["postgres"]["labels"],
            &doc["networks"]["default"]["labels"],
            &doc["volumes"]["pgdata"]["labels"],
        ] {
            assert_eq!(
                path["armada.workspace"].as_str(),
                Some("a3f91c02"),
                "{path:?}"
            );
            assert_eq!(
                path["armada.workspace_path"].as_str(),
                Some("/srv/repo"),
                "the id is a one-way hash; without the path a reap cannot stat it"
            );
            assert_eq!(path["armada.namespace"].as_str(), Some("ns-1"), "{path:?}");
        }
    }

    /// A network Armada never stamped is a network no verb can find again, and
    /// compose creates `default` whether or not the document names it.
    #[test]
    fn a_document_with_no_networks_block_gains_a_stamped_default() {
        let doc = run("services:\n  api:\n    image: alpine\n");
        assert_eq!(
            doc["networks"]["default"]["labels"]["armada.workspace"].as_str(),
            Some("a3f91c02")
        );
    }

    /// Only images Armada causes to be **built**. A pulled `postgres:16` is
    /// shared with everything else on the machine and was never Armada's.
    #[test]
    fn only_a_service_that_builds_gets_its_image_stamped() {
        let doc = run(
            "services:\n  api:\n    build:\n      context: /srv/repo\n  db:\n    image: postgres:16\n",
        );
        assert_eq!(
            doc["services"]["api"]["build"]["labels"]["armada.workspace"].as_str(),
            Some("a3f91c02")
        );
        assert!(
            doc["services"]["db"].get("build").is_none(),
            "a pulled image gained a build block"
        );
    }

    /// **Merged, not replaced.** A repo's own labels are its own, and measured,
    /// override merging works for `labels:` — it was `ports:` that did not.
    #[test]
    fn a_repos_own_labels_survive_the_stamp_in_either_spelling() {
        for block in [
            "    labels:\n      com.example.team: platform\n",
            "    labels: [\"com.example.team=platform\"]\n",
        ] {
            let doc = run(&format!("services:\n  api:\n    image: alpine\n{block}"));
            let labels = &doc["services"]["api"]["labels"];
            assert_eq!(labels["com.example.team"].as_str(), Some("platform"));
            assert_eq!(labels["armada.workspace"].as_str(), Some("a3f91c02"));
        }
    }

    /// **A published port nothing declared is refused rather than left alone.**
    /// Leaving it is the base port, every workspace binds it, and the collision
    /// arrives silently — which is the one failure §6.0 exists to prevent.
    #[test]
    fn an_undeclared_published_port_is_bad_config_and_says_what_to_add() {
        let error = transform(
            "services:\n  mailhog:\n    image: mailhog/mailhog\n    ports:\n    - \"8025:8025\"\n",
            &names(),
            &assigned(),
            &labels(),
            "armada.yml",
        )
        .unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.message.contains("8025"), "{}", error.message);
        assert!(error.message.contains("mailhog"), "{}", error.message);
        assert!(error.next_action.unwrap().contains("ports:"));
    }

    /// **A bare `ports:` entry publishes on an *ephemeral* host port, and this
    /// is the assertion that was backwards.**
    ///
    /// Measured against Docker 29.6.2 and Compose v5.3.1: `ports: ["6379"]`
    /// resolves to `{mode: ingress, target: 6379}` with no `published` key, and
    /// the container comes up on a random host port — 55918 in the run that
    /// established it (`docs/traps.md`). The first version of this test asserted
    /// the entry was *left alone*, on the reasoning that it "exposes a container
    /// port without binding a host one". That is `expose:`, which is a different
    /// key. The consequence was the exact failure the transform exists to
    /// prevent, arriving silently: the claimed block was bypassed, the service
    /// came up somewhere else, and a `tcp:` ready-check waited on a port nothing
    /// was ever going to bind.
    #[test]
    fn a_bare_port_is_published_into_the_block_rather_than_left_ephemeral() {
        // The long form `config` emits for a bare entry.
        let doc = run("services:\n  db:\n    image: postgres:16\n    ports:\n    \
             - mode: ingress\n      target: 5432\n      protocol: tcp\n");
        assert_eq!(
            doc["services"]["db"]["ports"][0]["published"].as_str(),
            Some("5460"),
            "a bare entry kept its ephemeral publish"
        );

        // `host_ip` and no published, which `"127.0.0.1::5432"` resolves to.
        let doc = run("services:\n  db:\n    image: postgres:16\n    ports:\n    \
             - mode: ingress\n      host_ip: 127.0.0.1\n      target: 5432\n");
        let port = &doc["services"]["db"]["ports"][0];
        assert_eq!(port["published"].as_str(), Some("5460"));
        assert_eq!(
            port["host_ip"].as_str(),
            Some("127.0.0.1"),
            "the declared interface was dropped"
        );

        // The short spellings, for a document that did not come through
        // `config`: a bare string, and a bare integer.
        let doc = run("services:\n  db:\n    image: postgres:16\n    ports:\n    - \"5432\"\n");
        assert_eq!(
            doc["services"]["db"]["ports"][0].as_str(),
            Some("5460:5432")
        );

        let doc = run("services:\n  db:\n    image: postgres:16\n    ports:\n    - 5432\n");
        assert_eq!(
            doc["services"]["db"]["ports"][0].as_str(),
            Some("5460:5432")
        );
    }

    /// A bare entry naming a port nothing declares is refused, exactly as a
    /// published one is — it publishes too, so it collides too.
    #[test]
    fn a_bare_port_that_no_component_declares_is_bad_config() {
        let error = transform(
            "services:\n  mailhog:\n    image: mailhog/mailhog\n    ports:\n    \
             - mode: ingress\n      target: 8025\n",
            &names(),
            &assigned(),
            &labels(),
            "armada.yml",
        )
        .unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.message.contains("8025"), "{}", error.message);
    }

    /// **An entry Armada cannot read is refused, not skipped.** Skipping leaves
    /// compose to publish it, and a port Armada did not place is a port outside
    /// the claimed block — which is the whole failure.
    #[test]
    fn an_unreadable_ports_entry_is_refused_rather_than_left_to_compose() {
        let error = transform(
            "services:\n  db:\n    image: postgres:16\n    ports:\n    - [5432]\n",
            &names(),
            &assigned(),
            &labels(),
            "armada.yml",
        )
        .unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.message.contains("cannot read"), "{}", error.message);
    }

    /// The short spelling keeps its shape, so a document that did not come
    /// through `config` is still handed back as compose gave it.
    #[test]
    fn the_short_spelling_is_rewritten_in_place() {
        let doc =
            run("services:\n  db:\n    image: postgres:16\n    ports:\n    - \"5432:5432\"\n");
        assert_eq!(
            doc["services"]["db"]["ports"][0].as_str(),
            Some("5460:5432")
        );
    }

    /// **`-p` on the resolve step and not only on the run step.** Measured:
    /// `config` bakes the project name into `networks.default.name`, derived
    /// from the *directory* when none is given.
    #[test]
    fn the_project_name_is_passed_to_the_resolve_as_well_as_the_run() {
        let argv = resolve_argv(
            &["docker-compose.yml".to_string(), "override.yml".to_string()],
            &project("a3f91c02"),
            "/srv/repo",
        );
        assert_eq!(
            argv,
            vec![
                "docker",
                "compose",
                "-f",
                "docker-compose.yml",
                "-f",
                "override.yml",
                "-p",
                "armada-a3f91c02",
                "--project-directory",
                "/srv/repo",
                "config",
            ]
        );
    }

    /// The document arrives on stdin and is never written to disk: measured,
    /// `config` inlines `env_file:` and `${VAR}` values, so a persisted copy is
    /// a cleartext credentials file for every repo.
    #[test]
    fn the_run_step_reads_the_document_from_stdin() {
        let argv = up_argv(&project("a3f91c02"), "/srv/repo");
        assert_eq!(argv[2..4], ["-f".to_string(), "-".to_string()]);
        assert!(argv.ends_with(&["up".to_string(), "-d".to_string()]));
        assert!(
            !argv.iter().any(|a| a.contains(".armada")),
            "a path into the workspace reached the argv: {argv:?}"
        );
    }

    /// `down` is pause. A named volume outlives it by design — `clean` is what
    /// releases one.
    #[test]
    fn down_does_not_remove_the_volumes() {
        let argv = down_argv(&project("a3f91c02"), "/srv/repo");
        assert!(argv.ends_with(&["down".to_string()]));
        assert!(
            !argv.iter().any(|a| a == "-v" || a == "--volumes"),
            "{argv:?}"
        );
    }

    fn config_of(yaml: &str) -> ResolvedConfig {
        let parsed = config::parse(yaml, "armada.yml").unwrap();
        config::resolve(parsed, &Defaults::built_in(), "armada.yml").unwrap()
    }

    /// **Compose services only.** A `command` component declaring `{ app: 3000 }`
    /// must not claim a compose service's 3000.
    #[test]
    fn the_port_map_covers_the_compose_services_and_nothing_else() {
        let config = config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   db:\n      run:\n        driver: compose\n        file: [c.yml]\n        ports: { pg: 5432 }\n\
             \x20   api:\n      run:\n        driver: command\n        cmd: serve\n        ports: { app: 3000 }\n",
        );
        assert_eq!(
            port_names(&config, "armada.yml").unwrap(),
            BTreeMap::from([(5432, "pg".to_string())])
        );
    }

    #[test]
    fn two_compose_services_on_one_container_port_is_bad_config() {
        let config = config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   one:\n      run:\n        driver: compose\n        file: [c.yml]\n        ports: { a: 5432 }\n\
             \x20   two:\n      run:\n        driver: compose\n        file: [c.yml]\n        ports: { b: 5432 }\n",
        );
        let error = port_names(&config, "armada.yml").unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.message.contains("5432"), "{}", error.message);
    }
}
