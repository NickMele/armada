//! `armada settings` — every setting Armada knows about: its current value,
//! where it lives, and which side of `PLAN.md` §13.1's line it is on.
//!
//! **Read-only, and deliberately so** (`docs/reserved/018-a-place-for-settings.md`).
//! A generic writer is where the type and validation questions live — per-key
//! validation, and an explicit sync-versus-machine decision for every key —
//! and none of that exists yet. What does exist today are purpose-named verbs
//! that already change one setting each — `armada helm enable`/`disable`, the
//! guild's own verbs — and they stay exactly as they are; this command only
//! shows what they left behind.
//!
//! # Three readers, reused rather than retyped
//!
//! - **Manifest's section** of `machine.yml`, via
//!   [`armada_manifest::machine::MachineConfig::read`]. Every field is
//!   destructured by name below, so a ninth key added to that struct and not
//!   added here is a compile error rather than a row silently missing.
//! - **Helm's section**, via [`crate::machine::read`] — the same rule and the
//!   same destructuring.
//! - **The guild's config-shaped items**, via
//!   [`armada_guild::inventory::Inventory::items`] — the same call `armada
//!   guild ls` makes. This does not open `settings.json` a second time: the
//!   count already computed for that listing is reused as this row's value.
//!
//! # The line, on every row
//!
//! `PLAN.md` §13.1: what describes *you* syncs; what describes *this machine
//! and its running processes* never does. That is [`Locality`], and it is the
//! STATUS word — the fact a reader most needs and the one most easily got
//! wrong, so it is not left to be inferred from the path in `at`.
//!
//! # A setting nobody touched still gets a row
//!
//! [`armada_manifest::machine::MachineConfig::read`] already merges a missing
//! `machine.yml` (or a missing key in one that exists) with the documented
//! defaults, so every one of Manifest's and Helm's keys is listed whether or
//! not anyone ever wrote it. "What can I configure" and "what have I
//! configured" are the same question here.

use std::path::Path;

use armada_core::envelope::{Envelope, Locality, SettingRow, SettingsData};
use armada_core::error::{ArmadaError, Status};
use armada_guild::inventory::{Inventory, Kind};

use crate::verbs::guild::Where;
use crate::verbs::machine::shown;
use crate::verbs::Output;

/// List every setting Armada knows about.
pub fn run(place: &Where) -> Result<Output, ArmadaError> {
    let mut settings = manifest_settings(&place.armada_home)?;
    settings.extend(helm_settings(&place.armada_home));
    settings.extend(guild_settings(place));

    Ok(Output::Settings(Box::new(Envelope::ok(
        "settings",
        None,
        Status::Ok,
        SettingsData { settings },
    ))))
}

/// Manifest's eight keys, read the one way this module is allowed to: through
/// the reader that already merges the file with its documented defaults
/// (`crates/manifest/src/machine.rs`).
fn manifest_settings(armada_home: &Path) -> Result<Vec<SettingRow>, ArmadaError> {
    let config = armada_manifest::machine::MachineConfig::read(armada_home)?;
    let at = shown(&armada_home.join("machine.yml"));

    // **Exhaustive destructuring, not a list of getters.** A ninth field added
    // to `MachineConfig` and not named here fails to compile, which is what
    // keeps this list derived rather than a second place it can go stale.
    let armada_manifest::machine::MachineConfig {
        cpu_slots,
        port_block_size,
        port_base,
        run_retention,
        check_timeout,
        acquire_timeout,
        docker_timeout,
        up_timeout,
    } = config;

    Ok(vec![
        machine_row("manifest.cpu_slots", cpu_slots.to_string(), &at),
        machine_row("manifest.port_block_size", port_block_size.to_string(), &at),
        machine_row("manifest.port_base", port_base.to_string(), &at),
        machine_row("manifest.run_retention", run_retention.to_string(), &at),
        machine_row("manifest.check_timeout", check_timeout.to_string(), &at),
        machine_row("manifest.acquire_timeout", acquire_timeout.to_string(), &at),
        machine_row("manifest.docker_timeout", docker_timeout.to_string(), &at),
        machine_row("manifest.up_timeout", up_timeout.to_string(), &at),
    ])
}

/// Helm's two keys, read the same way — [`crate::machine::read`] never errors;
/// a `machine.yml` it cannot parse reads as `enter: false` and the default
/// mode, which are the safe values to fail toward (see that module's own header
/// for why a second reader does not raise a second error about the same file).
///
/// **`helm.mode` is listed even though nothing writes it.** There is no
/// purpose-named verb for it the way `armada helm enable` is one for
/// `helm.enter`, so this listing is the only place a reader learns the key
/// exists and what it is currently set to — which is the whole argument of
/// `docs/reserved/018` for a read-only settings verb.
fn helm_settings(armada_home: &Path) -> Vec<SettingRow> {
    // Exhaustive destructuring for the same reason as above: a third field
    // added there without being named here would not compile.
    let crate::machine::HelmSection { enter, mode } = crate::machine::read(armada_home);
    let at = shown(&armada_home.join("machine.yml"));
    vec![
        machine_row(
            "helm.enter",
            if enter { "on" } else { "off" }.to_string(),
            &at,
        ),
        machine_row("helm.mode", mode, &at),
    ]
}

/// The guild's config-shaped items — `armada guild ls`'s own inventory, asked
/// for the rows that are settings rather than content.
///
/// **Content and configuration are different questions.** A skill or a
/// workflow is something you wrote; `settings.json`, `plugins.yml`, `mcp.yml`
/// and `permissions.yml` are switches. The distinction is made with an
/// exhaustive match rather than an allow-list, so a [`Kind`] added later has
/// to be placed on one side or the other to compile.
fn guild_settings(place: &Where) -> Vec<SettingRow> {
    let guild = place.guild();
    Inventory::items(guild.root())
        .into_iter()
        .filter(|item| is_setting(item.kind))
        .map(|item| SettingRow {
            locality: Locality::Synced,
            name: format!("guild.{}", item.name),
            // **Reused, not re-read.** `item.detail` is the same line `armada
            // guild ls` already computed for this file — a count of what is in
            // it, read once out of the file itself.
            value: item.detail,
            at: shown(&guild.root().join(&item.path)),
        })
        .collect()
}

/// Whether a guild item is a setting rather than content — exhaustive, so a
/// [`Kind`] this module has never heard of fails to compile rather than being
/// silently left off either side.
const fn is_setting(kind: Kind) -> bool {
    match kind {
        Kind::Settings | Kind::Plugins | Kind::Mcp | Kind::Permissions => true,
        Kind::Memory
        | Kind::Skill
        | Kind::Subagent
        | Kind::Workflow
        | Kind::Hook
        | Kind::Schema => false,
    }
}

fn machine_row(name: &str, value: String, at: &str) -> SettingRow {
    SettingRow {
        locality: Locality::Machine,
        name: name.to_string(),
        value,
        at: at.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::envelope::Locality;

    fn scratch() -> (tempfile::TempDir, Where) {
        let home = tempfile::tempdir().unwrap();
        let place = Where {
            armada_home: home.path().join(".armada"),
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        (home, place)
    }

    fn settings(output: &Output) -> Vec<SettingRow> {
        match output {
            Output::Settings(envelope) => envelope.data.settings.clone(),
            _ => panic!("not a settings envelope"),
        }
    }

    /// **The done-when.** A machine that has never written `machine.yml` and
    /// has no guild still lists every Manifest and Helm key, carrying its
    /// documented default — "what can I configure" is answered even when
    /// nobody has configured anything.
    #[test]
    fn a_fresh_machine_lists_every_machine_key_with_its_default() {
        let (_home, place) = scratch();
        let output = run(&place).unwrap();
        let rows = settings(&output);

        let defaults = armada_manifest::machine::MachineConfig::defaults();
        let cpu_slots = rows
            .iter()
            .find(|r| r.name == "manifest.cpu_slots")
            .expect("cpu_slots missing");
        assert_eq!(cpu_slots.value, defaults.cpu_slots.to_string());
        assert_eq!(cpu_slots.locality, Locality::Machine);

        let enter = rows
            .iter()
            .find(|r| r.name == "helm.enter")
            .expect("helm.enter missing");
        assert_eq!(enter.value, "off");
        assert_eq!(enter.locality, Locality::Machine);

        // No guild yet: nothing synced to report, and that is not a failure.
        assert!(rows.iter().all(|r| r.locality == Locality::Machine));
        assert_eq!(output.exit_code(), 0);
    }

    /// A `machine.yml` that sets one key still reports the rest at their
    /// defaults — the reader already merges, and this module must not
    /// second-guess it.
    #[test]
    fn a_customised_key_reads_back_and_the_rest_stay_at_default() {
        let (_home, place) = scratch();
        std::fs::create_dir_all(&place.armada_home).unwrap();
        std::fs::write(
            place.armada_home.join("machine.yml"),
            "manifest:\n  cpu_slots: 3\nhelm:\n  enter: true\n",
        )
        .unwrap();

        let rows = settings(&run(&place).unwrap());
        assert_eq!(
            rows.iter()
                .find(|r| r.name == "manifest.cpu_slots")
                .unwrap()
                .value,
            "3"
        );
        assert_eq!(
            rows.iter().find(|r| r.name == "helm.enter").unwrap().value,
            "on"
        );
        let defaults = armada_manifest::machine::MachineConfig::defaults();
        assert_eq!(
            rows.iter()
                .find(|r| r.name == "manifest.port_base")
                .unwrap()
                .value,
            defaults.port_base.to_string()
        );
    }

    /// **`helm.enter` never syncs, on this listing too.** It is the example
    /// `PLAN.md` §13.1 and `docs/reserved/018` both give for the line this
    /// verb exists to make visible.
    #[test]
    fn helm_enter_is_reported_as_machine_never_synced() {
        let (_home, place) = scratch();
        let rows = settings(&run(&place).unwrap());
        let enter = rows.iter().find(|r| r.name == "helm.enter").unwrap();
        assert_eq!(enter.locality, Locality::Machine);
    }

    /// The guild's settings.json is reused from the same inventory `armada
    /// guild ls` reads, not re-opened here — and it is reported as synced.
    #[test]
    fn a_guild_settings_file_is_listed_synced_and_reuses_the_inventory_count() {
        let (_home, place) = scratch();
        let guild = place.guild();
        std::fs::create_dir_all(guild.root()).unwrap();
        std::fs::write(
            guild.root().join("settings.json"),
            r#"{"env": {"FOO": "bar"}, "model": "opus"}"#,
        )
        .unwrap();

        let rows = settings(&run(&place).unwrap());
        let row = rows
            .iter()
            .find(|r| r.name == "guild.settings.json")
            .expect("settings.json not listed");
        assert_eq!(row.locality, Locality::Synced);
        assert_eq!(row.value, "2 settings");
        assert!(row.at.contains("guild/settings.json"), "{}", row.at);
    }

    /// Guild content — a skill, a workflow — is not a setting and must not
    /// appear on this listing; that is `armada guild ls`'s question.
    #[test]
    fn guild_content_is_not_a_setting() {
        let (_home, place) = scratch();
        let guild = place.guild();
        std::fs::create_dir_all(guild.root().join("skills/onboard-repo")).unwrap();
        std::fs::write(
            guild.root().join("skills/onboard-repo/SKILL.md"),
            "# onboard-repo\n",
        )
        .unwrap();

        let rows = settings(&run(&place).unwrap());
        assert!(
            !rows.iter().any(|r| r.name.contains("onboard-repo")),
            "{rows:?}"
        );
    }

    /// Every kind of guild item is either a setting or content — this test
    /// inverted would fail if a new [`Kind`] is ever added and left off both,
    /// which `is_setting`'s exhaustive match already makes impossible to
    /// compile, but this is the behavioural half of that guarantee.
    #[test]
    fn every_kind_is_settled_one_way_or_the_other() {
        for kind in [
            Kind::Memory,
            Kind::Skill,
            Kind::Subagent,
            Kind::Workflow,
            Kind::Hook,
            Kind::Settings,
            Kind::Plugins,
            Kind::Mcp,
            Kind::Permissions,
            Kind::Schema,
        ] {
            let _ = is_setting(kind);
        }
    }

    /// No path in the payload ever carries a real home directory — this
    /// repository is public permanently (`AGENTS.md` rule 2).
    #[test]
    fn no_row_carries_the_real_home_directory() {
        let (home, place) = scratch();
        std::fs::create_dir_all(&place.armada_home).unwrap();
        let guild = place.guild();
        std::fs::create_dir_all(guild.root()).unwrap();
        std::fs::write(guild.root().join("settings.json"), "{}").unwrap();

        let rows = settings(&run(&place).unwrap());
        let home_str = home.path().display().to_string();
        for row in &rows {
            assert!(!row.at.contains(&home_str), "{row:?}");
            assert!(row.at.starts_with('~') || !row.at.contains('/'), "{row:?}");
        }
    }
}
